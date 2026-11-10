use std::path::{Path, PathBuf};
use std::process::Stdio;

use openssh_sftp_client::metadata::MetaData;
use openssh_sftp_client::{Sftp, SftpOptions};
use tokio::process::{Child, Command};
use tracing::{debug, info};

use crate::errors::RsshError;

const SSH_PATH: &str = "/usr/bin/ssh";
const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const STDERR_LIMIT: usize = 4096;
const UPLOAD_CHUNK_SIZE: usize = 64 * 1024;

/// SFTP transport over an existing SSH multiplexed control socket.
///
/// Fail-closed: uses `-F /dev/null`, `BatchMode=yes`,
/// `ProxyCommand=/usr/bin/false` so a missing master never silently
/// reconnects or prompts for authentication.
pub struct SftpChannel {
    sftp: Sftp,
    child: Child,
}

impl std::fmt::Debug for SftpChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpChannel")
            .field("child_id", &self.child.id())
            .finish_non_exhaustive()
    }
}

impl SftpChannel {
    pub async fn open(host: &str, socket_path: &Path) -> Result<Self, RsshError> {
        if !socket_path.exists() {
            return Err(RsshError::Sftp(
                "control socket does not exist — master may have exited".into(),
            ));
        }

        let mut child = Command::new(SSH_PATH)
            .args([
                "-F",
                "/dev/null",
                "-S",
                &socket_path.to_string_lossy(),
                "-o",
                "ControlMaster=no",
                "-o",
                "BatchMode=yes",
                "-o",
                "ProxyCommand=/usr/bin/false",
                "-o",
                "ClearAllForwardings=yes",
                "-o",
                "RemoteCommand=none",
                "-T",
                "-s",
                "--",
                host,
                "sftp",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| RsshError::Sftp(format!("failed to spawn SFTP subsystem: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RsshError::Sftp("no stdin on SFTP child".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RsshError::Sftp("no stdout on SFTP child".into()))?;

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(drain_stderr(stderr));
        }

        let sftp = tokio::time::timeout(STARTUP_TIMEOUT, async {
            Sftp::new(stdin, stdout, SftpOptions::default()).await
        })
        .await
        .map_err(|_| RsshError::Sftp("SFTP handshake timed out".into()))?
        .map_err(|e| RsshError::Sftp(format!("SFTP handshake failed: {e}")))?;

        info!(host, "SFTP channel connected");

        Ok(Self { sftp, child })
    }

    pub async fn ensure_dir(&self, remote_dir: &Path) -> Result<(), RsshError> {
        let mut fs = self.sftp.fs();
        let mut to_create = Vec::new();
        let mut current = remote_dir.to_path_buf();

        loop {
            match fs.metadata(&current).await {
                Ok(meta) if is_dir(&meta) => break,
                Ok(_) => {
                    return Err(RsshError::Sftp(format!(
                        "remote path exists but is not a directory: {}",
                        current.display()
                    )));
                }
                Err(_) => {
                    to_create.push(current.clone());
                    match current.parent() {
                        Some(p) if p != current => current = p.to_path_buf(),
                        _ => break,
                    }
                }
            }
        }

        for dir in to_create.into_iter().rev() {
            debug!(dir = %dir.display(), "creating remote directory");
            fs.create_dir(&dir)
                .await
                .map_err(|e| RsshError::Sftp(format!("mkdir {}: {e}", dir.display())))?;
        }

        Ok(())
    }

    /// Upload bytes using staging (.part → rename).
    ///
    /// Does not advertise the file as complete until rename succeeds.
    pub async fn upload_bytes(&self, data: &[u8], remote_path: &Path) -> Result<u64, RsshError> {
        let staging = staging_name(remote_path);

        if let Some(parent) = remote_path.parent() {
            self.ensure_dir(parent).await?;
        }

        let mut remote_file = self
            .sftp
            .create(&staging)
            .await
            .map_err(|e| RsshError::Sftp(format!("create {}: {e}", staging.display())))?;

        for chunk in data.chunks(UPLOAD_CHUNK_SIZE) {
            remote_file
                .write_all(chunk)
                .await
                .map_err(|e| RsshError::Sftp(format!("write {}: {e}", staging.display())))?;
        }

        remote_file
            .close()
            .await
            .map_err(|e| RsshError::Sftp(format!("close {}: {e}", staging.display())))?;

        self.sftp
            .fs()
            .rename(&staging, remote_path)
            .await
            .map_err(|e| {
                RsshError::Sftp(format!(
                    "rename {} -> {}: {e}",
                    staging.display(),
                    remote_path.display()
                ))
            })?;

        let size = data.len() as u64;
        info!(remote = %remote_path.display(), size, "upload committed");
        Ok(size)
    }

    pub async fn close(self) -> Result<(), RsshError> {
        let Self {
            sftp, mut child, ..
        } = self;

        sftp.close()
            .await
            .map_err(|e| RsshError::Sftp(format!("SFTP close: {e}")))?;

        match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(RsshError::Sftp(format!("SFTP child wait error: {e}"))),
            Err(_) => {
                let _ = child.kill().await;
                Ok(())
            }
        }
    }
}

/// Generate a staging path: same directory, `.part` suffix.
fn staging_name(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());

    let id: u32 = {
        let mut buf = [0u8; 4];
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let _ = f.read_exact(&mut buf);
        }
        u32::from_ne_bytes(buf)
    };

    let name = format!(".rssh-{id:08x}-{file_name}.part");
    match path.parent() {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    }
}

fn is_dir(meta: &MetaData) -> bool {
    meta.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
}

async fn drain_stderr(mut stderr: tokio::process::ChildStderr) {
    use tokio::io::AsyncReadExt;

    let mut buf = Vec::with_capacity(1024);
    let mut total = 0usize;
    let mut tmp = [0u8; 512];

    loop {
        match stderr.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => {
                let remaining = STDERR_LIMIT.saturating_sub(total);
                if remaining > 0 {
                    let take = n.min(remaining);
                    buf.extend_from_slice(&tmp[..take]);
                }
                total += n;
            }
            Err(_) => break,
        }
    }

    if !buf.is_empty() {
        let text = String::from_utf8_lossy(&buf);
        tracing::debug!(bytes = total, "SFTP stderr: {}", text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn staging_name_preserves_parent() {
        let p = Path::new("/home/user/.cache/rssh/image.png");
        let s = staging_name(p);
        assert_eq!(s.parent(), Some(Path::new("/home/user/.cache/rssh")));
        let name = s.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with(".rssh-"));
        assert!(name.ends_with(".part"));
        assert!(name.contains("image.png"));
    }

    #[test]
    fn staging_name_no_parent() {
        let p = Path::new("file.txt");
        let s = staging_name(p);
        let name = s.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with(".rssh-"));
        assert!(name.ends_with(".part"));
    }

    #[test]
    fn missing_socket_is_detected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = SftpChannel::open("test-host", Path::new("/nonexistent/socket")).await;
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("control socket does not exist"));
        });
    }
}

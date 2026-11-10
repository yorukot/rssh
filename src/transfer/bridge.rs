use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

use crate::errors::RsshError;
use crate::transfer::storage::{TransactionId, UploadRecord, remote_upload_path, safe_remote_name};
use crate::transport::sftp::SftpChannel;

#[derive(Debug)]
pub struct UploadRequest {
    pub tx_id: TransactionId,
    pub data: Vec<u8>,
    pub original_name: String,
    pub source_label: PathBuf,
}

#[derive(Debug)]
pub enum UploadResult {
    Success(UploadRecord),
    Failed { tx_id: TransactionId, error: String },
}

pub struct TransferBridge {
    request_tx: std_mpsc::SyncSender<UploadRequest>,
    response_rx: std_mpsc::Receiver<UploadResult>,
    wake_read: OwnedFd,
    _thread: std::thread::JoinHandle<()>,
}

impl TransferBridge {
    pub fn start(
        host: String,
        socket_path: PathBuf,
        remote_root: String,
    ) -> Result<Self, RsshError> {
        let (request_tx, request_rx) = std_mpsc::sync_channel::<UploadRequest>(16);
        let (response_tx, response_rx) = std_mpsc::sync_channel::<UploadResult>(16);

        let (wake_read, wake_write) = pipe_fds()?;

        let thread = std::thread::Builder::new()
            .name("rssh-upload".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("failed to build tokio runtime: {e}");
                        return;
                    }
                };

                rt.block_on(async move {
                    bridge_loop(
                        host,
                        socket_path,
                        remote_root,
                        request_rx,
                        response_tx,
                        wake_write,
                    )
                    .await;
                });
            })
            .map_err(|e| RsshError::Transfer(format!("spawn upload thread: {e}")))?;

        Ok(Self {
            request_tx,
            response_rx,
            wake_read,
            _thread: thread,
        })
    }

    pub fn submit(&self, req: UploadRequest) -> Result<(), RsshError> {
        self.request_tx.try_send(req).map_err(|e| match e {
            std_mpsc::TrySendError::Full(_) => RsshError::Transfer("upload queue full".into()),
            std_mpsc::TrySendError::Disconnected(_) => {
                RsshError::Transfer("upload worker shut down".into())
            }
        })
    }

    pub fn drain_responses(&self) -> Vec<UploadResult> {
        let mut drain_buf = [0u8; 64];
        let _ = nix::unistd::read(&self.wake_read, &mut drain_buf);
        let mut results = Vec::new();
        while let Ok(r) = self.response_rx.try_recv() {
            results.push(r);
        }
        results
    }

    pub fn wake_fd(&self) -> BorrowedFd<'_> {
        self.wake_read.as_fd()
    }
}

fn pipe_fds() -> Result<(OwnedFd, OwnedFd), RsshError> {
    let (read_fd, write_fd) =
        nix::unistd::pipe().map_err(|e| RsshError::Transfer(format!("pipe: {e}")))?;

    set_nonblock(&read_fd)?;
    set_nonblock(&write_fd)?;

    Ok((read_fd, write_fd))
}

fn set_nonblock(fd: &OwnedFd) -> Result<(), RsshError> {
    let flags = nix::fcntl::fcntl(fd.as_fd(), nix::fcntl::FcntlArg::F_GETFL)
        .map_err(|e| RsshError::Transfer(format!("fcntl F_GETFL: {e}")))?;
    let mut oflags = nix::fcntl::OFlag::from_bits_truncate(flags);
    oflags |= nix::fcntl::OFlag::O_NONBLOCK;
    nix::fcntl::fcntl(fd.as_fd(), nix::fcntl::FcntlArg::F_SETFL(oflags))
        .map_err(|e| RsshError::Transfer(format!("fcntl F_SETFL: {e}")))?;
    Ok(())
}

async fn bridge_loop(
    host: String,
    socket_path: PathBuf,
    remote_root: String,
    request_rx: std_mpsc::Receiver<UploadRequest>,
    response_tx: std_mpsc::SyncSender<UploadResult>,
    wake_write: OwnedFd,
) {
    let sftp = match SftpChannel::open(&host, &socket_path).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("SFTP channel failed to open: {e}");
            while let Ok(req) = request_rx.recv() {
                let _ = response_tx.send(UploadResult::Failed {
                    tx_id: req.tx_id,
                    error: format!("SFTP unavailable: {e}"),
                });
                wake_pipe(&wake_write);
            }
            return;
        }
    };

    let remote_root_path = PathBuf::from(&remote_root);
    let _ = sftp.ensure_dir(&remote_root_path).await;

    // Use a tokio interval to poll the sync receiver, since we can't block
    // the async runtime on a sync recv.
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));

    loop {
        interval.tick().await;

        match request_rx.try_recv() {
            Ok(req) => {
                let result = do_upload(&sftp, &remote_root, req).await;
                if response_tx.send(result).is_err() {
                    break;
                }
                wake_pipe(&wake_write);
            }
            Err(std_mpsc::TryRecvError::Empty) => continue,
            Err(std_mpsc::TryRecvError::Disconnected) => break,
        }
    }

    let _ = sftp.close().await;
}

async fn do_upload(sftp: &SftpChannel, remote_root: &str, req: UploadRequest) -> UploadResult {
    let remote_name = safe_remote_name(&req.original_name, req.tx_id);
    let remote_path = remote_upload_path(remote_root, &remote_name);
    let size = req.data.len() as u64;

    match sftp.upload_bytes(&req.data, &remote_path).await {
        Ok(_) => {
            tracing::info!(
                tx = %req.tx_id,
                remote = %remote_path.display(),
                size,
                "upload committed"
            );
            UploadResult::Success(UploadRecord {
                tx_id: req.tx_id,
                source_label: req.source_label,
                remote_path,
                size,
                committed: true,
            })
        }
        Err(e) => UploadResult::Failed {
            tx_id: req.tx_id,
            error: e.to_string(),
        },
    }
}

fn wake_pipe(fd: &OwnedFd) {
    let _ = nix::unistd::write(fd, &[1u8]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_fds_are_valid() {
        let (read_fd, write_fd) = pipe_fds().unwrap();
        // Write a byte and read it back
        nix::unistd::write(&write_fd, &[42u8]).unwrap();
        let mut buf = [0u8; 1];
        nix::unistd::read(&read_fd, &mut buf).unwrap();
        assert_eq!(buf[0], 42);
    }
}

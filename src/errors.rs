use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RsshError {
    #[error("SSH process exited with status {0}")]
    SshExited(i32),

    #[error("SSH process terminated by signal")]
    SshSignaled,

    #[error("failed to spawn SSH: {0}")]
    SshSpawn(#[source] std::io::Error),

    #[error("PTY error: {0}")]
    Pty(#[source] std::io::Error),

    #[error("terminal state error: {0}")]
    Terminal(#[source] nix::Error),

    #[error("control socket path too long: {}", .0.display())]
    SocketPathTooLong(PathBuf),

    #[error("control socket directory error: {0}")]
    SocketDir(#[source] std::io::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("upload denied: {0}")]
    UploadDenied(String),

    #[error("SFTP error: {0}")]
    Sftp(String),

    #[error("transfer failed: {0}")]
    Transfer(String),

    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("no allowed roots configured for host")]
    NoAllowedRoots,
}

pub type Result<T> = std::result::Result<T, RsshError>;

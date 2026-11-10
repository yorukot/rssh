use std::path::PathBuf;

#[derive(Debug)]
pub enum SessionEvent {
    SshReady,
    SshExited(i32),
    SshSignaled,
    InputBytes(Vec<u8>),
    OutputBytes(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Starting,
    Interactive,
    ShuttingDown,
    Closed,
}

#[derive(Debug)]
pub struct SessionInfo {
    pub id: SessionId,
    pub host: String,
    pub socket_path: PathBuf,
    pub state: ConnectionState,
}

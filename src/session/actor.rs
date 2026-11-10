use crate::session::events::{ConnectionState, SessionId, SessionInfo};
use std::path::PathBuf;

pub struct SessionActor {
    info: SessionInfo,
}

impl SessionActor {
    pub fn new(host: String, socket_path: PathBuf) -> Self {
        Self {
            info: SessionInfo {
                id: SessionId::new(),
                host,
                socket_path,
                state: ConnectionState::Starting,
            },
        }
    }

    pub fn id(&self) -> SessionId {
        self.info.id
    }

    pub fn host(&self) -> &str {
        &self.info.host
    }

    pub fn socket_path(&self) -> &std::path::Path {
        &self.info.socket_path
    }

    pub fn state(&self) -> ConnectionState {
        self.info.state
    }

    pub fn set_state(&mut self, state: ConnectionState) {
        self.info.state = state;
    }
}

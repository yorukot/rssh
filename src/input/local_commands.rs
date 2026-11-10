use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCmd {
    UploadClipboard,
    ShowStatus,
    Discard,
}

impl fmt::Display for LocalCmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UploadClipboard => write!(f, "upload-clipboard"),
            Self::ShowStatus => write!(f, "show-status"),
            Self::Discard => write!(f, "discard"),
        }
    }
}

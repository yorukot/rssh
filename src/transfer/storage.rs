use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static TX_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(u64);

impl TransactionId {
    pub fn next() -> Self {
        Self(TX_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tx-{:04}", self.0)
    }
}

pub fn safe_remote_name(original: &str, tx_id: TransactionId) -> String {
    let stem = Path::new(original)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("upload");

    let ext = Path::new(original)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let sanitized_stem: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .take(200)
        .collect();

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if ext.is_empty() {
        format!("{sanitized_stem}-{ts}-{tx_id}")
    } else {
        format!("{sanitized_stem}-{ts}-{tx_id}.{ext}")
    }
}

pub fn remote_upload_path(remote_root: &str, filename: &str) -> PathBuf {
    PathBuf::from(remote_root).join(filename)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadRecord {
    pub tx_id: TransactionId,
    pub source_label: PathBuf,
    pub remote_path: PathBuf,
    pub size: u64,
    pub committed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_ids_increment() {
        let a = TransactionId::next();
        let b = TransactionId::next();
        assert!(b.as_u64() > a.as_u64());
    }

    #[test]
    fn safe_name_preserves_extension() {
        let tx = TransactionId::next();
        let name = safe_remote_name("image.png", tx);
        assert!(name.ends_with(".png"));
        assert!(name.contains(&format!("{tx}")));
    }

    #[test]
    fn safe_name_sanitizes_special_chars() {
        let tx = TransactionId::next();
        let name = safe_remote_name("my file (1).tar.gz", tx);
        assert!(!name.contains(' '));
        assert!(!name.contains('('));
    }

    #[test]
    fn safe_name_no_extension() {
        let tx = TransactionId::next();
        let name = safe_remote_name("Makefile", tx);
        assert!(!name.contains('.'));
    }

    #[test]
    fn remote_path_joins() {
        let p = remote_upload_path("/home/user/uploads", "test.png");
        assert_eq!(p, PathBuf::from("/home/user/uploads/test.png"));
    }

    #[test]
    fn display_format() {
        let tx = TransactionId::next();
        let s = format!("{tx}");
        assert!(s.starts_with("tx-"));
    }
}

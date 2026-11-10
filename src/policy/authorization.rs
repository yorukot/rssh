use std::fs::{self, File};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct UploadPolicy {
    allowed_roots: Vec<PathBuf>,
    max_file_bytes: u64,
    follow_symlinks: bool,
}

pub enum AuthorizationResult {
    Authorized(AuthorizedSource),
    Denied(DenialReason),
}

pub struct AuthorizedSource {
    pub path: PathBuf,
    pub handle: File,
    pub size: u64,
}

#[derive(Debug)]
pub enum DenialReason {
    NotFound,
    Symlink,
    NotRegularFile,
    OutsideAllowedRoots,
    TooLarge { size: u64, limit: u64 },
    NoAllowedRoots,
    PermissionDenied,
    IoError(std::io::Error),
}

impl std::fmt::Display for DenialReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "file not found"),
            Self::Symlink => write!(f, "symlinks are not allowed"),
            Self::NotRegularFile => write!(f, "not a regular file"),
            Self::OutsideAllowedRoots => write!(f, "file is outside allowed roots"),
            Self::TooLarge { size, limit } => {
                write!(f, "file too large ({size} bytes, limit {limit} bytes)")
            }
            Self::NoAllowedRoots => write!(f, "no allowed roots configured"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl UploadPolicy {
    pub fn new(allowed_roots: Vec<PathBuf>, max_file_bytes: u64, follow_symlinks: bool) -> Self {
        Self {
            allowed_roots,
            max_file_bytes,
            follow_symlinks,
        }
    }

    pub fn authorize(&self, path: &Path) -> AuthorizationResult {
        if self.allowed_roots.is_empty() {
            return AuthorizationResult::Denied(DenialReason::NoAllowedRoots);
        }

        // Check symlink before canonicalization when symlinks are forbidden
        if !self.follow_symlinks {
            match fs::symlink_metadata(path) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        return AuthorizationResult::Denied(DenialReason::Symlink);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return AuthorizationResult::Denied(DenialReason::NotFound);
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return AuthorizationResult::Denied(DenialReason::PermissionDenied);
                }
                Err(e) => {
                    return AuthorizationResult::Denied(DenialReason::IoError(e));
                }
            }
        }

        // Canonicalize to resolve ../ and get absolute path
        let canonical = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return AuthorizationResult::Denied(DenialReason::NotFound);
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return AuthorizationResult::Denied(DenialReason::PermissionDenied);
            }
            Err(e) => {
                return AuthorizationResult::Denied(DenialReason::IoError(e));
            }
        };

        // Component-aware containment check
        let inside = self.allowed_roots.iter().any(|root| {
            match fs::canonicalize(root) {
                Ok(canonical_root) => canonical.starts_with(&canonical_root),
                Err(_) => false, // root itself doesn't resolve — skip it
            }
        });
        if !inside {
            return AuthorizationResult::Denied(DenialReason::OutsideAllowedRoots);
        }

        // Open and check metadata using the canonical path
        let handle = match File::open(&canonical) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return AuthorizationResult::Denied(DenialReason::PermissionDenied);
            }
            Err(e) => {
                return AuthorizationResult::Denied(DenialReason::IoError(e));
            }
        };

        let meta = match handle.metadata() {
            Ok(m) => m,
            Err(e) => {
                return AuthorizationResult::Denied(DenialReason::IoError(e));
            }
        };

        if !meta.file_type().is_file() {
            return AuthorizationResult::Denied(DenialReason::NotRegularFile);
        }

        let size = meta.len();
        if size > self.max_file_bytes {
            return AuthorizationResult::Denied(DenialReason::TooLarge {
                size,
                limit: self.max_file_bytes,
            });
        }

        AuthorizationResult::Authorized(AuthorizedSource {
            path: canonical,
            handle,
            size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn authorize_regular_file_under_root() {
        let dir = make_temp_dir();
        let file_path = dir.path().join("image.png");
        File::create(&file_path)
            .unwrap()
            .write_all(b"data")
            .unwrap();

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&file_path) {
            AuthorizationResult::Authorized(src) => {
                assert_eq!(src.size, 4);
            }
            AuthorizationResult::Denied(reason) => panic!("should be authorized: {reason}"),
        }
    }

    #[test]
    fn deny_file_outside_root() {
        let root_dir = make_temp_dir();
        let other_dir = make_temp_dir();
        let file_path = other_dir.path().join("secret.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"secret")
            .unwrap();

        let policy = UploadPolicy::new(vec![root_dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&file_path) {
            AuthorizationResult::Denied(DenialReason::OutsideAllowedRoots) => {}
            other => panic!("expected OutsideAllowedRoots, got {}", denial_debug(&other)),
        }
    }

    #[test]
    fn deny_nonexistent_file() {
        let dir = make_temp_dir();
        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&dir.path().join("nope.txt")) {
            AuthorizationResult::Denied(DenialReason::NotFound) => {}
            other => panic!("expected NotFound, got {}", denial_debug(&other)),
        }
    }

    #[test]
    fn deny_directory() {
        let dir = make_temp_dir();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&sub) {
            AuthorizationResult::Denied(DenialReason::NotRegularFile) => {}
            other => panic!("expected NotRegularFile, got {}", denial_debug(&other)),
        }
    }

    #[test]
    fn deny_oversized_file() {
        let dir = make_temp_dir();
        let file_path = dir.path().join("big.bin");
        File::create(&file_path)
            .unwrap()
            .write_all(&[0u8; 2048])
            .unwrap();

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024, false);
        match policy.authorize(&file_path) {
            AuthorizationResult::Denied(DenialReason::TooLarge { size, limit }) => {
                assert_eq!(size, 2048);
                assert_eq!(limit, 1024);
            }
            other => panic!("expected TooLarge, got {}", denial_debug(&other)),
        }
    }

    #[cfg(unix)]
    #[test]
    fn deny_symlink_when_follow_disabled() {
        let dir = make_temp_dir();
        let real = dir.path().join("real.txt");
        File::create(&real).unwrap().write_all(b"data").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&link) {
            AuthorizationResult::Denied(DenialReason::Symlink) => {}
            other => panic!("expected Symlink, got {}", denial_debug(&other)),
        }
    }

    #[cfg(unix)]
    #[test]
    fn allow_symlink_when_follow_enabled() {
        let dir = make_temp_dir();
        let real = dir.path().join("real.txt");
        File::create(&real).unwrap().write_all(b"data").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, true);
        match policy.authorize(&link) {
            AuthorizationResult::Authorized(src) => {
                assert_eq!(src.size, 4);
            }
            AuthorizationResult::Denied(reason) => {
                panic!("should be authorized with follow_symlinks: {reason}")
            }
        }
    }

    #[test]
    fn deny_path_traversal() {
        let dir = make_temp_dir();
        let outside = make_temp_dir();
        let secret = outside.path().join("secret.txt");
        File::create(&secret).unwrap().write_all(b"x").unwrap();

        // Construct a traversal path: <root>/../<outside>/secret.txt
        let traversal = dir
            .path()
            .join("..")
            .join(outside.path().file_name().unwrap())
            .join("secret.txt");

        let policy = UploadPolicy::new(vec![dir.path().to_owned()], 1024 * 1024, false);
        match policy.authorize(&traversal) {
            AuthorizationResult::Denied(DenialReason::OutsideAllowedRoots) => {}
            AuthorizationResult::Denied(DenialReason::NotFound) => {
                // Also acceptable: temp dir names might not form a valid traversal
            }
            other => panic!(
                "expected OutsideAllowedRoots or NotFound, got {}",
                denial_debug(&other)
            ),
        }
    }

    #[test]
    fn deny_no_allowed_roots() {
        let dir = make_temp_dir();
        let file_path = dir.path().join("f.txt");
        File::create(&file_path).unwrap().write_all(b"x").unwrap();

        let policy = UploadPolicy::new(vec![], 1024 * 1024, false);
        match policy.authorize(&file_path) {
            AuthorizationResult::Denied(DenialReason::NoAllowedRoots) => {}
            other => panic!("expected NoAllowedRoots, got {}", denial_debug(&other)),
        }
    }

    #[test]
    fn multiple_roots_any_match() {
        let root_a = make_temp_dir();
        let root_b = make_temp_dir();
        let file_path = root_b.path().join("ok.txt");
        File::create(&file_path).unwrap().write_all(b"ok").unwrap();

        let policy = UploadPolicy::new(
            vec![root_a.path().to_owned(), root_b.path().to_owned()],
            1024 * 1024,
            false,
        );
        match policy.authorize(&file_path) {
            AuthorizationResult::Authorized(_) => {}
            AuthorizationResult::Denied(reason) => panic!("should be authorized: {reason}"),
        }
    }

    fn denial_debug(result: &AuthorizationResult) -> String {
        match result {
            AuthorizationResult::Authorized(_) => "Authorized".to_string(),
            AuthorizationResult::Denied(r) => format!("Denied({r})"),
        }
    }
}

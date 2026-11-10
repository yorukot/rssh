/// Resource limits for uploads and input buffering.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_file_bytes: u64,
    pub max_batch_bytes: u64,
    pub max_files_per_batch: u32,
    pub max_pending_input_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024 * 1024,
            max_batch_bytes: 256 * 1024 * 1024,
            max_files_per_batch: 8,
            max_pending_input_bytes: 256 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_file_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_batch_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_files_per_batch, 8);
        assert_eq!(limits.max_pending_input_bytes, 256 * 1024);
    }
}

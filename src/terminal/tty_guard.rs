use nix::sys::termios::{self, InputFlags, LocalFlags, OutputFlags, SetArg, Termios};
use std::os::fd::{AsFd, BorrowedFd};

use crate::errors::{Result, RsshError};

pub struct TtyGuard {
    fd: std::os::fd::OwnedFd,
    original: Termios,
    is_raw: bool,
}

impl TtyGuard {
    pub fn new() -> Result<Self> {
        use std::os::fd::FromRawFd;
        let stdin_fd = std::io::stdin();
        let fd = nix::fcntl::fcntl(stdin_fd.as_fd(), nix::fcntl::FcntlArg::F_DUPFD_CLOEXEC(0))
            .map_err(RsshError::Terminal)?;
        let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        let original = termios::tcgetattr(&fd).map_err(RsshError::Terminal)?;
        Ok(Self {
            fd,
            original,
            is_raw: false,
        })
    }

    pub fn enter_raw(&mut self) -> Result<()> {
        if self.is_raw {
            return Ok(());
        }
        let mut raw = self.original.clone();
        termios::cfmakeraw(&mut raw);
        // Keep ISIG off — Ctrl+C goes to remote, not to us
        raw.local_flags.remove(LocalFlags::ISIG);
        raw.local_flags.remove(LocalFlags::ECHO);
        raw.local_flags.remove(LocalFlags::ICANON);
        raw.local_flags.remove(LocalFlags::IEXTEN);
        raw.input_flags.remove(InputFlags::IXON);
        raw.input_flags.remove(InputFlags::IXOFF);
        raw.input_flags.remove(InputFlags::ICRNL);
        raw.output_flags.remove(OutputFlags::OPOST);
        termios::tcsetattr(&self.fd, SetArg::TCSAFLUSH, &raw).map_err(RsshError::Terminal)?;
        self.is_raw = true;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        if !self.is_raw {
            return Ok(());
        }
        termios::tcsetattr(&self.fd, SetArg::TCSAFLUSH, &self.original)
            .map_err(RsshError::Terminal)?;
        self.is_raw = false;
        Ok(())
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }

    pub fn is_raw(&self) -> bool {
        self.is_raw
    }
}

impl Drop for TtyGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_creation_does_not_panic() {
        // This test may fail in CI without a TTY; that's expected
        if let Ok(guard) = TtyGuard::new() {
            assert!(!guard.is_raw());
        }
    }
}

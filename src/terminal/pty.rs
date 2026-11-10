use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd};

use crate::errors::{Result, RsshError};

pub struct PtyPair {
    pub master: OwnedFd,
    slave_path: std::ffi::CString,
}

impl PtyPair {
    pub fn open() -> Result<Self> {
        let master_raw = nix::pty::posix_openpt(
            nix::fcntl::OFlag::O_RDWR | nix::fcntl::OFlag::O_NOCTTY | nix::fcntl::OFlag::O_CLOEXEC,
        )
        .map_err(|e| RsshError::Pty(std::io::Error::from(e)))?;

        nix::pty::grantpt(&master_raw).map_err(|e| RsshError::Pty(std::io::Error::from(e)))?;
        nix::pty::unlockpt(&master_raw).map_err(|e| RsshError::Pty(std::io::Error::from(e)))?;

        let slave_name = unsafe { nix::pty::ptsname(&master_raw) }
            .map_err(|e| RsshError::Pty(std::io::Error::from(e)))?;

        let slave_path = std::ffi::CString::new(slave_name)
            .map_err(|e| RsshError::Pty(std::io::Error::other(e)))?;

        let master = unsafe { OwnedFd::from_raw_fd(master_raw.into_raw_fd()) };

        Ok(Self { master, slave_path })
    }

    pub fn open_slave(&self) -> Result<OwnedFd> {
        let fd = nix::fcntl::open(
            self.slave_path.as_c_str(),
            nix::fcntl::OFlag::O_RDWR | nix::fcntl::OFlag::O_NOCTTY | nix::fcntl::OFlag::O_CLOEXEC,
            nix::sys::stat::Mode::empty(),
        )
        .map_err(|e| RsshError::Pty(std::io::Error::from(e)))?;
        Ok(fd)
    }

    pub fn master_fd(&self) -> BorrowedFd<'_> {
        self.master.as_fd()
    }

    pub fn set_size(&self, cols: u16, rows: u16) -> Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
        if ret < 0 {
            return Err(RsshError::Pty(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

pub fn get_terminal_size() -> Result<(u16, u16)> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
    if ret < 0 {
        return Err(RsshError::Pty(std::io::Error::last_os_error()));
    }
    Ok((ws.ws_col, ws.ws_row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_pair_opens_successfully() {
        let pair = PtyPair::open().unwrap();
        assert!(pair.master.as_raw_fd() >= 0);
    }

    #[test]
    fn pty_slave_opens() {
        let pair = PtyPair::open().unwrap();
        let slave = pair.open_slave().unwrap();
        assert!(slave.as_raw_fd() >= 0);
        assert_ne!(pair.master.as_raw_fd(), slave.as_raw_fd());
    }

    #[test]
    fn pty_set_size() {
        let pair = PtyPair::open().unwrap();
        // Open the slave first — TIOCSWINSZ on the master requires an active slave on some systems
        let _slave = pair.open_slave().unwrap();
        pair.set_size(80, 24).unwrap();
    }
}

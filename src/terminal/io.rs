use std::os::fd::BorrowedFd;

pub fn read_bytes(fd: BorrowedFd<'_>, buf: &mut [u8]) -> std::io::Result<usize> {
    nix::unistd::read(fd, buf).map_err(std::io::Error::from)
}

pub fn write_all(fd: BorrowedFd<'_>, mut data: &[u8]) -> std::io::Result<()> {
    while !data.is_empty() {
        match nix::unistd::write(fd, data) {
            Ok(n) => data = &data[n..],
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(std::io::Error::from(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsFd;

    #[test]
    fn write_all_to_pipe() {
        let (r, w) = nix::unistd::pipe().unwrap();

        let data = b"hello rssh";
        write_all(w.as_fd(), data).unwrap();
        drop(w);

        let mut buf = [0u8; 64];
        let n = read_bytes(r.as_fd(), &mut buf).unwrap();
        assert_eq!(&buf[..n], data);
    }
}

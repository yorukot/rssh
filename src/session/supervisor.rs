use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::clipboard;
use crate::config::Config;
use crate::errors::{Result, RsshError};
use crate::input::decoder::{Action, InputDecoder};
use crate::input::local_commands::LocalCmd;
use crate::policy::limits::ResourceLimits;
use crate::session::actor::SessionActor;
use crate::session::events::ConnectionState;
use crate::terminal::io;
use crate::terminal::pty::{self, PtyPair};
use crate::terminal::tty_guard::TtyGuard;
use crate::transfer::bridge::{TransferBridge, UploadRequest, UploadResult};
use crate::transfer::storage::{TransactionId, UploadRecord};

const SSH_PATH: &str = "/usr/bin/ssh";
const SOCKET_DIR_PREFIX: &str = "rssh";
const DEFAULT_REMOTE_ROOT: &str = ".cache/rssh";
const BP_START: &[u8] = b"\x1b[200~";
const BP_END: &[u8] = b"\x1b[201~";

fn dirs_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(dir).join("rssh/config.toml")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config/rssh/config.toml")
    } else {
        PathBuf::from("/dev/null")
    }
}

static SIGWINCH_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn sigwinch_handler(_sig: libc::c_int) {
    SIGWINCH_RECEIVED.store(true, Ordering::Relaxed);
}

pub struct Supervisor {
    actor: SessionActor,
    pty: PtyPair,
    tty_guard: TtyGuard,
    child_pid: Option<nix::unistd::Pid>,
    decoder: InputDecoder,
    remote_root: String,
    max_file_bytes: u64,
    completed_uploads: Vec<UploadRecord>,
    pending_count: usize,
    last_error: Option<String>,
    bridge: Option<TransferBridge>,
    notification_shown_at: Option<Instant>,
    notification_start_col: u16,
    notification_width: u16,
    notification_rows: u16,
}

fn create_socket_path() -> Result<PathBuf> {
    let uid = nix::unistd::getuid();
    let dir = std::env::temp_dir().join(format!("{}-{}", SOCKET_DIR_PREFIX, uid));

    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(RsshError::SocketDir)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&dir, perms).map_err(RsshError::SocketDir)?;
    }

    let id: u64 = {
        let mut buf = [0u8; 8];
        use std::io::Read;
        let mut f = std::fs::File::open("/dev/urandom").map_err(RsshError::SocketDir)?;
        f.read_exact(&mut buf).map_err(RsshError::SocketDir)?;
        u64::from_ne_bytes(buf)
    };

    let path = dir.join(format!("ctl-{:016x}", id));

    if path.as_os_str().len() > 100 {
        return Err(RsshError::SocketPathTooLong(path));
    }

    Ok(path)
}

fn build_ssh_args(host: &str, socket_path: &Path) -> Vec<String> {
    vec![
        "-M".into(),
        "-S".into(),
        socket_path.to_string_lossy().into_owned(),
        "-o".into(),
        "ControlPersist=no".into(),
        "-tt".into(),
        "--".into(),
        host.into(),
    ]
}

fn install_sigwinch_handler() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigwinch_handler as *const () as usize;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGWINCH, &sa, std::ptr::null_mut());
    }
}

const NOTIFICATION_TIMEOUT_MS: u64 = 3000;
fn char_display_width(c: char) -> usize {
    if ('\u{1100}'..='\u{115F}').contains(&c)
        || ('\u{2E80}'..='\u{A4CF}').contains(&c)
        || ('\u{AC00}'..='\u{D7AF}').contains(&c)
        || ('\u{F900}'..='\u{FAFF}').contains(&c)
        || ('\u{FE10}'..='\u{FE6F}').contains(&c)
        || ('\u{FF01}'..='\u{FF60}').contains(&c)
        || ('\u{FFE0}'..='\u{FFE6}').contains(&c)
        || c > '\u{1FFFF}'
    {
        2
    } else {
        1
    }
}

/// Check if a byte sequence looks like a local file path to upload.
fn looks_like_local_file(content: &[u8]) -> Option<PathBuf> {
    let text = std::str::from_utf8(content).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Handle paths that might be quoted or escaped by the terminal
    let path_str = trimmed
        .trim_matches('\'')
        .trim_matches('"')
        .replace("\\ ", " ");

    let path = PathBuf::from(&path_str);

    // Must be absolute path
    if !path.is_absolute() {
        return None;
    }

    // Must exist and be a regular file
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => Some(path),
        _ => None,
    }
}

/// Get a short display name for a path.
fn file_display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into())
}

impl Supervisor {
    pub fn new(host: &str) -> Result<Self> {
        let socket_path = create_socket_path()?;
        let actor = SessionActor::new(host.to_owned(), socket_path);
        let pty = PtyPair::open()?;
        let tty_guard = TtyGuard::new()?;

        let config_path = dirs_path();
        let config = Config::load(&config_path).unwrap_or_default();
        let host_config = config.host(host);

        let remote_root = host_config
            .and_then(|h| h.remote_root.as_ref())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| DEFAULT_REMOTE_ROOT.to_string());

        let limits = ResourceLimits::default();

        Ok(Self {
            actor,
            pty,
            tty_guard,
            child_pid: None,
            decoder: InputDecoder::new(),
            remote_root,
            max_file_bytes: limits.max_file_bytes,
            completed_uploads: Vec::new(),
            pending_count: 0,
            last_error: None,
            bridge: None,
            notification_shown_at: None,
            notification_start_col: 0,
            notification_width: 0,
            notification_rows: 0,
        })
    }

    pub fn socket_path(&self) -> &Path {
        self.actor.socket_path()
    }

    pub fn host(&self) -> &str {
        self.actor.host()
    }

    pub fn run(&mut self) -> Result<i32> {
        install_sigwinch_handler();

        let slave = self.pty.open_slave()?;
        if let Ok((cols, rows)) = pty::get_terminal_size() {
            let _ = self.pty.set_size(cols, rows);
        }
        let args = build_ssh_args(self.actor.host(), self.actor.socket_path());

        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Child) => {
                let _ = nix::unistd::setsid();

                unsafe {
                    libc::ioctl(slave.as_raw_fd(), libc::TIOCSCTTY as _, 0);
                }

                unsafe {
                    libc::dup2(slave.as_raw_fd(), 0);
                    libc::dup2(slave.as_raw_fd(), 1);
                    libc::dup2(slave.as_raw_fd(), 2);
                }

                if slave.as_raw_fd() > 2 {
                    drop(slave);
                }

                let c_ssh = std::ffi::CString::new(SSH_PATH).unwrap();
                let mut c_args: Vec<std::ffi::CString> = vec![c_ssh.clone()];
                for arg in &args {
                    c_args.push(std::ffi::CString::new(arg.as_str()).unwrap());
                }

                let _ = nix::unistd::execv(&c_ssh, &c_args);
                std::process::exit(127);
            }
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                drop(slave);
                self.child_pid = Some(child);
                self.actor.set_state(ConnectionState::Interactive);
            }
            Err(e) => {
                return Err(RsshError::SshSpawn(std::io::Error::from(e)));
            }
        }

        self.tty_guard.enter_raw()?;

        let exit_code = self.io_loop();

        let _ = self.tty_guard.restore();
        self.actor.set_state(ConnectionState::Closed);
        self.cleanup();

        exit_code
    }

    fn show_notification(&mut self, fd: BorrowedFd<'_>, msg: &str) {
        self.clear_notification(fd);

        let (cols, rows) = pty::get_terminal_size().unwrap_or((80, 24));
        let label = " rssh ";
        let content_width: usize = msg.chars().map(char_display_width).sum();
        let box_width = (content_width + label.len() + 4).min(cols as usize);
        let start_col = (cols as usize).saturating_sub(box_width) + 1;
        let border = "─".repeat(box_width.saturating_sub(2));

        // Draw in top-right, rows 1-3
        let esc = format!(
            "\x1b7\
             \x1b[1;{c}H╭{bdr}╮\
             \x1b[2;{c}H│\x1b[1;35m{label}\x1b[0m {msg}\x1b[2;{c2}H│\
             \x1b[3;{c}H╰{bdr}╯\
             \x1b8",
            c = start_col,
            c2 = start_col + box_width - 1,
            bdr = border,
            label = label,
            msg = msg,
        );
        let _ = io::write_all(fd, esc.as_bytes());

        self.notification_shown_at = Some(Instant::now());
        self.notification_start_col = start_col as u16;
        self.notification_width = box_width as u16;
        self.notification_rows = rows;
    }

    fn clear_notification(&mut self, fd: BorrowedFd<'_>) {
        if self.notification_shown_at.is_none() {
            return;
        }

        let start_col = self.notification_start_col as usize;
        let w = self.notification_width as usize;
        let blanks = " ".repeat(w);

        // Erase the notification box with blanks
        let esc = format!(
            "\x1b7\
             \x1b[1;{c}H{b}\
             \x1b[2;{c}H{b}\
             \x1b[3;{c}H{b}\
             \x1b8",
            c = start_col,
            b = blanks,
        );
        let _ = io::write_all(fd, esc.as_bytes());
        self.notification_shown_at = None;

        // Trigger repaint from remote app (works for full-screen TUIs)
        if let Some(pid) = self.child_pid {
            let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGWINCH);
        }
    }

    fn notification_remaining_ms(&self) -> Option<u64> {
        self.notification_shown_at.map(|t| {
            let elapsed = t.elapsed().as_millis() as u64;
            NOTIFICATION_TIMEOUT_MS.saturating_sub(elapsed)
        })
    }

    fn ensure_bridge(&mut self, stdout_fd: BorrowedFd<'_>) -> bool {
        if self.bridge.is_some() {
            return true;
        }
        match TransferBridge::start(
            self.actor.host().to_owned(),
            self.actor.socket_path().to_owned(),
            self.remote_root.clone(),
        ) {
            Ok(b) => {
                self.bridge = Some(b);
                true
            }
            Err(e) => {
                let msg = format!("SFTP failed: {e}");
                self.last_error = Some(msg.clone());
                self.show_notification(stdout_fd, &msg);
                false
            }
        }
    }

    fn propagate_resize(&self) {
        if let Ok((cols, rows)) = pty::get_terminal_size() {
            let _ = self.pty.set_size(cols, rows);
        }
    }

    fn process_upload_responses(&mut self, stdout_fd: BorrowedFd<'_>, master_raw_fd: i32) {
        let Some(bridge) = &self.bridge else { return };
        for result in bridge.drain_responses() {
            self.pending_count = self.pending_count.saturating_sub(1);
            match result {
                UploadResult::Success(record) => {
                    // Auto-insert remote path into terminal
                    let path_str = record.remote_path.to_string_lossy().into_owned();
                    let fd = unsafe { BorrowedFd::borrow_raw(master_raw_fd) };
                    let _ = io::write_all(fd, path_str.as_bytes());
                    self.show_notification(
                        stdout_fd,
                        &format!("uploaded → {path_str}"),
                    );
                    self.completed_uploads.push(record);
                }
                UploadResult::Failed { tx_id, error } => {
                    let msg = format!("upload {tx_id} failed: {error}");
                    self.last_error = Some(msg.clone());
                    self.show_notification(stdout_fd, &msg);
                }
            }
        }
    }

    fn submit_upload(
        &mut self,
        data: Vec<u8>,
        original_name: &str,
        source_label: &str,
        stdout_fd: BorrowedFd<'_>,
    ) {
        if !self.ensure_bridge(stdout_fd) {
            return;
        }

        if data.len() as u64 > self.max_file_bytes {
            self.show_notification(
                stdout_fd,
                &format!(
                    "file too large ({} bytes, max {})",
                    data.len(),
                    self.max_file_bytes
                ),
            );
            return;
        }

        let tx_id = TransactionId::next();
        let req = UploadRequest {
            tx_id,
            data,
            original_name: original_name.to_owned(),
            source_label: PathBuf::from(source_label),
        };

        match self.bridge.as_ref().unwrap().submit(req) {
            Ok(()) => {
                self.pending_count += 1;
                self.show_notification(stdout_fd, &format!("uploading {original_name}..."));
            }
            Err(e) => {
                let msg = format!("submit failed: {e}");
                self.last_error = Some(msg.clone());
                self.show_notification(stdout_fd, &msg);
            }
        }
    }

    fn handle_local_command(&mut self, cmd: LocalCmd, stdout_fd: BorrowedFd<'_>) {
        match cmd {
            LocalCmd::UploadClipboard => {
                self.handle_clipboard_upload(stdout_fd);
            }
            LocalCmd::ShowStatus => {
                let done = self.completed_uploads.len();
                let pending = self.pending_count;
                if done == 0 && pending == 0 {
                    self.show_notification(stdout_fd, "no uploads | Ctrl+] v: clipboard | drag file to upload");
                } else {
                    let mut msg = format!("{done} done, {pending} pending");
                    if let Some(last) = self.completed_uploads.last() {
                        msg.push_str(&format!(" | last: {}", last.remote_path.display()));
                    }
                    self.show_notification(stdout_fd, &msg);
                }
            }
            LocalCmd::Discard => {
                if self.completed_uploads.pop().is_some() {
                    self.show_notification(stdout_fd, "last upload discarded");
                } else {
                    self.show_notification(stdout_fd, "nothing to discard");
                }
            }
        }
    }

    fn handle_clipboard_upload(&mut self, stdout_fd: BorrowedFd<'_>) {
        self.show_notification(stdout_fd, "reading clipboard...");

        let content = match clipboard::read_clipboard() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("clipboard error: {e}");
                self.last_error = Some(msg.clone());
                self.show_notification(stdout_fd, &msg);
                return;
            }
        };

        match content {
            clipboard::ClipboardContent::Empty => {
                self.show_notification(stdout_fd, "clipboard is empty");
            }
            clipboard::ClipboardContent::Text(text) => {
                // Check if clipboard text is a file path
                if let Some(path) = looks_like_local_file(text.as_bytes()) {
                    self.upload_local_file(&path, stdout_fd);
                } else {
                    let data = text.into_bytes();
                    self.submit_upload(data, "clipboard.txt", "<clipboard:text>", stdout_fd);
                }
            }
            clipboard::ClipboardContent::Image(png_data) => {
                self.submit_upload(png_data, "clipboard.png", "<clipboard:image>", stdout_fd);
            }
        }
    }

    fn handle_bracketed_paste(
        &mut self,
        content: Vec<u8>,
        stdout_fd: BorrowedFd<'_>,
        master_raw_fd: i32,
    ) {
        // Check if the pasted content is a local file path
        if let Some(path) = looks_like_local_file(&content) {
            self.show_notification(stdout_fd, &format!("detected file: {}", file_display_name(&path)));
            self.upload_local_file(&path, stdout_fd);
        } else {
            // Not a file path — forward as normal bracketed paste
            let fd = unsafe { BorrowedFd::borrow_raw(master_raw_fd) };
            let _ = io::write_all(fd, BP_START);
            let _ = io::write_all(fd, &content);
            let _ = io::write_all(fd, BP_END);
        }
    }

    fn upload_local_file(&mut self, path: &Path, stdout_fd: BorrowedFd<'_>) {
        let name = file_display_name(path);
        match std::fs::read(path) {
            Ok(data) => {
                let label = path.to_string_lossy().into_owned();
                self.submit_upload(data, &name, &label, stdout_fd);
            }
            Err(e) => {
                let msg = format!("read {name}: {e}");
                self.last_error = Some(msg.clone());
                self.show_notification(stdout_fd, &msg);
            }
        }
    }

    fn io_loop(&mut self) -> Result<i32> {
        use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

        let stdin_raw = std::io::stdin();
        let stdout_raw = std::io::stdout();
        let master_raw_fd = self.pty.master_fd().as_raw_fd();

        let mut buf = [0u8; 8192];

        loop {
            if SIGWINCH_RECEIVED.swap(false, Ordering::Relaxed) {
                self.propagate_resize();
            }

            // Dismiss expired notification
            if let Some(remaining) = self.notification_remaining_ms() {
                if remaining == 0 {
                    self.clear_notification(stdout_raw.as_fd());
                }
            }

            let timeout = match self.notification_remaining_ms() {
                Some(ms) if ms > 0 => PollTimeout::from(ms as u16),
                _ => PollTimeout::NONE,
            };

            let master_fd = self.pty.master_fd();
            let bridge_wake_fd = self.bridge.as_ref().map(|b| b.wake_fd());

            let (poll_result, stdin_revents, master_revents, wake_revents) =
                if let Some(wake_fd) = bridge_wake_fd {
                    let mut fds = [
                        PollFd::new(stdin_raw.as_fd(), PollFlags::POLLIN),
                        PollFd::new(master_fd, PollFlags::POLLIN),
                        PollFd::new(wake_fd, PollFlags::POLLIN),
                    ];
                    let r = poll(&mut fds, timeout);
                    (r, fds[0].revents(), fds[1].revents(), fds[2].revents())
                } else {
                    let mut fds = [
                        PollFd::new(stdin_raw.as_fd(), PollFlags::POLLIN),
                        PollFd::new(master_fd, PollFlags::POLLIN),
                    ];
                    let r = poll(&mut fds, timeout);
                    (r, fds[0].revents(), fds[1].revents(), None)
                };

            match poll_result {
                Ok(_) => {}
                Err(nix::errno::Errno::EINTR) => {
                    if SIGWINCH_RECEIVED.swap(false, Ordering::Relaxed) {
                        self.propagate_resize();
                    }
                    continue;
                }
                Err(e) => return Err(RsshError::Pty(std::io::Error::from(e))),
            }

            // Upload responses
            if let Some(revents) = wake_revents {
                if revents.contains(PollFlags::POLLIN) {
                    self.process_upload_responses(stdout_raw.as_fd(), master_raw_fd);
                }
            }

            // Remote output → stdout
            if let Some(revents) = master_revents {
                if revents.contains(PollFlags::POLLIN) {
                    let master_fd = self.pty.master_fd();
                    match io::read_bytes(master_fd, &mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            io::write_all(stdout_raw.as_fd(), &buf[..n]).map_err(RsshError::Io)?;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            if e.raw_os_error() == Some(libc::EIO) {
                                break;
                            }
                            return Err(RsshError::Pty(e));
                        }
                    }
                }
                if revents.contains(PollFlags::POLLHUP) {
                    break;
                }
            }

            // User input → decoder → PTY / upload / local command
            if let Some(revents) = stdin_revents {
                if revents.contains(PollFlags::POLLIN) {
                    match io::read_bytes(stdin_raw.as_fd(), &mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let actions = self.decoder.feed_bytes(&buf[..n]);
                            for action in actions {
                                match action {
                                    Action::Forward(data) => {
                                        let fd =
                                            unsafe { BorrowedFd::borrow_raw(master_raw_fd) };
                                        io::write_all(fd, &data).map_err(RsshError::Pty)?;
                                    }
                                    Action::LocalCommand(cmd) => {
                                        self.handle_local_command(cmd, stdout_raw.as_fd());
                                    }
                                    Action::BracketedPaste(content) => {
                                        self.handle_bracketed_paste(
                                            content,
                                            stdout_raw.as_fd(),
                                            master_raw_fd,
                                        );
                                    }
                                    Action::Pending => {}
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => return Err(RsshError::Pty(e)),
                    }
                }
            }

            // Non-blocking child check
            if let Some(pid) = self.child_pid {
                match nix::sys::wait::waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                    Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => return Ok(code),
                    Ok(nix::sys::wait::WaitStatus::Signaled(_, sig, _)) => {
                        return Ok(128 + sig as i32);
                    }
                    Ok(nix::sys::wait::WaitStatus::StillAlive) => {}
                    Ok(_) => {}
                    Err(nix::errno::Errno::ECHILD) => break,
                    Err(_) => {}
                }
            }
        }

        if let Some(Action::Forward(data)) = self.decoder.flush() {
            let fd = unsafe { BorrowedFd::borrow_raw(master_raw_fd) };
            let _ = io::write_all(fd, &data);
        }

        if let Some(pid) = self.child_pid {
            match nix::sys::wait::waitpid(pid, None) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => Ok(code),
                Ok(nix::sys::wait::WaitStatus::Signaled(_, sig, _)) => Ok(128 + sig as i32),
                _ => Ok(1),
            }
        } else {
            Ok(0)
        }
    }

    fn cleanup(&self) {
        let socket = self.actor.socket_path();
        if socket.exists() {
            let _ = std::fs::remove_file(socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_is_short_enough() {
        let path = create_socket_path().unwrap();
        assert!(path.as_os_str().len() <= 100);
    }

    #[test]
    fn socket_dir_has_restrictive_permissions() {
        let path = create_socket_path().unwrap();
        let dir = path.parent().unwrap();
        let meta = std::fs::metadata(dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(meta.permissions().mode() & 0o777, 0o700);
        }
    }

    #[test]
    fn ssh_args_are_correct() {
        let args = build_ssh_args("myserver", Path::new("/tmp/rssh-501/ctl-abc"));
        assert_eq!(args[0], "-M");
        assert_eq!(args[1], "-S");
        assert_eq!(args[2], "/tmp/rssh-501/ctl-abc");
        assert_eq!(args[3], "-o");
        assert_eq!(args[4], "ControlPersist=no");
        assert_eq!(args[5], "-tt");
        assert_eq!(args[6], "--");
        assert_eq!(args[7], "myserver");
    }

    #[test]
    fn unique_socket_paths() {
        let p1 = create_socket_path().unwrap();
        let p2 = create_socket_path().unwrap();
        assert_ne!(p1, p2);
    }

    #[test]
    fn detect_file_path() {
        // /dev/null always exists
        let result = looks_like_local_file(b"/dev/null");
        assert!(result.is_none()); // /dev/null is not a regular file

        let result = looks_like_local_file(b"/nonexistent/path");
        assert!(result.is_none());

        let result = looks_like_local_file(b"relative/path");
        assert!(result.is_none());
    }

    #[test]
    fn detect_existing_file() {
        // Cargo.toml exists and is a regular file, but not absolute
        let result = looks_like_local_file(b"Cargo.toml");
        assert!(result.is_none());

        // This file definitely exists
        let path = std::env::current_exe().unwrap();
        let result = looks_like_local_file(path.to_string_lossy().as_bytes());
        assert!(result.is_some());
    }
}

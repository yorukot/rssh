# rssh

A session-scoped SSH wrapper that uploads local files to a remote server via SFTP. Built for macOS + Ghostty.

Drag a file into the terminal or paste from clipboard — rssh uploads it and inserts the remote path at your cursor.

## Install

```
cargo install --path .
```

## Usage

```
rssh <host>
```

Connect to a server just like `ssh`. Inside the session:

- **Drag and drop** a file into the terminal — auto-uploads and inserts the remote path
- **Ctrl+] v** — upload clipboard content (text, image, or file path)
- **Ctrl+] s** — show upload status
- **Ctrl+] d** — discard last upload record

Files are uploaded to `~/.cache/rssh/` on the remote server.

## How it works

rssh wraps your SSH session in a local PTY, detects file drops via bracketed paste, and transfers files over SFTP using a multiplexed control socket. Ordinary terminal I/O passes through unmodified.

## Config

Optional config at `~/.config/rssh/config.toml`:

```toml
[defaults]
remote_root = ".cache/rssh"

[hosts.myserver]
remote_root = "uploads"
```

## Requirements

- macOS with system OpenSSH (`/usr/bin/ssh`)
- A terminal that supports bracketed paste (Ghostty, iTerm2, etc.)

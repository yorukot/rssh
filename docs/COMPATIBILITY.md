# Compatibility

## Development Environment

| Component | Version | Architecture |
|---|---|---|
| macOS | 27.2 (Build 26B123) | Apple Silicon (arm64) |
| Ghostty | 1.4.1-main+6ba144158 | aarch64, Metal renderer |
| OpenSSH | 10.5p1, LibreSSL 3.3.6 | — |
| Rust | 1.96.0 stable-aarch64-apple-darwin | aarch64 |

## Test Matrix

| Platform | Terminal | SSH | Shell/TUI | Profile | Procedure | Result | Evidence |
|---|---|---|---|---|---|---|---|
| macOS 27.2 arm64 | Ghostty 1.4.1 | OpenSSH 10.5p1 | zsh | literal | — | Not run | — |
| macOS 27.2 arm64 | Ghostty 1.4.1 | OpenSSH 10.5p1 | Claude Code | claude | — | Not run | — |
| macOS 27.2 arm64 | Ghostty 1.4.1 | OpenSSH 10.5p1 | Codex | codex | — | Not run | — |
| macOS 27.2 arm64 | Ghostty 1.4.1 | OpenSSH 10.5p1 | bash | shell-posix | — | Not run | — |

Allowed results:
- Passed
- Failed
- Blocked
- Not run
- Unsupported

A visible path is not evidence of image consumption.

Do not populate version rows from memory or assumptions.

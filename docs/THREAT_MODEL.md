# Threat Model

See IMPLEMENTATION_PLAN.md §2 for non-negotiable invariants and §6.6 for authorization details.

## Assets

- **SSH credentials**: Private keys, agent sockets, known_hosts — never read, copied, or weakened by the wrapper.
- **Local files**: Only accessed after explicit authorization under configured allowed roots.
- **Clipboard content**: Read only on explicit user action, never continuously monitored.
- **Terminal session**: Interactive SSH session integrity — input/output byte-preservation, no injected control characters.
- **Remote uploaded files**: Stored under restrictive permissions in tool-owned directories.

## Trust Boundaries

### Local user ↔ Wrapper
The wrapper runs as the local user. It must not silently expand the user's intended authorization scope.

### Wrapper ↔ System OpenSSH
The wrapper delegates all authentication, host-key verification, and transport security to system OpenSSH. It must not weaken these.

### Wrapper ↔ Remote server
The remote server is partially trusted (user has credentials). SFTP operations use an absolute configured root. Remote output is untrusted data — it cannot authorize uploads or change targets.

### User input ↔ Automated behavior
Only explicit local actions (key sequences, CLI commands) or explicitly enabled auto-drop policy can authorize uploads. Paste content, remote output, and terminal replies cannot independently create authorization.

## Key Mitigations

- INV-01: Explicit authorization before any local content access
- INV-02: Immutable session target binding
- INV-03: Byte-preserving I/O for non-intercepted content
- INV-04: Single ordered write authority prevents race conditions
- INV-07: No synthesized Enter or control characters from filenames
- INV-11: No private content in default logs
- INV-12: Cleanup contained to verified tool-owned roots
- INV-13: SSH trust checks never weakened

## Residual Risks

- Terminal emulator vulnerabilities are outside scope (documented as environment trust boundary)
- Held file descriptors do not prevent concurrent modification by other processes
- Cannot guarantee exactly-once application-level interpretation of inserted paths
- Cannot distinguish all terminal replies from physical key input

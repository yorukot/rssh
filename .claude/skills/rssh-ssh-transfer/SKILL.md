---
name: rssh-ssh-transfer
description: Implement OpenSSH multiplexing, SFTP, authorized file handles, staging, manifests, or cleanup with fixed-target and failure tests.
---

# SSH, SFTP, and Storage Work

## Procedure

1. Preserve system OpenSSH authentication and host-key verification.
2. Use process argument arrays, not shell strings.
3. Allocate a private, short, session-owned control socket.
4. Never discover destinations through socket scans, prompts, or titles.
5. Start auxiliary SFTP only over the owned master.
6. Prove missing/stale sockets cannot silently establish another transport.
7. Keep SFTP stdout protocol-only.
8. Drain stderr into a bounded redacted buffer.
9. Add startup/shutdown timeouts and reap children.
10. Accept only authorized opened sources.
11. Validate containment, type, size, names, and symlink policy.
12. Use structured SFTP operations and an absolute configured root.
13. Create exclusive staging files in private transaction directories.
14. Acknowledge writes and close before committing names.
15. Publish paths only after remote commit.
16. Treat cancellation as a state/generation change, not rollback.
17. Reject stale success.
18. Clean up only positively identified owned objects.
19. Preserve completed attachments until authorized GC.

## Required failures

Disabled SFTP, restrictive session limits, permission errors, full disk,
close/rename failure, disconnect, local mutation, and cancellation at each
meaningful boundary.

## Distinctions

A local hash is not remote verification.

Rename visibility is not crash durability.

Several renames are not one atomic batch.

A host path may not exist in a container or agent sandbox.

Do not weaken trust or sandbox rules to conceal these limitations.

## Output

Transport/storage decisions, negative-test evidence, retention and cleanup
behavior, compatibility assumptions, and residual risks.

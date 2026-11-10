---
name: rssh-rust
description: Implement rssh Rust behavior with explicit ownership, typed errors, safe boundaries, bounded async resources, and testable interfaces.
---

# Implement Rust Behavior

## Read

Read the task and relevant architecture contracts.

Also use rssh-terminal or rssh-ssh-transfer when touching those boundaries.

## Procedure

1. Define the smallest behavior and its tests.
2. Read existing code and resolved dependency APIs.
3. Keep the binary thin and core behavior testable through the library.
4. Favor concrete types and small enums over speculative frameworks.
5. Make authority visible through private constructors, held source handles,
   target IDs, transaction IDs, and generations.
6. Use typed errors for expected domain failures.
7. Use Path and OsStr at filesystem/process boundaries.
8. Reject unsupported encodings rather than silently losing identity.
9. Give every descriptor, task, child, socket, temporary file, and staging
   object an owner and shutdown path.
10. Bound queue bytes and concurrency.
11. Avoid broad locks across await points.
12. Test partial reads/writes, EOF, timeout, cancellation, and stale results.
13. Run formatting, lint, tests, and the relevant integration gate.
14. Review the final diff for unrelated changes.

## Constraints

No unchecked I/O unwraps, production todo placeholders, unbounded channels,
hidden global state, or Arc<Mutex<Everything>>.

Do not assume dropping a future reverses an external side effect.

New unsafe code needs a narrow safety argument and review.

Do not hand-edit Cargo.lock or guess dependency features.

Comments should explain invariants, rationale, and platform assumptions,
not narrate obvious syntax.

## Output

Implemented behavior, ownership and error decisions, verification results,
dependency changes, and remaining limitations.

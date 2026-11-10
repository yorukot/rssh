# Test Strategy

See IMPLEMENTATION_PLAN.md §11 for the full test strategy specification.

## Test Categories

1. **Unit tests** — Configuration, CLI, OpenSSH argument construction, candidate grammar, source policy, remote path validation, injection rendering, transaction transitions, manifest validation, cleanup containment.

2. **Property tests** — Arbitrary input chunking equivalence, byte preservation, parsing/authorization separation, buffer budget enforcement, generation/insertion invariants, cleanup containment.

3. **State-machine and fault-injection tests** — Cancel at every boundary, disconnect, partial write, queue overflow, close/rename failure, source mutation, actor shutdown.

4. **SSH/SFTP integration tests** — Isolated fixtures with ephemeral keys, dedicated known_hosts, loopback-only. Covers normal connection, auth failure, SFTP disabled, session limits, stale sockets, permission errors, disk exhaustion.

5. **Manual macOS/Ghostty testing** — Recorded environment versions. Single/multi file, special characters, clipboard, IME, resize, multi-pane, agent consumption verification.

6. **Fuzzing** — Input decoder, path candidate parser, output-mode observer, configuration/manifest parsers, injection rendering.

7. **Performance** — Idle CPU, typing latency, memory vs file size, queue growth, upload throughput, saturated link, cancellation/shutdown latency.

## Routine Commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --doc
cargo build --locked
```

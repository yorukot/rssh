---
name: rssh-terminal
description: Modify PTY handling, raw mode, input framing, terminal response routing, mode observation, or insertion ordering with byte-preservation and lifecycle tests.
---

# Terminal and PTY Work

## Contract

The wrapper is a narrow input/output proxy, not a terminal emulator.

Read chunks are not semantic events.

Remote output never authorizes local file access.

## Procedure

1. Identify exactly which bytes and modes are observed or intercepted.
2. Keep everything else opaque and byte-preserving.
3. Use a bounded incremental parser.
4. Test markers split at every byte and multiple events in one read.
5. Preserve original bytes for non-candidates.
6. Never interpret local commands inside paste payloads or known reply frames.
7. Do not force bracketed paste globally.
8. Disable automatic detection when framing is unavailable.
9. Maintain one ordered writer with partial-write accounting.
10. Keep output and resize processing alive during upload.
11. Test query/response behavior for deadlocks.
12. Treat mode changes, long transfers, overflow, cancellation, and partial
    insertion as explicit states.
13. Save and restore outer terminal settings.
14. Close unused PTY handles and reap children.
15. Run synthetic tests and actual Ghostty tests when available.

## Required cases

UTF-8 split points, invalid bytes, incomplete markers, large payloads,
literal prefixes, IME, focus/mouse sequences, resize under load, stale
completion, mode reset, and terminal restoration.

Chunking tests compare normalized semantic behavior, not incidental
read-sized event counts.

Do not claim perfect differentiation of all terminal replies and key input.

## Output

Changed protocol behavior, affected invariants, byte-level evidence,
lifecycle evidence, GUI results or blockers, and residual ambiguity.

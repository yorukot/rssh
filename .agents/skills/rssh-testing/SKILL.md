---
name: rssh-testing
description: Build and execute unit, property, state-machine, fault-injection, SSH, fuzz, GUI, and performance tests with honest evidence.
---

# Test Behavior and Record Evidence

## Procedure

1. Identify observable behavior and the invariant a failure would violate.
2. Write the assertion before choosing a mock.
3. Use deterministic clocks, controlled event order, and faultable I/O.
4. Keep unit tests separate from real-process integration.
5. Exercise denial, cancellation, overflow, partial write, disconnect,
   stale completion, and shutdown.
6. Test no content read before authorization.
7. Test no insertion before remote commit.
8. Test no queued Enter after failure.
9. For parsers, generate arbitrary chunk partitions and compare normalized
   behavior plus exact retained bytes.
10. For state machines, verify effects across event sequences.
11. Use isolated SSH credentials and known_hosts.
12. Fuzz synthetic inputs and retain minimized regressions.
13. For GUI tests, record exact platform and application versions.
14. Run checks and preserve actual exit statuses.

## Quality rules

No empty tests, ignored failures, broad sleeps masking races, unreviewed
snapshot changes, or weakening assertions merely to get green.

A visible path is not proof an agent received an image.

Working-tree tests do not necessarily validate a different staged snapshot.

Missing tools, network, GUI, or server access means blocked or not run.

Coverage percentages do not replace trust-boundary tests.

Performance claims require a baseline and reproducible measurements.

## Output

Cases and invariant coverage, commands/procedures, environment,
passed/failed/blocked/not-run results, minimized failures, and remaining gates.

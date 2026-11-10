---
name: rssh-debug
description: Diagnose bugs, hangs, terminal corruption, ordering failures, flaky tests, or transfer problems using minimal reproducers and verified fixes.
---

# Debug Systematically

## Procedure

1. State observed and expected behavior.
2. Identify the affected invariant.
3. Record version, platform, command, and conditions without private payloads.
4. Reproduce with synthetic data and the smallest controlled setup.
5. Reduce the scope:
   parser, actor, blocking I/O, subprocess, SFTP, injection, or external app.
6. Form testable hypotheses.
7. Instrument IDs, sizes, state transitions, and timing rather than content.
8. Inspect ownership and ordering:
   writer ownership, blocked tasks, stderr draining, generation checks,
   and irreversible operations.
9. Add a failing regression or deterministic reproducer when practical.
10. Fix the narrow cause.
11. Run the reproducer, nearby tests, and relevant integration gate.
12. Review new side effects and update the evidence record.

## Prohibited shortcuts

Do not hide symptoms by disabling host checks, increasing every timeout,
swallowing errors, deleting assertions, or forcing terminal modes.

Do not invent a root cause when reproduction is unavailable.

## Flaky tests

A successful rerun does not resolve a flake.

Record frequency and conditions, identify nondeterminism, and track any
temporary quarantine with an owner and exit condition.

## Output

Failure, reproducer, hypotheses tested, evidenced cause, fix, regression,
actual checks, and unresolved uncertainty.

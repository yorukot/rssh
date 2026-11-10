---
name: rssh-security-review
description: Review authorization, filesystem access, terminal injection, SSH trust, cleanup, dependencies, diagnostics, and CI permissions with concrete evidence.
---

# Security Review

## When to use

Use when changing a trust boundary, permission, source-selection mechanism,
transport, generated input, storage, cleanup, diagnostics, or dependency.

## Procedure

1. Name the attacker, asset, entry point, and authority involved.
2. Trace untrusted data to side effects:
   local open/read, subprocess arguments, SFTP operations, terminal bytes,
   deletion, logs, and CI commands.
3. Verify authorization precedes content access and is target-bound.
4. Reject path existence or paste framing as proof of permission.
5. Review containment, symlinks, races, control characters, quoting context,
   private permissions, partial failure, stale results, and resource limits.
6. Verify remote output cannot create upload authority or change targets.
7. Review cancellation and shutdown for durable side effects.
8. Review dependency and skill provenance when relevant.
9. Add a concrete negative regression test for each confirmed issue.
10. Verify that tests exercise the boundary rather than only mocked returns.

## Findings

For each finding provide:

- Severity.
- Affected location.
- Attacker prerequisites.
- Evidence or reproduction.
- Impact.
- Mitigation.
- Regression test.

Separate confirmed issues, concerns needing investigation, and accepted
residual risks.

Use synthetic secrets and controlled fixtures.

Do not claim an exploit without evidence or declare the whole application
secure because a scan is clean.

## Output

Review scope, prioritized findings, tests, mitigations, residual risks, and
unreviewed areas.

Do not imply independent review unless it actually occurred.

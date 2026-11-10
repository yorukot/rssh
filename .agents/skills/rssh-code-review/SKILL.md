---
name: rssh-code-review
description: Review a bounded diff for correctness, security, failure behavior, compatibility, scope, and missing verification.
---

# Review a Change

## Procedure

1. Identify the base revision and review scope.
2. Inspect staged and unstaged state without conflating unrelated work.
3. Read the task, architecture contracts, and affected invariants.
4. Trace success, denial, cancellation, timeout, overflow, partial write,
   disconnect, and cleanup.
5. Look for multiple writers, hidden authority, unbounded resources,
   panic-based error handling, and stale-result hazards.
6. Inspect whether tests exercise real boundaries.
7. Identify mocks that hide filesystem, SSH, terminal, or process behavior.
8. Check whether required tests actually ran.
9. Review documentation and compatibility claims for overstatement.
10. Separate blockers from maintenance suggestions and style preferences.
11. Report actionable findings with location, scenario, impact, and fix/test.
12. State review limitations and remaining gates.

## Permissions

Review does not authorize code changes, commits, public comments, PR creation,
or publication.

Do not call self-review independent review.

A reviewer agent receives a bounded task and diff, not unrestricted authority
to rewrite the repository.

## Output

Findings by severity, verification gaps, residual risks, and disposition.

When no findings are identified, state the scope and limitations rather than
claiming the program is bug-free.

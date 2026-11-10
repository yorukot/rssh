---
name: rssh-git-commit
description: Prepare a scoped commit and create it only with explicit authorization, preserving unrelated changes and the existing index.
disable-model-invocation: true
---

# Prepare or Create a Local Commit

## Authorization

A commit requires an explicit user request or clearly scoped standing
authorization.

Without authorization, review changes and propose a message without modifying
the index.

This skill never authorizes push, tags, release, publication, PR creation,
identity changes, hook bypass, or history rewriting.

## Procedure

1. Inspect git status --short, git diff, git diff --cached, current branch,
   and recent message conventions.
2. Identify the requested change and actual test evidence.
3. Check for secrets, private captures, generated artifacts, debug logging,
   local configuration, and unrelated edits.
4. If unrelated work is already staged, stop the commit operation and report it.
5. Do not unstage, stash, reset, or silently include other work.
6. Separate logically independent changes.
7. Stage only reviewed paths or hunks for the authorized task.
8. Never use blanket git add in a dirty tree.
9. Inspect the final staged diff, stat, and whitespace check.
10. Run relevant checks and describe their exact scope.
11. Do not claim a staged snapshot passed tests merely because a different
    working tree passed.
12. Follow existing policy or use Conventional Commits.
13. Create the local commit only after authorization and verification.
14. Honor hooks, signing, identity, and real attribution requirements.
15. Inspect hook-generated changes or signing failures; do not bypass them.
16. Verify the resulting commit and remaining worktree state.

## Examples

    feat(transfer): stage uploads before exposing remote paths
    fix(input): preserve split bracketed-paste markers
    test(session): reject stale upload completions
    docs(security): document upload source authorization

Do not invent issue references, results, co-authors, or signatures.

## Output

Without authorization: proposed message, scope, and checks.

After an authorized commit: actual hash, summary, checks, remaining changes,
and confirmation that no push or publication occurred.

---
name: rssh-docs
description: Maintain architecture, decisions, experiments, compatibility, progress, and handoff documentation without duplicating sources of truth.
---

# Maintain Useful Documentation

## Procedure

1. Identify what changed:
   intended scope, runtime contract, threat model, tests, compatibility,
   decision rationale, or current progress.
2. Update the document that owns that information.
3. Keep AGENTS.md concise.
4. Distinguish proposed, implemented, tested, blocked, unsupported,
   and superseded behavior.
5. Never describe an unimplemented interface as available.
6. Explain durable decisions, alternatives, and reconsideration conditions.
7. Use ADRs for durable choices, not every local refactor.
8. Record experiments with hypothesis, environment, method, falsifier,
   actual result, and interpretation.
9. Keep private payloads out of examples.
10. Synchronize CLI help, configuration examples, limitations, and migrations.
11. Keep docs/PROGRESS.md compact and actionable.
12. Validate links, file paths, references, and skill adapters after moves.

## Writing rules

Explain purpose and contracts before implementation detail.

Prefer examples that can be verified.

Comments should explain invariants and rationale, not paraphrase syntax.

Do not copy the same rule into many documents and allow it to drift.

Generated copies must be marked and updated through the canonical source.

## Output

Documents changed, the behavior or decision they describe, checks performed,
and remaining evidence gaps.

A handoff must make the next task executable without previous chat history.

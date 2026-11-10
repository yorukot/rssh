---
name: rssh-plan
description: Plan a bounded rssh feature, refactor, or milestone with acceptance tests, invariant coverage, dependencies, and a small execution sequence.
---

# Plan a Work Package

## When to use

Before a feature, nontrivial refactor, architecture change, or resuming a
milestone after context loss.

Do not turn a trivial typo fix into a design exercise.

## Read

Read AGENTS.md, the relevant implementation phase, docs/PROGRESS.md, and
affected module contracts.

## Procedure

1. Inspect actual repository state and existing work.
2. Identify relevant platform, tooling, and permission limits.
3. State the requested behavior and smallest useful delta.
4. List explicit non-goals.
5. Identify affected invariant IDs, trust boundaries, interfaces, and
   failure paths.
6. Separate facts from assumptions.
7. Route uncertain protocol/API behavior through rssh-research.
8. Select a reviewable vertical slice.
9. Define acceptance tests before implementation:
   success, denial, cancellation, resource exhaustion, and regression.
10. Record dependencies and exact blockers.
11. Continue independent work while blocked gates remain visibly pending.
12. Use a task record and an ADR when a durable architecture decision changes.

## Constraints

Do not expand v0.1 into a daemon, new SSH stack, GUI, plugin framework,
recursive upload, or automatic host/container inference without an approved
scope change.

A phase can be code-complete while compatibility remains unverified.

## Output

Objective, non-goals, affected modules, invariants, acceptance criteria,
execution order, verification commands, and blockers.

When implementation was requested, implement the next unblocked step rather
than stopping after the plan.

## Exit check

Another agent should be able to resume from the task record without the
previous chat.

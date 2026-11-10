---
name: rssh-research
description: Resolve uncertain Rust APIs, terminal behavior, SSH/SFTP semantics, compatibility, or dependencies through primary sources and focused experiments.
---

# Research an Engineering Assumption

## When to use

Use when an API is unfamiliar, behavior is version-dependent, or a design
or security claim depends on protocol details.

## Procedure

1. State the exact question and the decision it affects.
2. Prefer official documentation, resolved crate docs, protocol
   specifications, and maintainer source.
3. Treat search snippets and tutorials as leads, not final evidence.
4. Record URL, retrieval date, version/tag/commit when available, and
   relevant symbol or section.
5. Separate documented, inferred, observed, and unverified statements.
6. Cross-check material behavior with source or a focused experiment.
7. Use synthetic terminal and filesystem fixtures.
8. Compare at least one simpler alternative when the decision warrants it.
9. Record evidence in docs/SOURCES.md or docs/experiments/.
10. Create an ADR when a durable decision changes.

## Dependency review

Check API fit, platforms, default features, MSRV, license, maintenance,
transitive dependencies, and advisories.

Do not install tools or execute third-party scripts merely to read a source.

Do not invent commit hashes, methods, flags, or results.

A moving branch does not establish behavior for every release.

A clean advisory scan is not a complete audit.

## Output

Question, evidence, exact version context, competing options, recommendation,
experiment, actual result, uncertainty, and affected task or ADR.

When browsing or execution is unavailable, mark the assumption pending.

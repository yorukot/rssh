---
name: rssh-ci-release
description: Prepare least-privilege CI and evidence-based releases while keeping tags, credentials, signing, and publication separately authorized.
disable-model-invocation: true
---

# CI and Release Preparation

## Permission boundary

Preparing a workflow or checklist does not authorize publication.

Do not push, create tags, upload release artifacts, change repository settings,
add secrets, select a license, or use signing identities without approval.

Default to preparation and validation.

## CI procedure

1. Define the actual support matrix and phase gates.
2. Separate pure tests, SSH integration, macOS core tests, fuzzing,
   and manual GUI evidence.
3. Pin the selected toolchain and verified full Action commit SHAs.
4. Never invent hashes.
5. Use minimal permissions and job timeouts.
6. Separate untrusted PR execution from privileged release jobs.
7. Avoid shell interpolation of untrusted event text.
8. Use isolated SSH credentials and no developer agent forwarding.
9. Run formatting, lint, tests, documentation, and skill checks.
10. Configure dependency checks and report unavailable databases honestly.
11. Treat untrusted caches and artifacts as untrusted inputs.
12. Do not hide failures behind unconditional success.

## Release procedure

1. Confirm maintainer decisions on name, license, version, platforms,
   publishing identity, and signing.
2. Verify an identified source revision and locked dependency build.
3. Run required gates and inspect compatibility evidence.
4. Build supported artifacts and perform installation/smoke checks.
5. Generate checksums.
6. Sign or attest only through approved mechanisms.
7. Write accurate behavior, limitation, and migration notes.
8. Stop before tagging, pushing, or publishing unless specifically authorized.
9. After authorized publication, verify real identifiers and artifact hashes.

## Output

Changes, pins, checks, permissions, artifact status, open approvals, and the
exact unpublished or published state.

Do not claim reproducibility, notarization, signing, or GUI support without
evidence.

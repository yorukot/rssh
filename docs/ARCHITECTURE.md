# Architecture

See IMPLEMENTATION_PLAN.md §3 for the full architecture specification.

## Module Contracts

### Terminal layer
Owns outer terminal state, PTY lifecycle, resize propagation, byte forwarding, narrow output mode observation, safe shutdown/restoration.

### Input layer  
Owns incremental framing, local command recognition, strict candidate parsing, raw non-candidate preservation.

### Policy layer
Owns source authorization, allowed roots, file type/resource restrictions, target binding, explicit vs automatic upload policy.

### Session actor
Owns transaction state, input ordering, cancellation generations, insertion eligibility, recovery, ordered writer requests.

### Transfer layer
Owns authorized source handles, SFTP operations, staging/commit, transfer cleanup, remote result reporting.

### Injection layer
Owns context-specific rendering of validated remote paths, control-character rejection, profile-specific behavior.

## State Model

Connection: Starting → Authenticating → TransportReady → Interactive → ShuttingDown → Closed

Transaction: Passthrough → CollectingPaste → Validating → Uploading → RemoteCommitted → ReadyToInsert → Inserting → DrainingQueuedInput → Passthrough

Failure: Cancelled | RecoveryRequired | InsertionPartial | InsertionOutcomeUnknown

# ADR-003: Job execution model

- Status: Accepted
- Date: 2026-09-10

## Decision

All conversion requests enter a first-in, first-out queue. Version 1 uses a concurrency limit of one. The limit is a constructor-level policy rather than a hardcoded execution path, allowing later parallel processing without replacing the queue model.

Job records are persisted as versioned JSON in the operating system's application-data directory. Queued work resumes after restart. A job that was running when the application closed is recorded as failed/interrupted because an external process cannot be safely assumed to have completed. Queued and active jobs expose cancellation; adapters terminate the child process and remove staged output when cancellation is observed.

## Consequences

Queued, running, completed, failed, and cancelled states are domain concepts. Persisting after state transitions and meaningful progress updates favors recoverability over minimum disk writes. A later parallel implementation must preserve the same state-machine and persistence behavior.

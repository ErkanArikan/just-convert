# ADR-008: Staged, validated qpdf operations

- Status: Accepted
- Date: 2026-09-11

## Context

The MVP requires PDF merge, split, reorder, and rotate operations that work offline and preserve the user's source files. The job system already provides persistent, single-worker execution and cancellation. PDF processing must fit that boundary without exposing command construction to the frontend or leaving partial output files behind after failure or cancellation.

## Decision

Use the pinned qpdf 12.4.1 executable as an unmodified external subprocess behind a typed Rust adapter.

- The frontend submits structured operation requests; it never supplies shell commands or qpdf arguments.
- The Rust adapter validates input paths, page expressions, rotation values, and output collision rules before launching qpdf.
- Commands are launched directly with `std::process::Command`; no shell is involved.
- Merge, reorder, and rotate write to a temporary `.part.pdf`, validate it with `qpdf --check`, and atomically rename it to the requested output.
- Split stages and validates the complete output set before moving any result into its final location.
- Cancellation terminates the child process and removes staged outputs.
- qpdf exit code 3 is treated as successful completion with warnings, consistent with qpdf's documented exit status.
- Jobs use the existing queue with concurrency one. The adapter remains independent so later queue parallelism does not require redesigning PDF operations.

The Windows release bundles the official 64-bit MSVC qpdf distribution and its required runtime DLLs under `vendor/qpdf/bin/`. Other platforms use platform-specific packaged resources or a discovered system executable through the same resolver and adapter contract.

## Consequences

Core PDF organization tools work offline without modifying originals. Output publication is transactional at the operation level, and failures are reported through the same job model as audio conversions. Packaging must include qpdf and its runtime files, with pinned hashes and license records. Future PDF features that qpdf does not provide, such as rasterization or document reconstruction, require separate typed adapters.

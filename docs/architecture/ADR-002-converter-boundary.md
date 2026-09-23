# ADR-002: Converter boundary

- Status: Accepted
- Date: 2026-09-10

## Decision

Every conversion engine is represented by a typed Rust adapter. Adapters validate inputs, construct process arguments without a command shell, report progress, support cancellation, validate staged output, and return structured errors. The React frontend requests operations but never builds or executes converter commands.

## Consequences

FFmpeg, qpdf, LibreOffice, PDFium, and later document workers remain replaceable. Original files are not modified. Output is produced in staging storage and finalized only after validation.

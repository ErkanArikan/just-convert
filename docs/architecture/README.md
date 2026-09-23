# Architecture Decision Records

These records capture decisions that are expensive to reverse. Proposed changes should update the relevant record or add a new one before implementation.

| Record                                            | Decision                                            |
| ------------------------------------------------- | --------------------------------------------------- |
| [ADR-001](ADR-001-desktop-stack.md)               | Tauri 2, React, TypeScript, and Rust                |
| [ADR-002](ADR-002-converter-boundary.md)          | Typed, process-isolated converter adapters          |
| [ADR-003](ADR-003-job-queue.md)                   | Queue-first execution with concurrency one in v1    |
| [ADR-004](ADR-004-localization-and-theme.md)      | English/Turkish localization and token-based themes |
| [ADR-005](ADR-005-third-party-distribution.md)    | Hybrid bundled and externally detected engines      |
| [ADR-006](ADR-006-file-operation-safety.md)       | Conservative local file-operation rules             |
| [ADR-007](ADR-007-audio-conversion.md)            | Staged, cancellable FFmpeg audio conversion         |
| [ADR-008](ADR-008-pdf-operations.md)              | Staged, validated qpdf PDF operations               |
| [ADR-009](ADR-009-image-and-raster-conversion.md) | Native image assembly and PDFium rasterization      |
| [ADR-010](ADR-010-office-to-pdf.md)               | External LibreOffice with isolated job profiles     |
| [ADR-011](ADR-011-pdf-to-docx.md)                 | External Python worker for text PDF-to-DOCX         |

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
| [ADR-012](ADR-012-mcp-server.md)                  | Headless stdio MCP server over the job service      |
| [ADR-013](ADR-013-macos-and-linux.md)             | macOS and Linux support with self-contained engines |
| [ADR-014](ADR-014-speech-to-text.md)              | Offline speech-to-text with whisper.cpp             |
| [ADR-015](ADR-015-video-conversion.md)            | Video conversion with the bundled FFmpeg            |
| [ADR-016](ADR-016-ocr.md)                         | Offline OCR with Tesseract                          |

# ADR-010: External LibreOffice conversion with isolated profiles

- Status: Accepted
- Date: 2026-09-11

## Context

The MVP requires DOCX, XLSX, and PPTX to PDF conversion. LibreOffice provides mature cross-platform rendering but is too large to bundle. A running user session may already have LibreOffice open, and command-line conversion must not lock, mutate, or depend on the user's normal LibreOffice profile.

## Decision

Detect a user-installed LibreOffice executable at runtime. On Windows, prefer the console launcher `soffice.com`; fall back to `soffice.exe`. macOS and Linux retain their platform-specific discovery paths.

Each queue job creates a unique hidden staging directory beside the requested output. LibreOffice runs headlessly with a unique `UserInstallation` URL and no first-start UI, writing its result into the staging directory. The adapter accepts only DOCX, XLSX, and PPTX inputs, never overwrites an existing output, validates the generated PDF and page count with bundled qpdf, and publishes the requested file only after validation. Cancellation terminates the LibreOffice child process and removes the isolated profile and staged output.

Folder mode scans only top-level DOCX, XLSX, and PPTX files, allocates collision-safe PDF destinations, and submits one normal queue job per document. Unsupported files and nested folders are skipped, and the concurrency-one queue processes the batch sequentially.

When LibreOffice is not detected, the converter remains accessible in a setup-required state and presents a direct official download link. The application must be restarted after installation so capability discovery can run again.

## Consequences

Office conversion remains fully local without increasing installer size or redistributing LibreOffice. Results follow LibreOffice's rendering behavior and may differ from Microsoft Office for complex documents. Tests require LibreOffice on the host and therefore the real integration smoke test is opt-in, while request validation remains part of the regular test suite.

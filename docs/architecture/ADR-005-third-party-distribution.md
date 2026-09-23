# ADR-005: Third-party engine distribution

- Status: Accepted
- Date: 2026-09-10

## Decision

Use a hybrid distribution model. Small core engines such as FFmpeg and qpdf are pinned and bundled when their integration is implemented. Large tools such as LibreOffice and Tesseract are detected at runtime and installed separately by the user. Every integration must add a manifest and human-readable license record before process-launching code is introduced.

## Consequences

The app remains useful offline without an oversized installer. Missing external tools produce actionable capability states instead of crashes. Checksums, source locations, versions, and license texts are maintained under `vendor/manifests/` and `docs/licenses/`.

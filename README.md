# Just Convert

A local-first desktop file manager and file converter for Windows 10 and Windows 11 x64.

Licensed under your choice of the [MIT or Apache 2.0 license](LICENSE).

## Features

### File Conversion

- **Audio** — Convert between MP3, WAV, FLAC, AAC, M4A, and OGG (single file or folder batch)
- **Images to PDF** — Combine JPG, PNG, WebP, and TIFF images into a single PDF or one PDF per image
- **PDF to Images** — Render PDF pages to PNG or JPEG at normal (150 DPI) or high (300 DPI) quality
- **PDF Tools** — Merge, split, reorder, and rotate PDF pages
- **Office to PDF** — Convert DOCX, XLSX, and PPTX files via detected LibreOffice (single file or folder batch)
- **PDF to Word** — Best-effort DOCX conversion for digitally generated PDFs via detected Python + PyMuPDF (single file or folder batch)

### File Management

- Browse local folders with list view, sorting, and filtering
- Copy, move, rename, create folders, and move files to trash
- Permanent deletion with explicit confirmation

### Privacy & Safety

- Fully local — no uploads, no network requests, no telemetry
- Originals are never modified; all outputs are staged and validated before saving
- Output filename collisions resolved automatically with incrementing suffixes

### Job Queue

- Persistent single-worker queue with cancellation support
- Active, Failed & Cancelled, and Completed sections with collapsible history
- Failure entries show the reason (missing dependency, unsupported file, etc.)

### Dependencies & Settings

- FFmpeg and qpdf bundled — work out of the box
- LibreOffice and Python detected automatically or configured manually in Settings
- English and Turkish interface
- Light, dark, and system themes

## Screenshots

![Convert Dashboard](docs/screenshots/dashboard.png)
![Job History](docs/screenshots/jobs.png)
![Settings & Dependencies](docs/screenshots/settings.png)

## Dependencies

- FFmpeg and qpdf are bundled with Just Convert; no separate installation is needed.
- LibreOffice must be installed separately for Office-to-PDF conversion. Download it from the [official LibreOffice website](https://www.libreoffice.org/download/download-libreoffice/).
- Python 3.10 or newer, plus the `pymupdf` and `pdf2docx` packages, must be installed separately for PDF-to-Word conversion. Install the packages with `pip install pymupdf pdf2docx`.
- All other features work out of the box with no additional setup.

## Windows v1 Distribution

Windows v1 is distributed only as an NSIS installer. MSI/WiX packaging is deferred to a later release.

The v1 executable and installer are unsigned. Windows SmartScreen may therefore display an "unrecognized app" or "unknown publisher" warning, especially on newly published downloads without an established reputation. Download releases only from the project's official repository and verify the published SHA-256 checksum before running the installer. Code signing is intentionally deferred for v1 because trusted certificates and signing infrastructure add recurring cost and release-process complexity.

## Third-Party Components

This application bundles FFmpeg under GPL v3, qpdf, PDFium, and other third-party components for selected conversion features. Each component remains subject to its own license and redistribution terms; see [Third-Party Licenses](docs/licenses/THIRD_PARTY_LICENSES.md) for versions, sources, checksums, attribution, and full license references.

PDF-to-Word uses a separately installed Python runtime with PyMuPDF (AGPL v3 or commercial) and pdf2docx; none of these dependencies is included in the installer, distributed by the project, or linked into the Rust core. The project treats these as optional external subprocess dependencies, like LibreOffice, as detailed in the third-party license record.

## Development

Built with Tauri 2, Rust, React, TypeScript, and pnpm.

- Install dependencies: `pnpm install`
- Run the browser UI: `pnpm dev`
- Run the desktop application: `pnpm tauri dev`
- Run the complete quality gate: `pnpm check`

## Post-v1 Roadmap

Cross-platform support is a possible future roadmap item, not a current commitment. Optional Tesseract OCR for scanned PDFs and parallel queue execution are also deferred until after the Windows v1 release.

Architectural decisions are recorded in [`docs/architecture/`](docs/architecture/README.md). Conversion engines must remain behind typed Rust adapters and may not be launched directly by frontend code.

# ADR-013: macOS and Linux support with self-contained engines

- Status: Proposed
- Date: 2026-10-10

## Context

The Rust core, adapters, and React UI are platform-neutral, but v1 bundles Windows-only FFmpeg, qpdf, and PDFium binaries, ships only an NSIS installer, and asks users to install LibreOffice and Python with PyMuPDF and pdf2docx themselves (ADR-005, ADR-010, ADR-011). Requiring separate installs is the most common reason a local converter "does not work" for a user.

## Decision

Every release, on every platform, contains every engine. Users install the app and nothing else.

- **Pinned sources.** Each engine manifest in `vendor/manifests/` lists `platformArchives`: an official or long-standing build per platform and architecture, with its SHA-256 checksum. `pnpm fetch:engines` downloads, verifies, and extracts them into `vendor/`, then builds the PDF-to-Word worker. Fetched files are git-ignored. Windows FFmpeg, qpdf, and PDFium stay in Git LFS as before.
- **FFmpeg 9.0.2:** static GPL builds from martin-riedl.de (macOS) and BtbN/FFmpeg-Builds (Linux).
- **qpdf 12.4.2:** official release binaries.
- **PDFium `chromium/7961`:** same release as Windows.
- **LibreOffice 26.8.1:** official Document Foundation builds, embedded and run headless. On Linux, optional Qt, KDE, GTK 4, and Java desktop modules are pruned; headless conversion never loads them.
- **PDF to Word:** `workers/pdf_to_docx.py` is frozen with PyInstaller into a self-contained `pdf-to-docx` executable. It uses a uv-managed CPython 3.12 and the hash-locked `workers/requirements.lock`, so neither users nor build machines need a Python installation. A Python runtime chosen in Settings still overrides the bundled worker.
- **Discovery order:** a manual override from Settings, then the bundled engine in the resource directory, then `PATH` and well-known package-manager directories. The capability registry resolves engines against the app's resource directory, so bundled engines are found inside `Contents/Resources` and `/usr/lib/<product>/`.
- **Packaging:** `pnpm bundle:macos` builds the `.app`, copies the engines in with `ditto` (Tauri's resource copier dereferences symlinks, which breaks LibreOffice's notarized signature), ad-hoc signs the outer bundle without `--deep`, and creates the DMG. `pnpm bundle:linux` builds the `.deb` normally. It also builds the AppImage without LibreOffice and the worker, because linuxdeploy cannot resolve or rewrite their libraries, then adds them and repacks it. The `.deb` depends on `libnss3`, and the AppImage carries NSS/NSPR itself, since LibreOffice links them. `pnpm bundle:windows` adds LibreOffice and the worker to the NSIS installer. Engine resources are applied only at packaging time (`src-tauri/engines/*.conf.json`), so building and testing never require the fetched engines.
- **OGG output** uses `libvorbis`, falling back to FFmpeg's built-in Vorbis encoder for builds without it.
- **CI:** `quality.yml` runs `pnpm check` on Windows, macOS, and Ubuntu. `engines.yml` checks the pinned downloads and runs the engine suite weekly and on engine changes. `release.yml` builds, tests, and drafts a GitHub release for `v*` tags. A composite action restores Git LFS objects from the Actions cache to save LFS bandwidth.

## Consequences

Installers grow to roughly 450–600 MB, mostly LibreOffice and the PDF-to-Word worker (OpenCV). In exchange, every converter works offline straight after installation. The distributed PDF-to-Word worker is subject to PyMuPDF's AGPL v3. The app still only launches it as a subprocess, and its full source is in this repository. Builds remain unsigned: macOS Gatekeeper warns on first launch, as SmartScreen does on Windows. The embedded LibreOffice keeps its notarized Document Foundation signature. License checksums are computed on byte-identical files on every platform, because `docs/licenses/**` is marked `-text`.

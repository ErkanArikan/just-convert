# Just Convert

A local-first desktop file manager and file converter for Windows 10 and 11 x64, macOS 11+, and Linux.

Licensed under your choice of the [MIT or Apache 2.0 license](LICENSE).

## Features

### File Conversion

- **Audio** — Convert between MP3, WAV, FLAC, AAC, M4A, and OGG, or extract audio from MP4, MOV, MKV, WebM, AVI, and M4V video (single file or folder batch)
- **Speech to text** — Transcribe audio or video to text, SRT or WebVTT subtitles, or timestamped JSON with Whisper, offline; an optional high-accuracy model downloads once on request
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

- Fully local — no uploads, no telemetry, and no network requests except the optional high-accuracy speech model, which downloads once only if you choose it
- Originals are never modified; all outputs are staged and validated before saving
- Output filename collisions resolved automatically with incrementing suffixes

### Job Queue

- Persistent single-worker queue with cancellation support
- Active, Failed & Cancelled, and Completed sections with collapsible history
- Failure entries show the reason (missing dependency, unsupported file, etc.)

### Dependencies & Settings

- Every engine is bundled (FFmpeg, qpdf, PDFium, LibreOffice, and the PDF-to-Word worker), so nothing else needs to be installed
- Engines can be overridden manually in Settings
- English and Turkish interface
- Light, dark, and system themes

## Screenshots

![Convert Dashboard](docs/screenshots/dashboard.png)
![Job History](docs/screenshots/jobs.png)
![Settings & Dependencies](docs/screenshots/settings.png)

## Dependencies

Nothing. Every converter works right after installation, offline, on Windows, macOS, and Linux. FFmpeg, qpdf, PDFium, LibreOffice, and a self-contained PDF-to-Word worker (Python, PyMuPDF, and pdf2docx) ship inside the app. A different LibreOffice or Python can still be selected in Settings.

## macOS and Linux

macOS builds are `.app`/`.dmg` and Linux builds are `.deb`/AppImage, roughly 450–600 MB because LibreOffice is included. These builds are unsigned too, so macOS Gatekeeper warns on first launch. See [ADR-013](docs/architecture/ADR-013-macos-and-linux.md).

## Windows v1 Distribution

Windows v1 is distributed only as an NSIS installer. MSI/WiX packaging is deferred to a later release.

The v1 executable and installer are unsigned. Windows SmartScreen may therefore display an "unrecognized app" or "unknown publisher" warning, especially on newly published downloads without an established reputation. Download releases only from the project's official repository and verify the published SHA-256 checksum before running the installer. Code signing is intentionally deferred for v1 because trusted certificates and signing infrastructure add recurring cost and release-process complexity.

## Third-Party Components

This application bundles FFmpeg under GPL v3, qpdf, PDFium, and other third-party components for selected conversion features. Each component remains subject to its own license and redistribution terms; see [Third-Party Licenses](docs/licenses/THIRD_PARTY_LICENSES.md) for versions, sources, checksums, attribution, and full license references.

PDF-to-Word uses a separately installed Python runtime with PyMuPDF (AGPL v3 or commercial) and pdf2docx; none of these dependencies is included in the installer, distributed by the project, or linked into the Rust core. The project treats these as optional external subprocess dependencies, like LibreOffice, as detailed in the third-party license record.

## Development

Built with Tauri 2, Rust, React, TypeScript, and pnpm.

- Install dependencies: `pnpm install`
- Fetch the engines for your platform (needs [uv](https://docs.astral.sh/uv/) to build the PDF-to-Word worker): `pnpm fetch:engines`. On Linux, also install the [Tauri system dependencies](https://tauri.app/start/prerequisites/#linux).
- Build installers: `pnpm bundle:windows`, `pnpm bundle:macos`, or `pnpm bundle:linux`
- Run the browser UI: `pnpm dev`
- Run the desktop application: `pnpm tauri dev`
- Run the complete quality gate: `pnpm check`

## AI Assistant Integration (MCP)

Just Convert is also a local [Model Context Protocol](https://modelcontextprotocol.io) server, so AI assistants such as Claude, Cursor, VS Code, Codex, and Gemini CLI can convert files with it. The installed app is the server: running it with `--mcp` serves MCP over stdio instead of opening a window, using the same bundled engines and job queue as the desktop app. It opens no network port, and files never leave the machine.

- **Claude Desktop:** install `just-convert-<version>.mcpb` from the release page. It is a small launcher that finds the installed app.
- **MCP Registry:** published as `io.github.ErkanArikan/just-convert` with every release.
- **Any other client:** add the app executable with the `--mcp` argument:

| Platform       | Command                                                      |
| -------------- | ------------------------------------------------------------ |
| macOS          | `/Applications/Just Convert.app/Contents/MacOS/just-convert` |
| Windows        | `%LOCALAPPDATA%\Just Convert\just-convert.exe`               |
| Linux (`.deb`) | `/usr/bin/just-convert`                                      |
| Linux AppImage | the `.AppImage` file itself                                  |

```json
{
  "mcpServers": {
    "just-convert": {
      "command": "/Applications/Just Convert.app/Contents/MacOS/just-convert",
      "args": ["--mcp"]
    }
  }
}
```

For Claude Code: `claude mcp add just-convert -- "/Applications/Just Convert.app/Contents/MacOS/just-convert" --mcp`. Call `list_converters` to see what is ready. See [ADR-012](docs/architecture/ADR-012-mcp-server.md).

## Continuous Integration and Releases

- **Quality** (`quality.yml`): `pnpm check` on Windows, macOS, and Ubuntu for every push and pull request.
- **Engines** (`engines.yml`): checks that every pinned engine download is still available, then fetches the engines and runs the full engine test suite on all three platforms. Runs weekly and on pull requests that touch engines.
- **Versioning** (`release-please.yml`): commits use [Conventional Commits](https://www.conventionalcommits.org/). On every push to `main`, Release Please keeps a release pull request up to date that bumps the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` and updates `CHANGELOG.md`. Merging it tags the release and starts the installer build.
- **Release** (`release.yml`): builds the Windows, macOS (Apple silicon and Intel), and Linux (x64 and arm64) installers and tests every bundled engine on each platform. For a Release Please release, it attaches the installers and `SHA256SUMS` and then publishes the release. A manually pushed `v<version>` tag gets a draft release instead. Pull requests that change packaging or engines, and manual runs, build the installers as workflow artifacts without publishing.

Optional repository secrets:

| Secret                                                                      | Effect                                                                                        |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `RELEASE_PLEASE_TOKEN`                                                      | Personal access token so release pull requests also run CI (`GITHUB_TOKEN` cannot trigger it) |
| `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` | Sign the macOS app and its engines with a Developer ID certificate and the hardened runtime   |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`                               | Also notarize and staple the app and DMG                                                      |
| `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`                       | Sign the Windows executable and installer                                                     |

Without them, builds stay unsigned as described above.

## Post-v1 Roadmap

Optional Tesseract OCR for scanned PDFs and parallel queue execution are deferred until after the Windows v1 release.

Architectural decisions are recorded in [`docs/architecture/`](docs/architecture/README.md). Conversion engines must remain behind typed Rust adapters and may not be launched directly by frontend code.

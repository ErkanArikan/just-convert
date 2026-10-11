# ADR-016: Offline OCR with Tesseract

- Status: Proposed
- Date: 2026-10-10

## Context

Users want scanned PDFs and photographed pages turned into searchable PDFs or text without a cloud service and without installing anything beyond the app (ADR-013). Tesseract has no official prebuilt binary for macOS or Linux, and a separate engine executable would have to be shipped and signed per platform.

## Decision

- **Engine:** Tesseract 5.5.2 and Leptonica 1.87.0, compiled into the app as static libraries by the `kreuzberg-tesseract` crate and called in-process behind a typed adapter (`application/ocr.rs`), the same way whisper.cpp is (ADR-014). ADR-002's staging, validation, and cancellation rules apply; cancellation is checked between pages.
- **Pinned sources:** the crate normally downloads unpinned source archives during the build. `vendor/manifests/tesseract.json` pins both archives by SHA-256, `pnpm fetch:engines` unpacks them into `vendor/tesseract-build/third_party`, and `TESSERACT_RS_CACHE_DIR` in `.cargo/config.toml` points the crate there, so the build never touches the network. CI runs `node scripts/fetch-engines.mjs tesseract` before building.
- **Languages:** the `tessdata_fast` English and Turkish models are bundled (about 9 MB), pinned to a commit. Both can be combined, but mixing languages lowers accuracy, so the UI recommends choosing only Turkish for Turkish documents.
- **Input:** PDFs, rendered page by page at 300 DPI with the bundled PDFium, and PNG, JPEG, TIFF, BMP, and WebP images.
- **Output:** plain text, or a searchable PDF. Tesseract's own PDF writer cannot be used because the crate builds Leptonica without zlib, so `application/searchable_pdf.rs` adds the text layer itself with `lopdf`: recognized words (from Tesseract's TSV output) are written as invisible text in the glyphless font Tesseract ships (`pdf.ttf`), the same technique Tesseract uses. For PDF input the original pages are kept untouched and the layer is overlaid, including rotated pages; image input becomes a one-page PDF with the image embedded as JPEG.
- **Surfaces:** a converter card, an `ocr` job kind, and the MCP `ocr` tool.

## Consequences

Installers grow by about 9 MB for language data, and Tesseract adds a few MB of code. Builds need CMake and a C++ toolchain, which GitHub's runners provide; the first build compiles Tesseract (a few minutes, cached in CI by manifest hash). Only English and Turkish ship; more `tessdata_fast` files can be added to the manifest. Recognition is single-threaded per job and runs on the job worker thread, so a long scan occupies the queue. Layout analysis is Tesseract's default, with no deskew or denoising, so poor scans lose accuracy.

# ADR-009: Native image assembly and bundled PDFium rasterization

- Status: Accepted
- Date: 2026-09-11

## Context

The MVP requires images-to-PDF and PDF-to-images conversion on all supported desktop platforms. Both workflows must remain local, preserve originals, participate in the persistent queue, and avoid publishing partial output sets. PDF rendering must be accurate enough for common documents without depending on a separately installed application.

## Decision

Use two typed Rust adapters with distinct engine boundaries.

### Images to PDF

Use `printpdf` with only JPG, PNG, WebP, and TIFF decoding features enabled. Each source image becomes one A4 page. Portrait or landscape orientation is selected from the image dimensions, aspect ratio is preserved, and the image is centered inside a 10 mm margin. The adapter writes a staged PDF, validates its page count through bundled qpdf, and renames it to the requested output only after validation succeeds.

### PDF to images

Use `pdfium-render` with a pinned, non-V8 PDFium dynamic library. Windows bundles PDFium 152.0.7961.0 from the official bblanchon binary distribution; macOS and Linux packaging will provide their matching platform libraries behind the same resolver. The adapter supports PNG and JPEG at 150 or 300 DPI, renders annotations and form data, uses a white page background, and caps either rendered dimension at 12,000 pixels to avoid unbounded memory use.

All page images are written to hidden staging paths and decoded again to validate their format and dimensions. The full output set is finalized only after every page succeeds. Cancellation between pages removes staged files. Existing destination files are rejected before rendering begins.

## Consequences

Both mandatory image workflows work offline and use the existing single-worker queue. Images-to-PDF adds no executable runtime. PDFium adds a small platform-specific dynamic library and permissive-license attribution requirements. Updating PDFium requires replacing the platform runtime, verifying the binding compatibility smoke test, updating its checksum manifest, and preserving the complete upstream license directory.

Folder mode remains a queue-planning concern rather than a second conversion engine. Images-to-PDF collects supported top-level images in deterministic filename order. The user can submit them as one combined multi-page PDF job or as one ordinary single-page PDF job per image; separate outputs reuse each image's base filename and collision suffix policy. PDF-to-images creates one collision-safe subfolder per top-level PDF and queues one ordinary render job per source. Unsupported files and nested folders are ignored.

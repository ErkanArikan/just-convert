# ADR-011: External Python worker for text PDF-to-DOCX conversion

- Status: Accepted
- Date: 2026-09-11

## Context

The MVP requires best-effort conversion of digitally generated PDFs with selectable text to DOCX. Python, PyMuPDF, and pdf2docx provide a practical cross-platform implementation, but they must not increase installer size. PyMuPDF is offered under AGPL v3 or a commercial license.

## Decision

Detect Python 3.10 or newer at runtime through an explicit `LOCAL_FILE_CONVERTER_PYTHON` override, standard per-user Windows installation directories, or `PATH`. Confirm readiness by importing both `pymupdf` and `pdf2docx`. Neither Python nor either package is bundled or distributed by the project. The project treats PyMuPDF as an optional user-installed external subprocess dependency, using the same distribution boundary as LibreOffice, and retains its own MIT or Apache-2.0 licensing.

The Rust job adapter launches the project-owned `workers/pdf_to_docx.py` script as an external child process without a shell. The worker first uses PyMuPDF to require at least some selectable text, then uses pdf2docx for conversion. It rejects image-only scanned PDFs; OCR remains a later optional Tesseract feature. Output is written to a hidden per-job staging path, validated as a DOCX archive, and atomically published without overwriting existing files. Cancellation terminates the child process and removes its staged output.

Folder mode scans only top-level PDFs, allocates collision-safe DOCX destinations, and submits one normal queue job per file. Unsupported files and nested folders are skipped. A scanned-only PDF fails only its own queued job, preserving a readable failure reason in history while later batch entries continue sequentially.

The UI exposes direct Python and PyMuPDF installation links, the pip command for both packages, and explicit expectations for complex layouts. Capability discovery reports setup-required until the interpreter, both packages, and worker are available.

## Consequences

The Rust core never statically or dynamically links Python, PyMuPDF, or pdf2docx, and the installer does not redistribute them. Conversion stays local and uses the existing concurrency-one queue. Users install and retain responsibility for their external Python environment independently of the application installer.

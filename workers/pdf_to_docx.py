"""Local PDF-to-DOCX worker executed by an external Python runtime.

Python, PyMuPDF, and pdf2docx are intentionally not bundled with the app.
"""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path


def progress(value: int) -> None:
    print(f"FILE_CONVERTER_PROGRESS:{value}", flush=True)


def fail(message: str, exit_code: int = 1) -> int:
    print(message, file=sys.stderr, flush=True)
    return exit_code


def validate_docx(path: Path) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise RuntimeError("The Python worker did not produce a DOCX file")
    if not zipfile.is_zipfile(path):
        raise RuntimeError("The Python worker produced an invalid DOCX archive")
    with zipfile.ZipFile(path, "r") as archive:
        if archive.testzip() is not None:
            raise RuntimeError("The generated DOCX archive is corrupt")
        required = {"[Content_Types].xml", "word/document.xml"}
        if not required.issubset(archive.namelist()):
            raise RuntimeError("The generated file is missing required DOCX content")


def main() -> int:
    if len(sys.argv) != 3:
        return fail("Usage: pdf_to_docx.py <input.pdf> <output.docx>", 2)

    input_path = Path(sys.argv[1]).resolve()
    output_path = Path(sys.argv[2]).resolve()
    if not input_path.is_file() or input_path.suffix.lower() != ".pdf":
        return fail("The PDF input does not exist or is not a PDF", 2)
    if output_path.exists() or output_path.suffix.lower() != ".docx":
        return fail("The DOCX output must not exist and must use the .docx extension", 2)
    if not output_path.parent.is_dir():
        return fail("The DOCX output directory does not exist", 2)

    try:
        import pymupdf
        from pdf2docx import Converter
    except ImportError as error:
        return fail(
            "PDF-to-Word dependencies are missing. Install PyMuPDF and pdf2docx "
            f"for this Python runtime ({error}).",
            4,
        )

    try:
        progress(5)
        with pymupdf.open(input_path) as document:
            if document.page_count == 0:
                return fail("The PDF has no pages", 3)
            has_selectable_text = any(
                page.get_text("text").strip() for page in document
            )
        if not has_selectable_text:
            return fail(
                "This PDF has no selectable text. Scanned PDF conversion requires "
                "the later OCR feature.",
                3,
            )

        progress(15)
        converter = Converter(str(input_path))
        try:
            converter.convert(str(output_path), start=0, end=None)
        finally:
            converter.close()
        progress(90)
        validate_docx(output_path)
        progress(100)
        return 0
    except Exception as error:  # The Rust caller surfaces this diagnostic to the job.
        output_path.unlink(missing_ok=True)
        return fail(f"PDF-to-Word conversion failed: {error}")


if __name__ == "__main__":
    raise SystemExit(main())

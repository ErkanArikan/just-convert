# ADR-018: Offline text-to-speech with Piper voices

- Status: Proposed
- Date: 2026-10-10

## Context

Users want to turn text into natural-sounding audio, in English and Turkish, without a cloud service and without installing anything beyond the app (ADR-013). Piper is the obvious engine, but its last official binaries (2023) contain an x86-64 macOS executable mislabeled as `aarch64`, which cannot run on Apple Silicon without Rosetta, and its maintained successor (piper1-gpl) is published only as Python wheels. Piper also needs eSpeak NG for phonemes.

## Decision

- **Engine:** the `sherpa-onnx-offline-tts` executable from sherpa-onnx 1.13.8, which runs Piper (VITS) voices. It has official, checksum-pinnable release archives for macOS arm64 and x64, Linux x64 and arm64, and Windows x64 (`vendor/manifests/tts.json`). It runs as a separate process behind a typed adapter (`application/text_to_speech.rs`) that follows ADR-002: no shell, staged output, validation, originals untouched, and cancellation by killing the child. The FFmpeg process supervisor in `application/ffmpeg.rs` was generalized (`supervise`) so both engines share it.
- **Voices:** two Piper voices in int8 form (about 18 MB each), bundled with the shared eSpeak NG data (18 MB): `en_US-ljspeech-medium` (public-domain dataset) and `tr_TR-fettah-medium` (CC0 dataset). Other Turkish Piper voices were skipped: `dfki` is CC BY-NC-SA, which a distributed installer should not carry.
- **Input and limits:** up to 20,000 characters of text, typed, pasted, or read from a `.txt`/`.md` file (at most 200 KB). The runtime takes the text as one command-line argument, which Windows caps near 32,000 characters; a leading dash is padded so it cannot be read as an option.
- **Output:** the engine writes a 22.05 kHz WAV. WAV is finalized directly; MP3, FLAC, AAC, M4A, and OGG are encoded with the bundled FFmpeg through the existing audio adapter, and the temporary WAV is always removed. Progress comes from the engine's sentence progress lines.
- **Speed:** slow, normal, and fast map to VITS length scales 1.25, 1.0, and 0.8.
- **Surfaces:** a converter card, a `text_to_speech` job kind, the MCP tool `text_to_speech`, and a `read_text_file` command for the UI's file loader.

## Consequences

Installers grow by about 90 MB (runtime 20-28 MB, voices 36 MB, phoneme data 18 MB). The executable statically links eSpeak NG, so it is GPL-3.0-or-later; like the GPL FFmpeg build it is only ever started as a separate process and is never linked into the app, and its license and source references are recorded in `docs/licenses`. int8 voices trade a little fidelity for size. The Turkish voice was fine-tuned from Piper's `lessac` checkpoint, whose training data license cannot be audited here. Synthesis is CPU-only; on an Apple M-series CPU it runs about eight times faster than real time, and a verification round trip through Whisper transcribed both voices almost word for word. Long texts must be split into parts of at most 20,000 characters.

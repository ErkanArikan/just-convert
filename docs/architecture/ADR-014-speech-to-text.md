# ADR-014: Offline speech-to-text with whisper.cpp

- Status: Proposed
- Date: 2026-10-10

## Context

Users want transcripts and subtitles from recordings and videos without sending audio to a cloud service, and without installing anything beyond the app (ADR-013).

## Decision

- **Engine:** whisper.cpp, compiled into the app through the `whisper-rs` binding, with Metal acceleration on macOS and CPU elsewhere. Unlike the other engines it runs in-process on the job worker thread, because a subprocess would mean shipping and signing a separate binary per platform. It stays behind a typed adapter (`application/transcription.rs`) and follows ADR-002's staging, validation, and cancellation rules.
- **Input:** any audio or video file FFmpeg can read. FFmpeg decodes it to 16 kHz mono samples over a pipe, without temporary files.
- **Models:** the multilingual `ggml-base-q5_1` model (60 MB) is bundled through `vendor/manifests/whisper.json`, so transcription works offline immediately in all Whisper languages, Turkish included. The optional `ggml-large-v3-turbo-q5_0` model (574 MB) is downloaded only when the user picks "High accuracy". The download comes from a pinned Hugging Face revision, its SHA-256 is verified, it is stored in the app-data `models/` directory, and it can be cancelled. This is the only network request the app makes, and only on that explicit choice.
- **Output:** plain text, SRT, WebVTT, or JSON with segment timestamps and the detected language.
- **Progress and cancellation:** the job thread polls whisper's progress callback through an atomic. Cancellation goes through whisper's abort callback, which is registered through the raw API because `whisper-rs` 0.16's `set_abort_callback_safe` casts its user data to the wrong type.
- **Video to audio:** the existing FFmpeg audio adapter also accepts MP4, MOV, MKV, WebM, AVI, and M4V input, extracting the audio track.
- **Surfaces:** a converter card, a `transcription` job kind, and the MCP `transcribe` tool.

## Consequences

Installers grow by about 60 MB, and builds need CMake and libclang, which GitHub's runners provide. whisper.cpp would otherwise optimize for the build machine's CPU, so `.cargo/config.toml` builds a portable engine instead: x86-64-v3 (AVX2, every x86 CPU since 2013) and baseline NEON on ARM64. Accuracy with the bundled model is moderate; the high-accuracy model is a one-time download. Long recordings hold their decoded samples in memory, about 230 MB per hour of audio.

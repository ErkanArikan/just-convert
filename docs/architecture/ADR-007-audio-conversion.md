# ADR-007: FFmpeg audio conversion

- Status: Accepted
- Date: 2026-09-10

## Decision

The Rust audio adapter invokes the bundled FFmpeg executable directly with an owned argument list and no command shell. It supports MP3, WAV, FLAC, AAC, M4A, and OGG output with explicit codecs; M4A uses the AAC encoder and its extension selects the container. FFprobe supplies duration when available so FFmpeg progress can be normalized for the queue UI.

Each conversion writes to a uniquely named staging file in the destination directory. The adapter validates that the staged output exists and is non-empty before renaming it to the requested destination. Existing destinations are rejected, original inputs are never modified, and cancelled or failed jobs remove their staging files.

Folder mode scans only the selected directory's top level, filters supported audio extensions, allocates collision-safe output names, and submits one ordinary conversion job per file. The queue retains its version 1 concurrency limit of one, so a batch is sequential without creating a separate execution path.

Development resolves binaries from `vendor/ffmpeg/bin/`. Packaged applications resolve them from Tauri resources. Environment overrides and `PATH` detection support development and future platform-specific distributions without exposing process construction to React.

## Consequences

The current repository contains the pinned Windows x64 distribution. macOS and Linux releases must add separately pinned, licensed binaries to their packaging inputs before those installers are published. Codec settings are intentionally conservative defaults; quality controls can be added to the typed request later without changing the engine boundary.

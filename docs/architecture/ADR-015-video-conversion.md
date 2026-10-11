# ADR-015: Video conversion with the bundled FFmpeg

- Status: Proposed
- Date: 2026-10-10

## Context

Users want to convert, shrink, and animate videos, and pull still frames out of them, without installing anything beyond the app (ADR-013). FFmpeg is already bundled for audio (ADR-005) and every pinned build is the GPL variant with libx264, libvpx-vp9, libopus, and the GIF palette filters.

## Decision

- **Engine:** the existing FFmpeg binary, behind a typed adapter (`application/video_converter.rs`) that follows ADR-002: no shell, staged output, validation, originals untouched, cancellation by killing the child process.
- **Shared runner:** the process supervision (progress from `-progress pipe:1`, cancellation, error capture) moved out of the audio adapter into `application/ffmpeg.rs`. Audio and video use it; staged files are cleaned up by each adapter.
- **Formats:** MP4 (H.264/AAC, `+faststart`), WebM (VP9/Opus), MOV and MKV (H.264/AAC), and animated GIF (two-pass `palettegen`/`paletteuse` in one filter graph). Input accepts common video containers and GIF. Only the first video and first audio stream are kept; subtitles and data streams are dropped, so MP4/MOV never fail on unsupported streams. Dimensions are forced even for yuv420p.
- **Compression:** three presets (high, balanced, small) map to CRF 18/23/28 for H.264 and 24/32/40 for VP9, with matching audio bitrates; for GIF they pick frame rate and default height. An optional maximum height (144 to 4320) downscales and never enlarges.
- **Frame extraction:** one PNG or JPEG every N seconds (1 to 3600) into a hidden staging directory, then renamed to `<name>-frame-001.<ext>` with incrementing suffixes on collisions. The staging directory is removed on every exit path.
- **Surfaces:** a "Convert video" card with a convert/extract task switch, `video_conversion` and `video_frames` job kinds, and the MCP tools `convert_video` and `extract_video_frames`.

## Consequences

No new binaries, so installers do not grow. The first missing encoder (for example an unbundled system FFmpeg without libx264) fails the job up front with a clear message. VP9 encoding is slow on long videos; the preset keeps `-cpu-used 4` and row multithreading as a speed/size compromise. Hardware encoders (VideoToolbox, NVENC) are not used, so output is identical across platforms. Frame progress is approximate because FFmpeg reports output timestamps. x265/AV1 output is out of scope.

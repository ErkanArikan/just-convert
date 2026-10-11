# ADR-017: Local image AI with tract

- Status: Proposed
- Date: 2026-10-10

## Context

Users want background removal and upscaling for photos without a cloud service and without installing anything beyond the app (ADR-013). The usual runtime, ONNX Runtime, is a native library that would have to be bundled, signed, and loaded per platform.

## Decision

- **Runtime:** [tract](https://github.com/sonos/tract), a pure-Rust ONNX inference engine, called in-process behind a typed adapter (`application/image_ai.rs`) that follows ADR-002's staging, validation, and cancellation rules. It adds no native library to package or sign, and compiles the same on every platform. In a release-mode prototype on an Apple M-series CPU, the segmentation model ran in about 0.3 seconds and the upscaler in about 0.65 seconds for a 256-pixel input.
- **Background removal:** U²-Net `u2netp` (4.6 MB, Apache-2.0). The image is resized to 320 pixels, normalized like rembg does, segmented, and the min-max normalized mask is scaled back to the original size and multiplied into the alpha channel. Output is a PNG.
- **Upscaling:** Real-ESRGAN `realesr-general-x4v3` (4.9 MB, BSD-3-Clause). The image is processed in 128-pixel tiles with a 10-pixel replicated border to avoid seams, always at 4x; a 2x request downsamples the 4x result with Lanczos. An alpha channel is upscaled separately with Lanczos. Input is limited to 4 megapixels and 16384 pixels per output side to bound memory and time. Output is PNG or JPG.
- **Models:** both are bundled through `vendor/manifests/image-ai.json`, pinned by SHA-256. The Real-ESRGAN ONNX is a community export, because upstream publishes only PyTorch weights; its outputs were verified bit-identical to an export of the official weights made with the architecture definition, so the pin is safe. Teams that prefer to host their own copy can replace the URL and checksum.
- **Surfaces:** two converter cards (remove background, upscale image) sharing one screen, an `image_ai` job kind, and the MCP tools `remove_background` and `upscale_image`.

## Consequences

Installers grow by about 9.5 MB. `u2netp` is a small model: it handles clear subjects well but is less accurate than larger models on hair and fine edges; a larger model can be added later as an on-demand download like the Whisper high-accuracy model (ADR-014). Upscaling is CPU-only and its time grows with the pixel count; debug builds are much slower than release builds. tract does not support every ONNX operator, so swapping in other models needs testing first.

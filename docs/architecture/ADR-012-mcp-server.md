# ADR-012: Headless MCP server over the job service

- Status: Proposed
- Date: 2026-10-10

## Context

AI assistants and agent harnesses (Claude, Cursor, VS Code, Codex, Gemini CLI, and others) can call local tools through the Model Context Protocol. Exposing Just Convert this way lets an agent convert files without uploading them, but it must not bypass the converter boundary (ADR-002), the queue (ADR-003), or the file-safety rules (ADR-006).

## Decision

Serve MCP from the desktop executable itself: `just-convert --mcp` runs a headless stdio MCP server instead of opening a window, using the official Rust SDK (`rmcp`) behind the default `mcp` cargo feature. It opens no network listener. Because every installer already ships that executable next to the bundled engines (ADR-013), MCP needs no separate install. A standalone `just-convert-mcp` binary target remains for development.

The server constructs its own `JobService` and calls the same `enqueue_*` methods as the Tauri commands, so every request goes through the existing adapter validation, staging, and queue. Tools mirror the job service one-to-one, plus `list_converters`, `get_job`, `cancel_job`, and `list_jobs`. Tools carry MCP annotations (read-only or non-destructive, closed-world).

Agent-facing conveniences live only in the MCP layer:

- Paths must be absolute. A leading `~` is expanded, and relative paths are rejected.
- Output paths are optional. When omitted, a non-colliding name is chosen next to the input, and existing files are never overwritten.
- Conversion tools wait for completion by default (up to 600 seconds, capped at 3600), emitting MCP progress notifications. After that the job keeps running and can be polled with `get_job`.

The server reads engine overrides from the desktop app's settings database, but it keeps its job history in a separate `mcp/` subdirectory of the app-data folder, so the desktop app and the server can run at the same time without contending for one history file. `JUST_CONVERT_DATA_DIR` overrides the base directory, for tests.

The server resolves the app's resource directory from the executable's location (next to it on Windows, `Contents/Resources` on macOS, `lib/Just Convert` on Linux), so it uses the bundled engines. Otherwise it falls back to the `LOCAL_FILE_CONVERTER_*` overrides and `PATH`.

Each new converter adds its MCP tools in its own `#[tool_router]` block, combined in `JustConvertServer::new`.

## Consequences

Agents get the same safety guarantees as the UI: originals are never modified, and output is staged and validated. Jobs started by an agent do not appear in the desktop job list in this version.

Distribution:

- Each release attaches `just-convert-<version>.mcpb`, a small MCP Bundle whose Node launcher finds the installed app (standard locations, or a configured path) and runs it with `--mcp`. Embedding the app and its engines would make the bundle about 1.5 GB.
- The release workflow publishes `server.json` to the official MCP Registry as `io.github.<owner>/just-convert`, using GitHub OIDC, so no credentials are stored.
- Release builds smoke-test `--mcp` on every platform with all engines ready.

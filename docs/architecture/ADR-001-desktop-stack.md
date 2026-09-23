# ADR-001: Desktop application stack

- Status: Accepted
- Date: 2026-09-10

## Decision

Use Tauri 2 as the desktop shell, React and TypeScript for the interface, and Rust for privileged local operations. Windows 10/11 x64 is the first shipping target, but platform-dependent behavior must remain behind dedicated Rust platform adapters so macOS and Linux require no structural rewrite.

## Consequences

The frontend cannot access the filesystem or spawn processes directly. Native behavior is exposed through narrow, serializable Tauri commands. Platform-specific installation paths, trash behavior, process termination, and packaging configuration remain isolated from conversion-domain code.

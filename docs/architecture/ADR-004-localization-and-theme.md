# ADR-004: Localization and theme

- Status: Accepted
- Date: 2026-09-10

## Decision

All user-facing text is addressed through localization keys. English and Turkish are complete first-release languages. Styling uses semantic design tokens with System, Light, and Dark preferences; feature code may not embed theme-specific colors.

## Consequences

Language, theme, and Jobs section expansion preferences persist locally. New features require English and Turkish strings in the same change. Additional languages can be added without changing components.

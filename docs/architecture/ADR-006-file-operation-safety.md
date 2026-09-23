# ADR-006: File-operation safety

- Status: Accepted
- Date: 2026-09-10

## Decision

Native filesystem commands accept absolute paths only and canonicalize existing inputs before operating on them. File and folder creation, rename, copy, and move operations never overwrite an existing destination. Filesystem roots and the user's home directory are protected from trash and permanent-delete commands. Trash is the default destructive flow; permanent deletion requires an explicit confirmation value from a separately confirmed UI action.

Symbolic links are identified distinctly in directory metadata. Version 1 does not recursively copy symbolic links because faithfully preserving links requires platform-specific privilege and target policies. Directory listings sort folders before files and do not expose hidden entries unless the user opts in.

## Consequences

Some operations fail rather than guessing, including name collisions, copying symbolic links, moving across filesystems, and copying a directory into itself. Errors are returned as structured command failures for localized presentation. Cross-volume move fallback and recoverable multi-item transaction reporting may be added without changing the public command boundary.

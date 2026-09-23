export type EntryKind = "directory" | "file" | "symlink" | "other";

export interface FileEntry {
  name: string;
  path: string;
  kind: EntryKind;
  sizeBytes: number | null;
  modifiedAt: number | null;
  extension: string | null;
  hidden: boolean;
}

export interface DirectoryListing {
  path: string;
  parent: string | null;
  entries: FileEntry[];
}

export interface FileOperationResult {
  affectedPaths: string[];
}

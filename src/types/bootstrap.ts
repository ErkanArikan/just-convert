export type ConverterStatus = "ready" | "external_required" | "planned";

export interface ConverterCapability {
  id: string;
  category: "document" | "pdf" | "image" | "audio";
  titleKey: string;
  descriptionKey: string;
  engine: string;
  status: ConverterStatus;
  bundled: boolean;
  inputExtensions: string[];
  outputExtensions: string[];
}

export type DependencyId = "libreoffice" | "python";
export type DependencyIssue =
  | "invalid_manual_path"
  | "windows_apps_alias"
  | "version_check_failed"
  | "packages_missing"
  | "worker_missing";

export interface DependencyStatus {
  id: DependencyId;
  automaticPath: string | null;
  manualPath: string | null;
  manualPathValid: boolean;
  resolvedPath: string | null;
  ready: boolean;
  issue: DependencyIssue | null;
}

export interface DependencyStatuses {
  libreoffice: DependencyStatus;
  python: DependencyStatus;
}

export interface AppBootstrap {
  appVersion: string;
  platform: string;
  converters: ConverterCapability[];
  dependencies: DependencyStatuses;
}

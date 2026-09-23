import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Language = "en" | "tr";
export type ThemePreference = "system" | "light" | "dark";
export type JobSectionId = "active" | "failed" | "completed";

interface JobSectionPreferences {
  active: boolean;
  failed: boolean;
  completed: boolean;
}

interface PreferencesState {
  language: Language;
  theme: ThemePreference;
  jobSections: JobSectionPreferences;
  setLanguage: (language: Language) => void;
  setTheme: (theme: ThemePreference) => void;
  setJobSectionExpanded: (section: JobSectionId, expanded: boolean) => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      language: "en",
      theme: "system",
      jobSections: {
        active: true,
        failed: true,
        completed: false,
      },
      setLanguage: (language) => set({ language }),
      setTheme: (theme) => set({ theme }),
      setJobSectionExpanded: (section, expanded) =>
        set((state) => ({
          jobSections: { ...state.jobSections, [section]: expanded },
        })),
    }),
    { name: "just-convert-preferences" },
  ),
);

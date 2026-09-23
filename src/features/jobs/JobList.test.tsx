import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ConversionJob } from "../../types/jobs";
import i18n from "../../i18n";
import { usePreferencesStore } from "../../store/preferences";

const mocks = vi.hoisted(() => ({
  cancelJob: vi.fn(),
  clearJobHistory: vi.fn(),
  clearJobHistorySection: vi.fn(),
  deleteJobHistory: vi.fn(),
  listJobs: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { JobList } from "./JobList";

function audioJob(id: string, status: ConversionJob["status"]): ConversionJob {
  return {
    id,
    jobType: "audio_conversion",
    kind: {
      type: "audio_conversion",
      input_path: `C:\\input\\${id}.wav`,
      output_path: `C:\\output\\${id}.mp3`,
      format: "mp3",
    },
    status,
    progress: status === "completed" ? 100 : 20,
    createdAt: 1_700_000_000,
    startedAt: status === "queued" ? null : 1_700_000_001,
    finishedAt:
      status === "completed" || status === "failed" || status === "cancelled"
        ? 1_700_000_002
        : null,
    error: null,
  };
}

describe("JobList history management", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    localStorage.clear();
    await i18n.changeLanguage("en");
    usePreferencesStore.setState({
      jobSections: { active: true, failed: true, completed: false },
    });
    mocks.listJobs.mockResolvedValue([
      audioJob("finished", "completed"),
      audioJob("active", "running"),
      {
        ...audioJob("failed", "failed"),
        error: { code: "conversion_failed", message: "LibreOffice not found" },
      },
      audioJob("cancelled", "cancelled"),
    ]);
    mocks.deleteJobHistory.mockResolvedValue(undefined);
    mocks.clearJobHistory.mockResolvedValue(1);
    mocks.clearJobHistorySection.mockResolvedValue(2);
  });

  it("deletes only terminal history entries and keeps active jobs cancellable", async () => {
    const user = userEvent.setup();
    render(<JobList desktopAvailable />);

    expect(
      await screen.findByRole("button", { name: "Completed (1)" }),
    ).toHaveAttribute("aria-expanded", "false");
    expect(
      screen.queryByRole("button", {
        name: "Delete finished.mp3 from history",
      }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", {
        name: "Delete active.mp3 from history",
      }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Active (1)" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Failed & Cancelled (2)" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Completed (1)" }),
    ).toBeInTheDocument();
    expect(screen.getByText("LibreOffice not found")).toBeInTheDocument();
    expect(screen.getByText("Cancelled by user")).toBeInTheDocument();

    expect(screen.getByRole("button", { name: "Active (1)" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Completed (1)" }));
    const deleteButton = await screen.findByRole("button", {
      name: "Delete finished.mp3 from history",
    });
    await user.click(deleteButton);
    await waitFor(() =>
      expect(mocks.deleteJobHistory).toHaveBeenCalledWith("finished"),
    );
  });

  it("confirms before clearing terminal history", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<JobList desktopAvailable />);

    await user.click(await screen.findByRole("button", { name: "Clear all" }));

    expect(confirm).toHaveBeenCalledWith(
      "Remove all completed and cancelled jobs from history? Failed jobs and converted output files will remain.",
    );
    await waitFor(() => expect(mocks.clearJobHistory).toHaveBeenCalledOnce());
    confirm.mockRestore();
  });

  it("clears failed and cancelled history independently", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<JobList desktopAvailable />);

    await screen.findByRole("heading", { name: "Failed & Cancelled (2)" });
    await user.click(
      screen.getByRole("button", { name: "Clear Failed & Cancelled" }),
    );

    expect(confirm).toHaveBeenCalledWith(
      "Remove all failed and cancelled jobs from history? Output files will not be deleted.",
    );
    await waitFor(() =>
      expect(mocks.clearJobHistorySection).toHaveBeenCalledWith("failed"),
    );
    confirm.mockRestore();
  });

  it("persists a section's collapse preference across remounts", async () => {
    const user = userEvent.setup();
    const first = render(<JobList desktopAvailable />);

    await screen.findByText("LibreOffice not found");
    await user.click(
      screen.getByRole("button", { name: "Failed & Cancelled (2)" }),
    );
    expect(screen.queryByText("LibreOffice not found")).not.toBeInTheDocument();
    expect(usePreferencesStore.getState().jobSections.failed).toBe(false);
    expect(localStorage.getItem("just-convert-preferences")).toContain(
      '"failed":false',
    );

    first.unmount();
    render(<JobList desktopAvailable />);
    expect(
      await screen.findByRole("button", { name: "Failed & Cancelled (2)" }),
    ).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("LibreOffice not found")).not.toBeInTheDocument();
  });
});

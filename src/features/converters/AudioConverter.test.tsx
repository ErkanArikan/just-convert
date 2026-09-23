import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseAudioFile: vi.fn(),
  chooseAudioOutput: vi.fn(),
  chooseDirectory: vi.fn(),
  enqueueAudioConversion: vi.fn(),
  enqueueAudioFolderConversion: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { AudioConverter } from "./AudioConverter";

describe("AudioConverter", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues a selected folder as individual sequential audio jobs", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\Music\\Source")
      .mockResolvedValueOnce("C:\\Music\\Converted");
    mocks.enqueueAudioFolderConversion.mockResolvedValueOnce([
      { id: "batch-1" },
      { id: "batch-2" },
    ]);

    render(
      <AudioConverter
        desktopAvailable
        onBack={() => undefined}
        onViewJobs={() => undefined}
      />,
    );

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.selectOptions(screen.getByLabelText("Output format"), "ogg");
    await user.click(
      screen.getByRole("button", { name: "Choose output folder" }),
    );
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueAudioFolderConversion).toHaveBeenCalledWith(
      "C:\\Music\\Source",
      "C:\\Music\\Converted",
      "ogg",
    );
    expect(
      screen.getByText("2 audio files added to the local queue."),
    ).toBeInTheDocument();
  });
});

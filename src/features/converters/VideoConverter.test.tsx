import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseDirectory: vi.fn(),
  chooseVideoFile: vi.fn(),
  chooseVideoOutput: vi.fn(),
  enqueueVideoConversion: vi.fn(),
  enqueueVideoFrames: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { VideoConverter } from "./VideoConverter";

function renderTool() {
  render(
    <VideoConverter
      desktopAvailable
      onBack={() => undefined}
      onViewJobs={() => undefined}
    />,
  );
}

describe("VideoConverter", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues a compressed WebM with a height limit", async () => {
    const user = userEvent.setup();
    mocks.chooseVideoFile.mockResolvedValueOnce("/videos/talk.mov");
    mocks.chooseVideoOutput.mockResolvedValueOnce("/videos/talk.webm");
    mocks.enqueueVideoConversion.mockResolvedValueOnce({ id: "video-1" });
    renderTool();

    await user.click(screen.getByRole("button", { name: "Choose video" }));
    await user.selectOptions(screen.getByLabelText("Output format"), "webm");
    await user.selectOptions(
      screen.getByLabelText("Size and quality"),
      "small",
    );
    await user.selectOptions(screen.getByLabelText("Maximum height"), "720");
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseVideoOutput).toHaveBeenCalledWith(
      "/videos/talk.webm",
      "webm",
    );
    expect(mocks.enqueueVideoConversion).toHaveBeenCalledWith(
      "/videos/talk.mov",
      "/videos/talk.webm",
      "webm",
      "small",
      720,
    );
    expect(
      screen.getByText("Video task added to the local queue."),
    ).toBeInTheDocument();
  });

  it("sends no height limit by default", async () => {
    const user = userEvent.setup();
    mocks.chooseVideoFile.mockResolvedValueOnce("/videos/clip.mp4");
    mocks.chooseVideoOutput.mockResolvedValueOnce("/videos/clip-copy.mkv");
    mocks.enqueueVideoConversion.mockResolvedValueOnce({ id: "video-2" });
    renderTool();

    await user.click(screen.getByRole("button", { name: "Choose video" }));
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueVideoConversion).toHaveBeenCalledWith(
      "/videos/clip.mp4",
      "/videos/clip-copy.mkv",
      "mp4",
      "balanced",
      null,
    );
  });

  it("extracts frames into a chosen folder at the chosen interval", async () => {
    const user = userEvent.setup();
    mocks.chooseVideoFile.mockResolvedValueOnce("/videos/clip.mp4");
    mocks.chooseDirectory.mockResolvedValueOnce("/videos/frames");
    mocks.enqueueVideoFrames.mockResolvedValueOnce({ id: "video-3" });
    renderTool();

    await user.selectOptions(
      screen.getByLabelText("What do you want to do?"),
      "frames",
    );
    await user.click(screen.getByRole("button", { name: "Choose video" }));
    const interval = screen.getByLabelText("Frame interval");
    await user.clear(interval);
    await user.type(interval, "5");
    await user.selectOptions(screen.getByLabelText("Image format"), "jpeg");
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueVideoFrames).toHaveBeenCalledWith(
      "/videos/clip.mp4",
      "/videos/frames",
      "jpeg",
      5,
    );
  });

  it("blocks an out-of-range frame interval", async () => {
    const user = userEvent.setup();
    mocks.chooseVideoFile.mockResolvedValueOnce("/videos/clip.mp4");
    mocks.chooseDirectory.mockResolvedValueOnce("/videos/frames");
    renderTool();

    await user.selectOptions(
      screen.getByLabelText("What do you want to do?"),
      "frames",
    );
    await user.click(screen.getByRole("button", { name: "Choose video" }));
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    const interval = screen.getByLabelText("Frame interval");
    await user.clear(interval);
    await user.type(interval, "0");

    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });
});

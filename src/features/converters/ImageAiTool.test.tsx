import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseImageFile: vi.fn(),
  chooseImageOutput: vi.fn(),
  enqueueImageAi: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { ImageAiTool } from "./ImageAiTool";

function renderTool(mode: "remove-background" | "upscale-image") {
  render(
    <ImageAiTool
      desktopAvailable
      mode={mode}
      onBack={() => undefined}
      onViewJobs={() => undefined}
    />,
  );
}

describe("ImageAiTool", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues a background removal to a transparent PNG", async () => {
    const user = userEvent.setup();
    mocks.chooseImageFile.mockResolvedValueOnce("/photos/cat.jpg");
    mocks.chooseImageOutput.mockResolvedValueOnce("/photos/cat-cutout.png");
    mocks.enqueueImageAi.mockResolvedValueOnce({ id: "ai-1" });
    renderTool("remove-background");

    await user.click(screen.getByRole("button", { name: "Choose image" }));
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseImageOutput).toHaveBeenCalledWith(
      "/photos/cat-cutout.png",
      ["png"],
    );
    expect(mocks.enqueueImageAi).toHaveBeenCalledWith(
      "/photos/cat.jpg",
      "/photos/cat-cutout.png",
      { type: "remove_background" },
    );
    expect(
      screen.getByText("Image task added to the local queue."),
    ).toBeInTheDocument();
  });

  it("queues a 2x JPG upscale", async () => {
    const user = userEvent.setup();
    mocks.chooseImageFile.mockResolvedValueOnce("/photos/small.png");
    mocks.chooseImageOutput.mockResolvedValueOnce("/photos/small-2x.jpg");
    mocks.enqueueImageAi.mockResolvedValueOnce({ id: "ai-2" });
    renderTool("upscale-image");

    await user.click(screen.getByRole("button", { name: "Choose image" }));
    await user.selectOptions(screen.getByLabelText("Enlargement"), "2");
    await user.selectOptions(screen.getByLabelText("Result"), "jpg");
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseImageOutput).toHaveBeenCalledWith(
      "/photos/small-2x.jpg",
      ["jpg"],
    );
    expect(mocks.enqueueImageAi).toHaveBeenCalledWith(
      "/photos/small.png",
      "/photos/small-2x.jpg",
      { type: "upscale", scale: 2 },
    );
  });
});

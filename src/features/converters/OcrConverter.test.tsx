import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseOcrFile: vi.fn(),
  chooseOcrOutput: vi.fn(),
  enqueueOcr: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { OcrConverter } from "./OcrConverter";

function renderTool() {
  render(
    <OcrConverter
      desktopAvailable
      onBack={() => undefined}
      onViewJobs={() => undefined}
    />,
  );
}

describe("OcrConverter", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues a searchable PDF in English by default", async () => {
    const user = userEvent.setup();
    mocks.chooseOcrFile.mockResolvedValueOnce("/scans/contract.pdf");
    mocks.chooseOcrOutput.mockResolvedValueOnce("/scans/contract-ocr.pdf");
    mocks.enqueueOcr.mockResolvedValueOnce({ id: "ocr-1" });
    renderTool();

    await user.click(screen.getByRole("button", { name: "Choose file" }));
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseOcrOutput).toHaveBeenCalledWith(
      "/scans/contract-ocr.pdf",
      "pdf",
    );
    expect(mocks.enqueueOcr).toHaveBeenCalledWith(
      "/scans/contract.pdf",
      "/scans/contract-ocr.pdf",
      "pdf",
      ["eng"],
    );
    expect(
      screen.getByText("Text recognition added to the local queue."),
    ).toBeInTheDocument();
  });

  it("queues Turkish text output for an image", async () => {
    const user = userEvent.setup();
    mocks.chooseOcrFile.mockResolvedValueOnce("/scans/page.png");
    mocks.chooseOcrOutput.mockResolvedValueOnce("/scans/page.txt");
    mocks.enqueueOcr.mockResolvedValueOnce({ id: "ocr-2" });
    renderTool();

    await user.click(screen.getByRole("button", { name: "Choose file" }));
    await user.selectOptions(screen.getByLabelText("Document language"), "tur");
    await user.selectOptions(screen.getByLabelText("Result"), "txt");
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseOcrOutput).toHaveBeenCalledWith(
      "/scans/page.txt",
      "txt",
    );
    expect(mocks.enqueueOcr).toHaveBeenCalledWith(
      "/scans/page.png",
      "/scans/page.txt",
      "txt",
      ["tur"],
    );
  });
});

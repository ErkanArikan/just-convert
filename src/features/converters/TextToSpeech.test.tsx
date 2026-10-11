import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseAudioOutput: vi.fn(),
  chooseTextFile: vi.fn(),
  enqueueTextToSpeech: vi.fn(),
  readTextFile: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { TextToSpeech } from "./TextToSpeech";

function renderTool() {
  render(
    <TextToSpeech
      desktopAvailable
      onBack={() => undefined}
      onViewJobs={() => undefined}
    />,
  );
}

describe("TextToSpeech", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues typed text with the chosen voice, speed and format", async () => {
    const user = userEvent.setup();
    mocks.chooseAudioOutput.mockResolvedValueOnce("/audio/hello.wav");
    mocks.enqueueTextToSpeech.mockResolvedValueOnce({ id: "tts-1" });
    renderTool();

    await user.type(screen.getByLabelText("Text"), "Merhaba dünya");
    await user.selectOptions(screen.getByLabelText("Voice"), "turkish");
    await user.selectOptions(screen.getByLabelText("Speaking speed"), "slow");
    await user.selectOptions(screen.getByLabelText("Audio format"), "wav");
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseAudioOutput).toHaveBeenCalledWith("speech.wav", "wav");
    expect(mocks.enqueueTextToSpeech).toHaveBeenCalledWith(
      "Merhaba dünya",
      "/audio/hello.wav",
      "wav",
      "turkish",
      "slow",
    );
    expect(
      screen.getByText("Speech added to the local queue."),
    ).toBeInTheDocument();
  });

  it("loads a text file and suggests its name for the output", async () => {
    const user = userEvent.setup();
    mocks.chooseTextFile.mockResolvedValueOnce("/docs/chapter-one.txt");
    mocks.readTextFile.mockResolvedValueOnce("Once upon a time.");
    mocks.chooseAudioOutput.mockResolvedValueOnce("/docs/chapter-one.mp3");
    renderTool();

    await user.click(screen.getByRole("button", { name: "Open text file" }));
    await user.click(screen.getByRole("button", { name: "Choose output" }));

    expect(screen.getByLabelText("Text")).toHaveValue("Once upon a time.");
    expect(mocks.chooseAudioOutput).toHaveBeenCalledWith(
      "chapter-one.mp3",
      "mp3",
    );
  });

  it("blocks empty text", () => {
    renderTool();
    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });
});

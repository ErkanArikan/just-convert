import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseAudioFile: vi.fn(),
  chooseTranscriptOutput: vi.fn(),
  enqueueTranscription: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { SpeechToText } from "./SpeechToText";

describe("SpeechToText", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("queues a subtitle transcription with the chosen language and model", async () => {
    const user = userEvent.setup();
    mocks.chooseAudioFile.mockResolvedValueOnce("/videos/talk.mp4");
    mocks.chooseTranscriptOutput.mockResolvedValueOnce("/videos/talk.srt");
    mocks.enqueueTranscription.mockResolvedValueOnce({ id: "speech-1" });

    render(
      <SpeechToText
        desktopAvailable
        onBack={() => undefined}
        onViewJobs={() => undefined}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: "Choose audio or video" }),
    );
    await user.selectOptions(screen.getByLabelText("Transcript format"), "srt");
    await user.selectOptions(screen.getByLabelText("Spoken language"), "tr");
    await user.selectOptions(
      screen.getByLabelText("Accuracy"),
      "high_accuracy",
    );
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.chooseTranscriptOutput).toHaveBeenCalledWith(
      "/videos/talk.srt",
      "srt",
    );
    expect(mocks.enqueueTranscription).toHaveBeenCalledWith(
      "/videos/talk.mp4",
      "/videos/talk.srt",
      "srt",
      "tr",
      "high_accuracy",
    );
    expect(
      screen.getByText("Transcription added to the local queue."),
    ).toBeInTheDocument();
  });

  it("sends no language when detection is automatic", async () => {
    const user = userEvent.setup();
    mocks.chooseAudioFile.mockResolvedValueOnce("/audio/note.wav");
    mocks.chooseTranscriptOutput.mockResolvedValueOnce("/audio/note.txt");
    mocks.enqueueTranscription.mockResolvedValueOnce({ id: "speech-2" });

    render(
      <SpeechToText
        desktopAvailable
        onBack={() => undefined}
        onViewJobs={() => undefined}
      />,
    );

    await user.click(
      screen.getByRole("button", { name: "Choose audio or video" }),
    );
    await user.click(screen.getByRole("button", { name: "Choose output" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueTranscription).toHaveBeenCalledWith(
      "/audio/note.wav",
      "/audio/note.txt",
      "txt",
      null,
      "standard",
    );
  });
});

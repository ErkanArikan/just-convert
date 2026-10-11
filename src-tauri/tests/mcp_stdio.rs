#![cfg(feature = "mcp")]

use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{json, Value};

struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Session {
    fn start(data_directory: &Path) -> Self {
        Self::start_program(env!("CARGO_BIN_EXE_just-convert-mcp"), &[], data_directory)
    }

    fn start_program(program: &str, args: &[&str], data_directory: &Path) -> Self {
        let mut child = Command::new(program)
            .args(args)
            .env("JUST_CONVERT_DATA_DIR", data_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut session = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        session.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        session
    }

    fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "server closed stdout"
            );
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == id {
                return message["result"].clone();
            }
        }
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn write_silent_wav(path: &Path) {
    let sample_rate: u32 = 8000;
    let data_len: u32 = sample_rate * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.resize(bytes.len() + data_len as usize, 0);
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn lists_every_conversion_tool() {
    let data = tempfile::tempdir().unwrap();
    let mut session = Session::start(data.path());

    let result = session.request("tools/list", json!({}));
    let names: Vec<_> = result["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    for expected in [
        "list_converters",
        "convert_audio",
        "transcribe",
        "ocr",
        "remove_background",
        "upscale_image",
        "convert_video",
        "extract_video_frames",
        "images_to_pdf",
        "pdf_merge",
        "pdf_to_images",
        "office_to_pdf",
        "pdf_to_docx",
        "get_job",
        "cancel_job",
    ] {
        assert!(names.contains(&expected.to_string()), "missing {expected}");
    }
}

#[test]
fn desktop_app_serves_mcp_with_the_mcp_flag() {
    let data = tempfile::tempdir().unwrap();
    let mut session =
        Session::start_program(env!("CARGO_BIN_EXE_just-convert"), &["--mcp"], data.path());

    let result = session.request("tools/list", json!({}));

    assert!(result["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "convert_audio"));
}

#[test]
fn relative_paths_are_rejected_as_tool_errors() {
    let data = tempfile::tempdir().unwrap();
    let mut session = Session::start(data.path());

    let result = session.call(
        "convert_audio",
        json!({ "input_path": "song.wav", "format": "mp3" }),
    );

    assert_eq!(result["isError"], true);
    assert!(result["structuredContent"]["error"]
        .as_str()
        .unwrap()
        .contains("absolute"));
}

#[test]
#[ignore = "runs the bundled FFmpeg binary"]
fn converts_audio_and_returns_the_finished_job() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("silence.wav");
    write_silent_wav(&input);
    let mut session = Session::start(data.path());

    let result = session.call(
        "convert_audio",
        json!({ "input_path": input, "format": "flac" }),
    );

    let job = &result["structuredContent"]["jobs"][0];
    assert_eq!(job["status"], "completed", "{result}");
    assert!(files.path().join("silence.flac").is_file());
}

fn write_test_clip(path: &Path) {
    let ffmpeg = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vendor/ffmpeg/bin")
        .join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX));
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg("testsrc=size=320x240:rate=15:duration=4")
        .args(["-pix_fmt", "yuv420p"])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn video_tools_reject_out_of_range_options_as_tool_errors() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("clip.mp4");
    std::fs::write(&input, b"not really a video").unwrap();
    let mut session = Session::start(data.path());

    let result = session.call(
        "convert_video",
        json!({ "input_path": input, "format": "webm", "max_height": 10 }),
    );
    assert_eq!(result["isError"], true);
    assert!(result["structuredContent"]["error"]
        .as_str()
        .unwrap()
        .contains("maximum height"));

    let result = session.call(
        "extract_video_frames",
        json!({ "input_path": input, "interval_seconds": 0 }),
    );
    assert_eq!(result["isError"], true);
    assert!(result["structuredContent"]["error"]
        .as_str()
        .unwrap()
        .contains("interval"));
}

#[test]
#[ignore = "runs the bundled FFmpeg binary"]
fn converts_video_and_extracts_frames() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("clip.mp4");
    write_test_clip(&input);
    let mut session = Session::start(data.path());

    let result = session.call(
        "convert_video",
        json!({ "input_path": input, "format": "webm", "quality": "small", "max_height": 144 }),
    );
    let job = &result["structuredContent"]["jobs"][0];
    assert_eq!(job["status"], "completed", "{result}");
    assert!(files.path().join("clip.webm").is_file());

    let result = session.call(
        "extract_video_frames",
        json!({ "input_path": input, "interval_seconds": 2, "format": "jpeg" }),
    );
    let job = &result["structuredContent"]["jobs"][0];
    assert_eq!(job["status"], "completed", "{result}");
    assert!(files.path().join("clip-frame-001.jpg").is_file());
    assert!(files.path().join("clip-frame-002.jpg").is_file());
}

#[test]
#[ignore = "needs the fetched Tesseract language data"]
fn recognizes_text_in_an_image() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("scan.png");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr-en.png"),
        &input,
    )
    .unwrap();
    let mut session = Session::start(data.path());

    let result = session.call("ocr", json!({ "input_path": input, "format": "txt" }));

    let job = &result["structuredContent"]["jobs"][0];
    assert_eq!(job["status"], "completed", "{result}");
    let text = std::fs::read_to_string(files.path().join("scan.txt")).unwrap();
    assert!(text.contains("Invoice 2026"), "{text}");
}

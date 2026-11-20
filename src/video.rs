use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub duration: f32,
    pub total_frames: u64,
}

#[derive(Deserialize)]
struct FfprobeStream {
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
    nb_frames: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

pub struct VideoProbe;

impl VideoProbe {
    pub fn probe(video_path: impl AsRef<Path>) -> Result<VideoMetadata> {
        let path = video_path.as_ref();
        if !path.exists() {
            bail!("Input video does not exist: {:?}", path);
        }

        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height,r_frame_rate,duration,nb_frames:format=duration",
                "-of",
                "json",
            ])
            .arg(path)
            .output()
            .with_context(|| "Failed to execute ffprobe. Ensure ffmpeg/ffprobe is installed.")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("ffprobe failed: {}", err);
        }

        let probe_res: FfprobeOutput = serde_json::from_slice(&output.stdout)
            .with_context(|| "Failed to parse ffprobe json output")?;

        let stream = probe_res
            .streams
            .as_ref()
            .and_then(|s| s.first())
            .ok_or_else(|| anyhow::anyhow!("No video stream found in {:?}", path))?;

        let width = stream.width.unwrap_or(0);
        let height = stream.height.unwrap_or(0);
        if width == 0 || height == 0 {
            bail!("Invalid video dimensions: {}x{}", width, height);
        }

        let fps = if let Some(ref r_fps) = stream.r_frame_rate {
            parse_fps_fraction(r_fps).unwrap_or(30.0)
        } else {
            30.0
        };

        let duration: f32 = stream
            .duration
            .as_ref()
            .and_then(|d| d.parse::<f32>().ok())
            .or_else(|| {
                probe_res
                    .format
                    .as_ref()
                    .and_then(|f| f.duration.as_ref())
                    .and_then(|d| d.parse::<f32>().ok())
            })
            .unwrap_or(0.0);

        let total_frames: u64 = stream
            .nb_frames
            .as_ref()
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or_else(|| (duration * fps).round() as u64);

        Ok(VideoMetadata {
            width,
            height,
            fps,
            duration,
            total_frames,
        })
    }
}

fn parse_fps_fraction(s: &str) -> Option<f32> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f32 = parts[0].trim().parse().ok()?;
        let den: f32 = parts[1].trim().parse().ok()?;
        if den != 0.0 {
            return Some(num / den);
        }
    } else if let Ok(f) = s.parse::<f32>() {
        return Some(f);
    }
    None
}

pub struct VideoDecoder {
    child: Child,
    stdout: ChildStdout,
    pub width: u32,
    pub height: u32,
    frame_size_bytes: usize,
}

impl VideoDecoder {
    pub fn spawn(
        video_path: impl AsRef<Path>,
        target_width: u32,
        target_height: u32,
        start_sec: Option<f32>,
        duration_sec: Option<f32>,
    ) -> Result<Self> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error"]);

        if let Some(start) = start_sec {
            cmd.args(["-ss", &format!("{:.3}", start)]);
        }

        cmd.arg("-i").arg(video_path.as_ref());

        if let Some(dur) = duration_sec {
            cmd.args(["-t", &format!("{:.3}", dur)]);
        }

        let scale_filter = format!("scale={}:{}:flags=bilinear", target_width, target_height);
        cmd.args(["-vf", &scale_filter]);

        cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn().with_context(|| "Failed to spawn ffmpeg decoder")?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to capture ffmpeg stdout"))?;

        let frame_size_bytes = (target_width * target_height * 4) as usize;

        Ok(Self {
            child,
            stdout,
            width: target_width,
            height: target_height,
            frame_size_bytes,
        })
    }

    /// Read next raw RGBA frame into provided buffer. Returns false when EOF reached.
    pub fn read_frame(&mut self, buffer: &mut Vec<u8>) -> Result<bool> {
        buffer.resize(self.frame_size_bytes, 0);
        match self.stdout.read_exact(buffer) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Failed reading frame from ffmpeg: {}", e)),
        }
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct VideoEncoder {
    child: Child,
    stdin: ChildStdin,
}

impl VideoEncoder {
    pub fn spawn(
        original_video: Option<&Path>,
        output_path: &Path,
        width: u32,
        height: u32,
        fps: f32,
        crf: u32,
        preset: &str,
        copy_audio: bool,
    ) -> Result<Self> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-y"]); // Overwrite output file

        // Input 0: raw video pipe
        cmd.args([
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{}x{}", width, height),
            "-r",
            &format!("{:.3}", fps),
            "-i",
            "-",
        ]);

        let has_audio_source = copy_audio && original_video.is_some() && original_video.unwrap().exists();

        if has_audio_source {
            // Input 1: original video for audio
            cmd.arg("-i").arg(original_video.unwrap());
            cmd.args([
                "-map",
                "0:v:0",
                "-map",
                "1:a:0?", // map audio if exists, optional
                "-c:a",
                "copy",
            ]);
        } else {
            cmd.args(["-map", "0:v:0"]);
        }

        cmd.args([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            &crf.to_string(),
            "-preset",
            preset,
            "-shortest",
        ]);

        cmd.arg(output_path);

        cmd.stdin(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn().with_context(|| "Failed to spawn ffmpeg encoder")?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("Failed to capture ffmpeg stdin"))?;

        Ok(Self { child, stdin })
    }

    pub fn write_frame(&mut self, buffer: &[u8]) -> Result<()> {
        self.stdin
            .write_all(buffer)
            .with_context(|| "Failed to write frame to ffmpeg encoder")
    }

    pub fn finish(mut self) -> Result<()> {
        drop(self.stdin); // Close stdin to signal EOF to ffmpeg
        let status = self.child.wait().with_context(|| "Failed waiting for ffmpeg encoder")?;
        if !status.success() {
            bail!("ffmpeg encoder exited with error: {:?}", status.code());
        }
        Ok(())
    }
}

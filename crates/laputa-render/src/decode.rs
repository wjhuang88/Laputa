use std::{
    io::{self, Read},
    path::PathBuf,
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};

use anyhow::{Context, Result, bail, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewDecodeRequest {
    pub input_path: PathBuf,
    pub start_ms: u64,
    pub end_ms: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl PreviewDecodeRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.input_path.as_os_str().is_empty(),
            "preview decode input path must not be empty"
        );
        ensure!(
            self.start_ms < self.end_ms,
            "preview decode range must be positive"
        );
        ensure!(self.width > 0, "preview decode width must be positive");
        ensure!(self.height > 0, "preview decode height must be positive");
        ensure!(self.fps > 0, "preview decode frame rate must be positive");
        frame_byte_len(self.width, self.height)?;
        Ok(())
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.end_ms - self.start_ms
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewFrame {
    pub timeline_ms: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct FfmpegPreviewDecoder {
    ffmpeg: PathBuf,
}

impl Default for FfmpegPreviewDecoder {
    fn default() -> Self {
        Self::new("ffmpeg")
    }
}

impl FfmpegPreviewDecoder {
    #[must_use]
    pub fn new(ffmpeg: impl Into<PathBuf>) -> Self {
        Self {
            ffmpeg: ffmpeg.into(),
        }
    }

    pub fn open(&self, request: PreviewDecodeRequest) -> Result<PreviewDecodeSession> {
        request.validate()?;
        ensure!(
            request.input_path.is_file(),
            "preview decode input does not exist: {}",
            request.input_path.display()
        );

        let scale = format!(
            "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1",
            request.width, request.height, request.width, request.height
        );
        let fps = request.fps.to_string();
        let mut child = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error"])
            .args(["-ss", &format_seconds(request.start_ms)])
            .arg("-i")
            .arg(&request.input_path)
            .args(["-t", &format_seconds(request.duration_ms())])
            .args(["-an", "-vf", &scale, "-r", &fps])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start {} for preview decode",
                    self.ffmpeg.display()
                )
            })?;

        let stdout = child
            .stdout
            .take()
            .context("preview decoder did not expose stdout")?;
        let stderr = child
            .stderr
            .take()
            .context("preview decoder did not expose stderr")?;
        let reader = RawFrameReader::new(stdout, request.clone())?;
        Ok(PreviewDecodeSession {
            child,
            stderr: Some(stderr),
            reader,
            finished: false,
        })
    }
}

pub struct PreviewDecodeSession {
    child: Child,
    stderr: Option<ChildStderr>,
    reader: RawFrameReader<ChildStdout>,
    finished: bool,
}

impl PreviewDecodeSession {
    pub fn read_frame(&mut self) -> Result<Option<PreviewFrame>> {
        if self.finished {
            return Ok(None);
        }

        match self.reader.read_frame() {
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => {
                self.finish()?;
                Ok(None)
            }
            Err(error) => {
                let process_error = self.finish().err();
                match process_error {
                    Some(process_error) => Err(error).context(process_error.to_string()),
                    None => Err(error).context("failed to read preview frame"),
                }
            }
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        if self.finished {
            return Ok(());
        }
        if self.child.try_wait()?.is_none() {
            self.child
                .kill()
                .context("failed to stop preview decoder process")?;
        }
        let _ = self.child.wait();
        self.finished = true;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let status = self
            .child
            .wait()
            .context("failed to wait for preview decoder process")?;
        self.finished = true;
        if status.success() {
            return Ok(());
        }

        let mut stderr_text = String::new();
        if let Some(mut stderr) = self.stderr.take() {
            let _ = stderr.read_to_string(&mut stderr_text);
        }
        bail!("ffmpeg preview decode failed: {}", stderr_text.trim())
    }
}

impl Drop for PreviewDecodeSession {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        self.finished = true;
    }
}

struct RawFrameReader<R> {
    reader: R,
    request: PreviewDecodeRequest,
    frame_byte_len: usize,
    frame_index: u64,
}

impl<R: Read> RawFrameReader<R> {
    fn new(reader: R, request: PreviewDecodeRequest) -> Result<Self> {
        let frame_byte_len = frame_byte_len(request.width, request.height)?;
        Ok(Self {
            reader,
            request,
            frame_byte_len,
            frame_index: 0,
        })
    }

    fn read_frame(&mut self) -> io::Result<Option<PreviewFrame>> {
        let mut rgba = vec![0_u8; self.frame_byte_len];
        let mut filled = 0_usize;
        while filled < rgba.len() {
            let read = self.reader.read(&mut rgba[filled..])?;
            if read == 0 {
                if filled == 0 {
                    return Ok(None);
                }
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!(
                        "preview decoder ended after {filled} of {} frame bytes",
                        rgba.len()
                    ),
                ));
            }
            filled += read;
        }

        let offset_ms = self
            .frame_index
            .saturating_mul(1_000)
            .checked_div(u64::from(self.request.fps))
            .unwrap_or(0);
        let timeline_ms = self
            .request
            .start_ms
            .saturating_add(offset_ms)
            .min(self.request.end_ms);
        self.frame_index = self.frame_index.saturating_add(1);

        Ok(Some(PreviewFrame {
            timeline_ms,
            width: self.request.width,
            height: self.request.height,
            rgba,
        }))
    }
}

fn frame_byte_len(width: u32, height: u32) -> Result<usize> {
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .context("preview frame dimensions overflow addressable memory")?;
    pixels
        .checked_mul(4)
        .context("preview RGBA frame size overflows addressable memory")
}

fn format_seconds(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1_000, milliseconds % 1_000)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use anyhow::Result;

    use super::{PreviewDecodeRequest, RawFrameReader, frame_byte_len};

    fn request() -> PreviewDecodeRequest {
        PreviewDecodeRequest {
            input_path: "preview.mp4".into(),
            start_ms: 1_500,
            end_ms: 2_500,
            width: 2,
            height: 1,
            fps: 2,
        }
    }

    #[test]
    fn decode_request_validates_range_and_dimensions() -> Result<()> {
        request().validate()?;
        assert_eq!(request().duration_ms(), 1_000);
        assert_eq!(frame_byte_len(2, 1)?, 8);

        let mut invalid = request();
        invalid.end_ms = invalid.start_ms;
        assert!(invalid.validate().is_err());
        invalid = request();
        invalid.width = 0;
        assert!(invalid.validate().is_err());
        Ok(())
    }

    #[test]
    fn raw_reader_emits_fixed_rgba_frames_with_timeline_clock() -> Result<()> {
        let bytes = (0_u8..16).collect::<Vec<_>>();
        let mut reader = RawFrameReader::new(Cursor::new(bytes), request())?;

        let first = reader.read_frame()?.expect("first frame");
        let second = reader.read_frame()?.expect("second frame");

        assert_eq!(first.timeline_ms, 1_500);
        assert_eq!(second.timeline_ms, 2_000);
        assert_eq!(first.rgba.len(), 8);
        assert_eq!(second.rgba.len(), 8);
        assert!(reader.read_frame()?.is_none());
        Ok(())
    }

    #[test]
    fn raw_reader_rejects_truncated_frame() -> Result<()> {
        let mut reader = RawFrameReader::new(Cursor::new(vec![0_u8; 9]), request())?;
        assert!(reader.read_frame()?.is_some());
        let error = reader
            .read_frame()
            .expect_err("partial frame must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
        Ok(())
    }
}

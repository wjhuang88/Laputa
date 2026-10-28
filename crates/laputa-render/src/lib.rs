//! Deterministic preview/export media engine.
//!
//! This crate deliberately knows nothing about Talos, providers, FilmState, or GPUI. Callers
//! translate canonical Laputa state into simple media clip requests before invoking it.

mod decode;

pub use decode::{FfmpegPreviewDecoder, PreviewDecodeRequest, PreviewDecodeSession, PreviewFrame};

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewClip {
    Image {
        path: PathBuf,
        duration_ms: u64,
    },
    Video {
        path: PathBuf,
        source_start_ms: u64,
        source_end_ms: u64,
    },
}

impl PreviewClip {
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Image { path, .. } | Self::Video { path, .. } => path,
        }
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        match self {
            Self::Image { duration_ms, .. } => *duration_ms,
            Self::Video {
                source_start_ms,
                source_end_ms,
                ..
            } => source_end_ms.saturating_sub(*source_start_ms),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRenderRequest {
    pub clips: Vec<PreviewClip>,
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl PreviewRenderRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.clips.is_empty(), "preview requires at least one clip");
        ensure!(self.width > 0, "preview width must be greater than zero");
        ensure!(self.height > 0, "preview height must be greater than zero");
        ensure!(self.fps > 0, "preview frame rate must be greater than zero");
        ensure!(
            !self.output_path.as_os_str().is_empty(),
            "preview output path must not be empty"
        );

        for (index, clip) in self.clips.iter().enumerate() {
            ensure!(
                !clip.path().as_os_str().is_empty(),
                "preview clip {index} has an empty source path"
            );
            match clip {
                PreviewClip::Image { duration_ms, .. } => {
                    ensure!(
                        *duration_ms > 0,
                        "preview image clip {index} must have positive duration"
                    );
                }
                PreviewClip::Video {
                    source_start_ms,
                    source_end_ms,
                    ..
                } => {
                    ensure!(
                        source_start_ms < source_end_ms,
                        "preview video clip {index} has an empty or reversed source range"
                    );
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.clips.iter().fold(0_u64, |duration, clip| {
            duration.saturating_add(clip.duration_ms())
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewArtifact {
    pub path: PathBuf,
    pub duration_ms: u64,
    pub byte_len: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Story D does not yet define audio-mix semantics, so the proxy is intentionally silent.
    pub has_audio: bool,
}

#[derive(Clone, Debug)]
pub struct FfmpegPreviewRenderer {
    ffmpeg: PathBuf,
}

impl Default for FfmpegPreviewRenderer {
    fn default() -> Self {
        Self::new("ffmpeg")
    }
}

impl FfmpegPreviewRenderer {
    #[must_use]
    pub fn new(ffmpeg: impl Into<PathBuf>) -> Self {
        Self {
            ffmpeg: ffmpeg.into(),
        }
    }

    pub fn render(&self, request: &PreviewRenderRequest) -> Result<PreviewArtifact> {
        request.validate()?;
        for (index, clip) in request.clips.iter().enumerate() {
            ensure!(
                clip.path().is_file(),
                "preview clip {index} source does not exist: {}",
                clip.path().display()
            );
        }

        let output_parent = usable_parent(&request.output_path);
        fs::create_dir_all(output_parent).with_context(|| {
            format!(
                "failed to create preview output directory {}",
                output_parent.display()
            )
        })?;
        let work_dir = output_parent.join(unique_work_dir_name()?);
        fs::create_dir(&work_dir).with_context(|| {
            format!(
                "failed to create preview work directory {}",
                work_dir.display()
            )
        })?;

        let result = self.render_inner(request, &work_dir);
        let cleanup = fs::remove_dir_all(&work_dir);
        match (result, cleanup) {
            (Ok(artifact), Ok(())) => Ok(artifact),
            (Ok(_), Err(error)) => Err(error).with_context(|| {
                format!(
                    "failed to clean preview work directory {}",
                    work_dir.display()
                )
            }),
            (Err(error), _) => Err(error),
        }
    }

    fn render_inner(
        &self,
        request: &PreviewRenderRequest,
        work_dir: &Path,
    ) -> Result<PreviewArtifact> {
        let mut segments = Vec::with_capacity(request.clips.len());
        for (index, clip) in request.clips.iter().enumerate() {
            let segment = work_dir.join(format!("segment-{index:04}.mp4"));
            self.render_segment(request, clip, &segment)
                .with_context(|| format!("failed to render preview segment {index}"))?;
            ensure!(
                segment.is_file(),
                "ffmpeg reported success without producing preview segment {index}"
            );
            segments.push(segment);
        }

        let concat_file = work_dir.join("concat.txt");
        let concat_manifest = segments
            .iter()
            .map(|path| format!("file {}\n", ffconcat_quote(path)))
            .collect::<String>();
        fs::write(&concat_file, concat_manifest).with_context(|| {
            format!(
                "failed to write preview concat manifest {}",
                concat_file.display()
            )
        })?;

        let rendered = work_dir.join("preview.mp4");
        let output = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "concat", "-safe", "0", "-i"])
            .arg(&concat_file)
            .args(["-c", "copy", "-movflags", "+faststart"])
            .arg(&rendered)
            .output()
            .with_context(|| self.start_error())?;
        ensure_success(&self.ffmpeg, output)?;
        ensure!(
            rendered.is_file(),
            "ffmpeg reported success without producing preview output"
        );

        install_replacement(&rendered, &request.output_path)?;
        let byte_len = fs::metadata(&request.output_path)
            .with_context(|| {
                format!(
                    "failed to inspect rendered preview {}",
                    request.output_path.display()
                )
            })?
            .len();

        Ok(PreviewArtifact {
            path: request.output_path.clone(),
            duration_ms: request.duration_ms(),
            byte_len,
            width: request.width,
            height: request.height,
            fps: request.fps,
            has_audio: false,
        })
    }

    fn render_segment(
        &self,
        request: &PreviewRenderRequest,
        clip: &PreviewClip,
        output_path: &Path,
    ) -> Result<()> {
        let filter = format!(
            "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1",
            request.width, request.height, request.width, request.height
        );
        let fps = request.fps.to_string();
        let mut command = Command::new(&self.ffmpeg);
        command.args(["-hide_banner", "-loglevel", "error", "-y"]);

        match clip {
            PreviewClip::Image { path, duration_ms } => {
                command.args(["-loop", "1", "-i"]).arg(path);
                command.args(["-t", &format_seconds(*duration_ms)]);
            }
            PreviewClip::Video {
                path,
                source_start_ms,
                source_end_ms,
            } => {
                command.arg("-i").arg(path);
                command.args(["-ss", &format_seconds(*source_start_ms)]);
                command.args(["-t", &format_seconds(source_end_ms - source_start_ms)]);
            }
        }

        let output = command
            .args(["-vf", &filter])
            .args(["-r", &fps])
            .args(["-an", "-c:v", "libx264", "-preset", "veryfast"])
            .args(["-pix_fmt", "yuv420p", "-movflags", "+faststart"])
            .arg(output_path)
            .output()
            .with_context(|| self.start_error())?;
        ensure_success(&self.ffmpeg, output)
    }

    fn start_error(&self) -> String {
        format!(
            "failed to start {}; install FFmpeg or configure another preview renderer",
            self.ffmpeg.display()
        )
    }
}

fn ensure_success(executable: &Path, output: Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("{} failed: {}", executable.display(), stderr.trim())
}

fn usable_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn format_seconds(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1_000, milliseconds % 1_000)
}

fn unique_work_dir_name() -> Result<String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_nanos();
    Ok(format!(".laputa-preview-{}-{nanos}", std::process::id()))
}

fn ffconcat_quote(path: &Path) -> String {
    let value = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "'\\''");
    format!("'{value}'")
}

fn install_replacement(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => {
            fs::remove_file(destination).with_context(|| {
                format!(
                    "failed to replace preview {} after rename error: {error}",
                    destination.display()
                )
            })?;
            fs::rename(source, destination)
                .with_context(|| format!("failed to install preview {}", destination.display()))
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to install preview {}", destination.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use anyhow::Result;
    use tempfile::tempdir;

    use super::{
        FfmpegPreviewRenderer, PreviewClip, PreviewRenderRequest, ffconcat_quote, format_seconds,
    };

    #[test]
    fn request_rejects_invalid_clips_and_reports_duration() -> Result<()> {
        let request = PreviewRenderRequest {
            clips: vec![
                PreviewClip::Image {
                    path: "photo.jpg".into(),
                    duration_ms: 2_500,
                },
                PreviewClip::Video {
                    path: "clip.mp4".into(),
                    source_start_ms: 1_000,
                    source_end_ms: 4_750,
                },
            ],
            output_path: "preview.mp4".into(),
            width: 1280,
            height: 720,
            fps: 30,
        };
        request.validate()?;
        assert_eq!(request.duration_ms(), 6_250);

        let mut invalid = request.clone();
        invalid.clips[1] = PreviewClip::Video {
            path: "clip.mp4".into(),
            source_start_ms: 5_000,
            source_end_ms: 5_000,
        };
        assert!(invalid.validate().is_err());
        Ok(())
    }

    #[test]
    fn formatting_is_stable_for_ffmpeg_time_and_concat_paths() {
        assert_eq!(format_seconds(0), "0.000");
        assert_eq!(format_seconds(61_234), "61.234");
        assert_eq!(ffconcat_quote(Path::new("a'b.mp4")), "'a'\\''b.mp4'");
    }

    #[cfg(unix)]
    #[test]
    fn renderer_orchestrates_segments_and_atomically_installs_proxy() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir()?;
        let image = temp.path().join("photo.jpg");
        let video = temp.path().join("clip.mp4");
        fs::write(&image, b"image")?;
        fs::write(&video, b"video")?;

        let log = temp.path().join("ffmpeg.log");
        let fake_ffmpeg = temp.path().join("ffmpeg-fixture");
        let script = format!(
            "#!/bin/sh\necho \"$@\" >> {}\nfor last; do :; done\n: > \"$last\"\n",
            shell_quote(&log)
        );
        fs::write(&fake_ffmpeg, script)?;
        let mut permissions = fs::metadata(&fake_ffmpeg)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_ffmpeg, permissions)?;

        let output = temp.path().join("cache/preview/current.mp4");
        let request = PreviewRenderRequest {
            clips: vec![
                PreviewClip::Image {
                    path: image,
                    duration_ms: 2_000,
                },
                PreviewClip::Video {
                    path: video,
                    source_start_ms: 1_000,
                    source_end_ms: 4_000,
                },
            ],
            output_path: output.clone(),
            width: 960,
            height: 540,
            fps: 30,
        };

        let artifact = FfmpegPreviewRenderer::new(&fake_ffmpeg).render(&request)?;

        assert_eq!(artifact.path, output);
        assert_eq!(artifact.duration_ms, 5_000);
        assert!(!artifact.has_audio);
        assert!(artifact.path.is_file());
        let calls = fs::read_to_string(log)?;
        assert_eq!(calls.lines().count(), 3);
        assert!(calls.contains("-loop 1"));
        assert!(calls.contains("-ss 1.000"));
        assert!(calls.contains("-f concat"));
        Ok(())
    }

    #[cfg(unix)]
    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }
}

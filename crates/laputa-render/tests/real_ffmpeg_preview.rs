use std::{fs, process::Command};

use anyhow::{Context, Result, ensure};
use laputa_render::{
    FfmpegPreviewDecoder, FfmpegPreviewRenderer, PreviewClip, PreviewDecodeRequest,
    PreviewRenderRequest,
};
use tempfile::tempdir;

#[test]
#[ignore = "requires system FFmpeg; CI executes this integration fixture explicitly"]
fn real_ffmpeg_renders_mixed_media_preview_and_decodes_seek_frame() -> Result<()> {
    let temp = tempdir()?;
    let image = temp.path().join("photo.ppm");
    let video = temp.path().join("clip.mp4");
    let output = temp.path().join("preview/current.mp4");

    write_ppm_fixture(&image, 64, 48)?;
    generate_video_fixture(&video)?;

    let request = PreviewRenderRequest {
        clips: vec![
            PreviewClip::Image {
                path: image,
                duration_ms: 1_000,
            },
            PreviewClip::Video {
                path: video,
                source_start_ms: 500,
                source_end_ms: 1_500,
            },
        ],
        output_path: output.clone(),
        width: 320,
        height: 180,
        fps: 15,
    };

    let artifact = FfmpegPreviewRenderer::default()
        .render(&request)
        .context("real FFmpeg preview render failed")?;

    assert_eq!(artifact.path, output);
    assert_eq!(artifact.duration_ms, 2_000);
    assert_eq!(artifact.width, 320);
    assert_eq!(artifact.height, 180);
    assert_eq!(artifact.fps, 15);
    assert!(!artifact.has_audio);
    assert!(artifact.byte_len > 0);
    assert!(artifact.path.is_file());

    let mut decoder = FfmpegPreviewDecoder::default()
        .open(PreviewDecodeRequest {
            input_path: artifact.path,
            start_ms: 1_200,
            end_ms: 1_600,
            width: artifact.width,
            height: artifact.height,
            fps: artifact.fps,
        })
        .context("real FFmpeg preview decoder failed to open")?;

    let frame = decoder
        .read_frame()?
        .context("real FFmpeg preview decoder produced no frame")?;
    assert_eq!(frame.timeline_ms, 1_200);
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 180);
    assert_eq!(frame.rgba.len(), 320 * 180 * 4);
    decoder.stop()?;

    Ok(())
}

fn generate_video_fixture(path: &std::path::Path) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-f", "lavfi", "-i", "color=c=blue:s=96x54:r=10:d=2"])
        .args(["-an", "-c:v", "libx264", "-pix_fmt", "yuv420p"])
        .arg(path)
        .status()
        .context("failed to start FFmpeg fixture generator")?;
    ensure!(status.success(), "FFmpeg fixture generator failed");
    ensure!(path.is_file(), "FFmpeg fixture generator produced no video");
    Ok(())
}

fn write_ppm_fixture(path: &std::path::Path, width: usize, height: usize) -> Result<()> {
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let red = ((x * 255) / width.max(1)) as u8;
            let green = ((y * 255) / height.max(1)) as u8;
            bytes.extend_from_slice(&[red, green, 96]);
        }
    }
    fs::write(path, bytes)?;
    Ok(())
}

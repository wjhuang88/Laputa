#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt};

use anyhow::Result;
use laputa_render::{FfmpegPreviewDecoder, PreviewDecodeRequest};
use tempfile::tempdir;

#[test]
fn decoder_reads_raw_frames_and_uses_requested_seek_window() -> Result<()> {
    let temp = tempdir()?;
    let input = temp.path().join("preview.mp4");
    fs::write(&input, b"proxy")?;
    let log = temp.path().join("ffmpeg.log");
    let ffmpeg = temp.path().join("fake-ffmpeg");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\nprintf '\\001\\002\\003\\004\\005\\006\\007\\010\\011\\012\\013\\014\\015\\016\\017\\020'\n",
        shell_quote(&log)
    );
    write_executable(&ffmpeg, &script)?;

    let mut session = FfmpegPreviewDecoder::new(&ffmpeg).open(PreviewDecodeRequest {
        input_path: input,
        start_ms: 1_500,
        end_ms: 2_500,
        width: 2,
        height: 1,
        fps: 2,
    })?;

    let first = session.read_frame()?.expect("first raw frame");
    let second = session.read_frame()?.expect("second raw frame");
    assert!(session.read_frame()?.is_none());

    assert_eq!(first.timeline_ms, 1_500);
    assert_eq!(second.timeline_ms, 2_000);
    assert_eq!(first.rgba, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(second.rgba, vec![9, 10, 11, 12, 13, 14, 15, 16]);

    let arguments = fs::read_to_string(log)?;
    assert!(arguments.contains("-ss 1.500"));
    assert!(arguments.contains("-t 1.000"));
    assert!(arguments.contains("-f rawvideo"));
    assert!(arguments.contains("-pix_fmt rgba"));
    Ok(())
}

#[test]
fn decoder_surfaces_ffmpeg_failure_after_stdout_closes() -> Result<()> {
    let temp = tempdir()?;
    let input = temp.path().join("preview.mp4");
    fs::write(&input, b"proxy")?;
    let ffmpeg = temp.path().join("fake-ffmpeg-failure");
    write_executable(
        &ffmpeg,
        "#!/bin/sh\necho 'fixture decode failure' >&2\nexit 7\n",
    )?;

    let mut session = FfmpegPreviewDecoder::new(&ffmpeg).open(PreviewDecodeRequest {
        input_path: input,
        start_ms: 0,
        end_ms: 1_000,
        width: 1,
        height: 1,
        fps: 1,
    })?;

    let error = session
        .read_frame()
        .expect_err("non-zero ffmpeg exit must remain explicit");
    assert!(error.to_string().contains("fixture decode failure"));
    Ok(())
}

fn write_executable(path: &std::path::Path, contents: &str) -> Result<()> {
    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn shell_quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

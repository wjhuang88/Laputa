//! Development-only persisted project fixture for the Story D playback smoke.
//!
//! This exercises the normal ProjectStore/Ffprobe/FilmStore path. It does not create a
//! product entrypoint and intentionally requires system FFmpeg/ffprobe.

use std::{env, fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, ensure};
use laputa_media::{FfprobeMediaProbe, ProjectStore};
use laputa_timeline::{
    FilmDraft, FilmState, FilmStore, IntentDraft, SceneDraft, SceneSource, StoryBeatDraft,
    StoryDraft,
};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Laputa playback fixture generation failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let project_root = project_root()?;
    ensure!(
        !project_root.exists(),
        "fixture project path already exists: {}",
        project_root.display()
    );
    let source_root = project_root.with_extension("sources");
    ensure!(
        !source_root.exists(),
        "fixture source path already exists: {}",
        source_root.display()
    );
    fs::create_dir_all(&source_root)?;

    let image = source_root.join("kyoto-photo.png");
    let video = source_root.join("kyoto-walk.mp4");
    generate_image(&image)?;
    generate_video(&video)?;

    let store = ProjectStore::at(&project_root);
    store.create("京都播放校准")?;
    let report = store.import_media(&[image, video], &FfprobeMediaProbe)?;
    ensure!(
        report.imported.len() == 2,
        "expected two imported fixture assets"
    );

    let project = store.open()?;
    let photo = &report.imported[0];
    let walk = &report.imported[1];
    let film = FilmState::from_draft(
        "做一个简洁温暖的京都旅行短片",
        FilmDraft {
            intent: IntentDraft {
                summary: "从安静照片进入街头行走的京都旅行片段".to_owned(),
                target_duration_ms: Some(3_500),
                tone_tags: vec!["温暖".to_owned(), "自然".to_owned()],
            },
            story: StoryDraft {
                summary: "先建立地点，再进入移动中的街景".to_owned(),
                beats: vec![
                    StoryBeatDraft {
                        key: "arrival".to_owned(),
                        title: "抵达".to_owned(),
                        purpose: "用静态画面建立京都氛围".to_owned(),
                    },
                    StoryBeatDraft {
                        key: "walk".to_owned(),
                        title: "漫步".to_owned(),
                        purpose: "进入有运动感的街头片段".to_owned(),
                    },
                ],
            },
            scenes: vec![
                SceneDraft {
                    beat_key: "arrival".to_owned(),
                    asset_id: photo.id,
                    title: "京都静景".to_owned(),
                    source: SceneSource::Image { duration_ms: 1_500 },
                },
                SceneDraft {
                    beat_key: "walk".to_owned(),
                    asset_id: walk.id,
                    title: "街头漫步".to_owned(),
                    source: SceneSource::Video {
                        start_ms: 500,
                        end_ms: 2_500,
                    },
                },
            ],
        },
        &project,
    )?;
    FilmStore::at(&project_root).save(&project, &film)?;

    println!("{}", project_root.display());
    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .context("usage: workspace_fixture <project-dir>")?;
    ensure!(
        args.next().is_none(),
        "usage: workspace_fixture <project-dir>"
    );
    Ok(root)
}

fn generate_image(path: &std::path::Path) -> Result<()> {
    run_ffmpeg(
        Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "lavfi", "-i", "color=c=0xD8C9B6:s=640x360"])
            .args(["-frames:v", "1"])
            .arg(path),
        "image fixture",
    )
}

fn generate_video(path: &std::path::Path) -> Result<()> {
    run_ffmpeg(
        Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=640x360:rate=24:duration=3",
            ])
            .args(["-an", "-c:v", "libx264", "-pix_fmt", "yuv420p"])
            .arg(path),
        "video fixture",
    )
}

fn run_ffmpeg(command: &mut Command, label: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to start FFmpeg for {label}"))?;
    ensure!(status.success(), "FFmpeg failed to generate {label}");
    Ok(())
}

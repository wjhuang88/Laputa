//! Six-scene, real persisted FilmState for layout/interaction CI.
//!
//! Its generated patterned media is deliberately NOT photographic visual
//! acceptance evidence. It verifies the product's real asset and render paths.
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
            eprintln!("six-scene workspace fixture failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let root = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: workspace_visual_fixture <new-project-dir>")?;
    ensure!(
        env::args_os().len() == 2,
        "usage: workspace_visual_fixture <new-project-dir>"
    );
    let source_root = root.with_extension("sources");
    ensure!(
        !root.exists() && !source_root.exists(),
        "fixture project/source path must not already exist"
    );
    fs::create_dir_all(&source_root)?;

    let sources = [
        ("01-arrival.png", "testsrc2=size=640x360:rate=1", false),
        ("02-street.mp4", "testsrc=size=640x360:rate=24", true),
        ("03-portrait.png", "smptebars=size=640x360:rate=1", false),
        ("04-motion.mp4", "testsrc2=size=640x360:rate=24", true),
        ("05-sunset.png", "testsrc=size=640x360:rate=1", false),
        ("06-farewell.mp4", "smptebars=size=640x360:rate=24", true),
    ];

    let mut paths = Vec::new();
    for (name, filter, video) in sources {
        let path = source_root.join(name);
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "lavfi", "-i", filter]);
        if video {
            cmd.args(["-t", "3", "-an", "-c:v", "mpeg4", "-pix_fmt", "yuv420p"]);
        } else {
            cmd.args(["-frames:v", "1", "-update", "1"]);
        }
        let output = cmd
            .arg(&path)
            .output()
            .with_context(|| format!("failed to start FFmpeg for {name}"))?;
        ensure!(
            output.status.success(),
            "FFmpeg fixture {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        paths.push(path);
    }

    let store = ProjectStore::at(&root);
    store.create("六场景产品构图校准")?;
    let report = store.import_media(&paths, &FfprobeMediaProbe)?;
    ensure!(report.imported.len() == 6, "expected six assets");

    let titles = ["抵达", "街头", "人物", "行走", "日落", "告别"];
    let beats = titles
        .iter()
        .enumerate()
        .map(|(index, title)| StoryBeatDraft {
            key: format!("beat-{index}"),
            title: (*title).to_owned(),
            purpose: format!("第 {} 段的构图与缩略图检查", index + 1),
        })
        .collect();
    let scenes = report
        .imported
        .iter()
        .enumerate()
        .map(|(index, asset)| SceneDraft {
            beat_key: format!("beat-{index}"),
            asset_id: asset.id,
            title: titles[index].to_owned(),
            source: if index % 2 == 0 {
                SceneSource::Image { duration_ms: 1_500 }
            } else {
                SceneSource::Video {
                    start_ms: 0,
                    end_ms: 2_500,
                }
            },
        })
        .collect();

    let project = store.open()?;
    let film = FilmState::from_draft(
        "以六段非私人测试媒体确认实际产品布局",
        FilmDraft {
            intent: IntentDraft {
                summary: "检查六片段 Film Workspace 的布局和交互".to_owned(),
                target_duration_ms: Some(12_000),
                tone_tags: vec!["校准".to_owned()],
            },
            story: StoryDraft {
                summary: "仅用于实际产品的构图与媒体接线回归".to_owned(),
                beats,
            },
            scenes,
        },
        &project,
    )?;
    FilmStore::at(&root).save(&project, &film)?;
    println!("{}", root.display());
    Ok(())
}

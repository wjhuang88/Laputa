//! Development-only real preview playback calibration page.
//!
//! Unlike `workspace_visual`, this example opens a persisted Laputa project with FilmState,
//! renders the deterministic Story D proxy, decodes real RGBA frames, and drives the same
//! playback/selection state that the product Film Workspace will consume. It intentionally
//! remains an example until the #17 visual baseline review gate is accepted.

use std::{
    env,
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context as _, Result};
use gpui::{
    App, Bounds, Context, InteractiveElement, ObjectFit, Render, Window, WindowBounds,
    WindowOptions, div, img, prelude::*, px, rgb, size,
};
use laputa_app::{
    desktop_state::{AssetCard, DesktopState},
    preview_controller::PreviewController,
    preview_frame_image::{RenderedPreviewFrame, into_rendered_preview_frame},
    preview_playback::{PreviewPlaybackEvent, spawn_preview_playback},
    preview_player::PreviewPlayerState,
    preview_request::{PreviewProfile, preview_render_request},
    project_session::open_project_state,
    workspace_state::SceneStripItem,
};
use laputa_domain::MediaKind;
use laputa_render::{FfmpegPreviewDecoder, FfmpegPreviewRenderer, PreviewDecodeRequest};
use laputa_timeline::SceneId;

const CANVAS: u32 = 0xf3f0e8;
const PAPER: u32 = 0xfcfbf7;
const OAT: u32 = 0xe9e3d8;
const OAT_SOFT: u32 = 0xf0ece4;
const TEXT: u32 = 0x25373d;
const MUTED: u32 = 0x6b706d;
const HAIRLINE: u32 = 0xd8d3c9;
const BLUE: u32 = 0x6f8fa8;
const BLUE_SOFT: u32 = 0xdfe8ed;
const SAGE: u32 = 0xa9b5a0;
const CLAY: u32 = 0xc8795a;
const FJORD: u32 = 0x30464b;
const SAND: u32 = 0xd8c9b6;
const SKY_BLOCK: u32 = 0xc9d9e3;
const ERROR: u32 = 0xa6534c;

struct PlaybackWorkspace {
    state: DesktopState,
    controller: PreviewController,
    decoder: FfmpegPreviewDecoder,
    frame: Option<RenderedPreviewFrame>,
    error: Option<String>,
}

impl PlaybackWorkspace {
    fn load(root: PathBuf) -> Result<Self> {
        let state = open_project_state(&root)?;
        let workspace = state
            .workspace()
            .context("project has no First Cut FilmState; run Story C first")?;
        let output = root
            .join(".laputa")
            .join("cache")
            .join("preview")
            .join("current.mp4");
        let request = preview_render_request(workspace, output, PreviewProfile::default());
        let artifact = FfmpegPreviewRenderer::default()
            .render(&request)
            .context("failed to render deterministic preview proxy")?;
        let controller = PreviewController::new(PreviewPlayerState::new(artifact)?);
        let decoder = FfmpegPreviewDecoder::default();

        let mut session = decoder
            .open(controller.player().decode_request(0))
            .context("failed to decode initial preview frame")?;
        let frame = session
            .read_frame()?
            .map(into_rendered_preview_frame)
            .transpose()?;
        session.stop()?;

        Ok(Self {
            state,
            controller,
            decoder,
            frame,
            error: None,
        })
    }

    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        let is_playing = self
            .state
            .workspace()
            .is_some_and(|workspace| workspace.is_playing());
        if is_playing {
            let (controller, state) = (&mut self.controller, &mut self.state);
            if let Some(workspace) = state.workspace_mut() {
                controller.pause(workspace);
            }
            cx.notify();
            return;
        }

        let prepared = {
            let (controller, state) = (&mut self.controller, &mut self.state);
            state
                .workspace_mut()
                .map(|workspace| controller.prepare_play(workspace))
        };
        if let Some((generation, request)) = prepared {
            self.start_playback(generation, request, cx);
        }
    }

    fn seek_by(&mut self, delta_ms: i64, cx: &mut Context<Self>) {
        let Some(workspace) = self.state.workspace() else {
            return;
        };
        let current = workspace.playhead_ms();
        let duration = workspace.duration_ms();
        let target = if delta_ms.is_negative() {
            current.saturating_sub(delta_ms.unsigned_abs())
        } else {
            current
                .saturating_add(delta_ms.unsigned_abs())
                .min(duration)
        };
        self.seek_to(target, cx);
    }

    fn seek_to(&mut self, target_ms: u64, cx: &mut Context<Self>) {
        let prepared = {
            let (controller, state) = (&mut self.controller, &mut self.state);
            state
                .workspace_mut()
                .and_then(|workspace| controller.prepare_seek(workspace, target_ms))
        };
        match prepared {
            Some((generation, request)) => self.start_playback(generation, request, cx),
            None => cx.notify(),
        }
    }

    fn select_scene(&mut self, scene_id: SceneId, cx: &mut Context<Self>) {
        let target = match self.state.workspace_mut() {
            Some(workspace) => match workspace.select_scene(scene_id) {
                Ok(()) => workspace.playhead_ms(),
                Err(error) => {
                    self.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            },
            None => return,
        };
        self.seek_to(target, cx);
    }

    fn start_playback(
        &mut self,
        generation: u64,
        request: PreviewDecodeRequest,
        cx: &mut Context<Self>,
    ) {
        match spawn_preview_playback(self.decoder.clone(), request) {
            Ok((handle, mut receiver)) => {
                self.controller.attach_playback(generation, handle);
                self.error = None;
                cx.spawn(async move |this, cx| {
                    while let Some(event) = receiver.recv().await {
                        let terminal = !matches!(&event, PreviewPlaybackEvent::Frame(_));
                        let _ = this.update(cx, |this, cx| {
                            this.consume_event(generation, event);
                            cx.notify();
                        });
                        if terminal {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(error) => {
                self.error = Some(error.to_string());
                let (controller, state) = (&mut self.controller, &mut self.state);
                if let Some(workspace) = state.workspace_mut() {
                    let _ = controller.apply_event(
                        workspace,
                        generation,
                        PreviewPlaybackEvent::Failed(error.to_string()),
                    );
                }
                cx.notify();
            }
        }
    }

    fn consume_event(&mut self, generation: u64, event: PreviewPlaybackEvent) {
        let result = {
            let (controller, state) = (&mut self.controller, &mut self.state);
            let Some(workspace) = state.workspace_mut() else {
                return;
            };
            controller.apply_event(workspace, generation, event)
        };

        match result {
            Ok(Some(frame)) => match into_rendered_preview_frame(frame) {
                Ok(frame) => {
                    self.frame = Some(frame);
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            },
            Ok(None) => {
                self.error = self.controller.player().error().map(ToOwned::to_owned);
            }
            Err(error) => {
                self.error = Some(error.to_string());
                let (controller, state) = (&mut self.controller, &mut self.state);
                if let Some(workspace) = state.workspace_mut() {
                    controller.pause(workspace);
                }
            }
        }
    }

    fn moment_card(card: AssetCard, index: usize) -> impl IntoElement {
        let tone = match card.kind {
            MediaKind::Image => SAND,
            MediaKind::Video => SKY_BLOCK,
        };
        div()
            .id(("playback-moment", index))
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .w_full()
                    .h(px(74.))
                    .rounded_md()
                    .bg(rgb(tone))
                    .flex()
                    .items_end()
                    .p_2()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(PAPER))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(card.kind_label),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(card.display_name),
            )
    }

    fn moments_rail(&self) -> impl IntoElement {
        let cards = self.state.asset_cards();
        div()
            .w(px(208.))
            .min_h_0()
            .flex_shrink_0()
            .bg(rgb(OAT))
            .border_r_1()
            .border_color(rgb(HAIRLINE))
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .pt_5()
                    .pb_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child("片段素材"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(cards.len().to_string()),
                    ),
            )
            .child(
                div()
                    .id("playback-moments-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .children(
                        cards
                            .into_iter()
                            .enumerate()
                            .map(|(index, card)| Self::moment_card(card, index)),
                    ),
            )
    }

    fn stage(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (playhead_ms, duration_ms, playing, scene_title) = self
            .state
            .workspace()
            .map(|workspace| {
                (
                    workspace.playhead_ms(),
                    workspace.duration_ms(),
                    workspace.is_playing(),
                    workspace.scene_at_playhead().title.clone(),
                )
            })
            .unwrap_or((0, 0, false, "无场景".to_owned()));
        let progress_width = if duration_ms == 0 {
            0.0
        } else {
            360.0 * (playhead_ms as f32 / duration_ms as f32)
        };

        let preview = if let Some(frame) = &self.frame {
            div()
                .w_full()
                .h_full()
                .bg(rgb(FJORD))
                .child(
                    img(frame.image.clone())
                        .object_fit(ObjectFit::Contain)
                        .size_full(),
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .bg(rgb(FJORD))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0xffffff))
                .child("等待预览帧")
                .into_any_element()
        };

        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .px_6()
            .pt_5()
            .pb_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(940.))
                            .h_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .rounded_lg()
                                    .overflow_hidden()
                                    .bg(rgb(FJORD))
                                    .child(preview),
                            )
                            .child(
                                div()
                                    .h(px(40.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id("playback-toggle")
                                            .w(px(34.))
                                            .h(px(34.))
                                            .rounded_full()
                                            .bg(rgb(TEXT))
                                            .cursor_pointer()
                                            .text_color(rgb(0xffffff))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(if playing { "Ⅱ" } else { "▶" })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.toggle_playback(cx);
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("seek-back-five")
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child("−5 秒")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.seek_by(-5_000, cx);
                                            })),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child(format_time(playhead_ms)),
                                    )
                                    .child(
                                        div()
                                            .w(px(360.))
                                            .max_w_full()
                                            .h(px(4.))
                                            .rounded_full()
                                            .bg(rgb(HAIRLINE))
                                            .child(
                                                div()
                                                    .w(px(progress_width))
                                                    .h(px(4.))
                                                    .rounded_full()
                                                    .bg(rgb(BLUE)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child(format_time(duration_ms)),
                                    )
                                    .child(
                                        div()
                                            .id("seek-forward-five")
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child("+5 秒")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.seek_by(5_000, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .h(px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(rgb(SAGE)))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child(format!("真实预览 · {scene_title}")),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(BLUE))
                            .child("DEV playback binding"),
                    ),
            )
    }

    fn director_rail(&self) -> impl IntoElement {
        let workspace = self.state.workspace();
        let scene = workspace.map(|workspace| {
            workspace
                .selected_scene()
                .and_then(|selected| {
                    workspace
                        .scene_strip()
                        .iter()
                        .find(|scene| scene.scene_id == selected)
                })
                .unwrap_or_else(|| workspace.scene_at_playhead())
        });
        let target = workspace
            .map(|workspace| format!("{:?}", workspace.command_context().default_target))
            .unwrap_or_else(|| "Project".to_owned());

        div()
            .w(px(266.))
            .min_h_0()
            .flex_shrink_0()
            .bg(rgb(PAPER))
            .border_l_1()
            .border_color(rgb(HAIRLINE))
            .px_5()
            .pt_5()
            .pb_4()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xl().child("Laputa"))
                    .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(rgb(CLAY))),
            )
            .when_some(scene, |view, scene| {
                view.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().text_sm().text_color(rgb(MUTED)).child("当前片段"))
                        .child(div().text_xl().child(scene.title.clone()))
                        .child(div().text_sm().text_color(rgb(MUTED)).child(format!(
                            "{} · {}",
                            scene.display_name,
                            format_time(scene.duration_ms())
                        ))),
                )
            })
            .child(
                div()
                    .rounded_lg()
                    .bg(rgb(OAT_SOFT))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("默认编辑目标"))
                    .child(div().text_sm().text_color(rgb(TEXT)).child(target)),
            )
            .child(
                div().text_sm().text_color(rgb(MUTED)).child(
                    "播放游标跨场景不会改变编辑 Selection；点击底部场景才会更新默认编辑目标。",
                ),
            )
            .child(div().flex_1())
            .when_some(self.error.as_deref(), |view, error| {
                view.child(
                    div()
                        .p_3()
                        .rounded_md()
                        .bg(rgb(0xf4e3df))
                        .text_sm()
                        .text_color(rgb(ERROR))
                        .child(error.to_owned()),
                )
            })
    }

    fn scene_card(
        &self,
        scene: SceneStripItem,
        index: usize,
        selected: bool,
        current: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let tone = match scene.media_kind {
            MediaKind::Image => SAND,
            MediaKind::Video => SKY_BLOCK,
        };
        let scene_id = scene.scene_id;
        let duration = format_time(scene.duration_ms());
        let title = scene.title;
        let card = div()
            .id(("playback-scene", index))
            .w(px(154.))
            .h(px(76.))
            .flex_shrink_0()
            .rounded_md()
            .cursor_pointer()
            .border_1()
            .p_2()
            .flex()
            .gap_2();
        let card = if selected {
            card.bg(rgb(PAPER)).border_color(rgb(BLUE))
        } else if current {
            card.bg(rgb(BLUE_SOFT)).border_color(rgb(HAIRLINE))
        } else {
            card.bg(rgb(OAT_SOFT)).border_color(rgb(HAIRLINE))
        };

        card.child(
            div()
                .w(px(50.))
                .h_full()
                .rounded_md()
                .bg(rgb(tone))
                .flex()
                .items_end()
                .p_1()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(format!("{:02}", index + 1)),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .justify_between()
                .child(div().text_sm().child(title))
                .child(div().text_sm().text_color(rgb(MUTED)).child(duration)),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_scene(scene_id, cx);
        }))
        .into_any_element()
    }

    fn scene_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(workspace) = self.state.workspace() else {
            return div().h(px(112.)).into_any_element();
        };
        let scenes = workspace.scene_strip().to_vec();
        let selected = workspace.selected_scene();
        let current = workspace.scene_at_playhead().scene_id;
        let count = scenes.len();
        let mut cards = Vec::with_capacity(count);
        for (index, scene) in scenes.into_iter().enumerate() {
            let is_selected = selected == Some(scene.scene_id);
            let is_current = current == scene.scene_id;
            cards.push(self.scene_card(scene, index, is_selected, is_current, cx));
        }

        div()
            .h(px(112.))
            .flex_shrink_0()
            .bg(rgb(PAPER))
            .border_t_1()
            .border_color(rgb(HAIRLINE))
            .px_5()
            .flex()
            .items_center()
            .gap_4()
            .child(
                div()
                    .w(px(62.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("场景"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(count.to_string()),
                    ),
            )
            .child(
                div()
                    .id("playback-scenes-scroll")
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .flex()
                    .gap_3()
                    .children(cards),
            )
            .into_any_element()
    }

    fn top_bar(&self) -> impl IntoElement {
        let project_name = self
            .state
            .project()
            .map_or_else(|| "未命名影片".to_owned(), |project| project.name.clone());
        div()
            .h(px(58.))
            .flex_shrink_0()
            .bg(rgb(PAPER))
            .border_b_1()
            .border_color(rgb(HAIRLINE))
            .px_5()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("‹"))
                    .child(div().text_xl().child(project_name)),
            )
            .child(
                div()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(BLUE_SOFT))
                    .text_sm()
                    .text_color(rgb(BLUE))
                    .child("真实播放校准"),
            )
    }
}

impl Render for PlaybackWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(self.top_bar())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.moments_rail())
                    .child(self.stage(cx))
                    .child(self.director_rail()),
            )
            .child(self.scene_strip(cx))
    }
}

fn format_time(milliseconds: u64) -> String {
    let total_seconds = milliseconds / 1_000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

#[cfg(target_os = "linux")]
fn has_display() -> bool {
    [env::var_os("DISPLAY"), env::var_os("WAYLAND_DISPLAY")]
        .into_iter()
        .flatten()
        .any(|value| !value.is_empty())
}

fn run(workspace: PlaybackWorkspace) -> std::process::ExitCode {
    #[cfg(target_os = "linux")]
    if !has_display() {
        eprintln!("Laputa playback calibration unavailable: DISPLAY/WAYLAND_DISPLAY is missing");
        return std::process::ExitCode::FAILURE;
    }

    let failed = Arc::new(AtomicBool::new(false));
    let launch_failed = failed.clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        gpui_platform::application().run(move |cx: &mut App| {
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(1100.), px(700.))),
                    ..Default::default()
                },
                |_window, cx| cx.new(|_| workspace),
            ) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    launch_failed.store(true, Ordering::Relaxed);
                    eprintln!("Laputa playback calibration window failed: {error:#}");
                    cx.quit();
                }
            }
        });
    }));

    if result.is_err() || failed.load(Ordering::Relaxed) {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}

fn project_root() -> Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    let root = args.next().map(PathBuf::from).context(
        "usage: cargo run -p laputa-app --features desktop-ui --example workspace_playback -- <project-dir>",
    )?;
    if args.next().is_some() {
        anyhow::bail!(
            "usage: cargo run -p laputa-app --features desktop-ui --example workspace_playback -- <project-dir>"
        );
    }
    Ok(root)
}

fn main() -> std::process::ExitCode {
    let result = project_root().and_then(PlaybackWorkspace::load);
    match result {
        Ok(workspace) => run(workspace),
        Err(error) => {
            eprintln!("Laputa playback calibration failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

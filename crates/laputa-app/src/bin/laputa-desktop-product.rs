use std::{
    env,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
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
    desktop_state::{AssetCard, AssetFilter, DesktopOperation, DesktopState},
    preview_controller::PreviewController,
    preview_frame_image::{RenderedPreviewFrame, into_rendered_preview_frame},
    preview_playback::{PreviewPlaybackEvent, spawn_preview_playback},
    preview_player::PreviewPlayerState,
    preview_request::{PreviewProfile, preview_render_request},
    project_session::open_project_state,
    workspace_state::{DefaultEditTarget, SceneStripItem},
};
use laputa_domain::MediaKind;
use laputa_media::{FfprobeMediaProbe, ProjectStore};
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
const SUCCESS: u32 = 0x5f806b;

#[derive(Clone, Copy)]
enum UiCommand {
    NewProject,
    OpenProject,
    AddMedia,
    Filter(AssetFilter),
    RetryPreview,
}

#[derive(Clone, Copy)]
enum ProjectPromptMode {
    Create,
    Open,
}

struct DesktopWindow {
    state: DesktopState,
    controller: Option<PreviewController>,
    decoder: FfmpegPreviewDecoder,
    frame: Option<RenderedPreviewFrame>,
    preview_error: Option<String>,
    preview_preparing: bool,
}

impl DesktopWindow {
    fn new(state: DesktopState) -> Self {
        Self {
            state,
            controller: None,
            decoder: FfmpegPreviewDecoder::default(),
            frame: None,
            preview_error: None,
            preview_preparing: false,
        }
    }

    fn execute(&mut self, command: UiCommand, cx: &mut Context<Self>) {
        match command {
            UiCommand::NewProject => self.choose_project(ProjectPromptMode::Create, cx),
            UiCommand::OpenProject => self.choose_project(ProjectPromptMode::Open, cx),
            UiCommand::AddMedia => self.choose_media(cx),
            UiCommand::Filter(filter) => {
                self.state.set_filter(filter);
                cx.notify();
            }
            UiCommand::RetryPreview => {
                self.controller = None;
                self.frame = None;
                self.preview_error = None;
                self.preview_preparing = false;
                self.prepare_preview(cx);
            }
        }
    }

    fn reset_preview(&mut self) {
        if let (Some(controller), Some(workspace)) =
            (self.controller.as_mut(), self.state.workspace_mut())
        {
            controller.pause(workspace);
        }
        self.controller = None;
        self.frame = None;
        self.preview_error = None;
        self.preview_preparing = false;
    }

    fn choose_project(&mut self, mode: ProjectPromptMode, cx: &mut Context<Self>) {
        if !self.state.begin(DesktopOperation::ChoosingProject) {
            return;
        }
        let prompt_text = match mode {
            ProjectPromptMode::Create => "选择一个文件夹作为影片项目",
            ProjectPromptMode::Open => "选择已有的 Laputa 项目文件夹",
        };
        let prompt = catch_unwind(AssertUnwindSafe(|| {
            cx.prompt_for_paths(gpui::PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(prompt_text.into()),
            })
        }));

        match prompt {
            Ok(prompt) => {
                cx.spawn(async move |this, cx| {
                    let result = prompt.await;
                    let _ = this.update(cx, |this, cx| {
                        let path = match result {
                            Ok(Ok(Some(paths))) => paths.into_iter().next(),
                            Ok(Ok(None)) => {
                                this.state.cancel_operation();
                                cx.notify();
                                return;
                            }
                            _ => {
                                this.state.fail("无法打开系统文件夹选择器，请重试");
                                cx.notify();
                                return;
                            }
                        };
                        let Some(path) = path else {
                            this.state.cancel_operation();
                            cx.notify();
                            return;
                        };
                        this.load_project(path, mode, cx);
                    });
                })
                .detach();
            }
            Err(_) => {
                self.state.fail("无法打开系统文件夹选择器，请重试");
                cx.notify();
            }
        }
    }

    fn load_project(&mut self, root: PathBuf, mode: ProjectPromptMode, cx: &mut Context<Self>) {
        self.state.cancel_operation();
        let operation = match mode {
            ProjectPromptMode::Create => DesktopOperation::CreatingProject,
            ProjectPromptMode::Open => DesktopOperation::OpeningProject,
        };
        if !self.state.begin(operation) {
            return;
        }

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    match mode {
                        ProjectPromptMode::Create => {
                            let store = ProjectStore::at(&root);
                            let project = store
                                .create(&project_name_from_root(&root))
                                .map_err(|error| error.to_string())?;
                            Ok::<_, String>(DesktopState::from_project(root, project))
                        }
                        ProjectPromptMode::Open => {
                            open_project_state(&root).map_err(|error| error.to_string())
                        }
                    }
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(state) => {
                        this.reset_preview();
                        this.state = state;
                    }
                    Err(error) => this.state.fail(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_media(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.state.project_root().map(Path::to_path_buf) else {
            self.state.fail("请先创建或打开一个影片项目");
            cx.notify();
            return;
        };
        if !self.state.begin(DesktopOperation::ChoosingMedia) {
            return;
        }

        let prompt = catch_unwind(AssertUnwindSafe(|| {
            cx.prompt_for_paths(gpui::PathPromptOptions {
                files: true,
                directories: false,
                multiple: true,
                prompt: Some("选择照片和视频".into()),
            })
        }));

        match prompt {
            Ok(prompt) => {
                cx.spawn(async move |this, cx| {
                    let result = prompt.await;
                    let _ = this.update(cx, |this, cx| {
                        let paths = match result {
                            Ok(Ok(Some(paths))) if !paths.is_empty() => paths,
                            Ok(Ok(_)) => {
                                this.state.cancel_operation();
                                cx.notify();
                                return;
                            }
                            _ => {
                                this.state.fail("无法读取系统文件选择结果，请重试");
                                cx.notify();
                                return;
                            }
                        };
                        this.import_media(root, paths, cx);
                    });
                })
                .detach();
            }
            Err(_) => {
                self.state.fail("无法打开系统文件选择器，请重试");
                cx.notify();
            }
        }
    }

    fn import_media(&mut self, root: PathBuf, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.state.cancel_operation();
        if !self.state.begin(DesktopOperation::ImportingMedia) {
            return;
        }

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    ProjectStore::at(root)
                        .import_media(&paths, &FfprobeMediaProbe)
                        .map_err(|error| error.to_string())
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(report) => this
                        .state
                        .import_completed(report.project, report.imported.len()),
                    Err(error) => this.state.fail(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn prepare_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview_preparing || self.controller.is_some() || self.state.workspace().is_none() {
            return;
        }
        let Some(root) = self.state.project_root().map(Path::to_path_buf) else {
            return;
        };
        let Some(workspace) = self.state.workspace().cloned() else {
            return;
        };
        self.preview_preparing = true;
        self.preview_error = None;

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let output = root
                        .join(".laputa")
                        .join("cache")
                        .join("preview")
                        .join("current.mp4");
                    let request =
                        preview_render_request(&workspace, output, PreviewProfile::default());
                    FfmpegPreviewRenderer::default()
                        .render(&request)
                        .map_err(|error| error.to_string())
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                this.preview_preparing = false;
                match result {
                    Ok(artifact) => match PreviewPlayerState::new(artifact) {
                        Ok(player) => {
                            let controller = PreviewController::new(player);
                            let initial = controller.player().decode_request(0);
                            match this.decode_one_frame(initial) {
                                Ok(frame) => {
                                    this.controller = Some(controller);
                                    this.frame = frame;
                                    this.preview_error = None;
                                }
                                Err(error) => this.preview_error = Some(error.to_string()),
                            }
                        }
                        Err(error) => this.preview_error = Some(error.to_string()),
                    },
                    Err(error) => this.preview_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn decode_one_frame(
        &self,
        request: PreviewDecodeRequest,
    ) -> Result<Option<RenderedPreviewFrame>> {
        let mut session = self.decoder.open(request)?;
        let frame = session
            .read_frame()?
            .map(into_rendered_preview_frame)
            .transpose()?;
        session.stop()?;
        Ok(frame)
    }

    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        let Some(controller) = self.controller.as_mut() else {
            return;
        };
        let is_playing = self
            .state
            .workspace()
            .is_some_and(|workspace| workspace.is_playing());
        if is_playing {
            if let Some(workspace) = self.state.workspace_mut() {
                controller.pause(workspace);
            }
            cx.notify();
            return;
        }

        let prepared = self
            .state
            .workspace_mut()
            .map(|workspace| controller.prepare_play(workspace));
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
        let Some(controller) = self.controller.as_mut() else {
            return;
        };
        let prepared = self
            .state
            .workspace_mut()
            .and_then(|workspace| controller.prepare_seek(workspace, target_ms));
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
                    self.preview_error = Some(error.to_string());
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
        let Some(controller) = self.controller.as_mut() else {
            return;
        };
        match spawn_preview_playback(self.decoder.clone(), request) {
            Ok((handle, mut receiver)) => {
                controller.attach_playback(generation, handle);
                self.preview_error = None;
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
                self.preview_error = Some(error.to_string());
                if let Some(workspace) = self.state.workspace_mut() {
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
        let Some(controller) = self.controller.as_mut() else {
            return;
        };
        let result = {
            let Some(workspace) = self.state.workspace_mut() else {
                return;
            };
            controller.apply_event(workspace, generation, event)
        };

        match result {
            Ok(Some(frame)) => match into_rendered_preview_frame(frame) {
                Ok(frame) => {
                    self.frame = Some(frame);
                    self.preview_error = None;
                }
                Err(error) => self.preview_error = Some(error.to_string()),
            },
            Ok(None) => {
                self.preview_error = controller.player().error().map(ToOwned::to_owned);
            }
            Err(error) => {
                self.preview_error = Some(error.to_string());
                if let Some(workspace) = self.state.workspace_mut() {
                    controller.pause(workspace);
                }
            }
        }
    }

    fn action_button(
        &self,
        id: &'static str,
        label: &'static str,
        command: UiCommand,
        primary: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let button = div()
            .id(id)
            .px_4()
            .py_2()
            .rounded_md()
            .border_1()
            .cursor_pointer()
            .text_sm();
        let button = if primary {
            button
                .bg(rgb(BLUE))
                .border_color(rgb(BLUE))
                .text_color(rgb(0xffffff))
        } else {
            button
                .bg(rgb(PAPER))
                .border_color(rgb(HAIRLINE))
                .text_color(rgb(TEXT))
        };
        button
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.execute(command, cx)))
    }

    fn filter_button(
        &self,
        filter: AssetFilter,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.state.filter() == filter;
        let button = div()
            .id(("asset-filter", index))
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .child(filter.label());
        let button = if selected {
            button.bg(rgb(BLUE_SOFT)).text_color(rgb(BLUE))
        } else {
            button.text_color(rgb(MUTED))
        };
        button.on_click(cx.listener(move |this, _, _, cx| {
            this.execute(UiCommand::Filter(filter), cx);
        }))
    }

    fn launcher(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.state.operation().is_busy();
        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(60.))
                    .px_8()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(9.)).h(px(9.)).rounded_full().bg(rgb(CLAY)))
                            .child("Laputa"),
                    )
                    .child(div().text_sm().text_color(rgb(MUTED)).child("V0.1")),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_8()
                    .pb_8()
                    .flex()
                    .gap_8()
                    .child(
                        div()
                            .flex_1()
                            .rounded_lg()
                            .bg(rgb(PAPER))
                            .border_1()
                            .border_color(rgb(HAIRLINE))
                            .p_8()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap_5()
                            .child(div().text_3xl().child("把片段变成故事"))
                            .child(div().text_color(rgb(MUTED)).child(
                                "从照片和视频开始，用一句话得到初剪，再从预览里继续塑造故事。",
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(self.action_button(
                                        "new-project",
                                        if busy {
                                            "处理中…"
                                        } else {
                                            "＋ 新建影片"
                                        },
                                        UiCommand::NewProject,
                                        true,
                                        cx,
                                    ))
                                    .child(self.action_button(
                                        "open-project",
                                        "打开已有项目",
                                        UiCommand::OpenProject,
                                        false,
                                        cx,
                                    )),
                            )
                            .when_some(self.state.error(), |view, error| {
                                view.child(
                                    div()
                                        .p_3()
                                        .rounded_md()
                                        .bg(rgb(0xf4e3df))
                                        .text_sm()
                                        .text_color(rgb(ERROR))
                                        .child(error.to_owned()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w(px(330.))
                            .flex_shrink_0()
                            .rounded_lg()
                            .bg(rgb(OAT))
                            .p_7()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(div().text_xl().child("Laputa"))
                                    .child(
                                        div()
                                            .text_color(rgb(MUTED))
                                            .child("添加 → 描述 → 初剪 → 观看 → 选中 → 修改"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("Moments → Story → Film"),
                            ),
                    ),
            )
    }

    fn import_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project = self
            .state
            .project()
            .expect("project workspace requires an open project");
        let cards = self.state.asset_cards();
        let asset_count = self.state.asset_count();

        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(self.product_top_bar("素材准备"))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.moments_rail(cards, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p_6()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(620.))
                                    .max_w_full()
                                    .rounded_lg()
                                    .bg(rgb(PAPER))
                                    .border_1()
                                    .border_color(rgb(HAIRLINE))
                                    .p_8()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_4()
                                    .child(
                                        div()
                                            .w(px(56.))
                                            .h(px(56.))
                                            .rounded_lg()
                                            .bg(rgb(BLUE_SOFT))
                                            .text_color(rgb(BLUE))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_xl()
                                            .child(if asset_count == 0 { "＋" } else { "✓" }),
                                    )
                                    .child(div().text_2xl().child(if asset_count == 0 {
                                        "把照片和视频添加到这里"
                                    } else {
                                        "素材已经准备好"
                                    }))
                                    .child(div().text_color(rgb(MUTED)).child(format!(
                                        "{} · {} 个素材 · 原始文件保持只读引用",
                                        project.name, asset_count
                                    )))
                                    .child(self.action_button(
                                        "add-media-stage",
                                        if self.state.operation().is_busy() {
                                            "处理中…"
                                        } else {
                                            "＋ 添加素材"
                                        },
                                        UiCommand::AddMedia,
                                        true,
                                        cx,
                                    ))
                                    .when_some(self.state.notice(), |view, notice| {
                                        view.child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(SUCCESS))
                                                .child(notice.to_owned()),
                                        )
                                    })
                                    .when_some(self.state.error(), |view, error| {
                                        view.child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(ERROR))
                                                .child(error.to_owned()),
                                        )
                                    }),
                            ),
                    ),
            )
    }

    fn product_top_bar(&self, status: &'static str) -> impl IntoElement {
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
                    .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(rgb(CLAY)))
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
                    .child(status),
            )
    }

    fn moments_rail(&self, cards: Vec<AssetCard>, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
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
                            .flex()
                            .gap_1()
                            .child(self.filter_button(AssetFilter::All, 0, cx))
                            .child(self.filter_button(AssetFilter::Photos, 1, cx))
                            .child(self.filter_button(AssetFilter::Videos, 2, cx)),
                    )
                    .child(self.action_button(
                        "add-media-sidebar",
                        "＋ 添加素材",
                        UiCommand::AddMedia,
                        false,
                        cx,
                    )),
            )
            .child(
                div()
                    .id("product-moments-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(cards.into_iter().enumerate().map(|(index, card)| {
                        let tone = match card.kind {
                            MediaKind::Image => SAND,
                            MediaKind::Video => SKY_BLOCK,
                        };
                        div()
                            .id(("product-moment", index))
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
                                            .child(card.kind_label),
                                    ),
                            )
                            .child(div().text_sm().child(card.display_name))
                    })),
            )
    }

    fn film_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let cards = self.state.asset_cards();
        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(self.product_top_bar("初剪已完成"))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.moments_rail(cards, cx))
                    .child(self.preview_stage(cx))
                    .child(self.director_rail()),
            )
            .child(self.scene_strip(cx))
    }

    fn preview_stage(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                .size_full()
                .bg(rgb(FJORD))
                .child(
                    img(frame.image.clone())
                        .object_fit(ObjectFit::Contain)
                        .size_full(),
                )
                .into_any_element()
        } else {
            let label = if self.preview_preparing {
                "正在生成预览…"
            } else if self.preview_error.is_some() {
                "预览生成失败"
            } else {
                "准备预览…"
            };
            div()
                .size_full()
                .bg(rgb(FJORD))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .text_color(rgb(0xffffff))
                .child(label)
                .when(self.preview_error.is_some(), |view| {
                    view.child(self.action_button(
                        "retry-preview",
                        "重新生成预览",
                        UiCommand::RetryPreview,
                        false,
                        cx,
                    ))
                })
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
                                            .id("product-playback-toggle")
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
                                            .id("product-seek-back")
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
                                            .id("product-seek-forward")
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
                    .when_some(self.preview_error.as_deref(), |view, error| {
                        view.child(
                            div()
                                .text_sm()
                                .text_color(rgb(ERROR))
                                .child(error.to_owned()),
                        )
                    }),
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
            .map(
                |workspace| match workspace.command_context().default_target {
                    DefaultEditTarget::Project => "整部影片".to_owned(),
                    DefaultEditTarget::Scene(_) => "当前选中片段".to_owned(),
                },
            )
            .unwrap_or_else(|| "整部影片".to_owned());

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
                    .child(div().text_sm().child(target)),
            )
            .child(
                div()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(HAIRLINE))
                    .bg(rgb(PAPER))
                    .p_3()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("告诉 Laputa 你的想法…"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("点击底部场景会更新默认编辑目标；播放跨场景不会改变 Selection。"),
            )
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
            .id(("product-scene", index))
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
        .on_click(cx.listener(move |this, _, _, cx| this.select_scene(scene_id, cx)))
        .into_any_element()
    }

    fn scene_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self
            .state
            .workspace()
            .expect("Film Workspace requires WorkspaceState");
        let scenes = workspace.scene_strip().to_vec();
        let selected = workspace.selected_scene();
        let current = workspace.scene_at_playhead().scene_id;
        let count = scenes.len();
        let cards = scenes
            .into_iter()
            .enumerate()
            .map(|(index, scene)| {
                self.scene_card(
                    scene.clone(),
                    index,
                    selected == Some(scene.scene_id),
                    current == scene.scene_id,
                    cx,
                )
            })
            .collect::<Vec<_>>();

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
                    .child(div().text_sm().child(count.to_string())),
            )
            .child(
                div()
                    .id("product-scenes-scroll")
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .flex()
                    .gap_3()
                    .children(cards),
            )
    }
}

impl Render for DesktopWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.state.workspace().is_some()
            && self.controller.is_none()
            && !self.preview_preparing
            && self.preview_error.is_none()
        {
            self.prepare_preview(cx);
        }

        if self.state.project().is_none() {
            self.launcher(cx).into_any_element()
        } else if self.state.workspace().is_some() {
            self.film_workspace(cx).into_any_element()
        } else {
            self.import_workspace(cx).into_any_element()
        }
    }
}

fn format_time(milliseconds: u64) -> String {
    let total_seconds = milliseconds / 1_000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

fn project_name_from_root(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("未命名影片")
        .to_owned()
}

fn initial_state() -> Result<DesktopState> {
    let mut args = env::args_os().skip(1);
    let Some(root) = args.next() else {
        return Ok(DesktopState::default());
    };
    if args.next().is_some() {
        anyhow::bail!("usage: laputa-desktop [project-dir]");
    }
    open_project_state(PathBuf::from(root))
}

#[cfg(target_os = "linux")]
fn has_display() -> bool {
    [env::var_os("DISPLAY"), env::var_os("WAYLAND_DISPLAY")]
        .into_iter()
        .flatten()
        .any(|value| !value.is_empty())
}

fn run(state: DesktopState) -> std::process::ExitCode {
    #[cfg(target_os = "linux")]
    if !has_display() {
        eprintln!("Laputa desktop unavailable: DISPLAY/WAYLAND_DISPLAY is missing");
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
                |_window, cx| cx.new(|_| DesktopWindow::new(state)),
            ) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    launch_failed.store(true, Ordering::Relaxed);
                    eprintln!("Laputa desktop window failed: {error:#}");
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

fn main() -> std::process::ExitCode {
    match initial_state() {
        Ok(state) => run(state),
        Err(error) => {
            eprintln!("Laputa desktop failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

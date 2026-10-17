use std::{
    env,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use gpui::{
    App, Bounds, Context, InteractiveElement, Render, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};
use laputa_app::desktop_state::{AssetCard, AssetFilter, DesktopOperation, DesktopState};
use laputa_domain::MediaKind;
use laputa_media::{FfprobeMediaProbe, ProjectStore};

const CANVAS: u32 = 0xf4f0e8;
const SURFACE: u32 = 0xfbf9f4;
const SURFACE_ALT: u32 = 0xe9e2d6;
const TEXT: u32 = 0x263238;
const MUTED: u32 = 0x6f7778;
const HAIRLINE: u32 = 0xd7d0c5;
const SKY: u32 = 0x6f8fa8;
const SKY_SOFT: u32 = 0xdce6eb;
const CLAY: u32 = 0xb9785d;
const ERROR: u32 = 0xa6534c;
const SUCCESS: u32 = 0x5f806b;

#[derive(Clone, Copy)]
enum UiCommand {
    NewProject,
    OpenProject,
    AddMedia,
    Filter(AssetFilter),
}

#[derive(Clone, Copy)]
enum ProjectPromptMode {
    Create,
    Open,
}

struct DesktopWindow {
    state: DesktopState,
}

impl DesktopWindow {
    fn new(state: DesktopState) -> Self {
        Self { state }
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
        }
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
                    let store = ProjectStore::at(&root);
                    let project = match mode {
                        ProjectPromptMode::Create => store.create(&project_name_from_root(&root)),
                        ProjectPromptMode::Open => store.open(),
                    }
                    .map_err(|error| error.to_string())?;
                    Ok::<_, String>((root, project))
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((root, project)) => this.state.project_loaded(root, project),
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
                .bg(rgb(SKY))
                .border_color(rgb(SKY))
                .text_color(rgb(0xffffff))
        } else {
            button
                .bg(rgb(SURFACE))
                .border_color(rgb(HAIRLINE))
                .text_color(rgb(TEXT))
        };
        button
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.execute(command, cx);
            }))
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
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .child(filter.label());
        let button = if selected {
            button.bg(rgb(SKY_SOFT)).text_color(rgb(SKY))
        } else {
            button.text_color(rgb(MUTED))
        };
        button.on_click(cx.listener(move |this, _, _, cx| {
            this.execute(UiCommand::Filter(filter), cx);
        }))
    }

    fn asset_card(card: AssetCard, index: usize) -> impl IntoElement {
        let media_fill = match card.kind {
            MediaKind::Image => 0xd8c9b6,
            MediaKind::Video => 0xc9d8d3,
        };
        div()
            .id(("asset-card", index))
            .w_full()
            .p_2()
            .rounded_md()
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(0xe1dbd1))
            .flex()
            .gap_3()
            .items_center()
            .child(
                div()
                    .w(px(64.))
                    .h(px(48.))
                    .flex_shrink_0()
                    .rounded_md()
                    .bg(rgb(media_fill))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(card.kind_label),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child(card.display_name))
                    .when(!card.detail.is_empty(), |view| {
                        view.child(div().text_sm().text_color(rgb(MUTED)).child(card.detail))
                    }),
            )
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
                    .h(px(64.))
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
                            .min_w_0()
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(HAIRLINE))
                            .p_8()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap_5()
                            .child(div().text_3xl().child("把片段变成故事"))
                            .child(div().text_color(rgb(MUTED)).child(
                                "把生活里的照片和视频放进来，先从一个安静、简单的影片项目开始。",
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
                            .w(px(360.))
                            .flex_shrink_0()
                            .rounded_lg()
                            .bg(rgb(SURFACE_ALT))
                            .p_8()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(div().text_xl().child("Laputa"))
                                    .child(div().text_color(rgb(MUTED)).child(
                                        "不是从复杂时间线开始，而是从你的素材和想表达的故事开始。",
                                    )),
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

    fn workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let project = self
            .state
            .project()
            .expect("workspace is rendered only when a project is open");
        let cards = self.state.asset_cards();
        let asset_count = self.state.asset_count();
        let operation = self.state.operation().label();

        let sidebar = div()
            .w(px(286.))
            .min_h_0()
            .flex_shrink_0()
            .bg(rgb(0xeee8de))
            .border_r_1()
            .border_color(rgb(HAIRLINE))
            .flex()
            .flex_col()
            .child(
                div()
                    .p_4()
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
                                    .child(asset_count.to_string()),
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
                    .id("asset-browser-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_3()
                    .pb_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(cards.is_empty(), |view| {
                        view.child(
                            div()
                                .mt_4()
                                .p_4()
                                .text_sm()
                                .text_color(rgb(MUTED))
                                .child("还没有符合当前筛选的素材"),
                        )
                    })
                    .children(
                        cards
                            .into_iter()
                            .enumerate()
                            .map(|(index, card)| Self::asset_card(card, index)),
                    ),
            );

        let stage = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .rounded_lg()
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(HAIRLINE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(520.))
                            .max_w_full()
                            .px_6()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(54.))
                                    .h(px(54.))
                                    .rounded_lg()
                                    .bg(rgb(SKY_SOFT))
                                    .text_color(rgb(SKY))
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
                            .child(
                                div()
                                    .text_color(rgb(MUTED))
                                    .child(if asset_count == 0 {
                                        "选择本地素材开始创作。原始文件只会被引用，不会被覆盖。".to_owned()
                                    } else {
                                        format!("已导入 {asset_count} 个素材。你可以继续添加，项目会自动保存素材清单。")
                                    }),
                            )
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
                            )),
                    )
                    .when_some(operation, |view, label| {
                        view.child(
                            div()
                                .absolute()
                                .top_4()
                                .right_4()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(rgb(SKY_SOFT))
                                .text_sm()
                                .text_color(rgb(SKY))
                                .child(label),
                        )
                    }),
            )
            .when_some(self.state.notice(), |view, notice| {
                view.child(
                    div()
                        .px_4()
                        .py_3()
                        .rounded_md()
                        .bg(rgb(0xe3eee6))
                        .text_sm()
                        .text_color(rgb(SUCCESS))
                        .child(notice.to_owned()),
                )
            })
            .when_some(self.state.error(), |view, error| {
                view.child(
                    div()
                        .px_4()
                        .py_3()
                        .rounded_md()
                        .bg(rgb(0xf4e3df))
                        .text_sm()
                        .text_color(rgb(ERROR))
                        .child(error.to_owned()),
                )
            });

        let director = div()
            .w(px(300.))
            .min_h_0()
            .flex_shrink_0()
            .bg(rgb(SURFACE))
            .border_l_1()
            .border_color(rgb(HAIRLINE))
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child("Laputa"))
                    .child(div().w(px(9.)).h(px(9.)).rounded_full().bg(rgb(CLAY))),
            )
            .child(
                div()
                    .rounded_lg()
                    .bg(rgb(CANVAS))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("当前影片"))
                    .child(project.name.clone())
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(format!("{asset_count} 个素材")),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("先把素材整理好。故事规划和初剪会在后续 Story 接入这里。"),
            );

        let scene_strip = div()
            .h(px(112.))
            .flex_shrink_0()
            .border_t_1()
            .border_color(rgb(HAIRLINE))
            .bg(rgb(0xf7f3ec))
            .px_5()
            .flex()
            .items_center()
            .gap_5()
            .child(div().text_sm().text_color(rgb(MUTED)).child("场景"))
            .child(
                div()
                    .flex_1()
                    .h(px(68.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xdfd8cd))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("初剪生成后，场景会在这里出现"),
            );

        div()
            .size_full()
            .bg(rgb(CANVAS))
            .text_color(rgb(TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(62.))
                    .flex_shrink_0()
                    .px_5()
                    .border_b_1()
                    .border_color(rgb(HAIRLINE))
                    .bg(rgb(SURFACE))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().w(px(9.)).h(px(9.)).rounded_full().bg(rgb(CLAY)))
                            .child(project.name.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("原始素材保持只读引用"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(sidebar)
                    .child(stage)
                    .child(director),
            )
            .child(scene_strip)
    }
}

impl Render for DesktopWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.state.project().is_some() {
            self.workspace(cx).into_any_element()
        } else {
            self.launcher(cx).into_any_element()
        }
    }
}

fn project_name_from_root(root: &Path) -> String {
    root.file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "未命名影片".to_owned())
}

fn initial_state() -> Result<DesktopState, String> {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        return Ok(DesktopState::launcher());
    };
    let Some(command) = command.to_str() else {
        return Err("启动参数不是有效文本".to_owned());
    };
    match command {
        "open" => {
            let root = args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| "用法: laputa-desktop open <project-dir>".to_owned())?;
            if args.next().is_some() {
                return Err("用法: laputa-desktop open <project-dir>".to_owned());
            }
            let project = ProjectStore::at(&root)
                .open()
                .map_err(|error| error.to_string())?;
            Ok(DesktopState::from_project(root, project))
        }
        "create" => {
            let root = args.next().map(PathBuf::from).ok_or_else(|| {
                "用法: laputa-desktop create <project-dir> [project-name]".to_owned()
            })?;
            let name = args
                .next()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| project_name_from_root(&root));
            if args.next().is_some() {
                return Err("用法: laputa-desktop create <project-dir> [project-name]".to_owned());
            }
            let project = ProjectStore::at(&root)
                .create(&name)
                .map_err(|error| error.to_string())?;
            Ok(DesktopState::from_project(root, project))
        }
        "--help" | "-h" | "help" => Err(
            "用法: laputa-desktop [open <project-dir> | create <project-dir> [project-name]]"
                .to_owned(),
        ),
        _ => Err(format!(
            "未知启动命令 {command:?}; 使用 laputa-desktop --help 查看用法"
        )),
    }
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
        eprintln!("Laputa desktop unavailable: neither DISPLAY nor WAYLAND_DISPLAY is configured");
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

            let bounds = Bounds::centered(None, size(px(1320.), px(820.)), cx);
            match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(980.), px(640.))),
                    ..Default::default()
                },
                |_window, cx| cx.new(|_| DesktopWindow::new(state.clone())),
            ) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    launch_failed.store(true, Ordering::Relaxed);
                    eprintln!("Laputa desktop window initialization failed: {error:#}");
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
        Err(message) => {
            eprintln!("{message}");
            if message.starts_with("用法:") {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
    }
}

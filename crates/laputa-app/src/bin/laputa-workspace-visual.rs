use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use gpui::{
    App, Bounds, Context, InteractiveElement, Render, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};

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
const WARM: u32 = 0xe3b995;
const PERSON: u32 = 0xbd806a;

#[derive(Clone, Copy)]
struct SceneDemo {
    number: &'static str,
    title: &'static str,
    duration: &'static str,
    tone: u32,
}

const SCENES: [SceneDemo; 6] = [
    SceneDemo {
        number: "01",
        title: "开场",
        duration: "0:28",
        tone: SKY_BLOCK,
    },
    SceneDemo {
        number: "02",
        title: "寺院",
        duration: "0:52",
        tone: SAND,
    },
    SceneDemo {
        number: "03",
        title: "人像",
        duration: "0:46",
        tone: SAGE,
    },
    SceneDemo {
        number: "04",
        title: "街景",
        duration: "0:48",
        tone: WARM,
    },
    SceneDemo {
        number: "05",
        title: "日落",
        duration: "0:58",
        tone: CLAY,
    },
    SceneDemo {
        number: "06",
        title: "结尾",
        duration: "0:40",
        tone: FJORD,
    },
];

struct VisualWorkspace {
    selected_scene: usize,
    playing: bool,
}

impl VisualWorkspace {
    fn new() -> Self {
        Self {
            selected_scene: 3,
            playing: false,
        }
    }

    fn utility_button(label: &'static str) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .text_sm()
            .text_color(rgb(MUTED))
            .child(label)
    }

    fn primary_button(label: &'static str) -> impl IntoElement {
        div()
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(BLUE))
            .text_sm()
            .text_color(rgb(0xffffff))
            .child(label)
    }

    fn chip(label: &'static str) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(HAIRLINE))
            .bg(rgb(PAPER))
            .text_sm()
            .text_color(rgb(TEXT))
            .child(label)
    }

    fn moment_card(
        index: usize,
        title: &'static str,
        kind: &'static str,
        tone: u32,
    ) -> impl IntoElement {
        div()
            .id(("moment", index))
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .w_full()
                    .h(px(82.))
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
                            .child(kind),
                    ),
            )
            .child(div().text_sm().text_color(rgb(TEXT)).child(title))
    }

    fn moments_rail(&self) -> impl IntoElement {
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
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_xl().child("片段素材"))
                            .child(div().text_sm().text_color(rgb(MUTED)).child("24")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(BLUE_SOFT))
                                    .text_sm()
                                    .text_color(rgb(BLUE))
                                    .child("全部"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("照片"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .child("视频"),
                            ),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(HAIRLINE))
                            .bg(rgb(PAPER))
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child("＋ 添加素材"),
                    ),
            )
            .child(
                div()
                    .id("visual-moments-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_4()
                    .pb_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::moment_card(0, "祇园街角", "照片", SAND))
                    .child(Self::moment_card(1, "清水寺", "视频", SKY_BLOCK))
                    .child(Self::moment_card(2, "和服人像", "照片", SAGE))
                    .child(Self::moment_card(3, "鸭川日落", "视频", WARM)),
            )
    }

    fn faux_preview(&self) -> impl IntoElement {
        div()
            .w_full()
            .flex_1()
            .min_h_0()
            .rounded_lg()
            .overflow_hidden()
            .bg(rgb(FJORD))
            .flex()
            .flex_col()
            .child(div().h(px(98.)).flex_shrink_0().bg(rgb(0xc4d7e1)))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .bg(rgb(0xd7b18e))
                    .flex()
                    .items_end()
                    .child(
                        div()
                            .w_full()
                            .h(px(150.))
                            .bg(rgb(FJORD))
                            .p_5()
                            .flex()
                            .items_end()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .text_color(rgb(0xffffff))
                                    .child(div().text_sm().child("场景 04"))
                                    .child(div().text_2xl().child("京都街景")),
                            )
                            .child(
                                div()
                                    .w(px(72.))
                                    .h(px(92.))
                                    .rounded_lg()
                                    .bg(rgb(PERSON))
                                    .border_1()
                                    .border_color(rgb(0xe7cbb8)),
                            ),
                    ),
            )
    }

    fn stage(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(self.faux_preview())
                            .child(
                                div()
                                    .h(px(38.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id("visual-play-toggle")
                                            .w(px(34.))
                                            .h(px(34.))
                                            .rounded_full()
                                            .bg(rgb(TEXT))
                                            .cursor_pointer()
                                            .text_color(rgb(0xffffff))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(if self.playing { "Ⅱ" } else { "▶" })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.playing = !this.playing;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().text_sm().text_color(rgb(MUTED)).child("0:00"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(3.))
                                            .rounded_full()
                                            .bg(rgb(HAIRLINE))
                                            .child(
                                                div()
                                                    .w(px(72.))
                                                    .h(px(3.))
                                                    .rounded_full()
                                                    .bg(rgb(BLUE)),
                                            ),
                                    )
                                    .child(div().text_sm().text_color(rgb(MUTED)).child("4:32"))
                                    .child(div().text_sm().text_color(rgb(MUTED)).child("音量"))
                                    .child(div().text_sm().text_color(rgb(MUTED)).child("全屏")),
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
                                    .child("初剪已完成 · 4 分 32 秒 · 6 个片段"),
                            ),
                    )
                    .child(div().text_sm().text_color(rgb(BLUE)).child("观看完整视频")),
            )
    }

    fn director_rail(&self) -> impl IntoElement {
        let scene = SCENES[self.selected_scene];
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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("当前片段"))
                    .child(
                        div()
                            .text_xl()
                            .child(format!("片段 {} · {}", scene.number, scene.title)),
                    )
                    .child(div().text_sm().text_color(rgb(MUTED)).child(scene.duration))
                    .child(
                        div()
                            .pt_2()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("保留街巷的生活感，让节奏自然，不要堆太多转场。"),
                    ),
            )
            .child(
                div()
                    .h(px(76.))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(HAIRLINE))
                    .bg(rgb(CANVAS))
                    .p_3()
                    .text_sm()
                    .text_color(rgb(MUTED))
                    .child("告诉 Laputa 你的想法…"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Self::chip("更短一些"))
                    .child(Self::chip("换一个镜头"))
                    .child(Self::chip("增加动感")),
            )
            .child(div().flex_1())
            .child(
                div()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(HAIRLINE))
                    .text_sm()
                    .text_color(rgb(BLUE))
                    .child("保留这个片段"),
            )
    }

    fn scene_card(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let scene = SCENES[index];
        let selected = self.selected_scene == index;
        let card = div()
            .id(("visual-scene", index))
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
        } else {
            card.bg(rgb(OAT_SOFT)).border_color(rgb(HAIRLINE))
        };
        card.child(
            div()
                .w(px(50.))
                .h_full()
                .rounded_md()
                .bg(rgb(scene.tone))
                .flex()
                .items_end()
                .p_1()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(scene.number),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .justify_between()
                .child(div().text_sm().child(scene.title))
                .child(div().text_sm().text_color(rgb(MUTED)).child(scene.duration)),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_scene = index;
            cx.notify();
        }))
    }

    fn scene_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(div().text_sm().text_color(rgb(TEXT)).child("6")),
            )
            .child(
                div()
                    .id("visual-scenes-scroll")
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .flex()
                    .gap_3()
                    .child(self.scene_card(0, cx))
                    .child(self.scene_card(1, cx))
                    .child(self.scene_card(2, cx))
                    .child(self.scene_card(3, cx))
                    .child(self.scene_card(4, cx))
                    .child(self.scene_card(5, cx)),
            )
    }

    fn top_bar(&self) -> impl IntoElement {
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
                    .child(div().text_xl().child("京都之旅")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(Self::utility_button("撤销"))
                    .child(Self::utility_button("重做"))
                    .child(div().w(px(10.)))
                    .child(Self::primary_button("导出")),
            )
    }
}

impl Render for VisualWorkspace {
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

#[cfg(target_os = "linux")]
fn has_display() -> bool {
    [
        std::env::var_os("DISPLAY"),
        std::env::var_os("WAYLAND_DISPLAY"),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.is_empty())
}

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "linux")]
    if !has_display() {
        eprintln!(
            "Laputa workspace visual unavailable: neither DISPLAY nor WAYLAND_DISPLAY is configured"
        );
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

            let bounds = Bounds::centered(None, size(px(1500.), px(940.)), cx);
            match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(1120.), px(720.))),
                    ..Default::default()
                },
                |_window, cx| cx.new(|_| VisualWorkspace::new()),
            ) {
                Ok(_) => cx.activate(true),
                Err(error) => {
                    launch_failed.store(true, Ordering::Relaxed);
                    eprintln!("Laputa workspace visual initialization failed: {error:#}");
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

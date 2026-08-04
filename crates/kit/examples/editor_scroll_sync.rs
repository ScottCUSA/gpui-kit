//! Keep two editors scrolled together with `scroll_offset` and
//! `set_scroll_offset`.
//!
//! - Scroll either editor with the wheel, its scrollbars, or the caret: the
//!   other follows on the same frame, vertically and horizontally.
//! - The editors are the same size but differ in line count and line length.
//!   Each clamps the offset to its own range, so the shorter one stops at its
//!   end while the longer one keeps scrolling.
//! - The sync runs in `render`, before either editor is laid out. Wheel input
//!   notifies the editor and a scrollbar drag notifies this view; observing
//!   the editors brings both paths here.
//!
//! Run: `cargo run -p gpui-kit --example editor_scroll_sync`

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme as _,
    input::{Editor, EditorState},
    *,
};
use gpui_kit::*;

const LONG_LINES: usize = 400;
const SHORT_LINES: usize = 120;

struct ScrollSyncExample {
    long: Entity<EditorState>,
    short: Entity<EditorState>,
    /// Each editor's scroll offset as last seen.
    scroll: [Point<Pixels>; 2],
    /// An offset pushed to an editor, from where that editor was: layout clamps
    /// it to the editor's own range, so it settles somewhere between the two.
    pushed: [Option<(Point<Pixels>, Point<Pixels>)>; 2],
    _subscriptions: Vec<Subscription>,
}

impl ScrollSyncExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let long = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(false)
                .default_value(sample_text(LONG_LINES, 12))
        });
        let short = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .soft_wrap(false)
                .default_value(sample_text(SHORT_LINES, 4))
        });

        let _subscriptions = [&long, &short]
            .map(|editor| cx.observe(editor, |_, _, cx| cx.notify()))
            .into();

        Self {
            long,
            short,
            scroll: [Point::default(); 2],
            pushed: [None; 2],
            _subscriptions,
        }
    }

    /// Moves the editor that did not scroll to the offset of the one that did.
    ///
    /// Layout may clamp a pushed offset to the other editor's shorter range.
    /// That clamped offset is the push settling, not a scroll to send back:
    /// sending it back would pull the longer editor to the shorter one's end.
    fn synchronize_scroll(&mut self, cx: &mut Context<Self>) {
        let editors = [self.long.clone(), self.short.clone()];
        let current = editors
            .each_ref()
            .map(|editor| editor.read(cx).scroll_offset());
        for side in 0..2 {
            if current[side] == self.scroll[side] {
                continue;
            }
            self.scroll[side] = current[side];
            let settled = self.pushed[side]
                .take()
                .is_some_and(|(from, to)| lies_between(current[side], from, to));
            if settled {
                continue;
            }
            let other = 1 - side;
            if current[other] != current[side] {
                let offset = current[side];
                editors[other].update(cx, |state, cx| state.set_scroll_offset(offset, cx));
                self.pushed[other] = Some((current[other], offset));
            }
            return;
        }
    }
}

/// Whether `point` lies between `from` and `to` on both axes.
fn lies_between(point: Point<Pixels>, from: Point<Pixels>, to: Point<Pixels>) -> bool {
    let within = |value: Pixels, a: Pixels, b: Pixels| a.min(b) <= value && value <= a.max(b);
    within(point.x, from.x, to.x) && within(point.y, from.y, to.y)
}

/// `count` numbered lines; every fifth carries `words` extra words so the
/// text also scrolls horizontally.
fn sample_text(count: usize, words: usize) -> String {
    (1..=count)
        .map(|line| {
            let mut text = format!("let line_{line} = {line};");
            if line % 5 == 0 {
                for word in 0..words {
                    text.push_str(&format!(" // note {word} on line {line}"));
                }
            }
            text
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn pane(title: &'static str, lines: usize, editor: &Entity<EditorState>, cx: &App) -> Div {
    let spacing = cx.theme().spacing_tokens();
    v_flex()
        .min_h_0()
        .gap(spacing.xs)
        .child(
            h_flex().gap(spacing.sm).text_sm().child(title).child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{lines} lines")),
            ),
        )
        .child(Editor::new(editor).flex_1().min_h_0())
}

impl Render for ScrollSyncExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.synchronize_scroll(cx);
        let spacing = cx.theme().spacing_tokens();
        h_flex()
            .size_full()
            .p(spacing.lg)
            .gap(spacing.md)
            .child(pane("Long", LONG_LINES, &self.long, cx).flex_1().h_full())
            .child(
                pane("Short", SHORT_LINES, &self.short, cx)
                    .flex_1()
                    .h_full(),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1000.), px(680.)), cx)),
            ..Default::default()
        };

        gpui_kit::open_window(window_options, cx, |window, cx| {
            cx.new(|cx| ScrollSyncExample::new(window, cx))
        })
        .expect("Failed to open window");
    });
}

use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, AppContext, Context, Entity, EntityInputHandler as _, IntoElement, Pixels, Render,
    Size, Styled, Window, px,
};
use gpui_component::{
    input::{Input, InputState, Position},
    text::{TextView, TextViewStyle},
    v_virtual_list,
};

pub(crate) use gpui_component::{
    button::{Button, ButtonVariants},
    input::InputEvent,
};

use crate::core::document::BlockKind;

use super::{
    BODY_FONT_SIZE,
    layout::{block_layout_metrics, position_for_byte_offset},
};

#[derive(Debug, Clone)]
pub(crate) struct BlockInput {
    state: Entity<InputState>,
}

impl BlockInput {
    pub(crate) fn new<V>(
        kind: &BlockKind,
        text: String,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Self {
        let state = cx.new(|cx| {
            let mut state = match kind {
                BlockKind::CodeFence { language } => InputState::new(window, cx)
                    .code_editor(language.clone().unwrap_or_else(|| "text".to_string()))
                    .line_number(false),
                _ => InputState::new(window, cx)
                    .multi_line(true)
                    .auto_grow(1, 24),
            };
            state.set_value(text, window, cx);
            state
        });
        Self { state }
    }

    pub(crate) fn entity(&self) -> &Entity<InputState> {
        &self.state
    }

    pub(crate) fn text_and_cursor<V>(&self, cx: &mut Context<V>) -> (String, usize) {
        self.state
            .update(cx, |input, _| (input.text().to_string(), input.cursor()))
    }

    pub(crate) fn selection_and_cursor<V>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> (Option<Range<usize>>, usize) {
        self.state.update(cx, |input, cx| {
            let selection = input
                .selected_text_range(true, window, cx)
                .and_then(|selection| (!selection.range.is_empty()).then_some(selection.range));
            (selection, input.cursor())
        })
    }

    pub(crate) fn sync<V>(
        &self,
        text: &str,
        desired_cursor: usize,
        window: &mut Window,
        cx: &mut Context<V>,
    ) {
        self.state.update(cx, |input, cx| {
            let current_text = input.text().to_string();
            let current_cursor = input.cursor();
            if current_text != text {
                input.set_value(text.to_string(), window, cx);
            }
            if current_cursor != desired_cursor {
                let (row, col) = position_for_byte_offset(text, desired_cursor);
                input.set_cursor_position(
                    Position {
                        line: row as u32,
                        character: col as u32,
                    },
                    window,
                    cx,
                );
            }
            input.focus(window, cx);
        });
    }

    pub(crate) fn render(&self, kind: &BlockKind) -> AnyElement {
        let metrics = block_layout_metrics(kind);
        Input::new(&self.state)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .text_size(px(metrics.font_size))
            .line_height(px(metrics.line_height))
            .into_any_element()
    }
}

pub(crate) fn render_markdown_preview<V>(
    block_id: u64,
    text: String,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    TextView::markdown(("preview", block_id), text, window, cx)
        .style(markdown_preview_style())
        .into_any_element()
}

pub(crate) fn render_virtual_block_list<V, F>(
    view: Entity<V>,
    sizes: Rc<Vec<Size<Pixels>>>,
    render_range: F,
) -> AnyElement
where
    V: 'static + Render,
    F: 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<AnyElement>,
{
    v_virtual_list(view, "document-blocks", sizes, render_range)
        .size_full()
        .into_any_element()
}

fn markdown_preview_style() -> TextViewStyle {
    TextViewStyle::default()
        .paragraph_gap(gpui::rems(0.45))
        .heading_font_size(|level, _| match level {
            1 => px(34.),
            2 => px(28.),
            3 => px(24.),
            4 => px(20.),
            5 => px(18.),
            _ => px(BODY_FONT_SIZE),
        })
}

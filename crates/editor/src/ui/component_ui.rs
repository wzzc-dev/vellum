use std::ops::Range;

use gpui::{
    AnyElement, App, AppContext, Context, Entity, EntityInputHandler as _, Focusable, IntoElement,
    ParentElement, StyleRefinement, Styled, Window, div, px, rems,
};
use gpui_component::{
    ActiveTheme,
    input::{Input, InputState, Position},
    text::{TextView, TextViewStyle},
};

pub(crate) use gpui_component::{
    button::{Button, ButtonVariants},
    input::InputEvent,
};

use crate::core::document::BlockKind;

use super::layout::{EditableSurfaceKind, block_presentation, position_for_byte_offset};

#[derive(Debug, Clone)]
pub(crate) struct BlockInput {
    state: Entity<InputState>,
}

#[derive(Debug, Clone)]
pub(crate) struct InputNavigationState {
    pub(crate) text: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) has_selection: bool,
}

impl BlockInput {
    pub(crate) fn new<V>(
        kind: &BlockKind,
        text: String,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Self {
        let presentation = block_presentation(kind);
        let state = cx.new(|cx| {
            let mut state = match presentation.surface_kind {
                EditableSurfaceKind::CodeEditor => InputState::new(window, cx)
                    .code_editor(code_editor_language(kind))
                    .line_number(false)
                    .indent_guides(false)
                    .rows(initial_code_editor_rows(&text)),
                EditableSurfaceKind::AutoGrowText => InputState::new(window, cx).auto_grow(1, 24),
            };
            if text.is_empty() && matches!(kind, BlockKind::Raw | BlockKind::Paragraph) {
                state = state.placeholder("Start writing...");
            }
            state.set_value(text, window, cx);
            state
        });
        Self { state }
    }

    pub(crate) fn entity(&self) -> &Entity<InputState> {
        &self.state
    }

    pub(crate) fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.state.focus_handle(cx).is_focused(window)
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

    pub(crate) fn navigation_state<V>(
        &self,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> InputNavigationState {
        self.state.update(cx, |input, cx| {
            let cursor = input.cursor_position();
            let has_selection = input
                .selected_text_range(true, window, cx)
                .map(|selection| !selection.range.is_empty())
                .unwrap_or(false);

            InputNavigationState {
                text: input.text().to_string(),
                line: cursor.line as usize,
                column: cursor.character as usize,
                has_selection,
            }
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
        let presentation = block_presentation(kind);
        Input::new(&self.state)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .px(px(0.))
            .py(px(0.))
            .text_size(px(presentation.font_size))
            .line_height(px(presentation.line_height))
            .into_any_element()
    }
}

pub(crate) fn render_markdown_preview<V>(
    block_id: u64,
    kind: &BlockKind,
    text: String,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    TextView::markdown(("preview", block_id), text, window, cx)
        .style(markdown_preview_style(kind, cx))
        .into_any_element()
}

pub(crate) fn render_block_list(items: impl IntoIterator<Item = AnyElement>) -> AnyElement {
    let mut list = div().w_full().flex().flex_col();
    for item in items {
        list = list.child(item);
    }
    list.into_any_element()
}

fn markdown_preview_style<V>(kind: &BlockKind, cx: &Context<V>) -> TextViewStyle {
    let presentation = block_presentation(kind);
    let code_presentation = block_presentation(&BlockKind::CodeFence { language: None });
    TextViewStyle::default()
        .paragraph_gap(rems(presentation.preview_paragraph_gap_rem))
        .heading_font_size(|level, _| match level {
            1..=6 => px(block_presentation(&BlockKind::Heading { depth: level }).font_size),
            _ => px(block_presentation(&BlockKind::Paragraph).font_size),
        })
        .code_block(
            StyleRefinement::default()
                .bg(cx.theme().transparent)
                .p_0()
                .text_size(px(code_presentation.font_size))
                .line_height(px(code_presentation.line_height)),
        )
}

fn code_editor_language(kind: &BlockKind) -> String {
    match kind {
        BlockKind::CodeFence { language } => language.clone().unwrap_or_else(|| "text".to_string()),
        _ => "text".to_string(),
    }
}

fn initial_code_editor_rows(text: &str) -> usize {
    text.lines().count().max(1).clamp(1, 12)
}

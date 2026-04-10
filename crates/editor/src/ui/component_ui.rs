use std::ops::Range;

use gpui::{
    AnyElement, App, AppContext, Context, Entity, EntityInputHandler as _, Focusable, FontStyle,
    FontWeight, IntoElement, ParentElement, StrikethroughStyle, Styled, StyledText, TextStyle,
    UnderlineStyle, WhiteSpace, Window, div, px,
};
use gpui_component::{
    ActiveTheme,
    input::{Input, InputState, Position},
};

pub(crate) use gpui_component::{
    button::{Button, ButtonVariants},
    input::InputEvent,
};

use crate::core::document::BlockKind;
use crate::core::syntax::{
    InlineSegment, InlineStyle, PreviewBlock, PreviewListItem, PreviewListMarker,
};

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

    pub(crate) fn is_entity(&self, entity: &Entity<InputState>) -> bool {
        self.state.entity_id() == entity.entity_id()
    }

    pub(crate) fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.state.focus_handle(cx).contains_focused(window, cx)
    }

    pub(crate) fn has_marked_text<V>(&self, window: &mut Window, cx: &mut Context<V>) -> bool {
        self.state.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        })
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
    kind: &BlockKind,
    preview: Option<&PreviewBlock>,
    text: &str,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    preview
        .map(|preview| render_preview_block(preview, window, cx))
        .unwrap_or_else(|| render_plain_text_block(kind, text.to_string(), window, cx))
}

pub(crate) fn render_block_list(items: impl IntoIterator<Item = AnyElement>) -> AnyElement {
    let mut list = div().w_full().flex().flex_col();
    for item in items {
        list = list.child(item);
    }
    list.into_any_element()
}

fn render_preview_block<V>(
    preview: &PreviewBlock,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    match preview {
        PreviewBlock::Paragraph { content } => {
            render_inline_block(&BlockKind::Paragraph, content, FontWeight::NORMAL, window, cx)
        }
        PreviewBlock::Heading { depth, content } => render_inline_block(
            &BlockKind::Heading { depth: *depth },
            content,
            FontWeight::SEMIBOLD,
            window,
            cx,
        ),
        PreviewBlock::List { items } => render_list_preview(items, window, cx),
        PreviewBlock::Blockquote { blocks } => render_blockquote_preview(blocks, window, cx),
        PreviewBlock::Table { header, rows } => render_table_preview(header, rows, window, cx),
        PreviewBlock::CodeFence { language, text } => {
            render_code_preview(language.as_deref(), text, window, cx)
        }
        PreviewBlock::ThematicBreak => div()
            .w_full()
            .h(px(1.))
            .bg(cx.theme().foreground.opacity(0.14))
            .into_any_element(),
        PreviewBlock::Html { text } => {
            render_plain_text_block(&BlockKind::Html, text.clone(), window, cx)
        }
        PreviewBlock::Raw { text } => {
            render_plain_text_block(&BlockKind::Raw, text.clone(), window, cx)
        }
        PreviewBlock::Unknown { text } => {
            render_plain_text_block(&BlockKind::Unknown, text.clone(), window, cx)
        }
    }
}

fn render_plain_text_block<V>(
    kind: &BlockKind,
    text: String,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    render_inline_block(
        kind,
        &[InlineSegment {
            text,
            style: InlineStyle::default(),
        }],
        FontWeight::NORMAL,
        window,
        cx,
    )
}

fn render_inline_block<V>(
    kind: &BlockKind,
    segments: &[InlineSegment],
    base_weight: FontWeight,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let presentation = block_presentation(kind);
    let mut line = div()
        .w_full()
        .min_h(px(presentation.line_height))
        .text_size(px(presentation.font_size))
        .line_height(px(presentation.line_height))
        .text_color(cx.theme().foreground);
    if base_weight != FontWeight::NORMAL {
        line = line.font_weight(base_weight);
    }

    line.child(styled_text_for_segments(
        segments,
        preview_text_style(base_weight, window, cx),
        cx,
    ))
    .into_any_element()
}

fn render_list_preview<V>(
    items: &[PreviewListItem],
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let mut list = div().w_full().flex().flex_col().gap_2();
    for item in items {
        let marker = match &item.marker {
            PreviewListMarker::Bullet => "•".to_string(),
            PreviewListMarker::Ordered(marker) => marker.trim().to_string(),
            PreviewListMarker::Task { checked } => {
                if *checked {
                    "[x]".to_string()
                } else {
                    "[ ]".to_string()
                }
            }
        };

        let mut blocks = div().flex_1().min_w(px(0.)).flex().flex_col().gap_1();
        for block in &item.blocks {
            blocks = blocks.child(render_preview_block(block, window, cx));
        }

        list = list.child(
            div()
                .w_full()
                .flex()
                .gap_3()
                .child(
                    div()
                        .min_w(px(28.))
                        .text_color(cx.theme().muted_foreground)
                        .font_weight(FontWeight::MEDIUM)
                        .child(marker),
                )
                .child(blocks),
        );
    }
    list.into_any_element()
}

fn render_blockquote_preview<V>(
    blocks: &[PreviewBlock],
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let mut content = div().flex_1().min_w(px(0.)).flex().flex_col().gap_2();
    for block in blocks {
        content = content.child(render_preview_block(block, window, cx));
    }

    div()
        .w_full()
        .flex()
        .gap_3()
        .child(
            div()
                .w(px(3.))
                .bg(cx.theme().foreground.opacity(0.18))
                .rounded(px(999.)),
        )
        .child(content)
        .into_any_element()
}

fn render_table_preview<V>(
    header: &[Vec<InlineSegment>],
    rows: &[Vec<Vec<InlineSegment>>],
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let mut table = div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .rounded(px(8.))
        .border_1()
        .border_color(cx.theme().foreground.opacity(0.12))
        .bg(cx.theme().foreground.opacity(0.03))
        .p_2();

    if !header.is_empty() {
        table = table.child(render_table_row(header, true, window, cx));
    }
    for row in rows {
        table = table.child(render_table_row(row, false, window, cx));
    }

    table.into_any_element()
}

fn render_table_row<V>(
    cells: &[Vec<InlineSegment>],
    is_header: bool,
    window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let mut row = div().w_full().flex().gap_2();
    for cell in cells {
        row = row.child(
            div()
                .flex_1()
                .min_w(px(0.))
                .rounded(px(6.))
                .bg(
                    if is_header {
                        cx.theme().foreground.opacity(0.06)
                    } else {
                        cx.theme().transparent
                    },
                )
                .px_2()
                .py_1()
                .child(render_inline_block(
                    &BlockKind::Paragraph,
                    cell,
                    if is_header {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    },
                    window,
                    cx,
                )),
        );
    }
    row.into_any_element()
}

fn render_code_preview<V>(
    language: Option<&str>,
    text: &str,
    _window: &mut Window,
    cx: &mut Context<V>,
) -> AnyElement {
    let kind = BlockKind::CodeFence {
        language: language.map(str::to_string),
    };
    let presentation = block_presentation(&kind);
    let mut content = div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(cx.theme().foreground.opacity(0.12))
        .bg(cx.theme().foreground.opacity(0.04))
        .px_3()
        .py_2()
        .text_size(px(presentation.font_size))
        .line_height(px(presentation.line_height))
        .text_color(cx.theme().foreground)
        .child(
            div()
                .w_full()
                .text_color(cx.theme().foreground)
                .child(text.to_string()),
        );

    if let Some(language) = language.filter(|language| !language.is_empty()) {
        content = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().muted_foreground)
                    .child(language.to_string()),
            )
            .child(content);
    }

    content.into_any_element()
}

fn preview_text_style<V>(base_weight: FontWeight, window: &Window, cx: &Context<V>) -> TextStyle {
    let mut style = window.text_style().clone();
    style.color = cx.theme().foreground;
    style.font_weight = base_weight;
    style.font_style = FontStyle::Normal;
    style.background_color = None;
    style.underline = None;
    style.strikethrough = None;
    style.white_space = WhiteSpace::Normal;
    style
}

fn styled_text_for_segments<V>(
    segments: &[InlineSegment],
    base_style: TextStyle,
    cx: &Context<V>,
) -> StyledText {
    let mut text = String::new();
    let mut runs = Vec::new();

    for segment in segments.iter().filter(|segment| !segment.text.is_empty()) {
        let mut style = base_style.clone();
        apply_inline_style(&mut style, &segment.style, cx);
        text.push_str(&segment.text);
        runs.push(style.to_run(segment.text.len()));
    }

    if runs.is_empty() {
        StyledText::new(String::new())
    } else {
        StyledText::new(text).with_runs(runs)
    }
}

fn apply_inline_style<V>(style: &mut TextStyle, inline: &InlineStyle, cx: &Context<V>) {
    if inline.strong {
        style.font_weight = FontWeight::BOLD;
    }
    if inline.emphasis {
        style.font_style = FontStyle::Italic;
    }
    if inline.strikethrough {
        style.strikethrough = Some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(cx.theme().foreground.opacity(0.68)),
        });
    }
    if inline.link {
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            color: Some(cx.theme().foreground.opacity(0.7)),
            wavy: false,
        });
    }
    if inline.code {
        if !inline.strong {
            style.font_weight = FontWeight::MEDIUM;
        }
        style.background_color = Some(cx.theme().foreground.opacity(0.08));
    }
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

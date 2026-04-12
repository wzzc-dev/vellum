use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, Entity, FontStyle, FontWeight, Hsla,
    InteractiveElement, IntoElement, MouseButton, PaintQuad, ParentElement, SharedString,
    StatefulInteractiveElement, StrikethroughStyle, Styled, StyledText, TextStyle, UnderlineStyle,
    WhiteSpace, Window, canvas, div, fill, point, px, size,
};
use gpui_component::ActiveTheme;

use crate::{
    BlockKind, EditCommand, RenderBlock, RenderSpan, RenderSpanKind, SelectionState,
    core::{controller::EditorSnapshot, text_ops::clamp_to_char_boundary},
};

use super::{BODY_FONT_SIZE, BODY_LINE_HEIGHT, layout::block_presentation, view::MarkdownEditor};

const LIST_MARKER_COLUMN_WIDTH: f32 = 28.;
const BLOCKQUOTE_BAR_WIDTH: f32 = 3.;
const DECORATION_GAP_X: f32 = 12.;

#[derive(Debug, Clone, Copy)]
struct RenderPalette {
    text_color: Hsla,
    muted_text_color: Hsla,
    selection_color: Hsla,
    caret_color: Hsla,
    code_background: Hsla,
    border_color: Hsla,
    blockquote_bar: Hsla,
    code_surface_background: Hsla,
}

#[derive(Clone)]
struct BlockOverlay {
    selection_quads: Vec<PaintQuad>,
    caret_quad: Option<PaintQuad>,
}

#[derive(Debug, Clone, Default)]
struct RenderedLine {
    fragments: Vec<RenderedFragment>,
}

#[derive(Debug, Clone)]
struct RenderedFragment {
    kind: RenderSpanKind,
    text: String,
    style: crate::RenderInlineStyle,
}

impl RenderedLine {
    fn push_fragment(&mut self, fragment: RenderedFragment) {
        if !fragment.text.is_empty() {
            self.fragments.push(fragment);
        }
    }

    fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }
}

impl MarkdownEditor {
    fn set_selection_from_surface_position(
        &mut self,
        block_id: u64,
        click_position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = self.block_bounds.borrow().get(&block_id).copied() else {
            self.focus_input(window, cx);
            return;
        };
        let Some((block_index, block)) = self
            .snapshot
            .display_map
            .blocks
            .iter()
            .enumerate()
            .find(|(_, block)| block.id == block_id)
            .map(|(index, block)| (index, block.clone()))
        else {
            self.focus_input(window, cx);
            return;
        };

        let local_visible_offset = visible_byte_offset_for_click_position(
            &self.snapshot.display_map.blocks,
            block_index,
            &block,
            click_position,
            bounds,
            window,
        );
        let visible_offset = clamp_to_char_boundary(
            &self.snapshot.display_map.visible_text,
            block.visible_range.start + local_visible_offset,
        );
        let selection = SelectionState::collapsed(
            self.snapshot
                .display_map
                .visible_to_source(visible_offset)
                .source_offset,
        );
        let effects = self
            .controller
            .dispatch(EditCommand::SetSelection { selection });
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
    }

    pub(super) fn handle_surface_click(
        &mut self,
        block_id: u64,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selection_from_surface_position(block_id, event.position(), window, cx);
    }

    pub(super) fn handle_surface_mouse_down(
        &mut self,
        block_id: u64,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selection_from_surface_position(block_id, position, window, cx);
    }
}

fn rendered_spans(block: &RenderBlock) -> impl Iterator<Item = &RenderSpan> {
    block
        .spans
        .iter()
        .filter(move |span| is_rendered_span(block, span))
}

fn is_rendered_span(block: &RenderBlock, span: &RenderSpan) -> bool {
    if renders_empty_block_linebreaks(block) && span.kind == RenderSpanKind::LineBreak {
        return !span.visible_text.is_empty();
    }

    span.source_range.start < block.content_range.end
        && !(span.kind == RenderSpanKind::LineBreak
            && span.source_range.end == block.content_range.end)
}

pub(super) fn renders_empty_block_linebreaks(block: &RenderBlock) -> bool {
    block.content_range.is_empty() && matches!(block.kind, BlockKind::Raw | BlockKind::Paragraph)
}

pub(super) fn rendered_visible_end(block: &RenderBlock) -> usize {
    rendered_spans(block)
        .map(|span| span.visible_range.end)
        .max()
        .unwrap_or(block.visible_range.start)
}

pub(super) fn rendered_visible_len(block: &RenderBlock) -> usize {
    rendered_visible_end(block).saturating_sub(block.visible_range.start)
}

fn has_rendered_text(block: &RenderBlock) -> bool {
    rendered_visible_len(block) > 0
}

pub(super) fn rendered_text_for_block(block: &RenderBlock) -> String {
    rendered_spans(block)
        .filter(|span| !span.visible_text.is_empty())
        .map(|span| span.visible_text.as_str())
        .collect()
}

pub(super) fn render_document_surface(
    view: &Entity<MarkdownEditor>,
    snapshot: &EditorSnapshot,
    input_focused: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    window: &mut Window,
    cx: &mut Context<MarkdownEditor>,
) -> AnyElement {
    let display_blocks = Rc::new(snapshot.display_map.blocks.clone());
    let palette = RenderPalette {
        text_color: cx.theme().foreground,
        muted_text_color: cx.theme().muted_foreground,
        selection_color: cx.theme().foreground.opacity(0.14),
        caret_color: cx.theme().foreground,
        code_background: cx.theme().foreground.opacity(0.08),
        border_color: cx.theme().foreground.opacity(0.12),
        blockquote_bar: cx.theme().foreground.opacity(0.18),
        code_surface_background: cx.theme().foreground.opacity(0.04),
    };

    if snapshot.display_map.blocks.is_empty() {
        let empty_view = view.clone();
        let empty_click_view = view.clone();
        return div()
            .id("empty-surface")
            .occlude()
            .w_full()
            .min_h(px(BODY_LINE_HEIGHT))
            .text_size(px(BODY_FONT_SIZE))
            .line_height(px(BODY_LINE_HEIGHT))
            .text_color(cx.theme().muted_foreground)
            .on_mouse_down(MouseButton::Left, move |_, window, app: &mut App| {
                app.stop_propagation();
                let _ = empty_view.update(app, |this, cx| {
                    let effects = this.controller.dispatch(EditCommand::SetSelection {
                        selection: SelectionState::collapsed(0),
                    });
                    this.apply_effects(window, cx, effects);
                    this.focus_input(window, cx);
                });
            })
            .on_click(move |_, window, cx| {
                let _ = empty_click_view.update(cx, |this, cx| {
                    let effects = this.controller.dispatch(EditCommand::SetSelection {
                        selection: SelectionState::collapsed(0),
                    });
                    this.apply_effects(window, cx, effects);
                    this.focus_input(window, cx);
                });
            })
            .child("Start writing...")
            .into_any_element();
    }

    let mut document = div().w_full().flex().flex_col();
    for (block_index, block) in snapshot.display_map.blocks.iter().cloned().enumerate() {
        document = document.child(render_display_block(
            view,
            snapshot,
            display_blocks.clone(),
            block_index,
            &block,
            input_focused,
            block_bounds.clone(),
            palette,
            window,
        ));
    }
    document.into_any_element()
}

fn render_display_block(
    view: &Entity<MarkdownEditor>,
    snapshot: &EditorSnapshot,
    display_blocks: Rc<Vec<RenderBlock>>,
    block_index: usize,
    block: &RenderBlock,
    input_focused: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    palette: RenderPalette,
    window: &mut Window,
) -> AnyElement {
    let presentation = block_presentation(&block.kind);
    let empty_line_count = surface_empty_block_line_count(display_blocks.as_ref(), block_index);
    let list_decorations = list_decoration_rows(block);
    let shows_list_decorations = matches!(block.kind, BlockKind::List)
        && list_decorations.iter().any(|marker| marker.is_some());
    let show_placeholder = snapshot.display_map.blocks.len() == 1
        && snapshot.display_map.visible_text.is_empty()
        && !input_focused;
    let visible_selection = snapshot.visible_selection.clone();
    let block_id = block.id;
    let block_clone = block.clone();
    let overlay_block = block.clone();
    let block_view = view.clone();
    let block_click_view = view.clone();

    let text_content = if show_placeholder {
        div()
            .w_full()
            .text_size(px(presentation.font_size))
            .line_height(px(presentation.line_height))
            .text_color(palette.muted_text_color)
            .child("Start writing...")
            .into_any_element()
    } else {
        match &block.kind {
            BlockKind::List if shows_list_decorations => {
                render_list_lines(block, &list_decorations, palette, window)
            }
            BlockKind::Blockquote => render_blockquote_lines(block, palette, window),
            _ if empty_line_count.is_some() => {
                render_empty_line_block(block, empty_line_count.unwrap_or(1))
            }
            _ => div()
                .w_full()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .when(matches!(block.kind, BlockKind::CodeFence { .. }), |this| {
                    this.font_family("Consolas")
                })
                .child(styled_text_for_block(&block_clone, palette, window))
                .into_any_element(),
        }
    };

    let text_area = div()
        .relative()
        .min_w(px(0.))
        .w_full()
        .min_h(px(presentation.line_height))
        .child(text_content)
        .child(
            canvas(
                move |bounds, window, _| {
                    block_bounds.borrow_mut().insert(overlay_block.id, bounds);
                    build_block_overlay(
                        &display_blocks,
                        block_index,
                        &overlay_block,
                        &visible_selection,
                        input_focused,
                        bounds,
                        palette,
                        window,
                    )
                },
                move |_, overlay, window, _| {
                    for quad in overlay.selection_quads {
                        window.paint_quad(quad);
                    }
                    if let Some(caret) = overlay.caret_quad {
                        window.paint_quad(caret);
                    }
                },
            )
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.)),
        )
        .into_any_element();

    let content = match &block.kind {
        BlockKind::Blockquote => div()
            .w_full()
            .flex()
            .gap(px(DECORATION_GAP_X))
            .child(
                div()
                    .w(px(BLOCKQUOTE_BAR_WIDTH))
                    .min_h(px(presentation.line_height))
                    .bg(palette.blockquote_bar),
            )
            .child(text_area)
            .into_any_element(),
        BlockKind::List => text_area,
        BlockKind::CodeFence { language } => {
            let mut code_surface = div()
                .w_full()
                .rounded(px(8.))
                .border_1()
                .border_color(palette.border_color)
                .bg(palette.code_surface_background)
                .px_3()
                .py_2()
                .child(text_area);

            if let Some(language) = language.as_ref().filter(|language| !language.is_empty()) {
                code_surface = div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(palette.muted_text_color)
                            .child(language.clone()),
                    )
                    .child(code_surface);
            }

            code_surface.into_any_element()
        }
        _ => text_area,
    };

    div()
        .id(("display-block", block.id))
        .w_full()
        .py(px(presentation.row_spacing_y))
        .child(
            div()
                .id(("surface-hit-target", block_id))
                .occlude()
                .w_full()
                .px_1()
                .py(px(presentation.block_padding_y))
                .on_mouse_down(MouseButton::Left, move |event, window, app: &mut App| {
                    app.stop_propagation();
                    let position = event.position;
                    let _ = block_view.update(app, |this, cx| {
                        this.handle_surface_mouse_down(block_id, position, window, cx);
                    });
                })
                .on_click(move |event, window, cx| {
                    let _ = block_click_view.update(cx, |this, cx| {
                        this.handle_surface_click(block_id, event, window, cx);
                    });
                })
                .child(content),
        )
        .into_any_element()
}

fn build_block_overlay(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    selection: &SelectionState,
    input_focused: bool,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> BlockOverlay {
    let selection_range = selection_range_in_block(block, selection);
    let selection_quads = selection_range
        .filter(|range| !range.is_empty())
        .map(|range| {
            selection_quads_for_block(blocks, block_index, block, range, bounds, palette, window)
        })
        .unwrap_or_default();
    let caret_quad = if input_focused && selection.is_collapsed() {
        caret_quad_for_block(
            blocks,
            block_index,
            block,
            selection.cursor(),
            bounds,
            palette,
            window,
        )
    } else {
        None
    };

    BlockOverlay {
        selection_quads,
        caret_quad,
    }
}

fn selection_range_in_block(
    block: &RenderBlock,
    selection: &SelectionState,
) -> Option<std::ops::Range<usize>> {
    let selection = selection.range();
    let start = selection.start.max(block.visible_range.start);
    let end = selection.end.min(rendered_visible_end(block));
    (start < end).then(|| {
        start.saturating_sub(block.visible_range.start)
            ..end.saturating_sub(block.visible_range.start)
    })
}

fn selection_quads_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    selection_range: std::ops::Range<usize>,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> Vec<PaintQuad> {
    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let text_x_offset = text_content_x_offset(block);
    if let Some(line_count) = surface_empty_block_line_count(blocks, block_index) {
        return selection_quads_for_empty_line_block(
            selection_range,
            line_count,
            bounds,
            line_height,
            text_x_offset,
            palette,
        );
    }
    let text_width = (bounds.size.width - text_x_offset).max(px(0.));
    let lines = shape_block_lines(block, text_width, window);
    if lines.is_empty() {
        return Vec::new();
    }

    let mut quads = Vec::new();
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_len = line.len();
        let line_range = byte_offset..byte_offset + line_len;
        if selection_range.start < line_range.end && line_range.start < selection_range.end {
            let local_start = selection_range.start.saturating_sub(line_range.start);
            let local_end =
                (selection_range.end.min(line_range.end)).saturating_sub(line_range.start);
            let mut wrap_start = 0usize;
            let mut wrap_y = px(0.);

            for wrap_end in line
                .wrap_boundaries()
                .iter()
                .map(|boundary| wrap_boundary_index(line, boundary))
                .chain(std::iter::once(line.len()))
            {
                let start = local_start.max(wrap_start);
                let end = local_end.min(wrap_end);
                if start < end {
                    let start_position = line
                        .position_for_index(start, line_height)
                        .unwrap_or_else(|| point(px(0.), wrap_y));
                    let end_position = line
                        .position_for_index(end, line_height)
                        .unwrap_or_else(|| point(px(0.), wrap_y));
                    quads.push(fill(
                        Bounds::new(
                            point(
                                bounds.left() + text_x_offset + start_position.x,
                                bounds.top() + y_offset + start_position.y,
                            ),
                            size((end_position.x - start_position.x).max(px(1.)), line_height),
                        ),
                        palette.selection_color,
                    ));
                }
                wrap_start = wrap_end;
                wrap_y += line_height;
            }
        }

        y_offset += line.size(line_height).height;
        byte_offset += line_len;
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    quads
}

pub(super) fn caret_visual_offset_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    cursor: usize,
) -> Option<usize> {
    let block = blocks.get(block_index)?;
    let block_start = block.visible_range.start;
    let block_end = rendered_visible_end(block);
    let previous_end = block_index
        .checked_sub(1)
        .and_then(|index| blocks.get(index))
        .map(rendered_visible_end)
        .unwrap_or(block_start);

    if !has_rendered_text(block) {
        return (cursor == block_start || (cursor > previous_end && cursor < block_start))
            .then_some(0);
    }

    if cursor < block_start {
        return (cursor > previous_end && cursor <= block_start).then_some(0);
    }

    if cursor < block_end {
        return Some(compressed_empty_block_visual_offset(
            blocks,
            block_index,
            cursor.saturating_sub(block_start),
        ));
    }

    if cursor == block_end {
        if blocks
            .get(block_index + 1)
            .map(|next| next.visible_range.start == cursor)
            .unwrap_or(false)
        {
            return None;
        }

        return Some(compressed_empty_block_visual_offset(
            blocks,
            block_index,
            block_end.saturating_sub(block_start),
        ));
    }

    None
}

fn caret_quad_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    cursor: usize,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> Option<PaintQuad> {
    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let local_cursor = caret_visual_offset_for_block(blocks, block_index, cursor)?;
    let text_x_offset = text_content_x_offset(block);

    if !has_rendered_text(block) {
        return Some(fill(
            Bounds::new(
                point(bounds.left() + text_x_offset, bounds.top()),
                size(px(2.), line_height),
            ),
            palette.caret_color,
        ));
    }

    if let Some(line_count) = surface_empty_block_line_count(blocks, block_index) {
        let line_index = local_cursor.min(line_count.saturating_sub(1));
        return Some(fill(
            Bounds::new(
                point(
                    bounds.left() + text_x_offset,
                    bounds.top() + px(presentation.line_height * line_index as f32),
                ),
                size(px(2.), line_height),
            ),
            palette.caret_color,
        ));
    }

    let text_width = (bounds.size.width - text_x_offset).max(px(0.));
    let lines = shape_block_lines(block, text_width, window);
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);

    for (line_ix, line) in lines.iter().enumerate() {
        let line_len = line.len();
        if local_cursor <= byte_offset + line_len {
            let local = local_cursor.saturating_sub(byte_offset);
            let position = line
                .position_for_index(local, line_height)
                .unwrap_or_else(|| point(px(0.), px(0.)));
            return Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + text_x_offset + position.x,
                        bounds.top() + y_offset + position.y,
                    ),
                    size(px(2.), line_height),
                ),
                palette.caret_color,
            ));
        }

        let line_height_span = line.size(line_height).height;
        if line_ix + 1 < lines.len() && local_cursor == byte_offset + line_len + 1 {
            return Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + text_x_offset,
                        bounds.top() + y_offset + line_height_span,
                    ),
                    size(px(2.), line_height),
                ),
                palette.caret_color,
            ));
        }

        y_offset += line_height_span;
        byte_offset += line_len;
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    lines.last().and_then(|line| {
        line.position_for_index(line.len(), line_height)
            .map(|position| {
                fill(
                    Bounds::new(
                        point(
                            bounds.left() + text_x_offset + position.x,
                            bounds.top() + y_offset - line.size(line_height).height + position.y,
                        ),
                        size(px(2.), line_height),
                    ),
                    palette.caret_color,
                )
            })
    })
}

fn styled_text_for_block(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
) -> StyledText {
    let text = rendered_text_for_block(block);
    if text.is_empty() {
        return StyledText::new(String::new());
    }

    let mut runs = Vec::new();
    let base_style = base_text_style_for_block(block, palette.text_color, window);

    for span in rendered_spans(block).filter(|span| !span.visible_text.is_empty()) {
        let mut style = base_style.clone();
        apply_fragment_style(&mut style, span.kind.clone(), span.style, palette);
        runs.push(style.to_run(span.visible_text.len()));
    }

    StyledText::new(text).with_runs(runs)
}

fn apply_fragment_style(
    style: &mut TextStyle,
    kind: RenderSpanKind,
    inline_style: crate::RenderInlineStyle,
    palette: RenderPalette,
) {
    if matches!(
        kind,
        RenderSpanKind::HiddenSyntax | RenderSpanKind::ListMarker
    ) {
        style.color = palette.muted_text_color;
    }
    if matches!(kind, RenderSpanKind::TaskMarker) {
        style.font_weight = FontWeight::MEDIUM;
        style.color = palette.muted_text_color;
    }

    if inline_style.strong {
        style.font_weight = FontWeight::BOLD;
    }
    if inline_style.emphasis {
        style.font_style = FontStyle::Italic;
    }
    if inline_style.strikethrough {
        style.strikethrough = Some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(palette.text_color.opacity(0.68)),
        });
    }
    if inline_style.link {
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            color: Some(palette.text_color.opacity(0.7)),
            wavy: false,
        });
    }
    if inline_style.code {
        if !inline_style.strong {
            style.font_weight = FontWeight::MEDIUM;
        }
        style.font_family = SharedString::from("Consolas");
        style.background_color = Some(palette.code_background);
    }
}

fn base_text_style_for_block(block: &RenderBlock, text_color: Hsla, window: &Window) -> TextStyle {
    let presentation = block_presentation(&block.kind);
    let mut style = window.text_style().clone();
    style.color = text_color;
    style.font_size = px(presentation.font_size).into();
    style.line_height = px(presentation.line_height).into();
    style.font_weight = match block.kind {
        BlockKind::Heading { depth } if depth <= 2 => FontWeight::BOLD,
        BlockKind::Heading { .. } => FontWeight::SEMIBOLD,
        _ => FontWeight::NORMAL,
    };
    style.font_style = FontStyle::Normal;
    style.white_space = WhiteSpace::Normal;
    if matches!(block.kind, BlockKind::CodeFence { .. }) {
        style.font_family = SharedString::from("Consolas");
    }
    style
}

fn rendered_lines_for_block(block: &RenderBlock) -> Vec<RenderedLine> {
    let mut lines = vec![RenderedLine::default()];

    for span in rendered_spans(block).filter(|span| !span.visible_text.is_empty()) {
        let mut remaining = span.visible_text.as_str();
        while let Some(newline_ix) = remaining.find('\n') {
            let mut piece = &remaining[..newline_ix];
            if let Some(stripped) = piece.strip_suffix('\r') {
                piece = stripped;
            }
            if !piece.is_empty() {
                lines
                    .last_mut()
                    .expect("at least one line")
                    .push_fragment(RenderedFragment {
                        kind: span.kind.clone(),
                        text: piece.to_string(),
                        style: span.style,
                    });
            }
            lines.push(RenderedLine::default());
            remaining = &remaining[newline_ix + 1..];
        }

        if !remaining.is_empty() {
            lines
                .last_mut()
                .expect("at least one line")
                .push_fragment(RenderedFragment {
                    kind: span.kind.clone(),
                    text: remaining.to_string(),
                    style: span.style,
                });
        }
    }

    if lines.is_empty() {
        vec![RenderedLine::default()]
    } else {
        lines
    }
}

fn render_list_lines(
    block: &RenderBlock,
    decorations: &[Option<String>],
    palette: RenderPalette,
    window: &Window,
) -> AnyElement {
    let lines = rendered_lines_for_block(block);
    let mut content = div().w_full().flex().flex_col();

    for (line_index, line) in lines.iter().enumerate() {
        let marker = decorations
            .get(line_index)
            .and_then(|marker| marker.clone())
            .unwrap_or_default();
        content = content.child(
            div()
                .w_full()
                .flex()
                .gap(px(DECORATION_GAP_X))
                .items_start()
                .child(
                    div()
                        .min_w(px(LIST_MARKER_COLUMN_WIDTH))
                        .text_color(palette.muted_text_color)
                        .font_weight(FontWeight::MEDIUM)
                        .text_size(px(BODY_FONT_SIZE))
                        .line_height(px(BODY_LINE_HEIGHT))
                        .child(marker),
                )
                .child(render_line_text(block, line, palette, window)),
        );
    }

    content.into_any_element()
}

fn render_blockquote_lines(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
) -> AnyElement {
    let lines = rendered_lines_for_block(block);
    let mut text_column = div().w_full().flex().flex_col();
    for line in &lines {
        text_column = text_column.child(render_line_text(block, line, palette, window));
    }
    text_column.into_any_element()
}

fn render_empty_line_block(block: &RenderBlock, line_count: usize) -> AnyElement {
    let presentation = block_presentation(&block.kind);
    let mut text_column = div().w_full().flex().flex_col();
    for _ in 0..line_count.max(1) {
        text_column = text_column.child(
            div()
                .w_full()
                .min_h(px(presentation.line_height))
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height)),
        );
    }
    text_column.into_any_element()
}

fn render_line_text(
    block: &RenderBlock,
    line: &RenderedLine,
    palette: RenderPalette,
    window: &Window,
) -> AnyElement {
    let presentation = block_presentation(&block.kind);
    div()
        .w_full()
        .min_h(px(presentation.line_height))
        .text_size(px(presentation.font_size))
        .line_height(px(presentation.line_height))
        .when(matches!(block.kind, BlockKind::CodeFence { .. }), |this| {
            this.font_family("Consolas")
        })
        .when(!line.is_empty(), |this| {
            this.child(styled_text_for_line(block, line, palette, window))
        })
        .into_any_element()
}

fn styled_text_for_line(
    block: &RenderBlock,
    line: &RenderedLine,
    palette: RenderPalette,
    window: &Window,
) -> StyledText {
    if line.is_empty() {
        return StyledText::new(String::new());
    }

    let mut text = String::new();
    let mut runs = Vec::new();
    let base_style = base_text_style_for_block(block, palette.text_color, window);

    for fragment in &line.fragments {
        text.push_str(&fragment.text);
        let mut style = base_style.clone();
        apply_fragment_style(&mut style, fragment.kind.clone(), fragment.style, palette);
        runs.push(style.to_run(fragment.text.len()));
    }

    StyledText::new(text).with_runs(runs)
}

pub(super) fn list_decoration_rows(block: &RenderBlock) -> Vec<Option<String>> {
    let mut rows = Vec::new();
    let mut current_marker = None;
    let mut saw_any_span = false;

    for span in &block.spans {
        saw_any_span = true;
        if span.kind == RenderSpanKind::ListMarker && !span.source_text.trim().is_empty() {
            current_marker = Some(normalize_list_marker(&span.source_text));
        }

        if span.kind == RenderSpanKind::LineBreak && span.source_text.contains('\n') {
            rows.push(current_marker.take());
        }
    }

    if saw_any_span {
        rows.push(current_marker.take());
    }

    if rows.is_empty() { vec![None] } else { rows }
}

fn normalize_list_marker(source_text: &str) -> String {
    let marker = source_text.trim();
    if marker
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ')'))
    {
        marker.to_string()
    } else {
        "\u{2022}".to_string()
    }
}

pub(super) fn text_content_x_offset(block: &RenderBlock) -> gpui::Pixels {
    match &block.kind {
        BlockKind::List
            if list_decoration_rows(block)
                .iter()
                .any(|marker| marker.is_some()) =>
        {
            px(LIST_MARKER_COLUMN_WIDTH + DECORATION_GAP_X)
        }
        _ => px(0.),
    }
}

fn visible_byte_offset_for_click_position(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    click_position: gpui::Point<gpui::Pixels>,
    bounds: Bounds<gpui::Pixels>,
    window: &Window,
) -> usize {
    if !has_rendered_text(block) {
        return 0;
    }

    let text_x_offset = text_content_x_offset(block);
    if let Some(line_count) = surface_empty_block_line_count(blocks, block_index) {
        let presentation = block_presentation(&block.kind);
        let line_height = px(presentation.line_height);
        let local_y = (click_position.y - bounds.top()).max(px(0.));
        let line_index = (local_y / line_height).floor() as usize;
        return line_index
            .min(line_count.saturating_sub(1))
            .min(rendered_visible_len(block));
    }

    let text_width = (bounds.size.width - text_x_offset).max(px(0.));
    if text_width <= px(0.) {
        return rendered_visible_len(block);
    }

    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let mut local = click_position - bounds.origin;
    local.x = (local.x - text_x_offset).max(px(0.));
    local.y = local.y.max(px(0.));

    let lines = shape_block_lines(block, text_width, window);
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_height_span = line.size(line_height).height;
        if local.y <= y_offset + line_height_span {
            let position = point(local.x, (local.y - y_offset).max(px(0.)));
            let local_offset = match line.closest_index_for_position(position, line_height) {
                Ok(offset) | Err(offset) => offset,
            };
            return (byte_offset + local_offset).min(rendered_visible_len(block));
        }

        y_offset += line_height_span;
        byte_offset += line.len();
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    rendered_visible_len(block)
}

pub(super) fn shape_block_lines(
    block: &RenderBlock,
    width: gpui::Pixels,
    window: &Window,
) -> Vec<gpui::WrappedLine> {
    if !has_rendered_text(block) {
        return Vec::new();
    }

    let base_style = base_text_style_for_block(block, window.text_style().color, window);
    let rendered_text = rendered_text_for_block(block);
    let runs = rendered_spans(block)
        .filter(|span| !span.visible_text.is_empty())
        .map(|span| {
            let mut style = base_style.clone();
            if span.style.strong {
                style.font_weight = FontWeight::BOLD;
            }
            if span.style.emphasis {
                style.font_style = FontStyle::Italic;
            }
            if span.style.code {
                style.font_family = SharedString::from("Consolas");
                if !span.style.strong {
                    style.font_weight = FontWeight::MEDIUM;
                }
            }
            style.to_run(span.visible_text.len())
        })
        .collect::<Vec<_>>();

    window
        .text_system()
        .shape_text(
            rendered_text.into(),
            base_style.font_size.to_pixels(window.rem_size()),
            &runs,
            Some(width),
            None,
        )
        .unwrap_or_default()
        .to_vec()
}

pub(super) fn surface_empty_block_line_count(
    blocks: &[RenderBlock],
    block_index: usize,
) -> Option<usize> {
    let block = blocks.get(block_index)?;
    let raw_line_count = rendered_empty_block_line_count(block)?;

    if is_collapsed_inter_block_empty_block(blocks, block_index) {
        Some(1)
    } else {
        Some(raw_line_count)
    }
}

pub(super) fn rendered_empty_block_line_count(block: &RenderBlock) -> Option<usize> {
    if !renders_empty_block_linebreaks(block) {
        return None;
    }

    let mut saw_linebreak = false;
    let mut newline_count = 0usize;
    for span in rendered_spans(block) {
        match span.kind {
            RenderSpanKind::LineBreak => {
                saw_linebreak = true;
                newline_count += span
                    .visible_text
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count();
            }
            _ if !span.visible_text.is_empty() => return None,
            _ => {}
        }
    }

    saw_linebreak.then_some(newline_count + 1)
}

fn is_collapsed_inter_block_empty_block(blocks: &[RenderBlock], block_index: usize) -> bool {
    let Some(block) = blocks.get(block_index) else {
        return false;
    };
    if block.kind != BlockKind::Raw
        || !block.content_range.is_empty()
        || block.source_range.is_empty()
    {
        return false;
    }

    let Some(previous) = block_index
        .checked_sub(1)
        .and_then(|index| blocks.get(index))
    else {
        return false;
    };
    let Some(next) = blocks.get(block_index + 1) else {
        return false;
    };

    previous.kind != BlockKind::Raw && next.kind != BlockKind::Raw
}

fn compressed_empty_block_visual_offset(
    blocks: &[RenderBlock],
    block_index: usize,
    local_offset: usize,
) -> usize {
    let Some(line_count) = surface_empty_block_line_count(blocks, block_index) else {
        return local_offset;
    };

    local_offset.min(line_count.saturating_sub(1))
}

fn selection_quads_for_empty_line_block(
    selection_range: std::ops::Range<usize>,
    line_count: usize,
    bounds: Bounds<gpui::Pixels>,
    line_height: gpui::Pixels,
    text_x_offset: gpui::Pixels,
    palette: RenderPalette,
) -> Vec<PaintQuad> {
    if selection_range.is_empty() || line_count == 0 {
        return Vec::new();
    }

    let start_line = selection_range.start.min(line_count.saturating_sub(1));
    let end_line = selection_range
        .end
        .saturating_sub(1)
        .min(line_count.saturating_sub(1));
    let width = (bounds.size.width - text_x_offset).max(px(1.));
    let mut quads = Vec::new();
    let mut y_offset = px(0.);

    for line_index in 0..line_count {
        if line_index >= start_line && line_index <= end_line {
            quads.push(fill(
                Bounds::new(
                    point(bounds.left() + text_x_offset, bounds.top() + y_offset),
                    size(width, line_height),
                ),
                palette.selection_color,
            ));
        }
        y_offset += line_height;
    }

    quads
}

fn wrap_boundary_index(line: &gpui::WrappedLine, boundary: &gpui::WrapBoundary) -> usize {
    let run = &line.runs()[boundary.run_ix];
    let glyph = &run.glyphs[boundary.glyph_ix];
    glyph.index
}

use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, Entity, FontStyle, FontWeight, Hsla,
    InteractiveElement, IntoElement, IsZero, MouseButton, MouseMoveEvent, ObjectFit, PaintQuad,
    ParentElement, ScrollHandle, SharedString, StatefulInteractiveElement, StrikethroughStyle,
    Styled, StyledImage, StyledText, TextStyle, UnderlineStyle, WhiteSpace, Window,
    canvas, div, fill, img, point, px, size,
};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants as _},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
};

use crate::{
    BlockKind, EditCommand, EmbeddedNodeKind, RenderBlock, RenderSpan, RenderSpanKind,
    RenderSpanMeta, SelectionState,
    core::{
        controller::EditorSnapshot,
        table::{TABLE_COLUMN_GAP, TableModel, char_display_width, str_display_width},
        text_ops::clamp_to_char_boundary,
    },
};

use super::{
    layout::block_presentation,
    typography::{body_font_size, body_line_height},
    view::{MarkdownEditor, SurfaceSelectionAnchor},
    MONOSPACE_FONT_FAMILY,
};

const LIST_MARKER_COLUMN_WIDTH: f32 = 28.;
const BLOCKQUOTE_BAR_WIDTH: f32 = 3.;
const DECORATION_GAP_X: f32 = 12.;
const TASK_MARKER_CLICK_WIDTH: f32 = 34.;

#[allow(dead_code)]
struct ShapeCacheKey {
    block_id: u64,
    width_f32: f32,
}

impl std::hash::Hash for ShapeCacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.block_id.hash(state);
        self.width_f32.to_bits().hash(state);
    }
}

impl PartialEq for ShapeCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.block_id == other.block_id && self.width_f32 == other.width_f32
    }
}

impl Eq for ShapeCacheKey {}

#[allow(dead_code)]
struct ShapeCache {
    entries: HashMap<ShapeCacheKey, Vec<gpui::WrappedLine>>,
}

#[allow(dead_code)]
impl ShapeCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get_or_shape(
        &mut self,
        block: &RenderBlock,
        width: gpui::Pixels,
        window: &Window,
    ) -> Vec<gpui::WrappedLine> {
        let key = ShapeCacheKey {
            block_id: block.id,
            width_f32: f32::from(width),
        };
        if let Some(lines) = self.entries.get(&key) {
            return lines.clone();
        }
        let lines = shape_block_lines_uncached(block, width, window);
        self.entries.insert(key, lines.clone());
        lines
    }
}

#[derive(Debug, Clone, Copy)]
struct RenderPalette {
    text_color: Hsla,
    muted_text_color: Hsla,
    selection_color: Hsla,
    caret_color: Hsla,
    code_background: Hsla,
    border_color: Hsla,
    blockquote_bar: Hsla,
    callout_bar: Hsla,
    callout_background: Hsla,
    code_surface_background: Hsla,
    find_match_color: Hsla,
    find_active_match_color: Hsla,
    link_color: Hsla,
    highlight_background: Hsla,
    code_keyword_color: Hsla,
    code_function_color: Hsla,
    code_string_color: Hsla,
    code_number_color: Hsla,
    code_comment_color: Hsla,
    code_type_color: Hsla,
    code_constant_color: Hsla,
    code_variable_color: Hsla,
    code_operator_color: Hsla,
    code_tag_color: Hsla,
    code_attribute_color: Hsla,
    code_escape_color: Hsla,
    math_color: Hsla,
    math_background: Hsla,
}

#[derive(Clone)]
struct BlockOverlay {
    find_highlight_quads: Vec<PaintQuad>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedImageSource {
    Path(PathBuf),
    Uri(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MermaidPreview {
    direction: Option<String>,
    edges: Vec<MermaidEdge>,
    unsupported_lines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MermaidGraphDirection {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MermaidEdge {
    from: MermaidNode,
    to: MermaidNode,
    label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MermaidNode {
    id: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TocPreviewEntry {
    depth: u8,
    title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FootnotePreview {
    label: String,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataPreviewEntry {
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
struct TableSurfaceLayout {
    row_tops: Vec<gpui::Pixels>,
    row_heights: Vec<gpui::Pixels>,
    column_starts: Vec<gpui::Pixels>,
    total_width: gpui::Pixels,
    total_height: gpui::Pixels,
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
    fn task_marker_hit_range(
        &self,
        block_id: u64,
        position: gpui::Point<gpui::Pixels>,
    ) -> Option<std::ops::Range<usize>> {
        let bounds = self.block_bounds.borrow().get(&block_id).copied()?;
        let block = self
            .snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| block.id == block_id)?;
        if block.kind != BlockKind::List {
            return None;
        }

        let local_x = (position.x - bounds.left() - text_content_x_offset(block)).max(px(0.));
        if local_x > px(TASK_MARKER_CLICK_WIDTH) {
            return None;
        }

        let line_height = px(block_presentation(&block.kind).line_height);
        let local_y = (position.y - bounds.top()).max(px(0.));
        let line_index = (local_y / line_height).floor() as usize;
        task_marker_range_for_line(block, line_index)
    }

    fn link_url_at_position(
        &self,
        block_id: u64,
        position: gpui::Point<gpui::Pixels>,
        window: &Window,
    ) -> Option<String> {
        let bounds = self.block_bounds.borrow().get(&block_id).copied()?;
        let (block_index, block) = self
            .snapshot
            .display_map
            .blocks
            .iter()
            .enumerate()
            .find(|(_, block)| block.id == block_id)
            .map(|(index, block)| (index, block.clone()))?;

        let local_visible_offset = visible_byte_offset_for_click_position(
            &self.snapshot.display_map.blocks,
            block_index,
            &block,
            position,
            bounds,
            window,
        );
        let visible_offset = block.visible_range.start + local_visible_offset;

        for span in &block.spans {
            if span.visible_range.start <= visible_offset
                && visible_offset <= span.visible_range.end
            {
                if let Some(crate::RenderSpanMeta::Link { target, .. }) = &span.meta {
                    if !target.is_empty() {
                        return Some(target.clone());
                    }
                }
            }
        }
        None
    }

    pub(super) fn link_url_at_cursor(&self) -> Option<String> {
        let cursor = self.snapshot.visible_selection.cursor();
        for block in &self.snapshot.display_map.blocks {
            if block.visible_range.start <= cursor && cursor <= block.visible_range.end {
                for span in &block.spans {
                    if span.visible_range.start <= cursor && cursor <= span.visible_range.end {
                        if let Some(crate::RenderSpanMeta::Link { target, .. }) = &span.meta {
                            if !target.is_empty() {
                                return Some(target.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn surface_selection_anchor_for_position(
        &self,
        block_id: u64,
        position: gpui::Point<gpui::Pixels>,
        selection_range: std::ops::Range<usize>,
        window: &Window,
    ) -> Option<SurfaceSelectionAnchor> {
        let bounds = self.block_bounds.borrow().get(&block_id).copied()?;
        let (block_index, block) = self
            .snapshot
            .display_map
            .blocks
            .iter()
            .enumerate()
            .find(|(_, block)| block.id == block_id)
            .map(|(index, block)| (index, block.clone()))?;

        let source_offset = if should_render_image_preview(&block, selection_range.clone()) {
            image_edit_cursor_offset(&block)
        } else if should_render_mermaid_preview(&block, selection_range.clone()) {
            mermaid_edit_cursor_offset(&block)
        } else if should_render_toc_preview(&block, selection_range.clone()) {
            toc_edit_cursor_offset(&block)
        } else if should_render_footnote_preview(&block, selection_range.clone()) {
            footnote_edit_cursor_offset(&block)
        } else if should_render_front_matter_preview(&block, selection_range) {
            front_matter_edit_cursor_offset(&block)
        } else {
            let local_visible_offset = visible_byte_offset_for_click_position(
                &self.snapshot.display_map.blocks,
                block_index,
                &block,
                position,
                bounds,
                window,
            );
            let visible_offset = clamp_to_char_boundary(
                &self.snapshot.display_map.visible_text,
                block.visible_range.start + local_visible_offset,
            );
            self.snapshot
                .display_map
                .visible_to_source(visible_offset)
                .source_offset
        };
        let visible_offset = self.snapshot.display_map.source_to_visible(source_offset);

        Some(SurfaceSelectionAnchor {
            source_offset,
            visible_offset,
        })
    }

    fn apply_surface_selection(
        &mut self,
        anchor: SurfaceSelectionAnchor,
        head: SurfaceSelectionAnchor,
        preserve_anchor: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selection = if preserve_anchor {
            SelectionState {
                anchor_byte: anchor.source_offset,
                head_byte: head.source_offset,
                preferred_column: None,
                affinity: if head.visible_offset < anchor.visible_offset {
                    crate::SelectionAffinity::Upstream
                } else {
                    crate::SelectionAffinity::Downstream
                },
            }
        } else {
            SelectionState::collapsed(head.source_offset)
        };
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
        if event.modifiers().platform {
            if let Some(url) = self.link_url_at_position(block_id, event.position(), window) {
                let _ = open::that(&url);
                return;
            }
        }

        if let Some(range) = self.task_marker_hit_range(block_id, event.position()) {
            self.toggle_task_marker(range, window, cx);
            return;
        }

        let selection_range = self.snapshot.selection.range();
        let Some(hit) = self.surface_selection_anchor_for_position(
            block_id,
            event.position(),
            selection_range,
            window,
        ) else {
            self.focus_input(window, cx);
            return;
        };

        if event.click_count() >= 3 {
            // 三击选整块：取点击所在 block 的完整 visible range → source
            let block = self
                .snapshot
                .display_map
                .blocks
                .iter()
                .find(|block| block.id == block_id)
                .cloned();
            if let Some(block) = block {
                let source_selection =
                    self.snapshot
                        .display_map
                        .visible_selection_to_source(&SelectionState {
                            anchor_byte: block.visible_range.start,
                            head_byte: block.visible_range.end,
                            preferred_column: None,
                            affinity: crate::SelectionAffinity::Downstream,
                        });
                let effects = self.controller.dispatch(EditCommand::SetSelection {
                    selection: source_selection,
                });
                self.apply_effects(window, cx, effects);
                self.focus_input(window, cx);
                self.drag_selection_anchor = None;
            } else {
                self.apply_surface_selection(hit, hit, false, window, cx);
                self.drag_selection_anchor = None;
            }
            return;
        }

        if event.click_count() == 2 {
            // 双击选词：基于 visible text 计算词边界，再映射回 source
            if let Some(visible_word) = word_range_at_visible_offset(
                &self.snapshot.display_map.visible_text,
                hit.visible_offset,
            ) {
                let source_selection =
                    self.snapshot
                        .display_map
                        .visible_selection_to_source(&SelectionState {
                            anchor_byte: visible_word.start,
                            head_byte: visible_word.end,
                            preferred_column: None,
                            affinity: crate::SelectionAffinity::Downstream,
                        });
                let effects = self.controller.dispatch(EditCommand::SetSelection {
                    selection: source_selection,
                });
                self.apply_effects(window, cx, effects);
                self.focus_input(window, cx);
                self.drag_selection_anchor = None;
            } else {
                // 双击空白/标点：退化为单击定位
                self.apply_surface_selection(hit, hit, false, window, cx);
                self.drag_selection_anchor = None;
            }
            return;
        }

        let anchor = if event.modifiers().shift {
            self.drag_selection_anchor
                .unwrap_or(SurfaceSelectionAnchor {
                    source_offset: self.snapshot.selection.anchor_byte,
                    visible_offset: self.snapshot.visible_selection.anchor_byte,
                })
        } else {
            hit
        };
        self.apply_surface_selection(anchor, hit, event.modifiers().shift, window, cx);
        self.drag_selection_anchor = None;
    }

    pub(super) fn handle_surface_mouse_down(
        &mut self,
        block_id: u64,
        position: gpui::Point<gpui::Pixels>,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.task_marker_hit_range(block_id, position).is_some() {
            self.focus_input(window, cx);
            return;
        }

        let selection_range = self.snapshot.selection.range();
        let Some(head) =
            self.surface_selection_anchor_for_position(block_id, position, selection_range, window)
        else {
            self.focus_input(window, cx);
            return;
        };
        let anchor = if shift {
            self.drag_selection_anchor
                .unwrap_or(SurfaceSelectionAnchor {
                    source_offset: self.snapshot.selection.anchor_byte,
                    visible_offset: self.snapshot.visible_selection.anchor_byte,
                })
        } else {
            head
        };
        self.drag_selection_anchor = Some(anchor);
        self.apply_surface_selection(anchor, head, shift, window, cx);
    }

    pub(super) fn handle_surface_mouse_move(
        &mut self,
        block_id: u64,
        event: &MouseMoveEvent,
        scroll_handle: &ScrollHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.dragging() {
            return;
        }
        let Some(anchor) = self.drag_selection_anchor else {
            return;
        };

        let viewport_bounds = scroll_handle.bounds();
        if !viewport_bounds.is_zero() {
            let edge_threshold = px(24.);
            let max_scroll_y = scroll_handle.max_offset().height.max(px(0.));
            let current_offset = scroll_handle.offset();
            let mut next_offset = current_offset;

            if event.position.y < viewport_bounds.top() + edge_threshold {
                let distance =
                    (viewport_bounds.top() + edge_threshold - event.position.y).max(px(0.));
                next_offset.y = (current_offset.y + distance.min(px(18.))).min(px(0.));
            } else if event.position.y > viewport_bounds.bottom() - edge_threshold {
                let distance =
                    (event.position.y - (viewport_bounds.bottom() - edge_threshold)).max(px(0.));
                next_offset.y = (current_offset.y - distance.min(px(18.))).max(-max_scroll_y);
            }

            if next_offset != current_offset {
                scroll_handle.set_offset(next_offset);
                cx.notify();
            }
        }

        let selection_range = anchor.source_offset..anchor.source_offset;
        let Some(head) = self.surface_selection_anchor_for_position(
            block_id,
            event.position,
            selection_range,
            window,
        ) else {
            return;
        };
        self.apply_surface_selection(anchor, head, true, window, cx);
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

const VIRTUAL_RENDER_MIN_BLOCKS: usize = 20;
const VIRTUAL_RENDER_OVERDRAW_PX: f32 = 400.;

fn estimated_block_height(block: &RenderBlock) -> gpui::Pixels {
    let presentation = block_presentation(&block.kind);
    let line_count = block
        .visible_text
        .chars()
        .filter(|&c| c == '\n')
        .count()
        .max(1);
    px(presentation.line_height * line_count as f32
        + presentation.block_padding_y * 2.
        + presentation.row_spacing_y * 2.)
}

fn compute_visible_block_range(
    block_heights: &HashMap<u64, gpui::Pixels>,
    blocks: &[RenderBlock],
    scroll_offset_y: gpui::Pixels,
    viewport_height: gpui::Pixels,
    overdraw: gpui::Pixels,
) -> Range<usize> {
    let top = -scroll_offset_y - overdraw;
    let bottom = -scroll_offset_y + viewport_height + overdraw;

    let mut cumulative_y = px(0.);
    let mut first_visible = 0;
    let mut last_visible = blocks.len().saturating_sub(1);
    let mut found_first = false;

    for (i, block) in blocks.iter().enumerate() {
        let height = block_heights
            .get(&block.id)
            .copied()
            .unwrap_or_else(|| estimated_block_height(block));
        let block_top = cumulative_y;
        let block_bottom = cumulative_y + height;

        if !found_first && block_bottom > top {
            first_visible = i;
            found_first = true;
        }
        if block_top < bottom {
            last_visible = i;
        }

        cumulative_y = block_bottom;
    }

    first_visible..last_visible + 1
}

fn render_placeholder_block(
    block: &RenderBlock,
    cached_height: Option<gpui::Pixels>,
) -> AnyElement {
    let height = cached_height.unwrap_or_else(|| estimated_block_height(block));
    div()
        .id(("placeholder-block", block.id))
        .w_full()
        .h(height)
        .into_any_element()
}

pub(super) fn render_document_surface(
    view: &Entity<MarkdownEditor>,
    snapshot: &EditorSnapshot,
    input_focused: bool,
    cursor_blink_visible: bool,
    focus_highlight_mode: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    block_heights: Rc<RefCell<HashMap<u64, gpui::Pixels>>>,
    scroll_handle: ScrollHandle,
    floating_panel_block_id: Option<u64>,
    mut floating_panel: Option<AnyElement>,
    math_render_cache: Rc<RefCell<crate::core::math_render::MathRenderCache>>,
    window: &mut Window,
    cx: &mut Context<MarkdownEditor>,
) -> AnyElement {
    let display_blocks = Rc::new(snapshot.display_map.blocks.clone());
    let fg = cx.theme().foreground;
    let is_dark = fg.l > 0.5;

    let syntax_theme = crate::ui::theme::get_syntax_theme();
    let (
        keyword_hue,
        string_hue,
        number_hue,
        comment_hue,
        type_hue,
        constant_hue,
        function_hue,
        tag_hue,
        attribute_hue,
        escape_hue,
    ) = syntax_theme.hue_set();

    let palette = RenderPalette {
        text_color: fg,
        muted_text_color: cx.theme().muted_foreground,
        selection_color: fg.opacity(0.14),
        caret_color: fg,
        code_background: fg.opacity(0.08),
        border_color: fg.opacity(0.12),
        blockquote_bar: fg.opacity(0.18),
        callout_bar: syntax_theme.link_color(is_dark).opacity(0.55),
        callout_background: syntax_theme.link_color(is_dark).opacity(0.08),
        code_surface_background: fg.opacity(0.04),
        find_match_color: fg.opacity(0.12),
        find_active_match_color: fg.opacity(0.30),
        link_color: syntax_theme.link_color(is_dark),
        highlight_background: syntax_theme.highlight_color(is_dark),
        code_keyword_color: Hsla {
            h: keyword_hue,
            s: syntax_theme.keyword_saturation(),
            l: if is_dark { 0.72 } else { 0.48 },
            a: 1.0,
        },
        code_function_color: Hsla {
            h: function_hue,
            s: 0.65,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        code_string_color: Hsla {
            h: string_hue,
            s: 0.55,
            l: if is_dark { 0.72 } else { 0.38 },
            a: 1.0,
        },
        code_number_color: Hsla {
            h: number_hue,
            s: 0.65,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        code_comment_color: Hsla {
            h: comment_hue,
            s: syntax_theme.comment_saturation(),
            l: if is_dark { 0.55 } else { 0.42 },
            a: 1.0,
        },
        code_type_color: Hsla {
            h: type_hue,
            s: 0.55,
            l: if is_dark { 0.72 } else { 0.38 },
            a: 1.0,
        },
        code_constant_color: Hsla {
            h: constant_hue,
            s: 0.65,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        code_variable_color: Hsla {
            h: 0.0,
            s: 0.0,
            l: if is_dark { 0.82 } else { 0.2 },
            a: 1.0,
        },
        code_operator_color: Hsla {
            h: 0.0,
            s: 0.0,
            l: if is_dark { 0.72 } else { 0.35 },
            a: 1.0,
        },
        code_tag_color: Hsla {
            h: tag_hue,
            s: 0.55,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        code_attribute_color: Hsla {
            h: attribute_hue,
            s: 0.45,
            l: if is_dark { 0.72 } else { 0.38 },
            a: 1.0,
        },
        code_escape_color: Hsla {
            h: escape_hue,
            s: 0.65,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        math_color: Hsla {
            h: 280.0,
            s: 0.55,
            l: if is_dark { 0.72 } else { 0.42 },
            a: 1.0,
        },
        math_background: fg.opacity(0.06),
    };

    if snapshot.display_map.blocks.is_empty() {
        let empty_view = view.clone();
        let empty_click_view = view.clone();
        return div()
            .id("empty-surface")
            .block_mouse_except_scroll()
            .w_full()
            .min_h(px(body_line_height()))
            .text_size(px(body_font_size()))
            .line_height(px(body_line_height()))
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

    let enable_virtual = snapshot.display_map.blocks.len() >= VIRTUAL_RENDER_MIN_BLOCKS;
    let visible_range = if enable_virtual {
        let offset = scroll_handle.offset();
        let viewport = scroll_handle.bounds();
        let heights = block_heights.borrow();
        compute_visible_block_range(
            &heights,
            &snapshot.display_map.blocks,
            offset.y,
            viewport.size.height,
            px(VIRTUAL_RENDER_OVERDRAW_PX),
        )
    } else {
        0..snapshot.display_map.blocks.len()
    };

    let heights_map = block_heights.borrow();
    let cursor_block_index = if focus_highlight_mode && input_focused {
        snapshot.display_map.blocks.iter().position(|block| {
            let cursor = snapshot.visible_selection.cursor();
            cursor >= block.visible_range.start && cursor <= rendered_visible_end(block)
        })
    } else {
        None
    };
    let mut document = div().w_full().flex().flex_col();
    for (block_index, block) in snapshot.display_map.blocks.iter().cloned().enumerate() {
        if visible_range.contains(&block_index) {
            let block_opacity = if let Some(cursor_idx) = cursor_block_index {
                let distance = if block_index >= cursor_idx {
                    block_index - cursor_idx
                } else {
                    cursor_idx - block_index
                };
                match distance {
                    0 => 1.0,
                    1 => 0.55,
                    2 => 0.35,
                    _ => 0.22,
                }
            } else {
                1.0
            };
            document = document.child(render_display_block(
                view,
                snapshot,
                display_blocks.clone(),
                block_index,
                &block,
                input_focused,
                cursor_blink_visible,
                block_bounds.clone(),
                scroll_handle.clone(),
                palette,
                block_opacity,
                if Some(block.id) == floating_panel_block_id {
                    floating_panel.take()
                } else {
                    None
                },
                math_render_cache.clone(),
                window,
            ));
        } else {
            let cached_height = heights_map.get(&block.id).copied();
            document = document.child(render_placeholder_block(&block, cached_height));
        }
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
    cursor_blink_visible: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    scroll_handle: ScrollHandle,
    palette: RenderPalette,
    block_opacity: f32,
    floating_panel: Option<AnyElement>,
    math_render_cache: Rc<RefCell<crate::core::math_render::MathRenderCache>>,
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
    let find_matches = snapshot.find_matches.clone();
    let active_find_index = snapshot.active_find_index;
    let display_map = &snapshot.display_map;
    let visible_find_ranges: Vec<(std::ops::Range<usize>, bool)> = find_matches
        .iter()
        .enumerate()
        .filter_map(|(i, source_range)| {
            let visible_start = display_map.source_to_visible(source_range.start);
            let visible_end = display_map.source_to_visible(source_range.end);
            if visible_start < visible_end {
                let is_active = active_find_index == Some(i);
                Some((visible_start..visible_end, is_active))
            } else {
                None
            }
        })
        .collect();
    let block_id = block.id;
    let block_clone = block.clone();
    let overlay_block = block.clone();
    let block_view = view.clone();
    let block_click_view = view.clone();
    let show_table_toolbar =
        matches!(block.kind, BlockKind::Table) && selection_is_within_render_block(snapshot, block);
    let show_image_preview = should_render_image_preview(block, snapshot.selection.range());
    let show_mermaid_preview = should_render_mermaid_preview(block, snapshot.selection.range());
    let show_toc_preview = should_render_toc_preview(block, snapshot.selection.range());
    let show_footnote_preview = should_render_footnote_preview(block, snapshot.selection.range());
    let show_front_matter_preview =
        should_render_front_matter_preview(block, snapshot.selection.range());

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
            BlockKind::Table => render_table_block(
                block,
                snapshot.document_text[block.content_range.clone()].to_string(),
                palette,
                window,
            ),
            BlockKind::ThematicBreak => render_thematic_break(palette),
            BlockKind::MathBlock => render_math_block(&block_clone, palette, window, &math_render_cache),
            _ if show_image_preview => render_image_block(snapshot, block, palette),
            _ if show_mermaid_preview => render_mermaid_block(block, palette),
            _ if show_toc_preview => render_toc_block(display_blocks.as_ref(), palette),
            _ if show_footnote_preview => render_footnote_block(block, palette, window),
            _ if show_front_matter_preview => render_front_matter_block(block, palette),
            _ if empty_line_count.is_some() => {
                render_empty_line_block(block, empty_line_count.unwrap_or(1))
            }
            _ => div()
                .w_full()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .when(
                    matches!(
                        block.kind,
                        BlockKind::CodeFence { .. } | BlockKind::SourceCode
                    ),
                    |this| this.font_family(MONOSPACE_FONT_FAMILY),
                )
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
                        cursor_blink_visible,
                        bounds,
                        palette,
                        &visible_find_ranges,
                        window,
                    )
                },
                move |_, overlay, window, _| {
                    for quad in overlay.find_highlight_quads {
                        window.paint_quad(quad);
                    }
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
        BlockKind::Callout { .. } => div()
            .w_full()
            .rounded(px(6.))
            .border_1()
            .border_color(palette.callout_bar.opacity(0.35))
            .bg(palette.callout_background)
            .px_3()
            .py_2()
            .flex()
            .gap(px(DECORATION_GAP_X))
            .child(
                div()
                    .w(px(BLOCKQUOTE_BAR_WIDTH))
                    .min_h(px(presentation.line_height))
                    .bg(palette.callout_bar),
            )
            .child(text_area)
            .into_any_element(),
        BlockKind::List => text_area,
        BlockKind::SourceCode => {
            let line_count = block.visible_text.lines().count().max(1);
            let line_number_width = format!("{}", line_count).len().max(2);
            let gutter_width = line_number_width as f32 * 8.0 + 16.0;

            let line_numbers = (1..=line_count)
                .map(|i| format!("{:>width$}", i, width = line_number_width))
                .collect::<Vec<_>>()
                .join("\n");

            let line_numbers_el = div()
                .w(px(gutter_width))
                .flex_shrink_0()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .font_family(MONOSPACE_FONT_FAMILY)
                .text_color(palette.muted_text_color.opacity(0.5))
                .pr(px(8.))
                .child(line_numbers);

            let code_content = div().flex_1().min_w(px(0.)).child(text_area);

            div()
                .w_full()
                .rounded(px(8.))
                .border_1()
                .border_color(palette.border_color)
                .bg(palette.code_surface_background)
                .px_3()
                .py_2()
                .flex()
                .child(line_numbers_el)
                .child(code_content)
                .into_any_element()
        }
        BlockKind::CodeFence { language } => {
            let line_count = block.visible_text.lines().count().max(1);
            let line_number_width = format!("{}", line_count).len().max(2);
            let gutter_width = line_number_width as f32 * 8.0 + 16.0;

            let line_numbers = (1..=line_count)
                .map(|i| format!("{:>width$}", i, width = line_number_width))
                .collect::<Vec<_>>()
                .join("\n");

            let line_numbers_el = div()
                .w(px(gutter_width))
                .flex_shrink_0()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .font_family(MONOSPACE_FONT_FAMILY)
                .text_color(palette.muted_text_color.opacity(0.5))
                .pr(px(8.))
                .child(line_numbers);

            let code_content = div().flex_1().min_w(px(0.)).child(text_area);

            let mut code_surface = div()
                .w_full()
                .rounded(px(8.))
                .border_1()
                .border_color(palette.border_color)
                .bg(palette.code_surface_background)
                .px_3()
                .py_2()
                .flex()
                .child(line_numbers_el)
                .child(code_content);

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
    let content = if show_table_toolbar {
        let add_row_view = view.clone();
        let remove_row_view = view.clone();
        let add_col_view = view.clone();
        let remove_col_view = view.clone();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new(("table-row-add", block.id))
                            .label("Row+")
                            .ghost()
                            .compact()
                            .tooltip("Insert row")
                            .on_click(move |_, window, app: &mut App| {
                                let _ = add_row_view.update(app, |this, cx| {
                                    this.insert_table_row(window, cx);
                                });
                            }),
                    )
                    .child(
                        Button::new(("table-row-del", block.id))
                            .label("Row-")
                            .ghost()
                            .compact()
                            .tooltip("Delete row")
                            .on_click(move |_, window, app: &mut App| {
                                let _ = remove_row_view.update(app, |this, cx| {
                                    this.delete_table_row(window, cx);
                                });
                            }),
                    )
                    .child(
                        Button::new(("table-col-add", block.id))
                            .label("Col+")
                            .ghost()
                            .compact()
                            .tooltip("Insert column")
                            .on_click(move |_, window, app: &mut App| {
                                let _ = add_col_view.update(app, |this, cx| {
                                    this.insert_table_column(window, cx);
                                });
                            }),
                    )
                    .child(
                        Button::new(("table-col-del", block.id))
                            .label("Col-")
                            .ghost()
                            .compact()
                            .tooltip("Delete column")
                            .on_click(move |_, window, app: &mut App| {
                                let _ = remove_col_view.update(app, |this, cx| {
                                    this.delete_table_column(window, cx);
                                });
                            }),
                    ),
            )
            .child(content)
            .into_any_element()
    } else {
        content
    };

    let context_menu_view = view.clone();
    let context_block_kind = block.kind.clone();

    div()
        .id(("display-block", block.id))
        .w_full()
        .py(px(presentation.row_spacing_y))
        .when(block_opacity < 1.0, |this| this.opacity(block_opacity))
        .context_menu(move |menu, _, cx| {
            let link_url = context_menu_view.read(cx).link_url_at_cursor();
            build_editor_context_menu(menu, &context_menu_view, &context_block_kind, link_url)
        })
        .child(
            div()
                .id(("surface-hit-target", block_id))
                .relative()
                .block_mouse_except_scroll()
                .w_full()
                .px_1()
                .py(px(presentation.block_padding_y))
                .on_mouse_down(MouseButton::Left, {
                    let block_view = block_view.clone();
                    move |event, window, app: &mut App| {
                        app.stop_propagation();
                        let position = event.position;
                        let shift = event.modifiers.shift;
                        let _ = block_view.update(app, |this, cx| {
                            this.handle_surface_mouse_down(block_id, position, shift, window, cx);
                        });
                    }
                })
                .on_mouse_move({
                    let scroll_handle = scroll_handle.clone();
                    move |event, window, app: &mut App| {
                        if !event.dragging() {
                            return;
                        }
                        app.stop_propagation();
                        let _ = block_view.update(app, |this, cx| {
                            this.handle_surface_mouse_move(
                                block_id,
                                event,
                                &scroll_handle,
                                window,
                                cx,
                            );
                        });
                    }
                })
                .on_click(move |event, window, cx| {
                    let _ = block_click_view.update(cx, |this, cx| {
                        this.handle_surface_click(block_id, event, window, cx);
                    });
                })
                .child(content)
                .when_some(floating_panel, |this, panel| this.child(panel)),
        )
        .into_any_element()
}

fn build_block_overlay(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    selection: &SelectionState,
    input_focused: bool,
    cursor_blink_visible: bool,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    visible_find_ranges: &[(std::ops::Range<usize>, bool)],
    window: &mut Window,
) -> BlockOverlay {
    let find_highlight_quads = find_highlight_quads_for_block(
        blocks,
        block_index,
        block,
        visible_find_ranges,
        bounds,
        palette,
        window,
    );
    let selection_range = selection_range_in_block(block, selection);
    let selection_quads = selection_range
        .filter(|range| !range.is_empty())
        .map(|range| {
            selection_quads_for_block(blocks, block_index, block, range, bounds, palette, window)
        })
        .unwrap_or_default();
    let caret_quad = if input_focused && cursor_blink_visible && selection.is_collapsed() {
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
        find_highlight_quads,
        selection_quads,
        caret_quad,
    }
}

fn find_highlight_quads_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    visible_find_ranges: &[(std::ops::Range<usize>, bool)],
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> Vec<PaintQuad> {
    let block_visible_end = rendered_visible_end(block);
    let mut quads = Vec::new();
    for (visible_range, is_active) in visible_find_ranges {
        let start = visible_range.start.max(block.visible_range.start);
        let end = visible_range.end.min(block_visible_end);
        if start >= end {
            continue;
        }
        let local_range = start.saturating_sub(block.visible_range.start)
            ..end.saturating_sub(block.visible_range.start);
        let color = if *is_active {
            palette.find_active_match_color
        } else {
            palette.find_match_color
        };
        let match_quads = highlight_quads_for_block_range(
            blocks,
            block_index,
            block,
            local_range,
            bounds,
            color,
            window,
        );
        quads.extend(match_quads);
    }
    quads
}

fn highlight_quads_for_block_range(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    range: std::ops::Range<usize>,
    bounds: Bounds<gpui::Pixels>,
    color: Hsla,
    window: &mut Window,
) -> Vec<PaintQuad> {
    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let text_x_offset = text_content_x_offset(block);
    if let Some(line_count) = surface_empty_block_line_count(blocks, block_index) {
        if range.is_empty() || line_count == 0 {
            return Vec::new();
        }
        let start_line = range.start.min(line_count.saturating_sub(1));
        let end_line = range
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
                    color,
                ));
            }
            y_offset += line_height;
        }
        return quads;
    }
    let text_width = (bounds.size.width - text_x_offset).max(px(0.));
    let lines = shape_block_lines_uncached(block, text_width, window);
    if lines.is_empty() {
        return Vec::new();
    }

    let mut quads = Vec::new();
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_len = line.len();
        let line_range = byte_offset..byte_offset + line_len;
        if range.start < line_range.end && line_range.start < range.end {
            let local_start = range.start.saturating_sub(line_range.start);
            let local_end = (range.end.min(line_range.end)).saturating_sub(line_range.start);
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
                        color,
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
    let lines = shape_block_lines_uncached(block, text_width, window);
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
    let lines = shape_block_lines_uncached(block, text_width, window);
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
    let base_font_size = px(block_presentation(&block.kind).font_size);

    for span in rendered_spans(block).filter(|span| !span.visible_text.is_empty()) {
        let mut style = base_style.clone();
        apply_fragment_style(
            &mut style,
            span.kind.clone(),
            span.style,
            span.meta.as_ref(),
            palette,
            base_font_size,
        );
        runs.push(style.to_run(span.visible_text.len()));
    }

    StyledText::new(text).with_runs(runs)
}

fn render_image_block(
    snapshot: &EditorSnapshot,
    block: &RenderBlock,
    palette: RenderPalette,
) -> AnyElement {
    let Some(RenderSpanMeta::Image { src, alt, .. }) =
        standalone_image_span(block).and_then(|span| span.meta.as_ref())
    else {
        return image_placeholder(
            "Image preview unavailable".to_string(),
            Some("Unsupported markdown image block".to_string()),
            palette,
        );
    };

    let Some(resolved_source) = resolve_image_source(src, document_base_dir(snapshot)) else {
        return image_placeholder(
            alt_if_present(alt, "Image path missing"),
            Some("Add a local or remote image path.".to_string()),
            palette,
        );
    };

    let fallback_title = alt_if_present(alt, "Image unavailable");
    let fallback_detail = src.clone();
    let loading_title = alt_if_present(alt, "Loading image");

    let image = match resolved_source {
        ResolvedImageSource::Path(path) => img(path),
        ResolvedImageSource::Uri(uri) => img(uri),
    }
    .w_full()
    .max_h(px(420.))
    .object_fit(ObjectFit::Contain)
    .with_fallback(move || {
        image_placeholder(
            fallback_title.clone(),
            Some(fallback_detail.clone()),
            palette,
        )
    })
    .with_loading(move || image_placeholder(loading_title.clone(), None, palette));

    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .p_2()
        .child(div().w_full().overflow_hidden().child(image))
        .into_any_element()
}

fn image_placeholder(title: String, detail: Option<String>, palette: RenderPalette) -> AnyElement {
    div()
        .w_full()
        .min_h(px(180.))
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .px_4()
        .py_5()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .text_color(palette.muted_text_color)
        .child(
            div()
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .when_some(detail, |this, detail| {
            this.child(
                div()
                    .text_sm()
                    .line_height(px(body_line_height()))
                    .child(detail),
            )
        })
        .into_any_element()
}

fn render_front_matter_block(block: &RenderBlock, palette: RenderPalette) -> AnyElement {
    let entries = front_matter_preview_entries(block);

    let mut body = div().w_full().flex().flex_col().gap_2();
    if entries.is_empty() {
        body = body.child(
            div()
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .text_color(palette.muted_text_color)
                .child("No metadata yet"),
        );
    } else {
        for entry in &entries {
            body = body.child(render_metadata_entry(entry, palette));
        }
    }

    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .px_3()
        .py_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.muted_text_color)
                .child("Metadata"),
        )
        .child(body)
        .into_any_element()
}

fn render_metadata_entry(entry: &MetadataPreviewEntry, palette: RenderPalette) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_start()
        .gap_2()
        .child(
            div()
                .min_w(px(92.))
                .rounded(px(6.))
                .bg(palette.text_color.opacity(0.06))
                .px_2()
                .py(px(2.))
                .text_sm()
                .line_height(px(body_line_height()))
                .text_color(palette.muted_text_color)
                .child(entry.key.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .text_color(palette.text_color)
                .child(entry.value.clone()),
        )
        .into_any_element()
}

fn front_matter_preview_entries(block: &RenderBlock) -> Vec<MetadataPreviewEntry> {
    rendered_text_for_block(block)
        .lines()
        .filter_map(parse_metadata_preview_entry)
        .collect()
}

fn parse_metadata_preview_entry(line: &str) -> Option<MetadataPreviewEntry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let delimiter = line.find(':').or_else(|| line.find('='))?;
    let key = line[..delimiter].trim();
    if key.is_empty() {
        return None;
    }

    Some(MetadataPreviewEntry {
        key: key.to_string(),
        value: clean_metadata_preview_value(&line[delimiter + 1..]),
    })
}

fn clean_metadata_preview_value(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(',').trim();
    let unwrapped = if let Some(list) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        list.split(',')
            .map(clean_metadata_scalar)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        clean_metadata_scalar(trimmed)
    };

    if unwrapped.is_empty() {
        "Empty".to_string()
    } else {
        unwrapped
    }
}

fn clean_metadata_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn render_footnote_block(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
) -> AnyElement {
    let preview = footnote_preview(block);
    let body = footnote_body_styled_text(block, palette, window)
        .unwrap_or_else(|| StyledText::new(preview.body.clone()));

    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .px_3()
        .py_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(palette.muted_text_color)
                        .child("Footnote"),
                )
                .child(render_footnote_label_chip(preview.label, palette)),
        )
        .child(
            div()
                .w_full()
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .text_color(palette.text_color)
                .child(body),
        )
        .into_any_element()
}

fn render_footnote_label_chip(label: String, palette: RenderPalette) -> AnyElement {
    div()
        .rounded(px(6.))
        .bg(palette.text_color.opacity(0.06))
        .px_2()
        .py(px(2.))
        .text_sm()
        .line_height(px(body_line_height()))
        .text_color(palette.muted_text_color)
        .child(label)
        .into_any_element()
}

fn footnote_preview(block: &RenderBlock) -> FootnotePreview {
    let label = footnote_label(block).unwrap_or_else(|| "?".to_string());
    let body = footnote_body_text(block);
    FootnotePreview { label, body }
}

fn footnote_label(block: &RenderBlock) -> Option<String> {
    block
        .spans
        .iter()
        .find_map(|span| extract_footnote_label(&span.source_text))
        .filter(|label| !label.is_empty())
}

fn extract_footnote_label(source: &str) -> Option<String> {
    let rest = source.trim_start().strip_prefix("[^")?;
    let mut label = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
        if escaped {
            label.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ']' {
            return Some(label);
        } else {
            label.push(ch);
        }
    }
    None
}

fn footnote_body_text(block: &RenderBlock) -> String {
    let body = rendered_spans(block)
        .filter(|span| {
            !span.visible_text.is_empty()
                && !span.hidden
                && !span.source_range.is_empty()
                && span.source_range.start >= block.content_range.start
        })
        .map(|span| span.visible_text.as_str())
        .collect::<String>()
        .trim()
        .to_string();

    if body.is_empty() {
        "Empty footnote".to_string()
    } else {
        body
    }
}

fn footnote_body_styled_text(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
) -> Option<StyledText> {
    let base_style = base_text_style_for_block(block, palette.text_color, window);
    let base_font_size = px(block_presentation(&block.kind).font_size);
    let mut text = String::new();
    let mut runs = Vec::new();

    for span in rendered_spans(block).filter(|span| {
        !span.visible_text.is_empty()
            && !span.hidden
            && !span.source_range.is_empty()
            && span.source_range.start >= block.content_range.start
    }) {
        let mut style = base_style.clone();
        apply_fragment_style(
            &mut style,
            span.kind.clone(),
            span.style,
            span.meta.as_ref(),
            palette,
            base_font_size,
        );
        text.push_str(&span.visible_text);
        runs.push(style.to_run(span.visible_text.len()));
    }

    if text.trim().is_empty() {
        None
    } else {
        Some(StyledText::new(text).with_runs(runs))
    }
}

fn render_toc_block(blocks: &[RenderBlock], palette: RenderPalette) -> AnyElement {
    let entries = collect_toc_preview_entries(blocks);

    let mut body = div().w_full().flex().flex_col().gap_1();
    if entries.is_empty() {
        body = body.child(
            div()
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .text_color(palette.muted_text_color)
                .child("No headings yet"),
        );
    } else {
        for entry in &entries {
            body = body.child(render_toc_entry(entry, palette));
        }
    }

    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .px_3()
        .py_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.muted_text_color)
                .child("Table of Contents"),
        )
        .child(body)
        .into_any_element()
}

fn render_toc_entry(entry: &TocPreviewEntry, palette: RenderPalette) -> AnyElement {
    let depth = entry.depth.saturating_sub(1).min(5) as f32;
    let marker_size = if entry.depth <= 2 { 5. } else { 4. };

    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .pl(px(depth * 14.))
        .py(px(1.))
        .child(
            div()
                .w(px(marker_size))
                .h(px(marker_size))
                .rounded(px(marker_size / 2.))
                .bg(palette.muted_text_color.opacity(0.55)),
        )
        .child(
            div()
                .min_w(px(0.))
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .font_weight(if entry.depth <= 2 {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(palette.text_color)
                .child(entry.title.clone()),
        )
        .into_any_element()
}

fn collect_toc_preview_entries(blocks: &[RenderBlock]) -> Vec<TocPreviewEntry> {
    blocks
        .iter()
        .filter_map(|block| {
            let BlockKind::Heading { depth } = &block.kind else {
                return None;
            };

            Some(TocPreviewEntry {
                depth: *depth,
                title: heading_title_for_toc(block),
            })
        })
        .collect()
}

fn heading_title_for_toc(block: &RenderBlock) -> String {
    let title = rendered_spans(block)
        .filter(|span| {
            !span.visible_text.is_empty()
                && span.kind != RenderSpanKind::HiddenSyntax
                && span.kind != RenderSpanKind::LineBreak
        })
        .map(|span| span.visible_text.as_str())
        .collect::<String>()
        .trim()
        .to_string();

    if title.is_empty() {
        "Untitled heading".to_string()
    } else {
        title
    }
}

fn render_mermaid_block(block: &RenderBlock, palette: RenderPalette) -> AnyElement {
    let source = rendered_text_for_block(block);
    let preview = parse_mermaid_preview(&source);

    let mut header = div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.muted_text_color)
                .child("Mermaid"),
        );

    if let Some(direction) = &preview.direction {
        header = header.child(render_mermaid_chip(direction.clone(), palette));
    }

    let nodes = collect_mermaid_preview_nodes(&preview);
    if !preview.edges.is_empty() {
        header = header.child(
            div()
                .text_sm()
                .text_color(palette.muted_text_color.opacity(0.7))
                .child(format!("{} nodes, {} links", nodes.len(), preview.edges.len())),
        );
    }

    let mut body = div().w_full().flex().flex_col().gap_2();
    if preview.edges.is_empty() {
        body = body.child(render_mermaid_source_fallback(&source, palette));
    } else {
        body = body.child(render_mermaid_graph_preview(&preview, palette));
        if preview.edges.len() > 8 {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(palette.muted_text_color)
                    .child(format!("+{} more links", preview.edges.len() - 8)),
            );
        }
        if preview.unsupported_lines > 0 {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(palette.muted_text_color.opacity(0.75))
                    .child(format!("{} other statements", preview.unsupported_lines)),
            );
        }
    }

    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.code_surface_background)
        .px_3()
        .py_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(header)
        .child(body)
        .into_any_element()
}

fn render_mermaid_graph_preview(preview: &MermaidPreview, palette: RenderPalette) -> AnyElement {
    let direction = mermaid_graph_direction(preview);
    let mut graph = div().w_full().flex().flex_col().gap_2();
    for edge in preview.edges.iter().take(8) {
        graph = graph.child(render_mermaid_edge_card(edge, direction, palette));
    }
    graph.into_any_element()
}

fn render_mermaid_edge_card(
    edge: &MermaidEdge,
    direction: MermaidGraphDirection,
    palette: RenderPalette,
) -> AnyElement {
    let horizontal = direction == MermaidGraphDirection::LeftRight;
    let mut card = div()
        .w_full()
        .rounded(px(7.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.text_color.opacity(0.025))
        .p_2()
        .flex()
        .gap_2()
        .items_center();

    if horizontal {
        card = card
            .flex_row()
            .child(render_mermaid_node_card(&edge.from, palette))
            .child(render_mermaid_connector(edge.label.as_deref(), true, palette))
            .child(render_mermaid_node_card(&edge.to, palette));
    } else {
        card = card
            .flex_col()
            .child(render_mermaid_node_card(&edge.from, palette))
            .child(render_mermaid_connector(edge.label.as_deref(), false, palette))
            .child(render_mermaid_node_card(&edge.to, palette));
    }

    card.into_any_element()
}

fn render_mermaid_connector(
    label: Option<&str>,
    horizontal: bool,
    palette: RenderPalette,
) -> AnyElement {
    let mut connector = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .text_color(palette.muted_text_color);

    if let Some(label) = label.filter(|label| !label.trim().is_empty()) {
        connector = connector.child(render_mermaid_chip(label.to_string(), palette));
    }

    connector
        .min_w(if horizontal { px(48.) } else { px(0.) })
        .text_size(px(body_font_size()))
        .font_family(MONOSPACE_FONT_FAMILY)
        .child(if horizontal { "-->" } else { "v" })
        .into_any_element()
}

fn render_mermaid_node_card(node: &MermaidNode, palette: RenderPalette) -> AnyElement {
    let label = if node.id == node.label {
        node.label.clone()
    } else {
        format!("{} ({})", node.label, node.id)
    };

    div()
        .min_w(px(112.))
        .max_w(px(220.))
        .rounded(px(7.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.text_color.opacity(0.06))
        .px_3()
        .py_2()
        .text_size(px(body_font_size()))
        .line_height(px(body_line_height()))
        .text_color(palette.text_color)
        .child(label)
        .into_any_element()
}

fn render_mermaid_chip(label: String, palette: RenderPalette) -> AnyElement {
    div()
        .rounded(px(6.))
        .bg(palette.text_color.opacity(0.06))
        .px_2()
        .py(px(2.))
        .text_sm()
        .line_height(px(body_line_height()))
        .text_color(palette.muted_text_color)
        .child(label)
        .into_any_element()
}

fn render_mermaid_source_fallback(source: &str, palette: RenderPalette) -> AnyElement {
    let source = if source.trim().is_empty() {
        "Empty Mermaid diagram".to_string()
    } else {
        source.trim().to_string()
    };

    div()
        .w_full()
        .rounded(px(6.))
        .border_1()
        .border_color(palette.border_color)
        .bg(palette.text_color.opacity(0.03))
        .px_3()
        .py_2()
        .font_family(MONOSPACE_FONT_FAMILY)
        .text_size(px(body_font_size()))
        .line_height(px(body_line_height()))
        .text_color(palette.text_color)
        .child(source)
        .into_any_element()
}

fn parse_mermaid_preview(source: &str) -> MermaidPreview {
    let mut preview = MermaidPreview::default();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(direction) = mermaid_direction(line) {
            preview.direction = Some(direction);
            continue;
        }

        if is_mermaid_non_edge_directive(line) {
            continue;
        }

        let edges = parse_mermaid_edges(line);
        if edges.is_empty() {
            preview.unsupported_lines += 1;
        } else {
            preview.edges.extend(edges);
        }
    }

    preview
}

fn mermaid_direction(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let head = parts.next()?.trim_end_matches(';');
    if !head.eq_ignore_ascii_case("graph") && !head.eq_ignore_ascii_case("flowchart") {
        return None;
    }

    let direction = parts
        .next()
        .unwrap_or("TD")
        .trim_matches(|ch: char| ch == ';' || ch == ',')
        .trim();
    if direction.is_empty() {
        Some("TD".to_string())
    } else {
        Some(direction.to_ascii_uppercase())
    }
}

fn is_mermaid_non_edge_directive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower == "end"
        || lower.starts_with("subgraph ")
        || lower.starts_with("direction ")
        || lower.starts_with("classdef ")
        || lower.starts_with("class ")
        || lower.starts_with("style ")
        || lower.starts_with("linkstyle ")
        || lower.starts_with("click ")
        || lower.starts_with("acctitle")
        || lower.starts_with("accdescr")
}

fn parse_mermaid_edges(line: &str) -> Vec<MermaidEdge> {
    let mut edges = Vec::new();
    let mut rest = line.trim();

    while let Some((edge, next_rest)) = parse_mermaid_edge_prefix(rest) {
        edges.push(edge);
        let Some(next_rest) = next_rest else {
            break;
        };
        let next_rest = next_rest.trim_start();
        if next_rest.len() >= rest.len() {
            break;
        }
        rest = next_rest;
    }

    edges
}

fn parse_mermaid_edge_prefix(line: &str) -> Option<(MermaidEdge, Option<&str>)> {
    let (operator_start, operator_end) = find_mermaid_edge_operator_from(line, 0)?;
    let operator = &line[operator_start..operator_end];
    let mut from_source = line[..operator_start].trim_end();
    let mut label = None;

    if mermaid_labeled_operator(operator)
        && let Some(label_start) = from_source.rfind("--")
    {
        let candidate = clean_mermaid_label(&from_source[label_start + 2..]);
        if !candidate.is_empty() {
            from_source = from_source[..label_start].trim_end();
            label = Some(candidate);
        }
    }

    let (to_start, pipe_label) = mermaid_pipe_label_after_operator(line, operator_end);
    if pipe_label.is_some() {
        label = pipe_label;
    }

    let (to_end, has_next_edge) = mermaid_chained_node_end(line, to_start);
    let from = parse_mermaid_node(from_source)?;
    let to = parse_mermaid_node(&line[to_start..to_end])?;

    Some((
        MermaidEdge { from, to, label },
        has_next_edge.then_some(&line[to_start..]),
    ))
}

fn mermaid_labeled_operator(operator: &str) -> bool {
    matches!(operator, "-->" | "==>" | "-.->" | "--o" | "--x")
}

fn mermaid_pipe_label_after_operator(line: &str, start: usize) -> (usize, Option<String>) {
    let rest = line[start..].trim_start();
    let label_start = start + line[start..].len() - rest.len();
    if !rest.starts_with('|') {
        return (label_start, None);
    }

    let after_first = &line[label_start + 1..];
    let Some(second_pipe) = after_first.find('|') else {
        return (label_start, None);
    };

    let label = clean_mermaid_label(&after_first[..second_pipe]);
    let after_label = label_start + 1 + second_pipe + 1;
    let rest_after_label = line[after_label..].trim_start();
    let node_start = after_label + line[after_label..].len() - rest_after_label.len();

    (
        node_start,
        if label.is_empty() { None } else { Some(label) },
    )
}

fn mermaid_chained_node_end(line: &str, node_start: usize) -> (usize, bool) {
    let Some((next_operator_start, _)) = find_mermaid_edge_operator_from(line, node_start) else {
        return (line.len(), false);
    };

    let before_next = line[node_start..next_operator_start].trim_end();
    let mut node_end = node_start + before_next.len();
    if let Some(label_start) = before_next.rfind("--") {
        let candidate = clean_mermaid_label(&before_next[label_start + 2..]);
        if !candidate.is_empty() {
            node_end = node_start + label_start;
        }
    }

    (node_end, true)
}

fn find_mermaid_edge_operator_from(line: &str, start: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if index < start {
            continue;
        }

        if let Some(quote_char) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }

        if depth == 0 {
            for operator in [
                "<-->", "-.->", "-->", "==>", "---", "-.-", "~~~", "--o", "--x", "o--", "x--",
                "<--",
            ] {
                if line[index..].starts_with(operator) {
                    return Some((index, index + operator.len()));
                }
            }
        }
    }

    None
}

fn parse_mermaid_node(source: &str) -> Option<MermaidNode> {
    let trimmed = source
        .trim()
        .trim_matches(|ch: char| ch == ';' || ch == ',')
        .trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_class = trimmed.split(":::").next().unwrap_or(trimmed).trim();
    let label_start = without_class.find(|ch| matches!(ch, '[' | '(' | '{'));
    let id = label_start
        .map(|start| clean_mermaid_label(&without_class[..start]))
        .unwrap_or_else(|| clean_mermaid_label(without_class));
    let label = label_start
        .map(|start| clean_mermaid_label(&without_class[start..]))
        .unwrap_or_default();

    let id = if id.is_empty() { label.clone() } else { id };
    let label = if label.is_empty() { id.clone() } else { label };

    (!id.is_empty() || !label.is_empty()).then_some(MermaidNode { id, label })
}

fn clean_mermaid_label(source: &str) -> String {
    source
        .trim()
        .trim_matches(|ch| matches!(ch, '[' | ']' | '(' | ')' | '{' | '}'))
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn mermaid_graph_direction(preview: &MermaidPreview) -> MermaidGraphDirection {
    match preview.direction.as_deref() {
        Some("LR") | Some("RL") => MermaidGraphDirection::LeftRight,
        _ => MermaidGraphDirection::TopDown,
    }
}

fn collect_mermaid_preview_nodes(preview: &MermaidPreview) -> Vec<MermaidNode> {
    let mut nodes = Vec::new();
    for edge in &preview.edges {
        push_mermaid_preview_node(&mut nodes, edge.from.clone());
        push_mermaid_preview_node(&mut nodes, edge.to.clone());
    }
    nodes
}

fn push_mermaid_preview_node(nodes: &mut Vec<MermaidNode>, node: MermaidNode) {
    if let Some(existing) = nodes.iter_mut().find(|existing| existing.id == node.id) {
        if existing.label == existing.id && node.label != node.id {
            existing.label = node.label;
        }
    } else {
        nodes.push(node);
    }
}

fn render_table_block(
    block: &RenderBlock,
    source_text: String,
    palette: RenderPalette,
    window: &Window,
) -> AnyElement {
    let block_for_canvas = block.clone();
    div()
        .relative()
        .w_full()
        .child(
            canvas(
                move |bounds, window, _| {
                    table_surface_layout(&block_for_canvas, &source_text, bounds, window)
                },
                move |bounds, layout, window, _| {
                    let Some(layout) = layout else {
                        return;
                    };
                    paint_table_underlay(bounds, &layout, palette, window);
                },
            )
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.)),
        )
        .child(
            div()
                .w_full()
                .text_size(px(body_font_size()))
                .line_height(px(body_line_height()))
                .font_family(MONOSPACE_FONT_FAMILY)
                .child(styled_text_for_block(block, palette, window)),
        )
        .into_any_element()
}

fn apply_fragment_style(
    style: &mut TextStyle,
    kind: RenderSpanKind,
    inline_style: crate::RenderInlineStyle,
    meta: Option<&crate::RenderSpanMeta>,
    palette: RenderPalette,
    base_font_size: gpui::Pixels,
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

    if let Some(crate::RenderSpanMeta::CodeToken { token_type }) = meta {
        use crate::core::CodeTokenType;
        match token_type {
            CodeTokenType::Keyword => style.color = palette.code_keyword_color,
            CodeTokenType::Function => {
                style.color = palette.code_function_color;
            }
            CodeTokenType::String => style.color = palette.code_string_color,
            CodeTokenType::Number => style.color = palette.code_number_color,
            CodeTokenType::Comment => {
                style.color = palette.code_comment_color;
                style.font_style = FontStyle::Italic;
            }
            CodeTokenType::Type => style.color = palette.code_type_color,
            CodeTokenType::Constant => style.color = palette.code_constant_color,
            CodeTokenType::Variable => style.color = palette.code_variable_color,
            CodeTokenType::Operator => style.color = palette.code_operator_color,
            CodeTokenType::Punctuation => style.color = palette.code_operator_color,
            CodeTokenType::Property => style.color = palette.code_function_color,
            CodeTokenType::Tag => style.color = palette.code_tag_color,
            CodeTokenType::Attribute => style.color = palette.code_attribute_color,
            CodeTokenType::Escape => style.color = palette.code_escape_color,
            CodeTokenType::Default => {}
        }
    }

    if let Some(crate::RenderSpanMeta::Math { .. }) = meta {
        style.color = palette.math_color;
        style.font_style = FontStyle::Italic;
        style.background_color = Some(palette.math_background);
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
        style.color = palette.link_color;
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            color: Some(palette.link_color.opacity(0.5)),
            wavy: false,
        });
    }
    if inline_style.code {
        if !inline_style.strong {
            style.font_weight = FontWeight::MEDIUM;
        }
        style.font_family = SharedString::from(MONOSPACE_FONT_FAMILY);
        style.background_color = Some(palette.code_background);
    }
    if inline_style.highlight {
        style.background_color = Some(palette.highlight_background);
    }
    if inline_style.superscript || inline_style.subscript {
        style.font_size = px(f32::from(base_font_size) * 0.78).into();
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
    style.white_space = if block.kind == BlockKind::Table {
        WhiteSpace::Nowrap
    } else {
        WhiteSpace::Normal
    };
    if matches!(
        block.kind,
        BlockKind::CodeFence { .. } | BlockKind::Table | BlockKind::SourceCode
    ) {
        style.font_family = SharedString::from(MONOSPACE_FONT_FAMILY);
    }
    style
}

fn table_surface_layout(
    block: &RenderBlock,
    source_text: &str,
    bounds: Bounds<gpui::Pixels>,
    window: &Window,
) -> Option<TableSurfaceLayout> {
    let table = TableModel::parse(source_text);
    if table.is_empty() {
        return None;
    }

    let lines = shape_block_lines_uncached(block, bounds.size.width.max(px(1.)), window);
    if lines.len() < table.visible_row_count() {
        return None;
    }

    let row_texts = block.visible_text.split('\n').collect::<Vec<_>>();
    if row_texts.len() < table.visible_row_count() {
        return None;
    }

    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let mut column_char_widths = vec![0usize; table.column_count()];
    for visible_row in 0..table.visible_row_count() {
        for column in 0..table.column_count() {
            let Some(cell_range) = table.cell_source_range(crate::core::table::TableCellRef {
                visible_row,
                column,
            }) else {
                continue;
            };
            let absolute_range = block.content_range.start + cell_range.start
                ..block.content_range.start + cell_range.end;
            column_char_widths[column] = column_char_widths[column]
                .max(table_cell_visible_char_count(block, &absolute_range));
        }
    }

    let mut column_char_starts = Vec::with_capacity(table.column_count());
    let mut total_char_width = 0usize;
    for (column, width) in column_char_widths.iter().copied().enumerate() {
        column_char_starts.push(total_char_width);
        total_char_width += width;
        if column + 1 < column_char_widths.len() {
            total_char_width += TABLE_COLUMN_GAP;
        }
    }

    let first_line = lines.first()?;
    let first_row = row_texts.first().copied().unwrap_or_default();
    let column_starts = column_char_starts
        .into_iter()
        .map(|char_offset| {
            first_line
                .position_for_index(
                    char_column_to_byte_offset(first_row, char_offset),
                    line_height,
                )
                .map(|position| position.x)
                .unwrap_or(px(0.))
        })
        .collect::<Vec<_>>();

    let mut row_tops = Vec::with_capacity(table.visible_row_count());
    let mut row_heights = Vec::with_capacity(table.visible_row_count());
    let mut total_height = px(0.);
    let mut total_width = px(0.);
    for line in lines.iter().take(table.visible_row_count()) {
        row_tops.push(total_height);
        let line_size = line.size(line_height);
        row_heights.push(line_size.height);
        total_height += line_size.height;
        if line_size.width > total_width {
            total_width = line_size.width;
        }
    }

    if total_char_width > 0 {
        let width_from_chars = first_line
            .position_for_index(
                char_column_to_byte_offset(first_row, total_char_width),
                line_height,
            )
            .map(|position| position.x)
            .unwrap_or(total_width);
        if width_from_chars > total_width {
            total_width = width_from_chars;
        }
    }
    total_width += trailing_table_padding_px(window);

    Some(TableSurfaceLayout {
        row_tops,
        row_heights,
        column_starts,
        total_width,
        total_height,
    })
}

fn paint_table_underlay(
    bounds: Bounds<gpui::Pixels>,
    layout: &TableSurfaceLayout,
    palette: RenderPalette,
    window: &mut Window,
) {
    if layout.row_tops.is_empty() {
        return;
    }

    let line_thickness = px(1.);
    let left = bounds.left();
    let top = bounds.top();
    let width = layout.total_width.max(px(1.));
    let height = layout.total_height.max(px(1.));

    window.paint_quad(fill(
        Bounds::new(
            point(left, top + layout.row_tops[0]),
            size(width, layout.row_heights[0].max(px(1.))),
        ),
        palette.code_surface_background,
    ));

    let mut vertical_lines = Vec::with_capacity(layout.column_starts.len().saturating_add(2));
    vertical_lines.push(px(0.));
    vertical_lines.extend(layout.column_starts.iter().copied().skip(1));
    vertical_lines.push(width);
    for x in vertical_lines {
        window.paint_quad(fill(
            Bounds::new(point(left + x, top), size(line_thickness, height)),
            palette.border_color,
        ));
    }

    for y in layout
        .row_tops
        .iter()
        .copied()
        .chain(std::iter::once(layout.total_height))
    {
        window.paint_quad(fill(
            Bounds::new(point(left, top + y), size(width, line_thickness)),
            palette.border_color,
        ));
    }
}

fn table_cell_visible_char_count(
    block: &RenderBlock,
    source_range: &std::ops::Range<usize>,
) -> usize {
    block
        .spans
        .iter()
        .filter(|span| {
            !span.visible_text.is_empty()
                && span.source_range.start < source_range.end
                && span.source_range.end > source_range.start
        })
        .map(|span| str_display_width(&span.visible_text))
        .sum()
}

fn trailing_table_padding_px(window: &Window) -> gpui::Pixels {
    window.text_style().font_size.to_pixels(window.rem_size()) * (0.6 * TABLE_COLUMN_GAP as f32)
}

fn char_column_to_byte_offset(text: &str, target_column: usize) -> usize {
    let mut width = 0usize;
    for (offset, ch) in text.char_indices() {
        if width >= target_column {
            return offset;
        }
        width += char_display_width(ch);
    }
    text.len()
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

fn rendered_lines_for_list_block(block: &RenderBlock) -> Vec<RenderedLine> {
    let mut lines = vec![RenderedLine::default()];

    for span in rendered_spans(block)
        .filter(|span| !span.visible_text.is_empty() && span.kind != RenderSpanKind::TaskMarker)
    {
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
    let lines = rendered_lines_for_list_block(block);
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
                        .text_size(px(body_font_size()))
                        .line_height(px(body_line_height()))
                        .child(marker),
                )
                .child(render_line_text(block, line, palette, window)),
        );
    }

    content.into_any_element()
}

fn selection_is_within_render_block(snapshot: &EditorSnapshot, block: &RenderBlock) -> bool {
    let range = snapshot.selection.range();
    range.start >= block.content_range.start && range.end <= block.content_range.end
}

fn selection_touches_render_block(selection: std::ops::Range<usize>, block: &RenderBlock) -> bool {
    if selection.is_empty() {
        return selection.start > block.content_range.start
            && selection.start < block.content_range.end;
    }

    selection.start < block.content_range.end && block.content_range.start < selection.end
}

fn should_render_image_preview(block: &RenderBlock, selection: std::ops::Range<usize>) -> bool {
    standalone_image_span(block).is_some() && !selection_touches_render_block(selection, block)
}

fn should_render_mermaid_preview(block: &RenderBlock, selection: std::ops::Range<usize>) -> bool {
    matches!(
        &block.embedded,
        Some(EmbeddedNodeKind::Diagram { language }) if language.eq_ignore_ascii_case("mermaid")
    ) && !selection_touches_render_block(selection, block)
}

fn should_render_toc_preview(block: &RenderBlock, selection: std::ops::Range<usize>) -> bool {
    matches!(&block.embedded, Some(EmbeddedNodeKind::Toc))
        && !selection_touches_render_block(selection, block)
}

fn should_render_footnote_preview(
    block: &RenderBlock,
    selection: std::ops::Range<usize>,
) -> bool {
    matches!(&block.embedded, Some(EmbeddedNodeKind::FootnoteDefinition))
        && !selection_touches_render_block(selection, block)
}

fn should_render_front_matter_preview(
    block: &RenderBlock,
    selection: std::ops::Range<usize>,
) -> bool {
    matches!(&block.kind, BlockKind::YamlFrontMatter)
        && !selection_touches_render_block(selection, block)
}

fn standalone_image_span(block: &RenderBlock) -> Option<&RenderSpan> {
    let mut visible_spans = rendered_spans(block).filter(|span| !span.visible_text.is_empty());
    let span = visible_spans.next()?;
    if visible_spans.next().is_some() {
        return None;
    }

    matches!(span.meta, Some(RenderSpanMeta::Image { .. }))
        .then_some(span)
        .filter(|span| span.visible_text == rendered_text_for_block(block))
}

fn image_edit_cursor_offset(block: &RenderBlock) -> usize {
    (block.content_range.start + 1).min(block.content_range.end)
}

fn mermaid_edit_cursor_offset(block: &RenderBlock) -> usize {
    block
        .spans
        .iter()
        .find(|span| span.kind == RenderSpanKind::Text && !span.source_range.is_empty())
        .map(|span| span.source_range.start)
        .unwrap_or(block.content_range.start)
        .min(block.content_range.end)
}

fn toc_edit_cursor_offset(block: &RenderBlock) -> usize {
    block.content_range.start
}

fn footnote_edit_cursor_offset(block: &RenderBlock) -> usize {
    block
        .spans
        .iter()
        .find(|span| !span.hidden && !span.source_range.is_empty())
        .map(|span| span.source_range.start)
        .unwrap_or(block.content_range.start)
        .min(block.content_range.end)
}

fn front_matter_edit_cursor_offset(block: &RenderBlock) -> usize {
    block
        .spans
        .iter()
        .find(|span| {
            !span.hidden
                && span.kind != RenderSpanKind::LineBreak
                && !span.visible_text.is_empty()
                && !span.source_range.is_empty()
        })
        .map(|span| span.source_range.start)
        .unwrap_or(block.content_range.start)
        .min(block.content_range.end)
}

fn document_base_dir(snapshot: &EditorSnapshot) -> Option<&Path> {
    snapshot
        .path
        .as_ref()
        .or(snapshot.suggested_path.as_ref())
        .and_then(|path| path.parent())
}

fn resolve_image_source(src: &str, base_dir: Option<&Path>) -> Option<ResolvedImageSource> {
    let src = src.trim();
    if src.is_empty() {
        return None;
    }

    if looks_like_image_uri(src) {
        return Some(ResolvedImageSource::Uri(src.to_string()));
    }

    let path = if let Some(rest) = src.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(src))
    } else {
        PathBuf::from(src)
    };

    if path.is_absolute() {
        return Some(ResolvedImageSource::Path(path));
    }

    Some(ResolvedImageSource::Path(
        base_dir
            .map(|base_dir| base_dir.join(&path))
            .unwrap_or(path),
    ))
}

fn looks_like_image_uri(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    ["http://", "https://", "file://", "data:"]
        .into_iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn alt_if_present(alt: &str, fallback: &str) -> String {
    if alt.trim().is_empty() {
        fallback.to_string()
    } else {
        alt.to_string()
    }
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

fn render_thematic_break(palette: RenderPalette) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .py(px(8.))
        .child(
            div()
                .w_full()
                .h(px(2.))
                .bg(palette.border_color.opacity(0.6)),
        )
        .into_any_element()
}

fn render_math_block(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
    math_render_cache: &Rc<RefCell<crate::core::math_render::MathRenderCache>>,
) -> AnyElement {
    let source = block
        .spans
        .iter()
        .find_map(|span| match &span.meta {
            Some(crate::RenderSpanMeta::Math { source, .. }) => Some(source.trim().to_string()),
            _ => None,
        })
        .unwrap_or_default();

    let display_text = if source.is_empty() {
        "Empty math formula".to_string()
    } else {
        let tree = math_render_cache
            .borrow_mut()
            .get_or_parse(
                &source,
                crate::core::math_render::MathRenderMode::Display,
                body_font_size(),
            );
        let rendered = crate::core::math_render::math_tree_to_display_text(&tree);
        if rendered.is_empty() {
            source.clone()
        } else {
            rendered
        }
    };

    let text_size = px(body_font_size());
    let line_height = px(body_line_height());
    let run_len = display_text.len().max(1);

    let styled = StyledText::new(display_text).with_runs(vec![TextStyle {
        color: palette.math_color,
        font_size: text_size.into(),
        line_height: line_height.into(),
        font_style: FontStyle::Italic,
        ..window.text_style().clone()
    }
    .to_run(run_len)]);

    div()
        .w_full()
        .flex()
        .justify_center()
        .py(px(6.))
        .px(px(16.))
        .rounded(px(6.))
        .bg(palette.math_background)
        .text_size(text_size)
        .line_height(line_height)
        .child(styled)
        .into_any_element()
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
            this.font_family(MONOSPACE_FONT_FAMILY)
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
    let base_font_size = px(block_presentation(&block.kind).font_size);

    for fragment in &line.fragments {
        text.push_str(&fragment.text);
        let mut style = base_style.clone();
        apply_fragment_style(
            &mut style,
            fragment.kind.clone(),
            fragment.style,
            None,
            palette,
            base_font_size,
        );
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
        if span.kind == RenderSpanKind::TaskMarker && !span.source_text.trim().is_empty() {
            current_marker = Some(task_marker_display_text(&span.source_text));
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

fn task_marker_range_for_line(
    block: &RenderBlock,
    target_line_index: usize,
) -> Option<std::ops::Range<usize>> {
    let mut line_index = 0usize;

    for span in &block.spans {
        if span.kind == RenderSpanKind::TaskMarker && line_index == target_line_index {
            return Some(span.source_range.clone());
        }

        if span.kind == RenderSpanKind::LineBreak && span.source_text.contains('\n') {
            line_index += span
                .source_text
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
            if line_index > target_line_index {
                break;
            }
        }
    }

    None
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

fn task_marker_display_text(source_text: &str) -> String {
    let checked = source_text
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .map(|inner| matches!(inner.chars().next(), Some('x' | 'X')))
        .unwrap_or(false);
    if checked {
        "\u{2611}".to_string()
    } else {
        "\u{2610}".to_string()
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

    let lines = shape_block_lines_uncached(block, text_width, window);
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

pub(super) fn shape_block_lines_uncached(
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
                style.font_family = SharedString::from(MONOSPACE_FONT_FAMILY);
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

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn word_range_at_visible_offset(text: &str, offset: usize) -> Option<std::ops::Range<usize>> {
    if text.is_empty() {
        return None;
    }

    let offset = clamp_to_char_boundary(text, offset.min(text.len()));
    let mut cursor = offset;

    if cursor == text.len() {
        cursor = text.char_indices().next_back().map(|(index, _)| index)?;
    } else if let Some(ch) = text[cursor..].chars().next() {
        if !is_word_char(ch) {
            cursor = text[..cursor]
                .char_indices()
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(cursor);
        }
    }

    let ch = text[cursor..].chars().next()?;
    if !is_word_char(ch) {
        return None;
    }

    let mut start = cursor;
    while start > 0 {
        let Some((previous_index, previous_char)) = text[..start].char_indices().next_back() else {
            break;
        };
        if !is_word_char(previous_char) {
            break;
        }
        start = previous_index;
    }

    let mut end = cursor + ch.len_utf8();
    while end < text.len() {
        let Some(next_char) = text[end..].chars().next() else {
            break;
        };
        if !is_word_char(next_char) {
            break;
        }
        end += next_char.len_utf8();
    }

    Some(start..end)
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

fn build_editor_context_menu(
    menu: PopupMenu,
    view: &Entity<MarkdownEditor>,
    block_kind: &BlockKind,
    link_url: Option<String>,
) -> PopupMenu {
    let view = view.clone();

    let is_code_block = matches!(block_kind, BlockKind::CodeFence { .. });
    let is_table = matches!(block_kind, BlockKind::Table);

    let mut menu = menu;

    if let Some(url) = &link_url {
        let url_clone = url.clone();
        menu = menu.item(PopupMenuItem::new("Open Link").on_click(move |_, _, _| {
            let _ = open::that(&url_clone);
        }));
        let url_clone = url.clone();
        menu = menu.item(
            PopupMenuItem::new("Copy Link Address").on_click(move |_, _, cx| {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(url_clone.clone()));
            }),
        );
        menu = menu.separator();
    }

    menu = menu
        .item(PopupMenuItem::new("Cut").on_click({
            let view = view.clone();
            move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.cut_selection(window, cx);
                });
            }
        }))
        .item(PopupMenuItem::new("Copy").on_click({
            let view = view.clone();
            move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.copy_selection(window, cx);
                });
            }
        }))
        .item(PopupMenuItem::new("Paste").on_click({
            let view = view.clone();
            move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.paste_at_cursor(window, cx);
                });
            }
        }))
        .separator()
        .item(PopupMenuItem::new("Select All").on_click({
            let view = view.clone();
            move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.select_all(window, cx);
                });
            }
        }));

    if !is_code_block {
        menu = menu
            .separator()
            .item(PopupMenuItem::new("Bold").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("**", "**", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Italic").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("*", "*", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Strikethrough").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("~~", "~~", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Highlight").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("==", "==", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Superscript").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("^", "^", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Subscript").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("~", "~", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Inline Code").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.apply_markup("`", "`", window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Link").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_link(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Image").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_image(window, cx);
                    });
                }
            }));
    }

    if !is_code_block && !is_table {
        menu = menu
            .separator()
            .item(PopupMenuItem::new("Heading 1").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_heading(1, window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Heading 2").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_heading(2, window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Heading 3").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_heading(3, window, cx);
                    });
                }
            }))
            .separator()
            .item(PopupMenuItem::new("Quote").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_blockquote(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Ordered List").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_list(true, window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Unordered List").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_list(false, window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Task List").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.toggle_task_list(window, cx);
                    });
                }
            }))
            .separator()
            .item(PopupMenuItem::new("Code Block").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_code_fence(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Mermaid Diagram").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_mermaid_diagram(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Inline Math").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_inline_math(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("HTML Block").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_html_block(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Callout").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_callout(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Table of Contents").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_toc(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Footnote").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_footnote(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Front Matter").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_front_matter(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Table").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_table(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Horizontal Rule").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_horizontal_rule(window, cx);
                    });
                }
            }));
    }

    if is_table {
        menu = menu
            .separator()
            .item(PopupMenuItem::new("Insert Row").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_table_row(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Delete Row").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.delete_table_row(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Insert Column").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.insert_table_column(window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("Delete Column").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.delete_table_column(window, cx);
                    });
                }
            }))
            .separator()
            .item(PopupMenuItem::new("Align Column Left").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.align_table_column(
                            crate::core::table::TableColumnAlignment::Left,
                            window,
                            cx,
                        );
                    });
                }
            }))
            .item(PopupMenuItem::new("Align Column Center").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.align_table_column(
                            crate::core::table::TableColumnAlignment::Center,
                            window,
                            cx,
                        );
                    });
                }
            }))
            .item(PopupMenuItem::new("Align Column Right").on_click({
                let view = view.clone();
                move |_, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.align_table_column(
                            crate::core::table::TableColumnAlignment::Right,
                            window,
                            cx,
                        );
                    });
                }
            }));
    }

    menu
}

#[cfg(test)]
mod tests {
    use super::{
        FootnotePreview, MermaidEdge, MermaidGraphDirection, MermaidNode, MetadataPreviewEntry,
        ResolvedImageSource, TocPreviewEntry, collect_mermaid_preview_nodes,
        collect_toc_preview_entries, footnote_edit_cursor_offset, footnote_preview,
        front_matter_edit_cursor_offset, front_matter_preview_entries, image_edit_cursor_offset,
        looks_like_image_uri, mermaid_edit_cursor_offset, mermaid_graph_direction,
        parse_mermaid_preview, parse_metadata_preview_entry, resolve_image_source,
        selection_touches_render_block, should_render_footnote_preview,
        should_render_front_matter_preview, should_render_image_preview,
        should_render_mermaid_preview, should_render_toc_preview, toc_edit_cursor_offset,
        word_range_at_visible_offset,
    };
    use crate::{
        BlockKind, EmbeddedNodeKind, RenderBlock, RenderInlineStyle, RenderSpan, RenderSpanKind,
        RenderSpanMeta,
    };
    use std::path::{Path, PathBuf};

    fn standalone_image_block(block_visible_text: &str, span_visible_text: &str) -> RenderBlock {
        RenderBlock {
            id: 1,
            kind: BlockKind::Paragraph,
            source_range: 0..18,
            content_range: 0..18,
            visible_range: 0..block_visible_text.len(),
            visible_text: block_visible_text.to_string(),
            spans: vec![RenderSpan {
                kind: RenderSpanKind::Text,
                source_range: 0..18,
                visible_range: 0..span_visible_text.len(),
                source_text: "![alt](img.png)".to_string(),
                visible_text: span_visible_text.to_string(),
                hidden: false,
                style: RenderInlineStyle::default(),
                meta: Some(RenderSpanMeta::Image {
                    src: "img.png".to_string(),
                    alt: "alt".to_string(),
                    title: None,
                }),
            }],
            embedded: Some(EmbeddedNodeKind::Image),
            source_hash: 0,
        }
    }

    fn mermaid_block(source: &str) -> RenderBlock {
        let body_start = "```mermaid\n".len();
        let body_end = body_start + source.len();
        let source_end = body_end + "\n```".len();
        RenderBlock {
            id: 2,
            kind: BlockKind::CodeFence {
                language: Some("mermaid".to_string()),
            },
            source_range: 0..source_end,
            content_range: 0..source_end,
            visible_range: 0..source.len(),
            visible_text: source.to_string(),
            spans: vec![RenderSpan {
                kind: RenderSpanKind::Text,
                source_range: body_start..body_end,
                visible_range: 0..source.len(),
                source_text: source.to_string(),
                visible_text: source.to_string(),
                hidden: false,
                style: RenderInlineStyle {
                    code: true,
                    ..RenderInlineStyle::default()
                },
                meta: None,
            }],
            embedded: Some(EmbeddedNodeKind::Diagram {
                language: "mermaid".to_string(),
            }),
            source_hash: 0,
        }
    }

    fn toc_block() -> RenderBlock {
        RenderBlock {
            id: 3,
            kind: BlockKind::Toc,
            source_range: 10..15,
            content_range: 10..15,
            visible_range: 0.."Table of Contents".len(),
            visible_text: "Table of Contents".to_string(),
            spans: vec![
                RenderSpan {
                    kind: RenderSpanKind::HiddenSyntax,
                    source_range: 10..15,
                    visible_range: 0..0,
                    source_text: "[toc]".to_string(),
                    visible_text: String::new(),
                    hidden: true,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: 10..10,
                    visible_range: 0.."Table of Contents".len(),
                    source_text: String::new(),
                    visible_text: "Table of Contents".to_string(),
                    hidden: false,
                    style: RenderInlineStyle {
                        strong: true,
                        ..RenderInlineStyle::default()
                    },
                    meta: None,
                },
            ],
            embedded: Some(EmbeddedNodeKind::Toc),
            source_hash: 0,
        }
    }

    fn heading_block(id: u64, depth: u8, marker: &str, title: &str) -> RenderBlock {
        let visible_text = format!("{marker}{title}");
        RenderBlock {
            id,
            kind: BlockKind::Heading { depth },
            source_range: 0..visible_text.len(),
            content_range: 0..visible_text.len(),
            visible_range: 0..visible_text.len(),
            visible_text: visible_text.clone(),
            spans: vec![
                RenderSpan {
                    kind: RenderSpanKind::HiddenSyntax,
                    source_range: 0..marker.len(),
                    visible_range: 0..marker.len(),
                    source_text: marker.to_string(),
                    visible_text: marker.to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: marker.len()..visible_text.len(),
                    visible_range: marker.len()..visible_text.len(),
                    source_text: title.to_string(),
                    visible_text: title.to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
            ],
            embedded: None,
            source_hash: 0,
        }
    }

    fn footnote_block(label_source: &str, body: &str) -> RenderBlock {
        let source_start = 20usize;
        let label_text = format!("[^{label_source}]:");
        let label_end = source_start + label_text.len();
        let body_start = label_end + 1;
        let source_end = body_start + body.len();
        let visible_text = format!("Footnote {body}");

        RenderBlock {
            id: 6,
            kind: BlockKind::FootnoteDefinition,
            source_range: source_start..source_end,
            content_range: source_start..source_end,
            visible_range: 0..visible_text.len(),
            visible_text: visible_text.clone(),
            spans: vec![
                RenderSpan {
                    kind: RenderSpanKind::HiddenSyntax,
                    source_range: source_start..label_end,
                    visible_range: 0..0,
                    source_text: label_text,
                    visible_text: String::new(),
                    hidden: true,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: source_start..source_start,
                    visible_range: 0.."Footnote".len(),
                    source_text: String::new(),
                    visible_text: "Footnote".to_string(),
                    hidden: false,
                    style: RenderInlineStyle {
                        strong: true,
                        ..RenderInlineStyle::default()
                    },
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: source_start..source_start,
                    visible_range: "Footnote".len().."Footnote ".len(),
                    source_text: String::new(),
                    visible_text: " ".to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: body_start..source_end,
                    visible_range: "Footnote ".len()..visible_text.len(),
                    source_text: body.to_string(),
                    visible_text: body.to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
            ],
            embedded: Some(EmbeddedNodeKind::FootnoteDefinition),
            source_hash: 0,
        }
    }

    fn front_matter_block() -> RenderBlock {
        let title = "title: Draft";
        let tags = "tags: [acceptance, export]";
        let source_text = format!("---\n{title}\n{tags}\n---");
        let title_start = "---\n".len();
        let title_end = title_start + title.len();
        let tags_start = title_end + 1;
        let tags_end = tags_start + tags.len();
        let closing_start = tags_end + 1;
        let visible_text = format!("\n{title}\n{tags}\n");

        RenderBlock {
            id: 7,
            kind: BlockKind::YamlFrontMatter,
            source_range: 0..source_text.len(),
            content_range: 0..source_text.len(),
            visible_range: 0..visible_text.len(),
            visible_text: visible_text.clone(),
            spans: vec![
                RenderSpan {
                    kind: RenderSpanKind::HiddenSyntax,
                    source_range: 0..3,
                    visible_range: 0..0,
                    source_text: "---".to_string(),
                    visible_text: String::new(),
                    hidden: true,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::LineBreak,
                    source_range: 3..4,
                    visible_range: 0..1,
                    source_text: "\n".to_string(),
                    visible_text: "\n".to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: title_start..title_end,
                    visible_range: 1..1 + title.len(),
                    source_text: title.to_string(),
                    visible_text: title.to_string(),
                    hidden: false,
                    style: RenderInlineStyle {
                        code: true,
                        ..RenderInlineStyle::default()
                    },
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::LineBreak,
                    source_range: title_end..title_end + 1,
                    visible_range: 1 + title.len()..2 + title.len(),
                    source_text: "\n".to_string(),
                    visible_text: "\n".to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::Text,
                    source_range: tags_start..tags_end,
                    visible_range: 2 + title.len()..2 + title.len() + tags.len(),
                    source_text: tags.to_string(),
                    visible_text: tags.to_string(),
                    hidden: false,
                    style: RenderInlineStyle {
                        code: true,
                        ..RenderInlineStyle::default()
                    },
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::LineBreak,
                    source_range: tags_end..tags_end + 1,
                    visible_range: 2 + title.len() + tags.len()
                        ..3 + title.len() + tags.len(),
                    source_text: "\n".to_string(),
                    visible_text: "\n".to_string(),
                    hidden: false,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
                RenderSpan {
                    kind: RenderSpanKind::HiddenSyntax,
                    source_range: closing_start..closing_start + 3,
                    visible_range: 3 + title.len() + tags.len()
                        ..3 + title.len() + tags.len(),
                    source_text: "---".to_string(),
                    visible_text: String::new(),
                    hidden: true,
                    style: RenderInlineStyle::default(),
                    meta: None,
                },
            ],
            embedded: None,
            source_hash: 0,
        }
    }

    #[test]
    fn resolves_relative_image_paths_from_document_directory() {
        let resolved = resolve_image_source("assets/cover.png", Some(Path::new("/tmp/docs")));
        assert_eq!(
            resolved,
            Some(ResolvedImageSource::Path(PathBuf::from(
                "/tmp/docs/assets/cover.png"
            )))
        );
    }

    #[test]
    fn leaves_remote_image_urls_as_uris() {
        let resolved = resolve_image_source("https://example.com/image.png", None);
        assert_eq!(
            resolved,
            Some(ResolvedImageSource::Uri(
                "https://example.com/image.png".to_string()
            ))
        );
        assert!(looks_like_image_uri("https://example.com/image.png"));
        assert!(looks_like_image_uri("HTTPS://example.com/image.png"));
        assert!(looks_like_image_uri("DATA:image/png;base64,abc"));
    }

    #[test]
    fn image_preview_is_disabled_when_selection_touches_block() {
        let block = standalone_image_block("[image: alt]", "[image: alt]");
        assert!(should_render_image_preview(&block, 20..20));
        assert!(!selection_touches_render_block(0..0, &block));
        assert!(should_render_image_preview(&block, 0..0));
        assert!(!selection_touches_render_block(18..18, &block));
        assert!(should_render_image_preview(&block, 18..18));
        assert!(selection_touches_render_block(4..4, &block));
        assert!(!should_render_image_preview(&block, 4..8));
    }

    #[test]
    fn image_preview_still_renders_when_block_visible_text_keeps_trailing_newline() {
        let block = standalone_image_block("[image: alt]\n", "[image: alt]");
        assert!(should_render_image_preview(&block, 20..20));
    }

    #[test]
    fn image_click_moves_cursor_inside_markdown_syntax() {
        let block = standalone_image_block("[image: alt]", "[image: alt]");
        assert_eq!(image_edit_cursor_offset(&block), 1);
    }

    #[test]
    fn parses_common_mermaid_flowchart_edges() {
        let preview = parse_mermaid_preview(
            "graph TD\n  A[Start] -->|go| B(End)\n  B -- retry --> C{Choice}\n",
        );

        assert_eq!(preview.direction.as_deref(), Some("TD"));
        assert_eq!(
            preview.edges,
            vec![
                MermaidEdge {
                    from: MermaidNode {
                        id: "A".to_string(),
                        label: "Start".to_string(),
                    },
                    to: MermaidNode {
                        id: "B".to_string(),
                        label: "End".to_string(),
                    },
                    label: Some("go".to_string()),
                },
                MermaidEdge {
                    from: MermaidNode {
                        id: "B".to_string(),
                        label: "B".to_string(),
                    },
                    to: MermaidNode {
                        id: "C".to_string(),
                        label: "Choice".to_string(),
                    },
                    label: Some("retry".to_string()),
                },
            ]
        );
        assert_eq!(preview.unsupported_lines, 0);
    }

    #[test]
    fn parses_chained_mermaid_flowchart_edges() {
        let preview = parse_mermaid_preview(
            "flowchart LR\n  Draft[Draft] -->|save| Review[Review] -- publish --> Done[Done]\n",
        );

        assert_eq!(preview.direction.as_deref(), Some("LR"));
        assert_eq!(
            preview.edges,
            vec![
                MermaidEdge {
                    from: MermaidNode {
                        id: "Draft".to_string(),
                        label: "Draft".to_string(),
                    },
                    to: MermaidNode {
                        id: "Review".to_string(),
                        label: "Review".to_string(),
                    },
                    label: Some("save".to_string()),
                },
                MermaidEdge {
                    from: MermaidNode {
                        id: "Review".to_string(),
                        label: "Review".to_string(),
                    },
                    to: MermaidNode {
                        id: "Done".to_string(),
                        label: "Done".to_string(),
                    },
                    label: Some("publish".to_string()),
                },
            ]
        );
        assert_eq!(preview.unsupported_lines, 0);
    }

    #[test]
    fn mermaid_graph_preview_collects_unique_labeled_nodes() {
        let preview = parse_mermaid_preview(
            "flowchart LR\n  Start[Draft] --> Save\n  Save[Save] --> Print[Print]\n",
        );

        assert_eq!(
            mermaid_graph_direction(&preview),
            MermaidGraphDirection::LeftRight
        );
        assert_eq!(
            collect_mermaid_preview_nodes(&preview),
            vec![
                MermaidNode {
                    id: "Start".to_string(),
                    label: "Draft".to_string(),
                },
                MermaidNode {
                    id: "Save".to_string(),
                    label: "Save".to_string(),
                },
                MermaidNode {
                    id: "Print".to_string(),
                    label: "Print".to_string(),
                },
            ]
        );
    }

    #[test]
    fn mermaid_preview_is_disabled_when_selection_touches_block() {
        let block = mermaid_block("graph TD\n  A --> B\n");

        assert!(should_render_mermaid_preview(&block, 100..100));
        assert!(should_render_mermaid_preview(&block, 0..0));
        assert!(selection_touches_render_block(12..12, &block));
        assert!(!should_render_mermaid_preview(&block, 12..12));
        assert!(!should_render_mermaid_preview(&block, 12..16));
    }

    #[test]
    fn mermaid_preview_click_moves_cursor_to_diagram_body() {
        let block = mermaid_block("graph TD\n  A --> B\n");
        assert_eq!(mermaid_edit_cursor_offset(&block), "```mermaid\n".len());
    }

    #[test]
    fn toc_preview_collects_heading_titles_without_revealed_markers() {
        let entries = collect_toc_preview_entries(&[
            toc_block(),
            heading_block(4, 1, "# ", "Title"),
            heading_block(5, 2, "## ", "Section"),
        ]);

        assert_eq!(
            entries,
            vec![
                TocPreviewEntry {
                    depth: 1,
                    title: "Title".to_string(),
                },
                TocPreviewEntry {
                    depth: 2,
                    title: "Section".to_string(),
                },
            ]
        );
    }

    #[test]
    fn toc_preview_is_disabled_when_selection_touches_block() {
        let block = toc_block();

        assert!(should_render_toc_preview(&block, 0..0));
        assert!(should_render_toc_preview(&block, 20..20));
        assert!(selection_touches_render_block(12..12, &block));
        assert!(!should_render_toc_preview(&block, 12..12));
        assert!(!should_render_toc_preview(&block, 10..15));
    }

    #[test]
    fn toc_preview_click_moves_cursor_to_marker() {
        let block = toc_block();
        assert_eq!(toc_edit_cursor_offset(&block), 10);
    }

    #[test]
    fn footnote_preview_extracts_label_and_body() {
        let block = footnote_block("note", "Body text");

        assert_eq!(
            footnote_preview(&block),
            FootnotePreview {
                label: "note".to_string(),
                body: "Body text".to_string(),
            }
        );
    }

    #[test]
    fn footnote_preview_unescapes_label_close() {
        let block = footnote_block(r"a\]b", "Escaped label");

        assert_eq!(footnote_preview(&block).label, "a]b");
    }

    #[test]
    fn footnote_preview_is_disabled_when_selection_touches_block() {
        let block = footnote_block("note", "Body text");

        assert!(should_render_footnote_preview(&block, 0..0));
        assert!(should_render_footnote_preview(&block, 80..80));
        assert!(selection_touches_render_block(24..24, &block));
        assert!(!should_render_footnote_preview(&block, 24..24));
        assert!(!should_render_footnote_preview(&block, 20..28));
    }

    #[test]
    fn footnote_preview_click_moves_cursor_to_body() {
        let block = footnote_block("note", "Body text");
        assert_eq!(footnote_edit_cursor_offset(&block), "[^note]: ".len() + 20);
    }

    #[test]
    fn metadata_preview_parses_yaml_and_toml_style_entries() {
        assert_eq!(
            parse_metadata_preview_entry("title: Draft"),
            Some(MetadataPreviewEntry {
                key: "title".to_string(),
                value: "Draft".to_string(),
            })
        );
        assert_eq!(
            parse_metadata_preview_entry("tags = [\"acceptance\", \"export\"]"),
            Some(MetadataPreviewEntry {
                key: "tags".to_string(),
                value: "acceptance, export".to_string(),
            })
        );
        assert_eq!(
            parse_metadata_preview_entry("date: "),
            Some(MetadataPreviewEntry {
                key: "date".to_string(),
                value: "Empty".to_string(),
            })
        );
        assert_eq!(parse_metadata_preview_entry("# comment"), None);
    }

    #[test]
    fn front_matter_preview_collects_visible_metadata_entries() {
        let block = front_matter_block();

        assert_eq!(
            front_matter_preview_entries(&block),
            vec![
                MetadataPreviewEntry {
                    key: "title".to_string(),
                    value: "Draft".to_string(),
                },
                MetadataPreviewEntry {
                    key: "tags".to_string(),
                    value: "acceptance, export".to_string(),
                },
            ]
        );
    }

    #[test]
    fn front_matter_preview_is_disabled_when_selection_touches_block() {
        let block = front_matter_block();

        assert!(should_render_front_matter_preview(&block, 100..100));
        assert!(should_render_front_matter_preview(&block, 0..0));
        assert!(selection_touches_render_block(6..6, &block));
        assert!(!should_render_front_matter_preview(&block, 6..6));
        assert!(!should_render_front_matter_preview(&block, 4..12));
    }

    #[test]
    fn front_matter_preview_click_moves_cursor_to_first_metadata_line() {
        let block = front_matter_block();
        assert_eq!(front_matter_edit_cursor_offset(&block), "---\n".len());
    }

    #[test]
    fn word_range_selects_ascii_word_from_middle() {
        let text = "hello world";
        // offset 2 lands in "hello"
        assert_eq!(word_range_at_visible_offset(text, 2), Some(0..5));
    }

    #[test]
    fn word_range_selects_ascii_word_from_start() {
        let text = "hello world";
        assert_eq!(word_range_at_visible_offset(text, 0), Some(0..5));
    }

    #[test]
    fn word_range_selects_ascii_word_from_end() {
        let text = "hello world";
        // offset 10 is 'd' of "world"
        assert_eq!(word_range_at_visible_offset(text, 10), Some(6..11));
    }

    #[test]
    fn word_range_at_space_falls_back_to_preceding_word() {
        let text = "hello world";
        // offset 5 is the space between words — function falls back to
        // the preceding word character ('o'), so it selects "hello"
        assert_eq!(word_range_at_visible_offset(text, 5), Some(0..5));
    }

    #[test]
    fn word_range_returns_none_for_leading_space() {
        let text = " hello";
        // offset 0 is a leading space with no preceding word char → None
        assert_eq!(word_range_at_visible_offset(text, 0), None);
    }

    #[test]
    fn word_range_selects_word_when_offset_at_end_of_text() {
        let text = "hello";
        // offset == len should fall back to last char
        assert_eq!(word_range_at_visible_offset(text, 5), Some(0..5));
    }

    #[test]
    fn word_range_handles_mixed_ascii_and_cjk_prefix() {
        // CJK characters are alphanumeric in Rust, so treated as word chars
        let text = "你好 world";
        // "你好" = 6 bytes (3 each), space at 6, "world" at 7
        assert_eq!(word_range_at_visible_offset(text, 0), Some(0..6));
        assert_eq!(word_range_at_visible_offset(text, 7), Some(7..12));
    }
}

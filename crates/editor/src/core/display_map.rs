use std::ops::Range;

use super::{
    document::{BlockKind, BlockProjection, DocumentBuffer, SelectionAffinity, SelectionModel},
    syntax::InlineStyle,
    table::{TABLE_COLUMN_GAP, TableCellRef, TableModel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenSyntaxPolicy {
    SelectionAware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderInlineStyle {
    pub strong: bool,
    pub emphasis: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: bool,
}

impl From<&InlineStyle> for RenderInlineStyle {
    fn from(value: &InlineStyle) -> Self {
        Self {
            strong: value.strong,
            emphasis: value.emphasis,
            strikethrough: value.strikethrough,
            code: value.code,
            link: value.link,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedNodeKind {
    CodeBlock { language: Option<String> },
    Table,
    Image,
    MathBlock,
    Diagram { language: String },
    HtmlBlock,
    FootnoteDefinition,
    Toc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSpanMeta {
    Link {
        target: String,
        title: Option<String>,
    },
    Image {
        src: String,
        alt: String,
        title: Option<String>,
    },
    Math {
        source: String,
        display: bool,
    },
    Html {
        source: String,
    },
    ReferenceLink {
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSpanKind {
    Text,
    HiddenSyntax,
    ListMarker,
    TaskMarker,
    LineBreak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSpan {
    pub kind: RenderSpanKind,
    pub source_range: Range<usize>,
    pub visible_range: Range<usize>,
    pub source_text: String,
    pub visible_text: String,
    pub hidden: bool,
    pub style: RenderInlineStyle,
    pub meta: Option<RenderSpanMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBlock {
    pub id: u64,
    pub kind: BlockKind,
    pub source_range: Range<usize>,
    pub content_range: Range<usize>,
    pub visible_range: Range<usize>,
    pub visible_text: String,
    pub spans: Vec<RenderSpan>,
    pub embedded: Option<EmbeddedNodeKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitTestResult {
    pub source_offset: usize,
    pub visible_offset: usize,
    pub block_id: Option<u64>,
    pub block_index: usize,
    pub is_hidden_syntax: bool,
    pub embedded: Option<EmbeddedNodeKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayMap {
    pub hidden_syntax_policy: HiddenSyntaxPolicy,
    pub visible_text: String,
    pub blocks: Vec<RenderBlock>,
    boundary_mappings: Vec<BoundaryMapping>,
}

impl DisplayMap {
    pub(crate) fn from_source_text(text: &str) -> Self {
        let span = RenderSpan {
            kind: RenderSpanKind::Text,
            source_range: 0..text.len(),
            visible_range: 0..text.len(),
            source_text: text.to_string(),
            visible_text: text.to_string(),
            hidden: false,
            style: RenderInlineStyle::default(),
            meta: None,
        };
        let block = RenderBlock {
            id: 0,
            kind: BlockKind::Raw,
            source_range: 0..text.len(),
            content_range: 0..text.len(),
            visible_range: 0..text.len(),
            visible_text: text.to_string(),
            spans: vec![span],
            embedded: None,
        };
        let blocks = vec![block];
        let boundary_mappings = build_boundary_mappings(text, &blocks);

        Self {
            hidden_syntax_policy: HiddenSyntaxPolicy::SelectionAware,
            visible_text: text.to_string(),
            blocks,
            boundary_mappings,
        }
    }

    pub(crate) fn from_document(
        document: &DocumentBuffer,
        selection: Option<&SelectionModel>,
        hidden_syntax_policy: HiddenSyntaxPolicy,
    ) -> Self {
        let mut visible_text = String::new();
        let mut blocks = Vec::with_capacity(document.blocks().len());

        for (block_index, block) in document.blocks().iter().enumerate() {
            let mut builder = BlockBuilder::new(block, document, selection, hidden_syntax_policy);
            builder.build();
            let mut render_block = builder.finish();

            render_block.visible_range =
                visible_text.len()..visible_text.len() + render_block.visible_text.len();
            for span in &mut render_block.spans {
                span.visible_range = span.visible_range.start + render_block.visible_range.start
                    ..span.visible_range.end + render_block.visible_range.start;
            }

            if block_index > 0 && !visible_text.is_empty() && !render_block.visible_text.is_empty()
            {
                // Keep the linearized visible document newline separated to make source/visible
                // mapping and tests easier, even when blocks render with visual spacing.
                visible_text.push('\n');
                for span in &mut render_block.spans {
                    span.visible_range = span.visible_range.start + 1..span.visible_range.end + 1;
                }
                render_block.visible_range =
                    render_block.visible_range.start + 1..render_block.visible_range.end + 1;
            }

            visible_text.push_str(&render_block.visible_text);
            blocks.push(render_block);
        }

        let boundary_mappings = build_boundary_mappings(&visible_text, &blocks);

        Self {
            hidden_syntax_policy,
            visible_text,
            blocks,
            boundary_mappings,
        }
    }

    pub fn source_to_visible(&self, source_offset: usize) -> usize {
        self.source_to_visible_with_affinity(source_offset, SelectionAffinity::Downstream)
    }

    pub fn source_to_visible_with_affinity(
        &self,
        source_offset: usize,
        affinity: SelectionAffinity,
    ) -> usize {
        let mut last_visible = 0usize;
        for (block_index, block) in self.blocks.iter().enumerate() {
            for (span_index, span) in block.spans.iter().enumerate() {
                last_visible = span.visible_range.end;
                if source_offset < span.source_range.start {
                    return span.visible_range.start;
                }
                if source_offset <= span.source_range.end {
                    if source_offset == span.source_range.end
                        && affinity == SelectionAffinity::Downstream
                        && should_prefer_next_block_start_at_hidden_boundary(
                            &self.blocks,
                            block_index,
                            source_offset,
                        )
                    {
                        continue;
                    }
                    if source_offset == span.source_range.end
                        && affinity == SelectionAffinity::Downstream
                        && should_prefer_next_table_span_boundary(block, span_index, source_offset)
                    {
                        continue;
                    }

                    if span.hidden {
                        return span.visible_range.start;
                    }

                    let relative = source_offset.saturating_sub(span.source_range.start);
                    let mapped = if affinity == SelectionAffinity::Upstream
                        && source_offset == span.source_range.start
                    {
                        span.visible_range.start
                    } else if span.source_range.is_empty() && !span.visible_range.is_empty() {
                        span.visible_range.end
                    } else {
                        (span.visible_range.start + relative).min(span.visible_range.end)
                    };
                    return clamp_to_char_boundary(&self.visible_text, mapped);
                }
            }
        }

        clamp_to_char_boundary(&self.visible_text, last_visible)
    }

    pub fn visible_to_source(&self, visible_offset: usize) -> HitTestResult {
        self.visible_to_source_with_affinity(visible_offset, SelectionAffinity::Downstream)
    }

    pub fn visible_to_source_with_affinity(
        &self,
        visible_offset: usize,
        affinity: SelectionAffinity,
    ) -> HitTestResult {
        let visible_offset = clamp_to_char_boundary(&self.visible_text, visible_offset);
        let source_offset = self.source_offset_for_visible_boundary(visible_offset, affinity);

        for (block_index, block) in self.blocks.iter().enumerate() {
            if source_offset > block.source_range.end {
                continue;
            }

            for span in &block.spans {
                if source_offset < span.source_range.start {
                    return HitTestResult {
                        source_offset,
                        visible_offset,
                        block_id: Some(block.id),
                        block_index,
                        is_hidden_syntax: false,
                        embedded: block.embedded.clone(),
                    };
                }

                if source_offset <= span.source_range.end {
                    let is_hidden_syntax = span.hidden
                        || (span.visible_range.is_empty() && span.source_range.is_empty());
                    return HitTestResult {
                        source_offset,
                        visible_offset,
                        block_id: Some(block.id),
                        block_index,
                        is_hidden_syntax,
                        embedded: block.embedded.clone(),
                    };
                }
            }

            if source_offset <= block.source_range.end {
                return HitTestResult {
                    source_offset,
                    visible_offset,
                    block_id: Some(block.id),
                    block_index,
                    is_hidden_syntax: false,
                    embedded: block.embedded.clone(),
                };
            }
        }

        HitTestResult {
            source_offset,
            visible_offset,
            block_id: None,
            block_index: 0,
            is_hidden_syntax: false,
            embedded: None,
        }
    }

    pub fn source_selection_to_visible(&self, selection: &SelectionModel) -> SelectionModel {
        let (anchor_affinity, head_affinity) = source_selection_affinities(selection);
        SelectionModel {
            anchor_byte: self
                .source_to_visible_with_affinity(selection.anchor_byte, anchor_affinity),
            head_byte: self.source_to_visible_with_affinity(selection.head_byte, head_affinity),
            preferred_column: selection.preferred_column,
            affinity: selection.affinity,
        }
    }

    pub fn visible_selection_to_source(&self, selection: &SelectionModel) -> SelectionModel {
        let (anchor_affinity, head_affinity) = visible_selection_affinities(selection);
        SelectionModel {
            anchor_byte: self
                .visible_to_source_with_affinity(selection.anchor_byte, anchor_affinity)
                .source_offset,
            head_byte: self
                .visible_to_source_with_affinity(selection.head_byte, head_affinity)
                .source_offset,
            preferred_column: selection.preferred_column,
            affinity: selection.affinity,
        }
    }

    fn source_offset_for_visible_boundary(
        &self,
        visible_offset: usize,
        affinity: SelectionAffinity,
    ) -> usize {
        let boundary = self
            .boundary_mappings
            .get(visible_offset.min(self.boundary_mappings.len().saturating_sub(1)))
            .copied()
            .unwrap_or_default();
        match affinity {
            SelectionAffinity::Upstream => boundary.upstream_source_offset,
            SelectionAffinity::Downstream => boundary.downstream_source_offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BoundaryMapping {
    upstream_source_offset: usize,
    downstream_source_offset: usize,
}

fn should_prefer_next_block_start_at_hidden_boundary(
    blocks: &[RenderBlock],
    current_index: usize,
    source_offset: usize,
) -> bool {
    let Some(current) = blocks.get(current_index) else {
        return false;
    };
    let Some(next) = blocks.get(current_index + 1) else {
        return false;
    };

    (current.kind == BlockKind::Raw
        && current.content_range.is_empty()
        && current.source_range.end == source_offset
        && next.source_range.start == source_offset
        && matches!(next.kind, BlockKind::Paragraph | BlockKind::Raw))
        || (current.content_range.end < source_offset
            && current.source_range.end == source_offset
            && next.content_range.start == source_offset
            && next.kind != BlockKind::Raw)
}

fn should_prefer_next_table_span_boundary(
    block: &RenderBlock,
    span_index: usize,
    source_offset: usize,
) -> bool {
    if block.kind != BlockKind::Table {
        return false;
    }
    let Some(span) = block.spans.get(span_index) else {
        return false;
    };
    if !span.hidden || span.source_range.end != source_offset {
        return false;
    }

    block
        .spans
        .iter()
        .skip(span_index + 1)
        .take_while(|next| next.source_range.start == source_offset)
        .any(|next| !next.hidden || !next.visible_range.is_empty())
}

fn source_selection_affinities(
    selection: &SelectionModel,
) -> (SelectionAffinity, SelectionAffinity) {
    if selection.is_collapsed() {
        return (selection.affinity, selection.affinity);
    }

    if selection.anchor_byte <= selection.head_byte {
        (SelectionAffinity::Upstream, SelectionAffinity::Downstream)
    } else {
        (SelectionAffinity::Downstream, SelectionAffinity::Upstream)
    }
}

fn visible_selection_affinities(
    selection: &SelectionModel,
) -> (SelectionAffinity, SelectionAffinity) {
    if selection.is_collapsed() {
        return (selection.affinity, selection.affinity);
    }

    if selection.anchor_byte <= selection.head_byte {
        (SelectionAffinity::Downstream, SelectionAffinity::Upstream)
    } else {
        (SelectionAffinity::Upstream, SelectionAffinity::Downstream)
    }
}

fn build_boundary_mappings(visible_text: &str, blocks: &[RenderBlock]) -> Vec<BoundaryMapping> {
    let visible_len = visible_text.len();
    let mut upstream = vec![None; visible_len + 1];
    let mut downstream = vec![None; visible_len + 1];

    let mut last_visible_end = 0usize;
    let mut last_source_end = 0usize;

    for block in blocks {
        if block.visible_range.start > last_visible_end {
            apply_virtual_gap_mapping(
                last_visible_end,
                block.visible_range.start,
                last_source_end,
                block.source_range.start,
                &mut upstream,
                &mut downstream,
            );
            last_visible_end = block.visible_range.start;
            last_source_end = block.source_range.start;
        }

        for span in &block.spans {
            if span.visible_range.start > last_visible_end {
                apply_virtual_gap_mapping(
                    last_visible_end,
                    span.visible_range.start,
                    last_source_end,
                    span.source_range.start,
                    &mut upstream,
                    &mut downstream,
                );
            }

            apply_span_mapping(span, &mut upstream, &mut downstream);
            last_visible_end = span.visible_range.end;
            last_source_end = span.source_range.end;
        }
    }

    let mut last = 0usize;
    for entry in &mut upstream {
        if let Some(offset) = *entry {
            last = offset;
        } else {
            *entry = Some(last);
        }
    }

    let mut next = last;
    for entry in downstream.iter_mut().rev() {
        if let Some(offset) = *entry {
            next = offset;
        } else {
            *entry = Some(next);
        }
    }

    upstream
        .into_iter()
        .zip(downstream)
        .map(|(upstream, downstream)| BoundaryMapping {
            upstream_source_offset: upstream.unwrap_or_default(),
            downstream_source_offset: downstream.unwrap_or_default(),
        })
        .collect()
}

fn apply_virtual_gap_mapping(
    visible_start: usize,
    visible_end: usize,
    upstream_source: usize,
    downstream_source: usize,
    upstream: &mut [Option<usize>],
    downstream: &mut [Option<usize>],
) {
    if visible_start >= visible_end {
        return;
    }

    set_if_none(upstream, visible_start, upstream_source);
    set_boundary(downstream, visible_start, downstream_source);

    for boundary in visible_start + 1..=visible_end {
        set_boundary(upstream, boundary, downstream_source);
        set_boundary(downstream, boundary, downstream_source);
    }
}

fn apply_span_mapping(
    span: &RenderSpan,
    upstream: &mut [Option<usize>],
    downstream: &mut [Option<usize>],
) {
    let visible_start = span.visible_range.start;
    let visible_end = span.visible_range.end;
    let source_start = span.source_range.start;
    let source_end = span.source_range.end;

    set_if_none(upstream, visible_start, source_start);

    if span.hidden || visible_start == visible_end {
        set_boundary(downstream, visible_start, source_end);
        return;
    }

    set_boundary(downstream, visible_start, source_start);

    let visible_len = visible_end.saturating_sub(visible_start);
    let source_len = source_end.saturating_sub(source_start);
    let shared_len = visible_len.min(source_len);

    for offset in 1..=shared_len {
        let source_offset = source_start + offset;
        let visible_offset = visible_start + offset;
        set_boundary(upstream, visible_offset, source_offset);
        set_boundary(downstream, visible_offset, source_offset);
    }

    if visible_len > source_len {
        for visible_offset in visible_start + shared_len + 1..=visible_end {
            set_boundary(upstream, visible_offset, source_end);
            set_boundary(downstream, visible_offset, source_end);
        }
    } else if source_len > visible_len {
        set_boundary(upstream, visible_end, source_end);
        set_boundary(downstream, visible_end, source_end);
    }
}

fn set_if_none(boundaries: &mut [Option<usize>], index: usize, source_offset: usize) {
    if let Some(entry) = boundaries.get_mut(index)
        && entry.is_none()
    {
        *entry = Some(source_offset);
    }
}

fn set_boundary(boundaries: &mut [Option<usize>], index: usize, source_offset: usize) {
    if let Some(entry) = boundaries.get_mut(index) {
        *entry = Some(source_offset);
    }
}

fn clamp_to_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

struct BlockBuilder<'a> {
    block: &'a BlockProjection,
    document: &'a DocumentBuffer,
    selection: Option<&'a SelectionModel>,
    hidden_syntax_policy: HiddenSyntaxPolicy,
    spans: Vec<RenderSpan>,
    visible_text: String,
}

impl<'a> BlockBuilder<'a> {
    fn new(
        block: &'a BlockProjection,
        document: &'a DocumentBuffer,
        selection: Option<&'a SelectionModel>,
        hidden_syntax_policy: HiddenSyntaxPolicy,
    ) -> Self {
        Self {
            block,
            document,
            selection,
            hidden_syntax_policy,
            spans: Vec::new(),
            visible_text: String::new(),
        }
    }

    fn finish(self) -> RenderBlock {
        let embedded = match &self.block.kind {
            BlockKind::CodeFence { language } => {
                if language
                    .as_deref()
                    .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
                {
                    Some(EmbeddedNodeKind::Diagram {
                        language: "mermaid".to_string(),
                    })
                } else {
                    Some(EmbeddedNodeKind::CodeBlock {
                        language: language.clone(),
                    })
                }
            }
            BlockKind::Table => Some(EmbeddedNodeKind::Table),
            BlockKind::MathBlock => Some(EmbeddedNodeKind::MathBlock),
            BlockKind::Html => Some(EmbeddedNodeKind::HtmlBlock),
            BlockKind::FootnoteDefinition | BlockKind::Footnote => {
                Some(EmbeddedNodeKind::FootnoteDefinition)
            }
            _ => None,
        };

        RenderBlock {
            id: self.block.id,
            kind: self.block.kind.clone(),
            source_range: self.block.byte_range.clone(),
            content_range: self.block.content_range.clone(),
            visible_range: 0..self.visible_text.len(),
            visible_text: self.visible_text,
            spans: self.spans,
            embedded,
        }
    }

    fn build(&mut self) {
        let text = self.document.block_text(self.block);
        match &self.block.kind {
            BlockKind::Heading { .. } => self.push_heading(&text),
            BlockKind::Blockquote => self.push_blockquote(&text),
            BlockKind::List => self.push_list(&text),
            BlockKind::Table => self.push_table(&text),
            BlockKind::CodeFence { .. } => self.push_code_fence(&text),
            _ => self.push_inline_text(
                self.block.content_range.start,
                &text,
                RenderInlineStyle::default(),
            ),
        }

        let trailing = self.document.block_trailing_text(self.block);
        if !trailing.is_empty() {
            self.push_span(
                RenderSpanKind::LineBreak,
                self.block.content_range.end..self.block.byte_range.end,
                trailing.clone(),
                trailing,
                false,
                RenderInlineStyle::default(),
                None,
            );
        }
    }

    fn push_heading(&mut self, text: &str) {
        let marker_len = heading_marker_len(text);
        if marker_len > 0 {
            self.push_hidden_or_visible(
                RenderSpanKind::HiddenSyntax,
                self.block.content_range.start..self.block.content_range.start + marker_len,
                &text[..marker_len],
            );
            self.push_inline_text(
                self.block.content_range.start + marker_len,
                &text[marker_len..],
                RenderInlineStyle::default(),
            );
        } else {
            self.push_inline_text(
                self.block.content_range.start,
                text,
                RenderInlineStyle::default(),
            );
        }
    }

    fn push_blockquote(&mut self, text: &str) {
        let mut source_offset = self.block.content_range.start;
        for segment in split_inclusive_lines(text) {
            let marker_len = blockquote_marker_len(segment);
            if marker_len > 0 {
                self.push_hidden_or_visible(
                    RenderSpanKind::HiddenSyntax,
                    source_offset..source_offset + marker_len,
                    &segment[..marker_len],
                );
                self.push_inline_text(
                    source_offset + marker_len,
                    &segment[marker_len..],
                    RenderInlineStyle::default(),
                );
            } else {
                self.push_inline_text(source_offset, segment, RenderInlineStyle::default());
            }
            source_offset += segment.len();
        }
    }

    fn push_list(&mut self, text: &str) {
        let mut source_offset = self.block.content_range.start;
        for segment in split_inclusive_lines(text) {
            let line = segment.trim_end_matches(['\r', '\n']);
            let newline_len = segment.len().saturating_sub(line.len());
            if let Some(prefix_len) = task_bullet_prefix_len(line) {
                let task_marker_end = task_source_suffix_start(line);
                self.push_hidden_or_visible(
                    RenderSpanKind::HiddenSyntax,
                    source_offset..source_offset + prefix_len,
                    &segment[..prefix_len],
                );
                self.push_span(
                    RenderSpanKind::TaskMarker,
                    source_offset + prefix_len..source_offset + task_marker_end,
                    line[prefix_len..task_marker_end].to_string(),
                    task_marker_text(line),
                    false,
                    RenderInlineStyle::default(),
                    None,
                );
                self.push_inline_text(
                    source_offset + task_marker_end,
                    &line[task_marker_end..],
                    RenderInlineStyle::default(),
                );
            } else if let Some(prefix_len) = generic_list_prefix_len(line) {
                self.push_hidden_or_visible(
                    RenderSpanKind::ListMarker,
                    source_offset..source_offset + prefix_len,
                    &segment[..prefix_len],
                );
                self.push_inline_text(
                    source_offset + prefix_len,
                    &line[prefix_len..],
                    RenderInlineStyle::default(),
                );
            } else {
                self.push_inline_text(source_offset, line, RenderInlineStyle::default());
            }

            if newline_len > 0 {
                let newline_start = source_offset + line.len();
                let newline_text = segment[line.len()..].to_string();
                self.push_span(
                    RenderSpanKind::LineBreak,
                    newline_start..newline_start + newline_len,
                    newline_text.clone(),
                    newline_text,
                    false,
                    RenderInlineStyle::default(),
                    None,
                );
            }
            source_offset += segment.len();
        }
    }

    fn push_code_fence(&mut self, text: &str) {
        let lines = split_inclusive_lines(text);
        if lines.is_empty() {
            return;
        }

        let mut source_offset = self.block.content_range.start;
        if let Some(first) = lines.first() {
            self.push_hidden_or_visible(
                RenderSpanKind::HiddenSyntax,
                source_offset..source_offset + first.len(),
                first,
            );
            source_offset += first.len();
        }

        if lines.len() > 1 {
            for middle in &lines[1..lines.len().saturating_sub(1)] {
                self.push_span(
                    RenderSpanKind::Text,
                    source_offset..source_offset + middle.len(),
                    middle.to_string(),
                    middle.to_string(),
                    false,
                    RenderInlineStyle {
                        code: true,
                        ..RenderInlineStyle::default()
                    },
                    None,
                );
                source_offset += middle.len();
            }

            if let Some(last) = lines.last() {
                self.push_hidden_or_visible(
                    RenderSpanKind::HiddenSyntax,
                    source_offset..source_offset + last.len(),
                    last,
                );
            }
        }
    }

    fn push_table(&mut self, text: &str) {
        let table = TableModel::parse(text);
        if table.is_empty() {
            self.push_inline_text(
                self.block.content_range.start,
                text,
                RenderInlineStyle::default(),
            );
            return;
        }

        let mut column_widths = vec![0usize; table.column_count()];
        for visible_row in 0..table.visible_row_count() {
            for column in 0..table.column_count() {
                let Some(cell_range) = table.cell_source_range(TableCellRef {
                    visible_row,
                    column,
                }) else {
                    continue;
                };
                let source_start = self.block.content_range.start + cell_range.start;
                let visible_width = self
                    .visible_inline_text(source_start, &text[cell_range.clone()])
                    .chars()
                    .count();
                column_widths[column] = column_widths[column].max(visible_width);
            }
        }

        let mut last_source_offset = self.block.content_range.start;
        let mut rendered_visible_row = false;

        for row in table.rows() {
            let row_start = self.block.content_range.start + row.line_start;
            let row_end = self.block.content_range.start + row.end_with_newline;
            if row.is_delimiter {
                self.push_hidden_source(last_source_offset..row_end);
                last_source_offset = row_end;
                continue;
            }

            let row_content_start = row
                .cells
                .first()
                .map(|cell| self.block.content_range.start + cell.source_range.start)
                .unwrap_or(row_start);
            self.push_hidden_source(last_source_offset..row_content_start);
            if rendered_visible_row {
                self.push_virtual_visible(
                    RenderSpanKind::LineBreak,
                    row_content_start,
                    "\n".to_string(),
                    RenderInlineStyle::default(),
                );
            }

            let mut cursor = row_content_start;
            for column in 0..table.column_count() {
                if let Some(cell) = row.cells.get(column) {
                    let cell_start = self.block.content_range.start + cell.source_range.start;
                    let cell_end = self.block.content_range.start + cell.source_range.end;
                    let cell_text = &text[cell.source_range.clone()];
                    let visible_width = self
                        .visible_inline_text(cell_start, cell_text)
                        .chars()
                        .count();

                    self.push_hidden_source(cursor..cell_start);
                    self.push_inline_text(cell_start, cell_text, RenderInlineStyle::default());
                    cursor = cell_end;

                    let gap = if column + 1 < table.column_count() {
                        let gap = column_widths[column]
                            .saturating_sub(visible_width)
                            .saturating_add(TABLE_COLUMN_GAP);
                        Some(gap)
                    } else {
                        Some(column_widths[column].saturating_sub(visible_width))
                    };
                    if let Some(gap) = gap.filter(|gap| *gap > 0) {
                        self.push_virtual_visible(
                            RenderSpanKind::Text,
                            cursor,
                            " ".repeat(gap),
                            RenderInlineStyle::default(),
                        );
                    }
                } else {
                    let gap = if column + 1 < table.column_count() {
                        column_widths[column] + TABLE_COLUMN_GAP
                    } else {
                        column_widths[column]
                    };
                    self.push_virtual_visible(
                        RenderSpanKind::Text,
                        cursor,
                        " ".repeat(gap),
                        RenderInlineStyle::default(),
                    );
                }
            }

            self.push_hidden_source(cursor..row_end);
            last_source_offset = row_end;
            rendered_visible_row = true;
        }

        self.push_hidden_source(last_source_offset..self.block.content_range.end);
    }

    fn visible_inline_text(&self, source_start: usize, text: &str) -> String {
        let mut visible = String::new();
        for token in parse_inline_tokens(text) {
            let range =
                source_start + token.local_range.start..source_start + token.local_range.end;
            let reveal_range = token.reveal_range.clone().map(|reveal_range| {
                source_start + reveal_range.start..source_start + reveal_range.end
            });
            let hidden = token.hidden && !self.should_reveal_inline(&range, reveal_range.as_ref());
            if !hidden {
                visible.push_str(&token.visible_text);
            }
        }
        visible
    }

    fn push_inline_text(&mut self, source_start: usize, text: &str, style: RenderInlineStyle) {
        for token in parse_inline_tokens(text) {
            let range =
                source_start + token.local_range.start..source_start + token.local_range.end;
            let reveal_range = token.reveal_range.clone().map(|reveal_range| {
                source_start + reveal_range.start..source_start + reveal_range.end
            });
            let hidden = token.hidden && !self.should_reveal_inline(&range, reveal_range.as_ref());
            let visible_text = if hidden {
                String::new()
            } else {
                token.visible_text.clone()
            };
            self.push_span(
                if hidden {
                    RenderSpanKind::HiddenSyntax
                } else {
                    RenderSpanKind::Text
                },
                range,
                token.source_text,
                visible_text,
                hidden,
                if hidden {
                    RenderInlineStyle::default()
                } else {
                    merge_inline_styles(style, token.style)
                },
                token.meta,
            );
        }
    }

    fn push_hidden_source(&mut self, source_range: Range<usize>) {
        if source_range.is_empty() {
            return;
        }

        self.push_span(
            RenderSpanKind::HiddenSyntax,
            source_range.clone(),
            self.document.text_for_range(source_range),
            String::new(),
            true,
            RenderInlineStyle::default(),
            None,
        );
    }

    fn push_virtual_visible(
        &mut self,
        kind: RenderSpanKind,
        source_offset: usize,
        visible_text: String,
        style: RenderInlineStyle,
    ) {
        if visible_text.is_empty() {
            return;
        }

        self.push_span(
            kind,
            source_offset..source_offset,
            String::new(),
            visible_text,
            false,
            style,
            None,
        );
    }

    fn push_hidden_or_visible(
        &mut self,
        kind: RenderSpanKind,
        source_range: Range<usize>,
        source_text: &str,
    ) {
        let hidden = !self.should_reveal_block_syntax(&source_range);
        self.push_span(
            kind,
            source_range,
            source_text.to_string(),
            if hidden {
                String::new()
            } else {
                source_text.to_string()
            },
            hidden,
            RenderInlineStyle::default(),
            None,
        );
    }

    fn push_span(
        &mut self,
        kind: RenderSpanKind,
        source_range: Range<usize>,
        source_text: String,
        visible_text: String,
        hidden: bool,
        style: RenderInlineStyle,
        meta: Option<RenderSpanMeta>,
    ) {
        let visible_start = self.visible_text.len();
        self.visible_text.push_str(&visible_text);
        let visible_end = self.visible_text.len();
        self.spans.push(RenderSpan {
            kind,
            source_range,
            visible_range: visible_start..visible_end,
            source_text,
            visible_text,
            hidden,
            style,
            meta,
        });
    }

    fn should_reveal(&self, source_range: &Range<usize>) -> bool {
        match self.hidden_syntax_policy {
            HiddenSyntaxPolicy::SelectionAware => self
                .selection
                .map(|selection| ranges_overlap(&selection.range(), source_range))
                .unwrap_or(false),
        }
    }

    fn should_reveal_inline(
        &self,
        source_range: &Range<usize>,
        reveal_range: Option<&Range<usize>>,
    ) -> bool {
        if self.should_reveal(source_range) {
            return true;
        }

        let Some(reveal_range) = reveal_range else {
            return false;
        };
        let Some(selection) = self.selection else {
            return false;
        };

        selection_intersects_range(selection, reveal_range)
    }

    fn should_reveal_block_syntax(&self, source_range: &Range<usize>) -> bool {
        if self.should_reveal(source_range) {
            return true;
        }

        let Some(selection) = self.selection else {
            return false;
        };
        if !selection.is_collapsed() {
            return false;
        }

        let cursor = selection.cursor();
        matches!(
            self.block.kind,
            BlockKind::Heading { .. } | BlockKind::Blockquote | BlockKind::List
        ) && cursor == source_range.end
            && selection.affinity == SelectionAffinity::Upstream
    }
}

#[derive(Debug, Clone)]
struct InlineToken {
    local_range: Range<usize>,
    reveal_range: Option<Range<usize>>,
    source_text: String,
    visible_text: String,
    hidden: bool,
    style: RenderInlineStyle,
    meta: Option<RenderSpanMeta>,
}

fn parse_inline_tokens(text: &str) -> Vec<InlineToken> {
    let mut tokens = Vec::new();
    parse_inline_tokens_into(text, 0, RenderInlineStyle::default(), &mut tokens);
    if tokens.is_empty() {
        tokens.push(InlineToken {
            local_range: 0..text.len(),
            reveal_range: None,
            source_text: text.to_string(),
            visible_text: text.to_string(),
            hidden: false,
            style: RenderInlineStyle::default(),
            meta: None,
        });
    }
    tokens
}

fn parse_inline_tokens_into(
    text: &str,
    base_offset: usize,
    style: RenderInlineStyle,
    tokens: &mut Vec<InlineToken>,
) {
    let mut offset = 0usize;
    while offset < text.len() {
        let rest = &text[offset..];

        if let Some(escaped) = rest.strip_prefix('\\')
            && let Some(ch) = escaped.chars().next()
        {
            push_escaped_text_token(
                tokens,
                base_offset + offset,
                1 + ch.len_utf8(),
                &ch.to_string(),
                style,
            );
            offset += 1 + ch.len_utf8();
            continue;
        }

        if let Some((delimiter, advance, update)) = [
            ("**", 2usize, InlineMarker::Strong),
            ("__", 2usize, InlineMarker::Strong),
            ("~~", 2usize, InlineMarker::Strike),
        ]
        .into_iter()
        .find(|(delimiter, _, _)| rest.starts_with(*delimiter))
        {
            if let Some(end) = text[offset + advance..].find(delimiter) {
                let inner_start = offset + advance;
                let inner_end = inner_start + end;
                push_hidden_marker(tokens, base_offset + offset, delimiter);
                let mut nested = style;
                match update {
                    InlineMarker::Strong => nested.strong = true,
                    InlineMarker::Strike => nested.strikethrough = true,
                }
                parse_inline_tokens_into(
                    &text[inner_start..inner_end],
                    base_offset + inner_start,
                    nested,
                    tokens,
                );
                push_hidden_marker(tokens, base_offset + inner_end, delimiter);
                offset = inner_end + advance;
                continue;
            }
        }

        if let Some((delimiter, advance)) = [("*", 1usize), ("_", 1usize)]
            .into_iter()
            .find(|(delimiter, _)| rest.starts_with(*delimiter))
        {
            if let Some(end) = text[offset + advance..].find(delimiter) {
                let inner_start = offset + advance;
                let inner_end = inner_start + end;
                push_hidden_marker(tokens, base_offset + offset, delimiter);
                let mut nested = style;
                nested.emphasis = true;
                parse_inline_tokens_into(
                    &text[inner_start..inner_end],
                    base_offset + inner_start,
                    nested,
                    tokens,
                );
                push_hidden_marker(tokens, base_offset + inner_end, delimiter);
                offset = inner_end + advance;
                continue;
            }
        }

        if let Some(tail) = rest.strip_prefix('`')
            && let Some(end) = tail.find('`')
        {
            push_hidden_marker(tokens, base_offset + offset, "`");
            let inner_start = offset + 1;
            let inner_end = inner_start + end;
            push_text_token(
                tokens,
                base_offset + inner_start,
                &text[inner_start..inner_end],
                RenderInlineStyle {
                    code: true,
                    ..style
                },
            );
            push_hidden_marker(tokens, base_offset + inner_end, "`");
            offset = inner_end + 1;
            continue;
        }

        if rest.starts_with("![")
            && let Some(close) = rest.find(']')
            && rest[close + 1..].starts_with('(')
            && let Some(close_paren) = rest[close + 2..].find(')')
        {
            let source_end = close + 3 + close_paren;
            let source = &rest[..source_end];
            let alt = rest[2..close].to_string();
            let raw_target = &rest[close + 2..close + 2 + close_paren];
            let (src, title) = parse_link_destination_and_title(raw_target);
            tokens.push(InlineToken {
                local_range: base_offset + offset..base_offset + offset + source.len(),
                reveal_range: Some(offset..offset + source.len()),
                source_text: source.to_string(),
                visible_text: if alt.is_empty() {
                    format!("[image: {src}]")
                } else {
                    format!("[image: {alt}]")
                },
                hidden: false,
                style: RenderInlineStyle {
                    link: true,
                    ..style
                },
                meta: Some(RenderSpanMeta::Image { src, alt, title }),
            });
            offset += source.len();
            continue;
        }

        if rest.starts_with('$')
            && let Some((source, inner, display)) = parse_math_span(rest)
        {
            tokens.push(InlineToken {
                local_range: base_offset + offset..base_offset + offset + source.len(),
                reveal_range: Some(offset..offset + source.len()),
                source_text: source.to_string(),
                visible_text: if display {
                    format!("[math: {}]", inner.trim())
                } else {
                    inner.to_string()
                },
                hidden: false,
                style,
                meta: Some(RenderSpanMeta::Math {
                    source: inner.to_string(),
                    display,
                }),
            });
            offset += source.len();
            continue;
        }

        if rest.starts_with('[')
            && let Some(close) = rest.find(']')
            && rest[close + 1..].starts_with('(')
            && let Some(close_paren) = rest[close + 2..].find(')')
        {
            let inner_start = offset + 1;
            let inner_end = offset + close;
            let target_start = inner_end + 2;
            let target_end = target_start + close_paren;
            let reveal_range = offset..target_end + 1;
            let (target, title) = parse_link_destination_and_title(&text[target_start..target_end]);
            let token_start = tokens.len();
            push_hidden_marker_with_reveal(tokens, base_offset + offset, "[", reveal_range.clone());
            parse_inline_tokens_into(
                &text[inner_start..inner_end],
                base_offset + inner_start,
                RenderInlineStyle {
                    link: true,
                    ..style
                },
                tokens,
            );
            push_hidden_marker_with_reveal(
                tokens,
                base_offset + inner_end,
                "]",
                reveal_range.clone(),
            );
            push_hidden_marker_with_reveal(
                tokens,
                base_offset + inner_end + 1,
                "(",
                reveal_range.clone(),
            );
            push_hidden_marker_with_reveal(
                tokens,
                base_offset + target_start,
                &text[target_start..target_end],
                reveal_range.clone(),
            );
            push_hidden_marker_with_reveal(tokens, base_offset + target_end, ")", reveal_range);
            if let Some(token) = tokens[token_start..].iter_mut().find(|token| !token.hidden) {
                token.meta = Some(RenderSpanMeta::Link { target, title });
            }
            offset = target_end + 1;
            continue;
        }

        let next_special = rest
            .char_indices()
            .skip(1)
            .find(|(_, ch)| matches!(ch, '\\' | '*' | '_' | '~' | '`' | '[' | '!' | '$'))
            .map(|(idx, _)| idx)
            .unwrap_or(rest.len());
        push_text_token(tokens, base_offset + offset, &rest[..next_special], style);
        offset += next_special;
    }
}

fn push_hidden_marker(tokens: &mut Vec<InlineToken>, offset: usize, marker: &str) {
    tokens.push(InlineToken {
        local_range: offset..offset + marker.len(),
        reveal_range: None,
        source_text: marker.to_string(),
        visible_text: marker.to_string(),
        hidden: true,
        style: RenderInlineStyle::default(),
        meta: None,
    });
}

fn push_hidden_marker_with_reveal(
    tokens: &mut Vec<InlineToken>,
    offset: usize,
    marker: &str,
    reveal_range: Range<usize>,
) {
    tokens.push(InlineToken {
        local_range: offset..offset + marker.len(),
        reveal_range: Some(reveal_range),
        source_text: marker.to_string(),
        visible_text: marker.to_string(),
        hidden: true,
        style: RenderInlineStyle::default(),
        meta: None,
    });
}

fn push_text_token(
    tokens: &mut Vec<InlineToken>,
    offset: usize,
    text: &str,
    style: RenderInlineStyle,
) {
    if text.is_empty() {
        return;
    }

    tokens.push(InlineToken {
        local_range: offset..offset + text.len(),
        reveal_range: None,
        source_text: text.to_string(),
        visible_text: text.to_string(),
        hidden: false,
        style,
        meta: None,
    });
}

fn push_escaped_text_token(
    tokens: &mut Vec<InlineToken>,
    offset: usize,
    source_len: usize,
    visible_text: &str,
    style: RenderInlineStyle,
) {
    if visible_text.is_empty() {
        return;
    }

    tokens.push(InlineToken {
        local_range: offset..offset + source_len,
        reveal_range: None,
        source_text: visible_text.to_string(),
        visible_text: visible_text.to_string(),
        hidden: false,
        style,
        meta: None,
    });
}

fn parse_link_destination_and_title(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let target = parts
        .next()
        .unwrap_or_default()
        .trim_matches(['<', '>'])
        .to_string();
    let title = parts.next().and_then(|rest| {
        let title = rest.trim().trim_matches(['"', '\'', '(', ')']).to_string();
        (!title.is_empty()).then_some(title)
    });

    (target, title)
}

fn parse_math_span(rest: &str) -> Option<(&str, &str, bool)> {
    if let Some(tail) = rest.strip_prefix("$$") {
        let end = tail.find("$$")?;
        let source_end = 2 + end + 2;
        return Some((&rest[..source_end], &tail[..end], true));
    }

    let tail = rest.strip_prefix('$')?;
    if tail.starts_with(char::is_whitespace) {
        return None;
    }
    let end = tail.find('$')?;
    if end == 0 {
        return None;
    }
    let source_end = 1 + end + 1;
    Some((&rest[..source_end], &tail[..end], false))
}

#[derive(Debug, Clone, Copy)]
enum InlineMarker {
    Strong,
    Strike,
}

fn heading_marker_len(text: &str) -> usize {
    let trimmed = text.trim_end_matches(['\r', '\n']);
    let depth = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&depth) {
        return 0;
    }

    let rest = &trimmed[depth..];
    if rest.starts_with(' ') { depth + 1 } else { 0 }
}

fn blockquote_marker_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut ix = 0usize;
    while ix < bytes.len() && matches!(bytes[ix], b' ' | b'\t') {
        ix += 1;
    }

    let start = ix;
    while ix < bytes.len() && bytes[ix] == b'>' {
        ix += 1;
        while ix < bytes.len() && matches!(bytes[ix], b' ' | b'\t') {
            ix += 1;
        }
    }

    if ix == start { 0 } else { ix }
}

fn generic_list_prefix_len(line: &str) -> Option<usize> {
    if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
        return Some(2);
    }

    let digit_len = line
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_len == 0 {
        return None;
    }
    match line.as_bytes().get(digit_len).copied() {
        Some(b'.' | b')') if matches!(line.as_bytes().get(digit_len + 1), Some(b' ')) => {
            Some(digit_len + 2)
        }
        _ => None,
    }
}

fn task_bullet_prefix_len(line: &str) -> Option<usize> {
    for prefix in [
        "- [ ] ", "* [ ] ", "+ [ ] ", "- [x] ", "* [x] ", "+ [x] ", "- [X] ", "* [X] ", "+ [X] ",
    ] {
        if line.starts_with(prefix) {
            return Some(2);
        }
    }
    None
}

fn task_marker_text(line: &str) -> String {
    let checked = matches!(line.as_bytes().get(3), Some(b'x' | b'X'));
    if checked {
        "\u{2611} ".to_string()
    } else {
        "\u{2610} ".to_string()
    }
}

fn task_source_suffix_start(line: &str) -> usize {
    if line.len() >= 6 { 6 } else { line.len() }
}

fn split_inclusive_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            lines.push(&text[start..index + 1]);
            start = index + 1;
        }
    }

    if start < text.len() {
        lines.push(&text[start..]);
    }

    lines
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn selection_intersects_range(selection: &SelectionModel, range: &Range<usize>) -> bool {
    if selection.is_collapsed() {
        range.start < selection.cursor() && selection.cursor() < range.end
    } else {
        ranges_overlap(&selection.range(), range)
    }
}

fn merge_inline_styles(base: RenderInlineStyle, overlay: RenderInlineStyle) -> RenderInlineStyle {
    RenderInlineStyle {
        strong: base.strong || overlay.strong,
        emphasis: base.emphasis || overlay.emphasis,
        strikethrough: base.strikethrough || overlay.strikethrough,
        code: base.code || overlay.code,
        link: base.link || overlay.link,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::document::DocumentBuffer;

    fn revealed_boundary_selection(offset: usize) -> SelectionModel {
        SelectionModel {
            anchor_byte: offset,
            head_byte: offset,
            preferred_column: None,
            affinity: SelectionAffinity::Upstream,
        }
    }

    #[test]
    fn heading_display_map_hides_prefix_until_selection_reaches_marker() {
        let doc = DocumentBuffer::from_text("# Heading");
        let hidden = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert_eq!(hidden.visible_text, "Heading");

        let revealed = DisplayMap::from_document(
            &doc,
            Some(&revealed_boundary_selection(1)),
            HiddenSyntaxPolicy::SelectionAware,
        );
        assert_eq!(revealed.visible_text, "# Heading");
    }

    #[test]
    fn inline_markup_produces_hidden_marker_spans() {
        let doc = DocumentBuffer::from_text("Hello **world**");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert_eq!(map.visible_text, "Hello world");
        assert!(
            map.blocks[0]
                .spans
                .iter()
                .any(|span| span.hidden && span.source_text == "**")
        );
    }

    #[test]
    fn code_fence_is_marked_as_embedded_node() {
        let doc = DocumentBuffer::from_text("```rust\nfn main() {}\n```");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert!(matches!(
            map.blocks[0].embedded,
            Some(EmbeddedNodeKind::CodeBlock { .. })
        ));
        assert_eq!(map.visible_text, "fn main() {}\n");
    }

    #[test]
    fn visible_to_source_returns_monotonic_hit_result() {
        let doc = DocumentBuffer::from_text("# Heading");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        let hit = map.visible_to_source(3);
        assert_eq!(hit.source_offset, 5);
        assert_eq!(map.source_to_visible(5), 3);
    }

    #[test]
    fn heading_boundary_tracks_hidden_prefix_by_affinity() {
        let doc = DocumentBuffer::from_text("# Heading");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(
            map.visible_to_source_with_affinity(0, SelectionAffinity::Upstream)
                .source_offset,
            0
        );
        assert_eq!(
            map.visible_to_source_with_affinity(0, SelectionAffinity::Downstream)
                .source_offset,
            2
        );
    }

    #[test]
    fn inline_markup_boundary_tracks_hidden_markers_by_affinity() {
        let doc = DocumentBuffer::from_text("Hello **world**");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(
            map.visible_to_source_with_affinity(6, SelectionAffinity::Upstream)
                .source_offset,
            6
        );
        assert_eq!(
            map.visible_to_source_with_affinity(6, SelectionAffinity::Downstream)
                .source_offset,
            8
        );
        assert_eq!(
            map.visible_to_source_with_affinity(11, SelectionAffinity::Upstream)
                .source_offset,
            13
        );
        assert_eq!(
            map.visible_to_source_with_affinity(11, SelectionAffinity::Downstream)
                .source_offset,
            15
        );
    }

    #[test]
    fn source_selection_round_trips_through_visible_selection() {
        let doc = DocumentBuffer::from_text("Hello **world**");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        let source = SelectionModel {
            anchor_byte: 8,
            head_byte: 13,
            preferred_column: Some(3),
            affinity: SelectionAffinity::Downstream,
        };

        let visible = map.source_selection_to_visible(&source);
        assert_eq!(visible.range(), 6..11);

        let round_trip = map.visible_selection_to_source(&visible);
        assert_eq!(round_trip.range(), source.range());
        assert_eq!(round_trip.preferred_column, source.preferred_column);
    }

    #[test]
    fn task_item_marker_maps_source_and_visible_offsets() {
        let doc = DocumentBuffer::from_text("- [ ] task");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(map.visible_text, "\u{2610} task");
        assert_eq!(map.source_to_visible(2), 0);
        assert_eq!(map.source_to_visible(6), 4);
        assert_eq!(
            map.visible_to_source_with_affinity(0, SelectionAffinity::Downstream)
                .source_offset,
            2
        );
        assert_eq!(
            map.visible_to_source_with_affinity(4, SelectionAffinity::Upstream)
                .source_offset,
            6
        );
    }

    #[test]
    fn table_display_map_hides_pipe_markup_and_delimiter_row() {
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let doc = DocumentBuffer::from_text(source);
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(map.visible_text, "Name   Role\nAda    Eng ");
        assert!(!map.visible_text.contains("---"));

        let second_line_start = map.visible_text.find('\n').unwrap_or(0) + 1;
        assert_eq!(
            map.visible_to_source_with_affinity(second_line_start, SelectionAffinity::Downstream)
                .source_offset,
            source.find("Ada").unwrap_or(0)
        );
    }

    #[test]
    fn empty_table_row_boundary_maps_to_start_of_next_visible_line() {
        let source = "| Name | Role |\n| --- | --- |\n|  |  |";
        let doc = DocumentBuffer::from_text(source);
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        let empty_row_cell_start = source.rfind("|  |  |").unwrap_or(0) + 1;
        assert_eq!(
            map.source_to_visible_with_affinity(
                empty_row_cell_start,
                SelectionAffinity::Downstream
            ),
            map.visible_text.find('\n').unwrap_or(0) + 1
        );
        assert!(map.visible_text.ends_with(' '));
    }

    #[test]
    fn escaped_pipes_inside_table_cells_render_without_backslashes() {
        let source = "| Name | Note |\n| --- | --- |\n| Ada | a\\|b |";
        let doc = DocumentBuffer::from_text(source);
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert!(map.visible_text.contains("a|b"));
        assert!(!map.visible_text.contains("\\|"));
    }

    #[test]
    fn generic_list_prefix_is_hidden_until_cursor_reaches_marker_boundary() {
        let doc = DocumentBuffer::from_text("- item");
        let hidden = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert_eq!(hidden.visible_text, "item");

        let revealed = DisplayMap::from_document(
            &doc,
            Some(&revealed_boundary_selection(2)),
            HiddenSyntaxPolicy::SelectionAware,
        );
        assert_eq!(revealed.visible_text, "- item");
        assert_eq!(
            revealed
                .source_selection_to_visible(&revealed_boundary_selection(2))
                .cursor(),
            2
        );
    }

    #[test]
    fn blockquote_prefix_is_hidden_until_cursor_reaches_marker_boundary() {
        let doc = DocumentBuffer::from_text("> quote");
        let hidden = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert_eq!(hidden.visible_text, "quote");

        let revealed = DisplayMap::from_document(
            &doc,
            Some(&revealed_boundary_selection(2)),
            HiddenSyntaxPolicy::SelectionAware,
        );
        assert_eq!(revealed.visible_text, "> quote");
        assert_eq!(
            revealed
                .source_selection_to_visible(&revealed_boundary_selection(2))
                .cursor(),
            2
        );
    }

    #[test]
    fn link_and_inline_code_hide_markup_but_keep_visible_content() {
        let doc = DocumentBuffer::from_text("[docs](https://example.com) and `code`");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(map.visible_text, "docs and code");
        assert!(
            map.blocks[0]
                .spans
                .iter()
                .any(|span| span.hidden && span.source_text == "(")
        );
        assert!(
            map.blocks[0]
                .spans
                .iter()
                .any(|span| span.hidden && span.source_text == "`")
        );
    }

    #[test]
    fn link_markup_is_revealed_when_cursor_is_inside_link_text() {
        let doc = DocumentBuffer::from_text("[官网](https://box86.org/)");
        let hidden = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);
        assert_eq!(hidden.visible_text, "官网");

        let selection = SelectionModel {
            anchor_byte: 4,
            head_byte: 4,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        };
        let revealed =
            DisplayMap::from_document(&doc, Some(&selection), HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(revealed.visible_text, "[官网](https://box86.org/)");
        assert_eq!(revealed.source_selection_to_visible(&selection).cursor(), 4);
    }

    #[test]
    fn inter_block_virtual_newline_maps_to_adjacent_source_boundary() {
        let doc = DocumentBuffer::from_text("First\n\nSecond");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        assert_eq!(map.visible_text, "First\n\n\nSecond");
        assert_eq!(
            map.visible_to_source_with_affinity(7, SelectionAffinity::Upstream)
                .source_offset,
            7
        );
        assert_eq!(
            map.visible_to_source_with_affinity(7, SelectionAffinity::Downstream)
                .source_offset,
            7
        );
    }

    #[test]
    fn downstream_mapping_prefers_next_block_start_after_extra_blank_separator() {
        let doc = DocumentBuffer::from_text("A\n\n\nB");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        let next_block = map
            .blocks
            .iter()
            .find(|block| block.visible_text == "B")
            .expect("next paragraph block");
        assert_eq!(
            map.source_to_visible_with_affinity(4, SelectionAffinity::Downstream),
            next_block.visible_range.start
        );
    }

    #[test]
    fn downstream_mapping_prefers_next_block_start_after_standard_separator() {
        let doc = DocumentBuffer::from_text("12\n\n34");
        let map = DisplayMap::from_document(&doc, None, HiddenSyntaxPolicy::SelectionAware);

        let next_block = map
            .blocks
            .iter()
            .find(|block| block.visible_text == "34")
            .expect("next paragraph block");
        assert_eq!(
            map.source_to_visible_with_affinity(
                next_block.content_range.start,
                SelectionAffinity::Downstream
            ),
            next_block.visible_range.start
        );
    }
}

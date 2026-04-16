use std::ops::Range;

use super::{
    document::{BlockKind, BlockProjection, DocumentBuffer, SelectionAffinity, SelectionModel},
    syntax::InlineStyle,
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
            for span in &block.spans {
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

                    if span.hidden {
                        return span.visible_range.start;
                    }

                    let relative = source_offset.saturating_sub(span.source_range.start);
                    let mapped = if affinity == SelectionAffinity::Upstream
                        && source_offset == span.source_range.start
                    {
                        span.visible_range.start
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
            BlockKind::CodeFence { language } => Some(EmbeddedNodeKind::CodeBlock {
                language: language.clone(),
            }),
            BlockKind::Table => Some(EmbeddedNodeKind::Table),
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

    fn push_inline_text(&mut self, source_start: usize, text: &str, style: RenderInlineStyle) {
        for token in parse_inline_tokens(text) {
            let range =
                source_start + token.local_range.start..source_start + token.local_range.end;
            let hidden = token.hidden && !self.should_reveal(&range);
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
            );
        }
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
    source_text: String,
    visible_text: String,
    hidden: bool,
    style: RenderInlineStyle,
}

fn parse_inline_tokens(text: &str) -> Vec<InlineToken> {
    let mut tokens = Vec::new();
    parse_inline_tokens_into(text, 0, RenderInlineStyle::default(), &mut tokens);
    if tokens.is_empty() {
        tokens.push(InlineToken {
            local_range: 0..text.len(),
            source_text: text.to_string(),
            visible_text: text.to_string(),
            hidden: false,
            style: RenderInlineStyle::default(),
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

        if rest.starts_with('[')
            && let Some(close) = rest.find(']')
            && rest[close + 1..].starts_with('(')
            && let Some(close_paren) = rest[close + 2..].find(')')
        {
            push_hidden_marker(tokens, base_offset + offset, "[");
            let inner_start = offset + 1;
            let inner_end = offset + close;
            parse_inline_tokens_into(
                &text[inner_start..inner_end],
                base_offset + inner_start,
                RenderInlineStyle {
                    link: true,
                    ..style
                },
                tokens,
            );
            push_hidden_marker(tokens, base_offset + inner_end, "]");
            push_hidden_marker(tokens, base_offset + inner_end + 1, "(");
            let target_start = inner_end + 2;
            let target_end = target_start + close_paren;
            push_hidden_marker(
                tokens,
                base_offset + target_start,
                &text[target_start..target_end],
            );
            push_hidden_marker(tokens, base_offset + target_end, ")");
            offset = target_end + 1;
            continue;
        }

        let next_special = rest
            .char_indices()
            .skip(1)
            .find(|(_, ch)| matches!(ch, '*' | '_' | '~' | '`' | '['))
            .map(|(idx, _)| idx)
            .unwrap_or(rest.len());
        push_text_token(tokens, base_offset + offset, &rest[..next_special], style);
        offset += next_special;
    }
}

fn push_hidden_marker(tokens: &mut Vec<InlineToken>, offset: usize, marker: &str) {
    tokens.push(InlineToken {
        local_range: offset..offset + marker.len(),
        source_text: marker.to_string(),
        visible_text: String::new(),
        hidden: true,
        style: RenderInlineStyle::default(),
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
        source_text: text.to_string(),
        visible_text: text.to_string(),
        hidden: false,
        style,
    });
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

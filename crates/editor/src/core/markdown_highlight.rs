use tree_sitter::{Node, Parser};

use super::code_highlight::{CodeHighlightSpan, CodeTokenType};

pub fn highlight_markdown_source(text: &str) -> Vec<CodeHighlightSpan> {
    let mut block_parser = Parser::new();
    block_parser
        .set_language(&tree_sitter_md::LANGUAGE.into())
        .expect("failed to load markdown block grammar");
    let Some(block_tree) = block_parser.parse(text, None) else {
        return vec![CodeHighlightSpan {
            start: 0,
            end: text.len(),
            token_type: CodeTokenType::Default,
        }];
    };

    let mut inline_parser = Parser::new();
    inline_parser
        .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
        .expect("failed to load markdown inline grammar");

    let mut spans = Vec::new();
    let block_root = block_tree.root_node();
    collect_block_spans(&block_root, &mut inline_parser, text, &mut spans);

    if spans.is_empty() {
        spans.push(CodeHighlightSpan {
            start: 0,
            end: text.len(),
            token_type: CodeTokenType::Default,
        });
    } else {
        fill_gaps(&mut spans, text.len());
    }

    spans
}

fn collect_block_spans(
    node: &Node,
    inline_parser: &mut Parser,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    if !node.is_named() {
        return;
    }

    match node.kind() {
        "document" | "section" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_block_spans(&child, inline_parser, text, spans);
            }
        }
        "atx_heading" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "atx_h1_marker"
                    | "atx_h2_marker"
                    | "atx_h3_marker"
                    | "atx_h4_marker"
                    | "atx_h5_marker"
                    | "atx_h6_marker" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                    }
                    "inline" => {
                        collect_inline_spans_for_node(&child, inline_parser, text, spans);
                    }
                    _ => {}
                }
            }
        }
        "setext_heading" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "paragraph" => {
                        let mut pc = child.walk();
                        for gc in child.children(&mut pc) {
                            if gc.kind() == "inline" {
                                collect_inline_spans_for_node(&gc, inline_parser, text, spans);
                            }
                        }
                    }
                    "setext_h1_underline" | "setext_h2_underline" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                    }
                    _ => {}
                }
            }
        }
        "block_quote" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "block_quote_marker" | "block_continuation" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                    }
                    _ => {
                        collect_block_spans(&child, inline_parser, text, spans);
                    }
                }
            }
        }
        "list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "list_item" {
                    collect_list_item_spans(&child, inline_parser, text, spans);
                } else {
                    collect_block_spans(&child, inline_parser, text, spans);
                }
            }
        }
        "fenced_code_block" => {
            let mut cursor = node.walk();
            let mut code_content_start = None;
            let mut code_content_end = None;
            let mut language = None;

            for child in node.children(&mut cursor) {
                match child.kind() {
                    "fenced_code_block_delimiter" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                    }
                    "info_string" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Type);
                        language = extract_language(child, text);
                    }
                    "code_fence_content" => {
                        code_content_start = Some(child.start_byte());
                        code_content_end = Some(child.end_byte());
                    }
                    "language" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Type);
                    }
                    _ => {}
                }
            }

            if let (Some(start), Some(end)) = (code_content_start, code_content_end) {
                let code_text = &text[start..end];
                let highlighted = language.as_deref().and_then(|lang| {
                    static HIGHLIGHTER: std::sync::OnceLock<super::code_highlight::CodeHighlighter> =
                        std::sync::OnceLock::new();
                    let highlighter = HIGHLIGHTER.get_or_init(super::code_highlight::CodeHighlighter::new);
                    highlighter.highlight(lang, code_text)
                });

                if let Some(result) = highlighted {
                    for span in &result.spans {
                        push_span(spans, start + span.start, start + span.end, span.token_type);
                    }
                }
            }
        }
        "indented_code_block" => {
            push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::String);
        }
        "thematic_break" => {
            push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Keyword);
        }
        "html_block" => {
            push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Tag);
        }
        "minus_metadata" | "plus_metadata" => {
            push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Keyword);
        }
        "pipe_table" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "pipe_table_header" | "pipe_table_row" => {
                        let mut rc = child.walk();
                        for gc in child.children(&mut rc) {
                            if gc.kind() == "|" {
                                push_span(spans, gc.start_byte(), gc.end_byte(), CodeTokenType::Punctuation);
                            } else if gc.kind() == "pipe_table_cell" {
                                collect_inline_spans_for_node(&gc, inline_parser, text, spans);
                            }
                        }
                    }
                    "pipe_table_delimiter_row" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Operator);
                    }
                    _ => {}
                }
            }
        }
        "paragraph" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "inline" {
                    collect_inline_spans_for_node(&child, inline_parser, text, spans);
                }
            }
        }
        "link_reference_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "link_label" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Function);
                    }
                    "link_destination" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::String);
                    }
                    "link_title" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::String);
                    }
                    "[" | "]" | ":" | "(" | ")" => {
                        push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Punctuation);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn collect_list_item_spans(
    node: &Node,
    inline_parser: &mut Parser,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "list_marker_plus" | "list_marker_minus" | "list_marker_star" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
            }
            "list_marker_dot" | "list_marker_parenthesis" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Number);
            }
            "task_list_marker_checked" | "task_list_marker_unchecked" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Attribute);
            }
            _ => {
                collect_block_spans(&child, inline_parser, text, spans);
            }
        }
    }
}

fn collect_inline_spans_for_node(
    inline_node: &Node,
    inline_parser: &mut Parser,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let inline_text = &text[inline_node.start_byte()..inline_node.end_byte()];
    let inline_tree = inline_parser.parse(inline_text, None);
    if let Some(tree) = inline_tree {
        let root = tree.root_node();
        walk_inline(&root, inline_node.start_byte(), spans);
    }
}

fn walk_inline(node: &Node, offset: usize, spans: &mut Vec<CodeHighlightSpan>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.child_count() > 0 && child.is_named() {
            walk_inline(&child, offset, spans);
        } else if child.is_named() || !child.is_extra() {
            let token_type = classify_inline_node(&child);
            if token_type != CodeTokenType::Default || child.child_count() == 0 {
                push_span(
                    spans,
                    offset + child.start_byte(),
                    offset + child.end_byte(),
                    token_type,
                );
            }
        }
    }
}

fn classify_inline_node(node: &Node) -> CodeTokenType {
    match node.kind() {
        "emphasis_delimiter" | "code_span_delimiter" => CodeTokenType::Operator,
        "strong_emphasis" | "emphasis" => CodeTokenType::Default,
        "code_span" => CodeTokenType::String,
        "link_destination" | "uri_autolink" => CodeTokenType::String,
        "link_label" | "link_text" | "image_description" => CodeTokenType::Function,
        "link_title" => CodeTokenType::String,
        "backslash_escape" | "hard_line_break" => CodeTokenType::Escape,
        "shortcut_link" | "inline_link" | "image" | "email_autolink"
        | "full_reference_link" | "collapsed_reference_link" => CodeTokenType::Default,
        "[" | "]" | "(" | ")" | "!" => CodeTokenType::Punctuation,
        _ => CodeTokenType::Default,
    }
}

fn extract_language(info_string_node: Node, text: &str) -> Option<String> {
    let mut cursor = info_string_node.walk();
    for child in info_string_node.named_children(&mut cursor) {
        if child.kind() == "language" {
            let lang_text = &text[child.start_byte()..child.end_byte()];
            let trimmed = lang_text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    let text_content = &text[info_string_node.start_byte()..info_string_node.end_byte()];
    let trimmed = text_content.trim();
    if let Some(lang) = trimmed.split_whitespace().next().filter(|s| !s.is_empty()) {
        return Some(lang.to_string());
    }
    None
}

fn push_span(spans: &mut Vec<CodeHighlightSpan>, start: usize, end: usize, token_type: CodeTokenType) {
    if start >= end {
        return;
    }
    spans.push(CodeHighlightSpan {
        start,
        end,
        token_type,
    });
}

fn fill_gaps(spans: &mut Vec<CodeHighlightSpan>, total_len: usize) {
    if spans.is_empty() {
        return;
    }

    spans.sort_by_key(|s| s.start);

    let mut filled = Vec::new();
    let mut pos = 0;

    for span in spans.iter() {
        if span.start > pos {
            filled.push(CodeHighlightSpan {
                start: pos,
                end: span.start,
                token_type: CodeTokenType::Default,
            });
        }
        if span.start < pos {
            continue;
        }
        filled.push(span.clone());
        pos = span.end;
    }

    if pos < total_len {
        filled.push(CodeHighlightSpan {
            start: pos,
            end: total_len,
            token_type: CodeTokenType::Default,
        });
    }

    *spans = filled;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_heading_marker() {
        let spans = highlight_markdown_source("# Hello\n");
        let keyword_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.token_type == CodeTokenType::Keyword)
            .collect();
        assert!(!keyword_spans.is_empty());
    }

    #[test]
    fn highlights_code_fence_delimiter() {
        let spans = highlight_markdown_source("```rust\nfn main() {}\n```\n");
        let keyword_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.token_type == CodeTokenType::Keyword)
            .collect();
        assert!(keyword_spans.len() >= 2);
    }

    #[test]
    fn highlights_list_markers() {
        let spans = highlight_markdown_source("- item\n- another\n");
        let keyword_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.token_type == CodeTokenType::Keyword)
            .collect();
        assert!(!keyword_spans.is_empty());
    }

    #[test]
    fn highlights_thematic_break() {
        let spans = highlight_markdown_source("---\n");
        let keyword_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.token_type == CodeTokenType::Keyword)
            .collect();
        assert!(!keyword_spans.is_empty());
    }

    #[test]
    fn fills_gaps_with_default() {
        let spans = highlight_markdown_source("hello world\n");
        let total: usize = spans.iter().map(|s| s.end - s.start).sum();
        assert_eq!(total, "hello world\n".len());
    }
}

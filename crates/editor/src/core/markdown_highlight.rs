use tree_sitter::{Parser, TreeCursor};

use super::code_highlight::{CodeHighlightSpan, CodeHighlighter, CodeTokenType};

pub fn highlight_markdown_source(text: &str) -> Vec<CodeHighlightSpan> {
    let mut spans = Vec::new();
    if text.is_empty() {
        return spans;
    }

    let tree = parse_markdown(text);
    let root = tree.root_node();
    let mut cursor = root.walk();
    walk_block_nodes(&mut cursor, text, &mut spans);

    if spans.is_empty() {
        spans.push(CodeHighlightSpan {
            start: 0,
            end: text.len(),
            token_type: CodeTokenType::Default,
        });
    } else {
        CodeHighlighter::fill_gaps(&mut spans, text.len());
    }

    spans
}

fn parse_markdown(text: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_md::LANGUAGE.into())
        .expect("failed to load markdown grammar");
    parser
        .parse(text, None)
        .expect("markdown parse should succeed")
}

fn walk_block_nodes(cursor: &mut TreeCursor, text: &str, spans: &mut Vec<CodeHighlightSpan>) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        match kind {
            "document" | "section" => {
                if cursor.goto_first_child() {
                    loop {
                        walk_block_nodes(cursor, text, spans);
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }
            }
            "atx_heading" => highlight_atx_heading(cursor, text, spans),
            "setext_heading" => highlight_setext_heading(cursor, text, spans),
            "fenced_code_block" => highlight_fenced_code_block(cursor, text, spans),
            "block_quote" => highlight_block_quote(cursor, text, spans),
            "list" => highlight_list(cursor, text, spans),
            "thematic_break" => {
                push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Keyword);
            }
            "html_block" => {
                push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Tag);
            }
            "minus_metadata" | "plus_metadata" => {
                push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::Keyword);
            }
            "link_reference_definition" => {
                highlight_link_reference_definition(cursor, text, spans);
            }
            "paragraph" => highlight_inline_content(cursor, text, spans),
            "indented_code_block" => highlight_indented_code_block(cursor, text, spans),
            _ => {
                if cursor.goto_first_child() {
                    loop {
                        walk_block_nodes(cursor, text, spans);
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }
            }
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn highlight_atx_heading(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "atx_h1_marker"
                | "atx_h2_marker"
                | "atx_h3_marker"
                | "atx_h4_marker"
                | "atx_h5_marker"
                | "atx_h6_marker" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                    highlight_trailing_spaces_after_marker(child, text, spans);
                }
                "inline" => {
                    highlight_inline_with_inline_grammar(child, text, spans);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    fill_uncovered(node, spans);
}

fn highlight_setext_heading(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "setext_h1_underline" | "setext_h2_underline" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                }
                "paragraph" => {
                    highlight_inline_content(cursor, text, spans);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    fill_uncovered(node, spans);
}

fn highlight_fenced_code_block(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    let mut code_content_start = None;
    let mut code_content_end = None;
    let mut language = None;

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "fenced_code_block_delimiter" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                }
                "info_string" => {
                    let lang_node = find_named_child(child, "language");
                    if let Some(lang_node) = lang_node {
                        language = Some(
                            text[lang_node.start_byte()..lang_node.end_byte()].to_string(),
                        );
                        push_span(
                            spans,
                            lang_node.start_byte(),
                            lang_node.end_byte(),
                            CodeTokenType::Type,
                        );
                    } else {
                        push_span(
                            spans,
                            child.start_byte(),
                            child.end_byte(),
                            CodeTokenType::Type,
                        );
                    }
                }
                "code_fence_content" => {
                    code_content_start = Some(child.start_byte());
                    code_content_end = Some(child.end_byte());
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    if let (Some(start), Some(end)) = (code_content_start, code_content_end) {
        let content_text = &text[start..end];

        let (code_only_end, closing_delim_start) = find_closing_fence_delimiter(content_text);

        let code_text = &content_text[..code_only_end];

        if let Some(lang) = language.as_deref() {
            static HIGHLIGHTER: std::sync::OnceLock<CodeHighlighter> = std::sync::OnceLock::new();
            let highlighter = HIGHLIGHTER.get_or_init(CodeHighlighter::new);
            if let Some(result) = highlighter.highlight(lang, code_text) {
                for span in &result.spans {
                    push_span(
                        spans,
                        start + span.start,
                        start + span.end,
                        span.token_type,
                    );
                }
            } else {
                push_span(spans, start, start + code_only_end, CodeTokenType::String);
            }
        } else {
            push_span(spans, start, start + code_only_end, CodeTokenType::String);
        }

        if let Some(delim_start) = closing_delim_start {
            if delim_start > code_only_end {
                push_span(
                    spans,
                    start + code_only_end,
                    start + delim_start,
                    CodeTokenType::Default,
                );
            }
            push_span(
                spans,
                start + delim_start,
                end,
                CodeTokenType::Keyword,
            );
        } else {
            if code_only_end < content_text.len() {
                push_span(
                    spans,
                    start + code_only_end,
                    end,
                    CodeTokenType::Default,
                );
            }
        }
    }

    fill_uncovered(node, spans);
}

fn find_closing_fence_delimiter(content: &str) -> (usize, Option<usize>) {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return (0, None);
    }

    for (i, line) in lines.iter().enumerate().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let code_end: usize = lines[..i].iter().map(|l| l.len() + 1).sum();
            let delim_start = code_end + line.len() - trimmed.len();
            return (code_end, Some(delim_start));
        }
        if !trimmed.is_empty() {
            break;
        }
    }

    (content.len(), None)
}

fn highlight_block_quote(cursor: &mut TreeCursor, text: &str, spans: &mut Vec<CodeHighlightSpan>) {
    let node = cursor.node();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "block_quote_marker" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                }
                "block_continuation" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                }
                _ => {
                    walk_block_nodes(cursor, text, spans);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    fill_uncovered(node, spans);
}

fn highlight_list(cursor: &mut TreeCursor, text: &str, spans: &mut Vec<CodeHighlightSpan>) {
    let node = cursor.node();
    highlight_list_items(cursor, text, spans);
    fill_uncovered(node, spans);
}

fn highlight_list_items(cursor: &mut TreeCursor, text: &str, spans: &mut Vec<CodeHighlightSpan>) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        if child.kind() == "list_item" {
            highlight_single_list_item(cursor, text, spans);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn highlight_single_list_item(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        match child.kind() {
            "list_marker_dot" | "list_marker_minus" | "list_marker_plus"
            | "list_marker_star" | "list_marker_parenthesis" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
                highlight_trailing_spaces_after_marker(child, text, spans);
            }
            "task_list_marker_checked" | "task_list_marker_unchecked" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Attribute);
            }
            "block_continuation" => {
                push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Keyword);
            }
            "paragraph" => {
                highlight_inline_content(cursor, text, spans);
            }
            "list" => {
                highlight_list_items(cursor, text, spans);
            }
            "block_quote" => {
                highlight_block_quote(cursor, text, spans);
            }
            _ => {
                walk_block_nodes(cursor, text, spans);
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn highlight_indented_code_block(
    cursor: &mut TreeCursor,
    _text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    push_span(spans, node.start_byte(), node.end_byte(), CodeTokenType::String);
}

fn highlight_link_reference_definition(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "[" | "]" | ":" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Operator);
                }
                "link_label" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::Function);
                }
                "link_destination" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::String);
                }
                "link_title" => {
                    push_span(spans, child.start_byte(), child.end_byte(), CodeTokenType::String);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    fill_uncovered(node, spans);
}

fn highlight_inline_content(
    cursor: &mut TreeCursor,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        if child.kind() == "inline" {
            highlight_inline_with_inline_grammar(child, text, spans);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn highlight_inline_with_inline_grammar(
    inline_node: tree_sitter::Node,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let start = inline_node.start_byte();
    let end = inline_node.end_byte();
    let inline_text = &text[start..end];

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
        .expect("failed to load markdown inline grammar");

    let mut included_ranges = vec![tree_sitter::Range {
        start_byte: 0,
        end_byte: inline_text.len(),
        start_point: tree_sitter::Point::new(0, 0),
        end_point: tree_sitter::Point::new(
            inline_text.lines().count().saturating_sub(1),
            inline_text.lines().last().map(|l| l.len()).unwrap_or(0),
        ),
    }];
    parser.set_included_ranges(&included_ranges).ok();

    if let Some(tree) = parser.parse(inline_text, None) {
        let root = tree.root_node();
        let mut cursor = root.walk();
        walk_inline_tree(&mut cursor, inline_text, start, spans);
    }
}

fn walk_inline_tree(
    cursor: &mut TreeCursor,
    inline_text: &str,
    offset: usize,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        match kind {
            "document" | "inline" => {
                if cursor.goto_first_child() {
                    loop {
                        walk_inline_tree(cursor, inline_text, offset, spans);
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }
            }
            "code_span" => highlight_inline_code_span(cursor, inline_text, offset, spans),
            "emphasis_delimiter" | "strong_emphasis_delimiter" => {
                push_span(spans, offset + node.start_byte(), offset + node.end_byte(), CodeTokenType::Operator);
            }
            "emphasis" | "strong_emphasis" => {
                if cursor.goto_first_child() {
                    loop {
                        walk_inline_tree(cursor, inline_text, offset, spans);
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }
            }
            "shortcut_link" | "link" | "full_reference_link" | "collapsed_reference_link" => {
                highlight_inline_link(cursor, inline_text, offset, spans);
            }
            "image" => highlight_inline_image(cursor, inline_text, offset, spans),
            "strikethrough" => {
                highlight_inline_delimited(cursor, inline_text, offset, spans, "~~", CodeTokenType::Operator);
            }
            "email_autolink" | "autolink" => {
                push_span(spans, offset + node.start_byte(), offset + node.end_byte(), CodeTokenType::String);
            }
            "backslash_escape" => {
                push_span(spans, offset + node.start_byte(), offset + node.end_byte(), CodeTokenType::Escape);
            }
            "[" | "]" | "(" | ")" | "!" => {
                push_span(spans, offset + node.start_byte(), offset + node.end_byte(), CodeTokenType::Operator);
            }
            _ => {}
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn highlight_inline_code_span(
    cursor: &mut TreeCursor,
    _inline_text: &str,
    offset: usize,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let node = cursor.node();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "`" {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::Punctuation);
            } else {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::String);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    } else {
        push_span(spans, offset + node.start_byte(), offset + node.end_byte(), CodeTokenType::String);
    }
}

fn highlight_inline_link(
    cursor: &mut TreeCursor,
    _inline_text: &str,
    offset: usize,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        match child.kind() {
            "[" | "]" | "(" | ")" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::Operator);
            }
            "link_text" | "link_label" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::Function);
            }
            "link_destination" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::String);
            }
            "link_title" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::String);
            }
            _ => {}
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn highlight_inline_image(
    cursor: &mut TreeCursor,
    _inline_text: &str,
    offset: usize,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        match child.kind() {
            "!" | "[" | "]" | "(" | ")" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::Operator);
            }
            "image_description" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::Function);
            }
            "link_destination" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::String);
            }
            "link_title" => {
                push_span(spans, offset + child.start_byte(), offset + child.end_byte(), CodeTokenType::String);
            }
            _ => {}
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn highlight_inline_delimited(
    cursor: &mut TreeCursor,
    inline_text: &str,
    offset: usize,
    spans: &mut Vec<CodeHighlightSpan>,
    _delimiter: &str,
    delimiter_type: CodeTokenType,
) {
    let node = cursor.node();
    let full_text = &inline_text[node.start_byte()..node.end_byte()];
    let delim_len = if full_text.starts_with("~~") { 2 } else { 1 };

    push_span(
        spans,
        offset + node.start_byte(),
        offset + node.start_byte() + delim_len,
        delimiter_type,
    );
    if full_text.len() > delim_len * 2 {
        push_span(
            spans,
            offset + node.start_byte() + delim_len,
            offset + node.end_byte() - delim_len,
            CodeTokenType::Default,
        );
    }
    if full_text.len() > delim_len {
        push_span(
            spans,
            offset + node.end_byte() - delim_len,
            offset + node.end_byte(),
            delimiter_type,
        );
    }
}

fn highlight_trailing_spaces_after_marker(
    marker_node: tree_sitter::Node,
    text: &str,
    spans: &mut Vec<CodeHighlightSpan>,
) {
    let after_marker = &text[marker_node.end_byte()..];
    let space_len = after_marker
        .bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count();
    if space_len > 0 {
        push_span(
            spans,
            marker_node.end_byte(),
            marker_node.end_byte() + space_len,
            CodeTokenType::Keyword,
        );
    }
}

fn find_named_child<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
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

fn fill_uncovered(node: tree_sitter::Node, spans: &mut Vec<CodeHighlightSpan>) {
    spans.sort_by_key(|s| s.start);
    let mut pos = node.start_byte();
    let mut gaps = Vec::new();
    for span in spans.iter() {
        if span.start > pos {
            gaps.push(CodeHighlightSpan {
                start: pos,
                end: span.start,
                token_type: CodeTokenType::Default,
            });
        }
        pos = pos.max(span.end);
    }
    if pos < node.end_byte() {
        gaps.push(CodeHighlightSpan {
            start: pos,
            end: node.end_byte(),
            token_type: CodeTokenType::Default,
        });
    }
    spans.extend(gaps);
    spans.sort_by_key(|s| s.start);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debug_print_tree(text: &str) {
        let tree = parse_markdown(text);
        eprintln!("=== TREE for {:?} ===", text);
        print_tree(tree.root_node(), text, 0);

        let spans = highlight_markdown_source(text);
        eprintln!("--- SPANS ---");
        for s in &spans {
            let content = &text[s.start..s.end].replace('\n', "\\n");
            eprintln!("  {:?} [{}..{}] {:?}", s.token_type, s.start, s.end, content);
        }
    }

    fn print_tree(node: tree_sitter::Node, text: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let start = node.start_byte();
        let end = node.end_byte();
        let snippet = if end - start > 30 {
            format!("{}...", &text[start..start + 30].replace('\n', "\\n"))
        } else {
            text[start..end].replace('\n', "\\n")
        };
        eprintln!(
            "{}{} [{}..{}] {:?}",
            indent,
            node.kind(),
            start,
            end,
            snippet
        );
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(child, text, depth + 1);
        }
    }

    #[test]
    fn highlights_heading_marker() {
        debug_print_tree("# Title\n");
        let spans = highlight_markdown_source("# Title\n");
        let marker_span = spans.iter().find(|s| s.token_type == CodeTokenType::Keyword);
        assert!(marker_span.is_some(), "should have a Keyword span for # marker");
    }

    #[test]
    fn highlights_code_fence_delimiter() {
        debug_print_tree("```rust\ncode\n```");
        let spans = highlight_markdown_source("```rust\ncode\n```");
        let keyword_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.token_type == CodeTokenType::Keyword)
            .collect();
        assert!(keyword_spans.len() >= 2, "should have Keyword spans for fence delimiters, got: {:?}", keyword_spans);
    }

    #[test]
    fn highlights_code_fence_language() {
        let spans = highlight_markdown_source("```rust\ncode\n```");
        let type_span = spans.iter().find(|s| s.token_type == CodeTokenType::Type);
        assert!(type_span.is_some(), "should have a Type span for language");
    }

    #[test]
    fn highlights_block_quote_marker() {
        let spans = highlight_markdown_source("> quote");
        let marker_span = spans.iter().find(|s| s.token_type == CodeTokenType::Keyword);
        assert!(marker_span.is_some(), "should have a Keyword span for > marker");
    }

    #[test]
    fn highlights_list_marker() {
        let spans = highlight_markdown_source("- item");
        let marker_span = spans.iter().find(|s| s.token_type == CodeTokenType::Keyword);
        assert!(marker_span.is_some(), "should have a Keyword span for list marker");
    }

    #[test]
    fn highlights_inline_code() {
        debug_print_tree("`code`");
        let spans = highlight_markdown_source("`code`");
        let string_span = spans.iter().find(|s| s.token_type == CodeTokenType::String);
        assert!(string_span.is_some(), "should have a String span for code content, got spans: {:?}", spans);
    }

    #[test]
    fn fills_entire_document() {
        let text = "# Title\n\nParagraph\n";
        let spans = highlight_markdown_source(text);
        let covered: usize = spans.iter().map(|s| s.end - s.start).sum();
        assert_eq!(covered, text.len(), "spans should cover entire document");
    }

    #[test]
    fn empty_text_returns_empty() {
        let spans = highlight_markdown_source("");
        assert!(spans.is_empty());
    }

    #[test]
    fn highlights_thematic_break() {
        let spans = highlight_markdown_source("---\n");
        let keyword_span = spans.iter().find(|s| s.token_type == CodeTokenType::Keyword);
        assert!(keyword_span.is_some(), "should have a Keyword span for thematic break");
    }

    #[test]
    fn highlights_task_marker() {
        let spans = highlight_markdown_source("- [ ] task\n");
        let attr_span = spans.iter().find(|s| s.token_type == CodeTokenType::Attribute);
        assert!(attr_span.is_some(), "should have an Attribute span for task marker");
    }
}

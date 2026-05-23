use std::collections::HashMap;

use anyhow::{Result, anyhow};
use markdown::{CompileOptions, Options, to_html_with_options};

pub(super) fn export_markdown_to_html(markdown: &str, title: &str) -> Result<String> {
    let headings = collect_headings(strip_front_matter(markdown));
    let prepared = prepare_typora_extensions(markdown);
    let options = markdown_options();
    let document_title = front_matter_title(markdown).unwrap_or(title);

    let body = to_html_with_options(&prepared, &options)
        .map_err(|err| anyhow!("failed to render markdown: {err:?}"))?;
    let body = add_heading_ids(&body, &headings);
    Ok(wrap_html_document(&body, document_title))
}

fn markdown_options() -> Options {
    let mut options = Options::gfm();
    options.compile = CompileOptions {
        allow_dangerous_html: true,
        allow_dangerous_protocol: true,
        allow_any_img_src: true,
        ..CompileOptions::gfm()
    };
    options
}

fn prepare_typora_extensions(markdown: &str) -> String {
    let markdown = strip_front_matter(markdown);
    let headings = collect_headings(markdown);
    let markdown = replace_toc(markdown, &headings);
    let markdown = replace_mermaid_fences(&markdown);
    let markdown = replace_typora_math_and_inline_markup(&markdown);
    replace_callouts(&markdown)
}

fn replace_mermaid_fences(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut mermaid_block = String::new();
    let mut in_mermaid_block = false;

    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let newline = &segment[line.len()..];
        if in_mermaid_block {
            if is_fence_close(line) {
                out.push_str(&render_mermaid_block(&mermaid_block));
                mermaid_block.clear();
                in_mermaid_block = false;
            } else {
                mermaid_block.push_str(line);
                mermaid_block.push_str(newline);
            }
            continue;
        }

        if is_mermaid_fence_open(line) {
            in_mermaid_block = true;
            continue;
        }

        out.push_str(segment);
    }

    if in_mermaid_block {
        out.push_str("```mermaid\n");
        out.push_str(&mermaid_block);
    }

    out
}

fn is_mermaid_fence_open(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed
        .strip_prefix("```")
        .or_else(|| trimmed.strip_prefix("~~~"))
    else {
        return false;
    };
    let info = rest.trim_start();
    let Some(rest) = info.strip_prefix("mermaid") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

fn is_fence_close(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn strip_front_matter(markdown: &str) -> &str {
    let Some(front_matter) = parse_front_matter(markdown) else {
        return markdown;
    };
    markdown[front_matter.end_offset..].trim_start_matches(['\r', '\n'])
}

fn front_matter_title(markdown: &str) -> Option<&str> {
    let front_matter = parse_front_matter(markdown)?;
    for line in front_matter.body.lines() {
        let line = line.trim();
        let value = if let Some(value) = line.strip_prefix("title:") {
            value
        } else if let Some(value) = line.strip_prefix("title") {
            let value = value.trim_start();
            let Some(value) = value.strip_prefix('=') else {
                continue;
            };
            value
        } else {
            continue;
        };
        let title = trim_quoted_scalar(value.trim());
        if !title.is_empty() {
            return Some(title);
        }
    }
    None
}

fn parse_front_matter(markdown: &str) -> Option<FrontMatter<'_>> {
    let marker = if markdown.starts_with("---\n") {
        "---"
    } else if markdown.starts_with("+++\n") {
        "+++"
    } else {
        return None;
    };

    let mut offset = 0usize;
    let mut body_start = 0usize;
    for (index, segment) in markdown.split_inclusive('\n').enumerate() {
        offset += segment.len();
        if index == 0 {
            body_start = offset;
            continue;
        }
        if segment.trim_end_matches(['\r', '\n']).trim() == marker {
            let body_end = offset - segment.len();
            return Some(FrontMatter {
                body: &markdown[body_start..body_end],
                end_offset: offset,
            });
        }
    }

    None
}

fn trim_quoted_scalar(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn replace_typora_math_and_inline_markup(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_fence = false;
    let mut math_block = String::new();
    let mut in_math_block = false;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let newline = &segment[line.len()..];
        let is_fence = line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~");
        if is_fence {
            out.push_str(segment);
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push_str(segment);
            continue;
        }
        if line.trim() == "$$" {
            if in_math_block {
                out.push_str(&render_math_block(&math_block));
                math_block.clear();
                in_math_block = false;
            } else {
                in_math_block = true;
            }
            continue;
        }
        if in_math_block {
            math_block.push_str(line);
            math_block.push_str(newline);
            continue;
        }
        out.push_str(&replace_inline_typora_markup_in_line(line));
        out.push_str(newline);
    }
    if in_math_block {
        out.push_str("$$\n");
        out.push_str(&math_block);
    }
    out
}

fn replace_inline_typora_markup_in_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;

    while index < line.len() {
        let rest = &line[index..];
        if rest.starts_with('`') {
            let len = code_span_len(rest).unwrap_or(1);
            out.push_str(&rest[..len]);
            index += len;
        } else if let Some((raw, html)) = parse_inline_math(rest) {
            out.push_str(&html);
            index += raw.len();
        } else if let Some((raw, html)) = parse_inline_extension(rest, "==", "mark") {
            out.push_str(&html);
            index += raw.len();
        } else if let Some((raw, html)) = parse_inline_extension(rest, "^", "sup") {
            out.push_str(&html);
            index += raw.len();
        } else if let Some((raw, html)) = parse_inline_extension(rest, "~", "sub") {
            out.push_str(&html);
            index += raw.len();
        } else if let Some(ch) = rest.chars().next() {
            out.push(ch);
            index += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

fn render_math_block(source: &str) -> String {
    let source = source.trim();
    format!(
        "<div class=\"math math-block\"><code>{}</code></div>\n",
        escape_html(source)
    )
}

fn render_mermaid_block(source: &str) -> String {
    let source = source.trim();
    format!("<pre class=\"mermaid\">{}</pre>\n", escape_html(source))
}

fn parse_inline_math(rest: &str) -> Option<(&str, String)> {
    if !rest.starts_with('$') || rest.starts_with("$$") {
        return None;
    }
    let after_open = &rest[1..];
    let close = after_open.find('$')?;
    if close == 0 {
        return None;
    }
    let inner = &after_open[..close];
    if inner.trim().is_empty() || inner.contains('\n') {
        return None;
    }
    let raw = &rest[..close + 2];
    Some((
        raw,
        format!(
            "<span class=\"math math-inline\"><code>{}</code></span>",
            escape_html(inner.trim())
        ),
    ))
}

fn code_span_len(rest: &str) -> Option<usize> {
    let ticks = rest.chars().take_while(|ch| *ch == '`').count();
    if ticks == 0 {
        return None;
    }
    let delimiter = "`".repeat(ticks);
    let after_open = &rest[delimiter.len()..];
    let close = after_open.find(&delimiter)?;
    Some(delimiter.len() + close + delimiter.len())
}

fn parse_inline_extension<'a>(rest: &'a str, delimiter: &str, tag: &str) -> Option<(&'a str, String)> {
    if !rest.starts_with(delimiter) || rest[delimiter.len()..].starts_with(delimiter) {
        return None;
    }
    let after_open = &rest[delimiter.len()..];
    let close = after_open.find(delimiter)?;
    if close == 0 {
        return None;
    }
    let inner = &after_open[..close];
    if inner.chars().any(char::is_whitespace) && delimiter != "==" {
        return None;
    }
    let raw_end = delimiter.len() + close + delimiter.len();
    let raw = &rest[..raw_end];
    Some((raw, format!("<{tag}>{}</{tag}>", escape_html(inner))))
}

fn collect_headings(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut slug_counts = HashMap::new();
    let mut previous_line: Option<&str> = None;
    let mut in_fence = false;

    for line in markdown.lines() {
        if is_fence_close(line) {
            in_fence = !in_fence;
            previous_line = None;
            continue;
        }

        if in_fence {
            continue;
        }

        if let Some((level, title)) = parse_atx_heading(line) {
            push_heading(&mut headings, &mut slug_counts, level, title);
            previous_line = None;
        } else if let Some(level) = parse_setext_heading_marker(line) {
            if let Some(title) = previous_line.and_then(parse_setext_heading_title) {
                push_heading(&mut headings, &mut slug_counts, level, title);
            }
            previous_line = None;
        } else {
            previous_line = Some(line);
        };
    }
    headings
}

fn push_heading(
    headings: &mut Vec<Heading>,
    slug_counts: &mut HashMap<String, usize>,
    level: usize,
    title: &str,
) {
    let visible_title = heading_visible_text(title);
    headings.push(Heading {
        level,
        title: visible_title.clone(),
        slug: unique_slug(&visible_title, slug_counts),
    });
}

fn heading_visible_text(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut index = 0usize;

    while index < title.len() {
        let rest = &title[index..];
        if rest.starts_with('`') {
            let ticks = rest.chars().take_while(|ch| *ch == '`').count();
            let delimiter = "`".repeat(ticks);
            let after_open = &rest[delimiter.len()..];
            if let Some(close) = after_open.find(&delimiter) {
                out.push_str(&after_open[..close]);
                index += delimiter.len() + close + delimiter.len();
            } else {
                index += delimiter.len();
            }
        } else if let Some((raw, visible)) = parse_image_visible_text(rest) {
            out.push_str(visible);
            index += raw.len();
        } else if let Some((raw, visible)) = parse_link_visible_text(rest) {
            out.push_str(visible);
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "**") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "__") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "~~") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "==") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "*") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "_") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "^") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if let Some((raw, visible)) = parse_delimited_visible_text(rest, "~") {
            out.push_str(&heading_visible_text(visible));
            index += raw.len();
        } else if rest.starts_with('\\') {
            let after_escape = &rest[1..];
            if let Some(ch) = after_escape.chars().next() {
                out.push(ch);
                index += 1 + ch.len_utf8();
            } else {
                index += 1;
            }
        } else if let Some(ch) = rest.chars().next() {
            out.push(ch);
            index += ch.len_utf8();
        } else {
            break;
        }
    }

    out.trim().to_string()
}

fn parse_image_visible_text(rest: &str) -> Option<(&str, &str)> {
    let after_open = rest.strip_prefix("![")?;
    let close_label = after_open.find(']')?;
    let label = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];
    let close_destination = after_label.strip_prefix('(')?.find(')')?;
    let raw_len = 2 + close_label + 1 + 1 + close_destination + 1;
    Some((&rest[..raw_len], label))
}

fn parse_link_visible_text(rest: &str) -> Option<(&str, &str)> {
    let after_open = rest.strip_prefix('[')?;
    let close_label = after_open.find(']')?;
    let label = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];
    let close_destination = after_label.strip_prefix('(')?.find(')')?;
    let raw_len = 1 + close_label + 1 + 1 + close_destination + 1;
    Some((&rest[..raw_len], label))
}

fn parse_delimited_visible_text<'a>(rest: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    if !rest.starts_with(delimiter) || rest[delimiter.len()..].starts_with(delimiter) {
        return None;
    }
    let after_open = &rest[delimiter.len()..];
    let close = after_open.find(delimiter)?;
    if close == 0 {
        return None;
    }
    let raw_end = delimiter.len() + close + delimiter.len();
    Some((&rest[..raw_end], &after_open[..close]))
}

fn unique_slug(title: &str, slug_counts: &mut HashMap<String, usize>) -> String {
    let base = {
        let slug = slugify(title);
        if slug.is_empty() {
            "section".to_string()
        } else {
            slug
        }
    };
    let count = slug_counts.entry(base.clone()).or_insert(0);
    let slug = if *count == 0 {
        base
    } else {
        format!("{base}-{count}")
    };
    *count += 1;
    slug
}

fn parse_atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let title = rest.trim().trim_end_matches('#').trim();
    if title.is_empty() {
        return None;
    }
    Some((level, title))
}

fn parse_setext_heading_marker(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().all(|ch| ch == '=') {
        Some(1)
    } else if trimmed.chars().all(|ch| ch == '-') {
        Some(2)
    } else {
        None
    }
}

fn parse_setext_heading_title(line: &str) -> Option<&str> {
    let title = line.trim();
    if title.is_empty()
        || parse_atx_heading(title).is_some()
        || parse_setext_heading_marker(title).is_some()
    {
        None
    } else {
        Some(title)
    }
}

fn replace_toc(markdown: &str, headings: &[Heading]) -> String {
    let toc = render_toc_markdown(headings);
    let mut out = String::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        if is_fence_close(line) {
            in_fence = !in_fence;
            out.push_str(line);
            out.push('\n');
        } else if !in_fence && line.trim().eq_ignore_ascii_case("[toc]") {
            out.push_str(&toc);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn render_toc_markdown(headings: &[Heading]) -> String {
    if headings.is_empty() {
        return String::new();
    }

    let mut toc = String::new();
    for heading in headings {
        let indent = "  ".repeat(heading.level.saturating_sub(1));
        toc.push_str(&format!(
            "{indent}- [{}](#{})\n",
            escape_markdown_link_text(&heading.title),
            heading.slug
        ));
    }
    toc
}

fn add_heading_ids(html: &str, headings: &[Heading]) -> String {
    if headings.is_empty() {
        return html.to_string();
    }

    let mut out = String::with_capacity(html.len() + headings.len() * 12);
    let mut rest = html;
    let mut heading_index = 0usize;

    while let Some(start) = rest.find("<h") {
        out.push_str(&rest[..start]);
        rest = &rest[start..];

        let Some(level) = rest.as_bytes().get(2).copied() else {
            out.push_str(rest);
            return out;
        };
        if !(b'1'..=b'6').contains(&level) || rest.as_bytes().get(3) != Some(&b'>') {
            out.push_str("<h");
            rest = &rest[2..];
            continue;
        }

        let close_tag = format!("</h{}>", level as char);
        let Some(close_start) = rest.find(&close_tag) else {
            out.push_str(rest);
            return out;
        };
        let heading_html = &rest[..close_start + close_tag.len()];
        let inner_html = &rest[4..close_start];

        if let Some(heading) = headings.get(heading_index) {
            if heading.level == (level - b'0') as usize
                && html_visible_text(inner_html) == heading.title
            {
                out.push_str(&format!(
                    "<h{} id=\"{}\">",
                    level as char,
                    escape_attr_value(&heading.slug)
                ));
                out.push_str(inner_html);
                out.push_str(&close_tag);
                rest = &rest[heading_html.len()..];
                heading_index += 1;
                continue;
            }
        }

        out.push_str(heading_html);
        rest = &rest[heading_html.len()..];
    }

    out.push_str(rest);
    out
}

fn html_visible_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut index = 0usize;

    while index < html.len() {
        let rest = &html[index..];
        if rest.starts_with("<img") {
            if let Some(close) = rest.find('>') {
                if let Some(alt) = html_attr_value(&rest[..=close], "alt") {
                    out.push_str(&decode_html_entities(&alt));
                }
                index += close + 1;
                continue;
            }
        }

        if rest.starts_with('<') {
            if let Some(close) = rest.find('>') {
                index += close + 1;
                continue;
            }
        }

        if rest.starts_with('&') {
            if let Some(close) = rest.find(';') {
                out.push_str(&decode_html_entities(&rest[..=close]));
                index += close + 1;
                continue;
            }
        }

        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            index += ch.len_utf8();
        } else {
            break;
        }
    }

    out.trim().to_string()
}

fn html_attr_value(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=");
    let start = tag.find(&pattern)? + pattern.len();
    let value = &tag[start..];
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &value[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn replace_callouts(markdown: &str) -> String {
    let mut out = String::new();
    let mut lines = markdown.lines().peekable();
    let mut in_fence = false;

    while let Some(line) = lines.next() {
        if is_fence_close(line) {
            in_fence = !in_fence;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        let Some(callout) = parse_callout_open(line) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };

        out.push_str(&format!(
            "<div class=\"callout callout-{}\">\n<p class=\"callout-title\">{}</p>\n",
            escape_attr(&callout.kind),
            escape_html(&callout.title)
        ));

        let mut body = String::new();
        while let Some(next) = lines.peek().copied() {
            if next.trim().is_empty() {
                lines.next();
                break;
            }
            let Some(content) = strip_blockquote_marker(next) else {
                break;
            };
            body.push_str(strip_one_leading_space(content));
            body.push('\n');
            lines.next();
        }

        out.push_str(&render_callout_body(&body));
        out.push_str("</div>\n");
    }

    out
}

fn render_callout_body(markdown: &str) -> String {
    let prepared = replace_mermaid_fences(markdown);
    let prepared = replace_typora_math_and_inline_markup(&prepared);
    to_html_with_options(&prepared, &markdown_options()).unwrap_or_else(|_| {
        let mut fallback = String::from("<p>");
        fallback.push_str(&escape_html(markdown.trim()));
        fallback.push_str("</p>\n");
        fallback
    })
}

fn parse_callout_open(line: &str) -> Option<CalloutOpen> {
    let text = strip_blockquote_marker(line)?.trim_start();
    let rest = text.strip_prefix("[!")?;
    let close = rest.find(']')?;
    let kind = rest[..close].trim().to_ascii_lowercase();
    if kind.is_empty() {
        return None;
    }
    let title = rest[close + 1..].trim();
    let title = if title.is_empty() {
        title_case(&kind)
    } else {
        title.to_string()
    };
    Some(CalloutOpen { kind, title })
}

fn strip_blockquote_marker(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix('>')
}

fn strip_one_leading_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch);
        } else if ch.is_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch);
        } else {
            pending_dash = true;
        }
    }
    slug
}

fn wrap_html_document(body: &str, title: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
<style>
:root {{
  color-scheme: light dark;
  --bg: #f7f6f2;
  --fg: #202124;
  --muted: #70757a;
  --rule: #d7d3ca;
  --accent: #2563eb;
  --code-bg: #ebe8df;
  --mark-bg: #fff3a3;
}}
@media (prefers-color-scheme: dark) {{
  :root {{
    --bg: #171717;
    --fg: #ece7dc;
    --muted: #aaa397;
    --rule: #38342d;
    --accent: #7aa2f7;
    --code-bg: #27231e;
    --mark-bg: #5f4f14;
  }}
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 17px/1.72 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}}
main {{
  width: min(860px, calc(100vw - 40px));
  margin: 56px auto 72px;
}}
h1, h2, h3, h4, h5, h6 {{ line-height: 1.25; margin: 1.8em 0 .65em; }}
p, ul, ol, blockquote, pre, table {{ margin: 1em 0; }}
a {{ color: var(--accent); }}
img {{ max-width: 100%; height: auto; }}
hr {{ border: 0; border-top: 1px solid var(--rule); margin: 2em 0; }}
blockquote {{
  border-left: 4px solid var(--rule);
  color: var(--muted);
  margin-left: 0;
  padding-left: 1em;
}}
pre, code {{
  background: var(--code-bg);
  border-radius: 5px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}}
code {{ padding: .12em .32em; }}
pre {{ overflow-x: auto; padding: 1em; }}
pre code {{ padding: 0; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid var(--rule); padding: .45em .65em; }}
th {{ background: color-mix(in srgb, var(--code-bg) 76%, transparent); }}
mark {{ background: var(--mark-bg); color: inherit; padding: .05em .16em; border-radius: 3px; }}
.math code {{ background: transparent; padding: 0; }}
.math-inline {{ font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif; white-space: nowrap; }}
.math-block {{
  display: block;
  overflow-x: auto;
  margin: 1.25em 0;
  padding: .85em 1em;
  text-align: center;
  background: color-mix(in srgb, var(--code-bg) 52%, transparent);
  border-radius: 6px;
}}
.mermaid {{
  overflow-x: auto;
  margin: 1.25em 0;
  padding: 1em;
  background: color-mix(in srgb, var(--code-bg) 52%, transparent);
  border: 1px solid var(--rule);
  border-radius: 6px;
}}
.contains-task-list {{ list-style: none; padding-left: 1.2em; }}
.task-list-item input {{ margin-right: .45em; }}
.footnotes {{ color: var(--muted); font-size: .92em; }}
.callout {{
  border-left: 4px solid var(--accent);
  background: color-mix(in srgb, var(--accent) 9%, transparent);
  border-radius: 6px;
  padding: .75em 1em;
}}
.callout-title {{ font-weight: 700; margin-top: 0; }}
</style>
</head>
<body>
<main>
{}
</main>
</body>
</html>
"#,
        escape_html(title),
        body
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_attr(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect()
}

fn escape_attr_value(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_markdown_link_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

struct Heading {
    level: usize,
    title: String,
    slug: String,
}

struct FrontMatter<'a> {
    body: &'a str,
    end_offset: usize,
}

struct CalloutOpen {
    kind: String,
    title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_full_html_document() {
        let html = export_markdown_to_html("# Hello", "Draft <One>").unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>Draft &lt;One&gt;</title>"));
        assert!(html.contains("<h1 id=\"hello\">Hello</h1>"));
    }

    #[test]
    fn exports_gfm_tables_tasks_and_footnotes() {
        let html = export_markdown_to_html(
            "- [x] done\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\nNote[^a]\n\n[^a]: footnote",
            "GFM",
        )
        .unwrap();
        assert!(html.contains("contains-task-list"));
        assert!(html.contains("<table>"));
        assert!(html.contains("data-footnotes"));
    }

    #[test]
    fn expands_toc_marker() {
        let html = export_markdown_to_html("[toc]\n\n# Intro\n\n## Details", "TOC").unwrap();
        assert!(html.contains("<a href=\"#intro\">Intro</a>"));
        assert!(html.contains("<a href=\"#details\">Details</a>"));
    }

    #[test]
    fn exported_toc_links_match_heading_ids() {
        let html = export_markdown_to_html("[toc]\n\n# Intro\n\n## Details", "TOC").unwrap();
        assert!(html.contains("<a href=\"#intro\">Intro</a>"));
        assert!(html.contains("<h1 id=\"intro\">Intro</h1>"));
        assert!(html.contains("<a href=\"#details\">Details</a>"));
        assert!(html.contains("<h2 id=\"details\">Details</h2>"));
    }

    #[test]
    fn duplicate_headings_get_unique_toc_links_and_ids() {
        let html = export_markdown_to_html(
            "[toc]\n\n# Intro\n\n## Intro\n\nIntro\n---",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<a href=\"#intro\">Intro</a>"));
        assert!(html.contains("<a href=\"#intro-1\">Intro</a>"));
        assert!(html.contains("<a href=\"#intro-2\">Intro</a>"));
        assert!(html.contains("<h1 id=\"intro\">Intro</h1>"));
        assert!(html.contains("<h2 id=\"intro-1\">Intro</h2>"));
        assert!(html.contains("<h2 id=\"intro-2\">Intro</h2>"));
    }

    #[test]
    fn heading_with_empty_slug_gets_section_fallback() {
        let html = export_markdown_to_html("[toc]\n\n# !!!", "TOC").unwrap();
        assert!(html.contains("<a href=\"#section\">!!!</a>"));
        assert!(html.contains("<h1 id=\"section\">!!!</h1>"));
    }

    #[test]
    fn toc_uses_visible_text_for_inline_markdown_headings() {
        let html = export_markdown_to_html(
            "[toc]\n\n# **Bold** [Link](https://example.com) `code` ==mark==",
            "TOC",
        )
        .unwrap();
        assert!(html.contains(
            "<a href=\"#bold-link-code-mark\">Bold Link code mark</a>"
        ));
        assert!(html.contains("<h1 id=\"bold-link-code-mark\"><strong>Bold</strong>"));
        assert!(html.contains("<a href=\"https://example.com\">Link</a>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<mark>mark</mark>"));
    }

    #[test]
    fn raw_html_headings_do_not_steal_markdown_heading_ids() {
        let html = export_markdown_to_html(
            "[toc]\n\n<h1>Raw</h1>\n\n# Markdown",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<h1>Raw</h1>"));
        assert!(html.contains("<a href=\"#markdown\">Markdown</a>"));
        assert!(html.contains("<h1 id=\"markdown\">Markdown</h1>"));
        assert!(!html.contains("<h1 id=\"markdown\">Raw</h1>"));
    }

    #[test]
    fn toc_uses_image_alt_text_for_heading_titles() {
        let html = export_markdown_to_html(
            "[toc]\n\n# ![Diagram Alt](diagram.png) Overview",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<a href=\"#diagram-alt-overview\">Diagram Alt Overview</a>"));
        assert!(html.contains("<h1 id=\"diagram-alt-overview\"><img src=\"diagram.png\" alt=\"Diagram Alt\" /> Overview</h1>"));
    }

    #[test]
    fn toc_includes_setext_headings() {
        let html = export_markdown_to_html(
            "[toc]\n\nTitle\n=\n\nSection\n---",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<a href=\"#title\">Title</a>"));
        assert!(html.contains("<a href=\"#section\">Section</a>"));
        assert!(html.contains("<h1 id=\"title\">Title</h1>"));
        assert!(html.contains("<h2 id=\"section\">Section</h2>"));
    }

    #[test]
    fn toc_ignores_headings_inside_code_fences() {
        let html = export_markdown_to_html(
            "[toc]\n\n```markdown\n# Hidden\n```\n\n# Visible",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<a href=\"#visible\">Visible</a>"));
        assert!(!html.contains("#hidden"));
    }

    #[test]
    fn toc_marker_inside_code_fence_stays_literal() {
        let html = export_markdown_to_html(
            "```markdown\n[toc]\n```\n\n# Visible",
            "TOC",
        )
        .unwrap();
        assert!(html.contains("<code class=\"language-markdown\">[toc]\n</code>"));
        assert!(!html.contains("<a href=\"#visible\">Visible</a>"));
    }

    #[test]
    fn exports_callouts_as_styled_blocks() {
        let html =
            export_markdown_to_html("> [!warning] Careful\n> Read this first.", "Callout").unwrap();
        assert!(html.contains("callout callout-warning"));
        assert!(html.contains("Careful"));
        assert!(html.contains("Read this first."));
    }

    #[test]
    fn callout_body_renders_nested_markdown() {
        let html = export_markdown_to_html(
            "> [!note] Rich\n> **Bold** [link](https://example.com)\n> - item",
            "Callout",
        )
        .unwrap();
        assert!(html.contains("<strong>Bold</strong>"));
        assert!(html.contains("<a href=\"https://example.com\">link</a>"));
        assert!(html.contains("<li>item</li>"));
    }

    #[test]
    fn callout_body_keeps_code_span_typora_markers_literal() {
        let html = export_markdown_to_html(
            "> [!tip] Code\n> `==literal==` and ==marked==",
            "Callout",
        )
        .unwrap();
        assert!(html.contains("<code>==literal==</code>"));
        assert!(html.contains("<mark>marked</mark>"));
    }

    #[test]
    fn callout_marker_inside_code_fence_stays_literal() {
        let html = export_markdown_to_html(
            "```markdown\n> [!note] Literal\n> Body\n```",
            "Callout",
        )
        .unwrap();
        assert!(html.contains("<code class=\"language-markdown\">&gt; [!note] Literal\n&gt; Body\n</code>"));
        assert!(!html.contains("callout callout-note"));
    }

    #[test]
    fn exports_typora_inline_extensions() {
        let html = export_markdown_to_html("==mark== H~2~O x^2^ `==code==`", "Inline").unwrap();
        assert!(html.contains("<mark>mark</mark>"));
        assert!(html.contains("H<sub>2</sub>O"));
        assert!(html.contains("x<sup>2</sup>"));
        assert!(html.contains("<code>==code==</code>"));
    }

    #[test]
    fn inline_extension_export_escapes_html() {
        let html = export_markdown_to_html("==<tag>== x^<2>^", "Inline").unwrap();
        assert!(html.contains("<mark>&lt;tag&gt;</mark>"));
        assert!(html.contains("<sup>&lt;2&gt;</sup>"));
    }

    #[test]
    fn exports_inline_and_block_math() {
        let html = export_markdown_to_html(
            "Inline $E = mc^2$\n\n$$\n\\int_0^1 x^2 dx\n$$",
            "Math",
        )
        .unwrap();
        assert!(html.contains("<span class=\"math math-inline\"><code>E = mc^2</code></span>"));
        assert!(html.contains("<div class=\"math math-block\"><code>\\int_0^1 x^2 dx</code></div>"));
    }

    #[test]
    fn math_export_skips_code_and_escapes_html() {
        let html = export_markdown_to_html("`$x$` and $<x>$", "Math").unwrap();
        assert!(html.contains("<code>$x$</code>"));
        assert!(html.contains("<span class=\"math math-inline\"><code>&lt;x&gt;</code></span>"));
    }

    #[test]
    fn exports_mermaid_fence_as_diagram_container() {
        let html = export_markdown_to_html(
            "```mermaid\ngraph TD\n  A --> B\n```",
            "Mermaid",
        )
        .unwrap();
        assert!(html.contains("<pre class=\"mermaid\">graph TD\n  A --&gt; B</pre>"));
        assert!(html.contains(".mermaid"));
    }

    #[test]
    fn mermaid_export_escapes_html_and_keeps_regular_code_fences() {
        let html = export_markdown_to_html(
            "```mermaid\ngraph TD\n  A[<tag>] --> B\n```\n\n```rust\nfn main() {}\n```",
            "Mermaid",
        )
        .unwrap();
        assert!(html.contains("A[&lt;tag&gt;] --&gt; B"));
        assert!(html.contains("<code class=\"language-rust\">fn main() {}\n</code>"));
    }

    #[test]
    fn export_strips_front_matter() {
        let html = export_markdown_to_html(
            "---\ntitle: Hidden\ntags: [draft]\n---\n\n# Visible",
            "Front Matter",
        )
        .unwrap();
        assert!(!html.contains("title: Hidden"));
        assert!(!html.contains("tags: [draft]"));
        assert!(html.contains("<h1 id=\"visible\">Visible</h1>"));
    }

    #[test]
    fn export_uses_front_matter_title_for_html_title() {
        let html = export_markdown_to_html(
            "---\ntitle: Front Matter Title\n---\n\n# Visible",
            "Fallback",
        )
        .unwrap();
        assert!(html.contains("<title>Front Matter Title</title>"));
    }

    #[test]
    fn export_front_matter_title_supports_quotes_and_fallback() {
        let quoted = export_markdown_to_html(
            "---\ntitle: \"Quoted Title\"\n---\n\n# Visible",
            "Fallback",
        )
        .unwrap();
        let empty = export_markdown_to_html("---\ntitle: \n---\n\n# Visible", "Fallback").unwrap();
        assert!(quoted.contains("<title>Quoted Title</title>"));
        assert!(empty.contains("<title>Fallback</title>"));
    }

    #[test]
    fn export_uses_toml_front_matter_title() {
        let html = export_markdown_to_html(
            "+++\ntitle = \"TOML Title\"\ntags = [\"draft\"]\n+++\n\n# Visible",
            "Fallback",
        )
        .unwrap();
        assert!(html.contains("<title>TOML Title</title>"));
        assert!(!html.contains("tags ="));
        assert!(html.contains("<h1 id=\"visible\">Visible</h1>"));
    }

    #[test]
    fn empty_toml_front_matter_title_falls_back() {
        let html = export_markdown_to_html("+++\ntitle = \"\"\n+++\n\n# Visible", "Fallback")
            .unwrap();
        assert!(html.contains("<title>Fallback</title>"));
    }

    #[test]
    fn front_matter_is_ignored_when_expanding_toc() {
        let html = export_markdown_to_html(
            "---\ntitle: Hidden\n---\n\n[toc]\n\n# Visible",
            "Front Matter",
        )
        .unwrap();
        assert!(html.contains("<a href=\"#visible\">Visible</a>"));
        assert!(!html.contains("Hidden</a>"));
    }
}

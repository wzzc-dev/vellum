use anyhow::{Result, anyhow};
use markdown::{CompileOptions, Options, to_html_with_options};

pub(super) fn export_markdown_to_html(markdown: &str, title: &str) -> Result<String> {
    let prepared = prepare_typora_extensions(markdown);
    let mut options = Options::gfm();
    options.compile = CompileOptions {
        allow_dangerous_html: true,
        allow_dangerous_protocol: true,
        allow_any_img_src: true,
        ..CompileOptions::gfm()
    };

    let body = to_html_with_options(&prepared, &options)
        .map_err(|err| anyhow!("failed to render markdown: {err:?}"))?;
    Ok(wrap_html_document(&body, title))
}

fn prepare_typora_extensions(markdown: &str) -> String {
    let headings = collect_headings(markdown);
    let markdown = replace_toc(markdown, &headings);
    replace_callouts(&markdown)
}

fn collect_headings(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    for line in markdown.lines() {
        let Some((level, title)) = parse_atx_heading(line) else {
            continue;
        };
        headings.push(Heading {
            level,
            title: title.to_string(),
            slug: slugify(title),
        });
    }
    headings
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

fn replace_toc(markdown: &str, headings: &[Heading]) -> String {
    let toc = render_toc_markdown(headings);
    let mut out = String::new();
    for line in markdown.lines() {
        if line.trim().eq_ignore_ascii_case("[toc]") {
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

fn replace_callouts(markdown: &str) -> String {
    let mut out = String::new();
    let mut lines = markdown.lines().peekable();

    while let Some(line) = lines.next() {
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

        while let Some(next) = lines.peek().copied() {
            if next.trim().is_empty() {
                out.push('\n');
                lines.next();
                break;
            }
            let Some(content) = strip_blockquote_marker(next) else {
                break;
            };
            out.push_str(&escape_html(content.trim_start()));
            out.push('\n');
            lines.next();
        }

        out.push_str("</div>\n");
    }

    out
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
}}
@media (prefers-color-scheme: dark) {{
  :root {{
    --bg: #171717;
    --fg: #ece7dc;
    --muted: #aaa397;
    --rule: #38342d;
    --accent: #7aa2f7;
    --code-bg: #27231e;
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
        assert!(html.contains("<h1>Hello</h1>"));
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
    fn exports_callouts_as_styled_blocks() {
        let html =
            export_markdown_to_html("> [!warning] Careful\n> Read this first.", "Callout").unwrap();
        assert!(html.contains("callout callout-warning"));
        assert!(html.contains("Careful"));
        assert!(html.contains("Read this first."));
    }
}

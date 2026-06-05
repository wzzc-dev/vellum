use std::{
    collections::{HashMap, HashSet},
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, anyhow};
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

pub(super) fn export_markdown_to_html_file(
    markdown: &str,
    title: &str,
    source_dir: Option<&Path>,
    output_path: &Path,
) -> Result<()> {
    let markdown = match source_dir {
        Some(source_dir) => rewrite_local_image_assets(markdown, source_dir, output_path)?,
        None => markdown.to_string(),
    };
    let html = export_markdown_to_html(&markdown, title)?;
    fs::write(output_path, html)
        .with_context(|| format!("failed to write {}", output_path.display()))
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

struct ExportAssetContext<'a> {
    source_dir: &'a Path,
    output_dir: PathBuf,
    asset_dir_name: String,
    copied: HashMap<PathBuf, String>,
    used_names: HashSet<String>,
}

impl<'a> ExportAssetContext<'a> {
    fn new(source_dir: &'a Path, output_path: &Path) -> Self {
        let output_dir = output_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            source_dir,
            output_dir,
            asset_dir_name: export_asset_dir_name(output_path),
            copied: HashMap::new(),
            used_names: HashSet::new(),
        }
    }

    fn exported_destination(&mut self, destination: &str) -> Result<Option<String>> {
        if should_leave_image_destination(destination) {
            return Ok(None);
        }

        let Some((source_path, reference_suffix)) =
            local_image_source_path(self.source_dir, destination)
        else {
            return Ok(None);
        };
        let cache_key = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.clone());
        if let Some(destination) = self.copied.get(&cache_key) {
            return Ok(Some(format!("{destination}{reference_suffix}")));
        }

        let file_name = self.unique_asset_file_name(&source_path);
        let asset_dir = self.output_dir.join(&self.asset_dir_name);
        fs::create_dir_all(&asset_dir)
            .with_context(|| format!("failed to create {}", asset_dir.display()))?;
        let target_path = asset_dir.join(&file_name);
        fs::copy(&source_path, &target_path).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;

        let exported = format!("{}/{}", self.asset_dir_name, file_name);
        self.copied.insert(cache_key, exported.clone());
        Ok(Some(format!("{exported}{reference_suffix}")))
    }

    fn unique_asset_file_name(&mut self, source_path: &Path) -> String {
        let stem = source_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("image");
        let ext = source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin");
        let stem = sanitize_asset_name_part(stem);
        let ext = sanitize_asset_extension(ext);
        let mut candidate = format!("{stem}.{ext}");
        if self.used_names.insert(candidate.clone()) {
            return candidate;
        }

        for index in 2.. {
            candidate = format!("{stem}-{index}.{ext}");
            if self.used_names.insert(candidate.clone()) {
                return candidate;
            }
        }

        unreachable!("unbounded suffix search should always find an asset name")
    }
}

fn rewrite_local_image_assets(
    markdown: &str,
    source_dir: &Path,
    output_path: &Path,
) -> Result<String> {
    let mut assets = ExportAssetContext::new(source_dir, output_path);
    let image_reference_labels = collect_image_reference_labels(markdown);
    let mut out = String::with_capacity(markdown.len());
    let mut fence_marker: Option<FenceMarker> = None;

    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let newline = &segment[line.len()..];

        if let Some(marker) = fence_marker {
            out.push_str(segment);
            if marker.closes(line) {
                fence_marker = None;
            }
            continue;
        }

        if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            fence_marker = Some(marker);
            out.push_str(segment);
            continue;
        }

        out.push_str(&rewrite_line_image_assets(
            line,
            &mut assets,
            &image_reference_labels,
        )?);
        out.push_str(newline);
    }

    Ok(out)
}

fn rewrite_line_image_assets(
    line: &str,
    assets: &mut ExportAssetContext<'_>,
    image_reference_labels: &HashSet<String>,
) -> Result<String> {
    if let Some(definition) = parse_reference_definition(line) {
        if image_reference_labels.contains(&normalize_reference_label(definition.label)) {
            if let Some(destination) = assets.exported_destination(&definition.destination)? {
                return Ok(format!(
                    "{}{}{}",
                    definition.prefix,
                    markdown_link_destination(&destination),
                    definition.title_suffix
                ));
            }
        }
    }

    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;

    while index < line.len() {
        let rest = &line[index..];
        if rest.starts_with('`') {
            let len = code_span_len(rest).unwrap_or(1);
            out.push_str(&rest[..len]);
            index += len;
        } else if let Some(image) = parse_markdown_image(rest) {
            if let Some(destination) = assets.exported_destination(&image.destination)? {
                out.push_str("![");
                out.push_str(image.label);
                out.push_str("](");
                out.push_str(&markdown_link_destination(&destination));
                out.push_str(image.title_suffix);
                out.push(')');
            } else {
                out.push_str(image.raw);
            }
            index += image.raw.len();
        } else if let Some(tag) = parse_html_asset_tag(rest) {
            if let Some(rewritten) = rewrite_html_asset_tag(&tag, assets)? {
                out.push_str(&rewritten);
            } else {
                out.push_str(tag.raw);
            }
            index += tag.raw.len();
        } else if let Some(ch) = rest.chars().next() {
            out.push(ch);
            index += ch.len_utf8();
        } else {
            break;
        }
    }

    Ok(out)
}

fn collect_image_reference_labels(markdown: &str) -> HashSet<String> {
    let mut labels = HashSet::new();
    let mut fence_marker: Option<FenceMarker> = None;

    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);

        if let Some(marker) = fence_marker {
            if marker.closes(line) {
                fence_marker = None;
            }
            continue;
        }

        if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            fence_marker = Some(marker);
            continue;
        }

        collect_image_reference_labels_from_line(line, &mut labels);
    }

    labels
}

fn collect_image_reference_labels_from_line(line: &str, labels: &mut HashSet<String>) {
    let mut index = 0usize;
    while index < line.len() {
        let rest = &line[index..];
        if rest.starts_with('`') {
            let len = code_span_len(rest).unwrap_or(1);
            index += len;
        } else if let Some((raw_len, label)) = parse_image_reference_use(rest) {
            labels.insert(label);
            index += raw_len;
        } else if let Some(ch) = rest.chars().next() {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
}

fn parse_image_reference_use(rest: &str) -> Option<(usize, String)> {
    let after_open = rest.strip_prefix("![")?;
    let close_label = find_unescaped_char(after_open, ']')?;
    let alt = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];

    if after_label.starts_with('(') {
        return None;
    }

    if let Some(after_ref_open) = after_label.strip_prefix('[') {
        let close_ref = find_unescaped_char(after_ref_open, ']')?;
        let explicit_label = &after_ref_open[..close_ref];
        let label = if explicit_label.is_empty() {
            alt
        } else {
            explicit_label
        };
        let normalized = normalize_reference_label(label);
        if normalized.is_empty() {
            return None;
        }
        let raw_len = 2 + close_label + 1 + 1 + close_ref + 1;
        Some((raw_len, normalized))
    } else {
        let normalized = normalize_reference_label(alt);
        if normalized.is_empty() {
            return None;
        }
        let raw_len = 2 + close_label + 1;
        Some((raw_len, normalized))
    }
}

struct ReferenceDefinition<'a> {
    label: &'a str,
    destination: String,
    prefix: &'a str,
    title_suffix: &'a str,
}

fn parse_reference_definition(line: &str) -> Option<ReferenceDefinition<'_>> {
    let indent_len = line.len() - line.trim_start_matches(' ').len();
    if indent_len > 3 {
        return None;
    }

    let after_indent = &line[indent_len..];
    let after_open = after_indent.strip_prefix('[')?;
    let close_label = find_unescaped_char(after_open, ']')?;
    let label = &after_open[..close_label];
    if normalize_reference_label(label).is_empty() {
        return None;
    }

    let after_label = &after_open[close_label + 1..];
    let after_colon = after_label.strip_prefix(':')?;
    let leading_ws = after_colon.len() - after_colon.trim_start().len();
    let destination_start = line.len() - after_colon.len() + leading_ws;
    let (destination, title_suffix) = split_image_destination(&line[destination_start..])?;

    Some(ReferenceDefinition {
        label,
        destination,
        prefix: &line[..destination_start],
        title_suffix,
    })
}

fn normalize_reference_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

struct MarkdownImage<'a> {
    raw: &'a str,
    label: &'a str,
    destination: String,
    title_suffix: &'a str,
}

struct HtmlAssetTag<'a> {
    raw: &'a str,
    src_range: Option<Range<usize>>,
    srcset_range: Option<Range<usize>>,
}

fn parse_markdown_image(rest: &str) -> Option<MarkdownImage<'_>> {
    let after_open = rest.strip_prefix("![")?;
    let close_label = find_unescaped_char(after_open, ']')?;
    let label = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];
    let destination_body = after_label.strip_prefix('(')?;
    let close_destination = find_image_destination_close(destination_body)?;
    let inner = &destination_body[..close_destination];
    let (destination, title_suffix) = split_image_destination(inner)?;
    let raw_len = 2 + close_label + 1 + 1 + close_destination + 1;
    Some(MarkdownImage {
        raw: &rest[..raw_len],
        label,
        destination,
        title_suffix,
    })
}

fn parse_html_asset_tag(rest: &str) -> Option<HtmlAssetTag<'_>> {
    let allows_src = html_asset_tag_allows_src(rest)?;
    let raw_len = html_asset_tag_len(rest)?;
    let raw = &rest[..raw_len];
    let src_range = allows_src.then(|| html_attr_value_range(raw, "src")).flatten();
    let srcset_range = html_attr_value_range(raw, "srcset");
    if src_range.is_none() && srcset_range.is_none() {
        return None;
    }

    Some(HtmlAssetTag {
        raw,
        src_range,
        srcset_range,
    })
}

struct HtmlAttributeReplacement {
    range: Range<usize>,
    value: String,
}

fn rewrite_html_asset_tag(
    tag: &HtmlAssetTag<'_>,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    let mut replacements = Vec::new();

    if let Some(range) = tag.src_range.clone() {
        let destination = decode_html_entities(&tag.raw[range.clone()]);
        if let Some(destination) = assets.exported_destination(&destination)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: escape_attr_value(&destination),
            });
        }
    }

    if let Some(range) = tag.srcset_range.clone() {
        if let Some(srcset) = rewrite_html_srcset(&tag.raw[range.clone()], assets)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: srcset,
            });
        }
    }

    if replacements.is_empty() {
        return Ok(None);
    }

    replacements.sort_by_key(|replacement| replacement.range.start);
    let mut out = String::with_capacity(tag.raw.len());
    let mut cursor = 0usize;
    for replacement in replacements {
        out.push_str(&tag.raw[cursor..replacement.range.start]);
        out.push_str(&replacement.value);
        cursor = replacement.range.end;
    }
    out.push_str(&tag.raw[cursor..]);
    Ok(Some(out))
}

fn rewrite_html_srcset(
    value: &str,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    let mut out = String::with_capacity(value.len());
    let mut changed = false;
    let mut candidate_start = 0usize;

    for (index, ch) in value.char_indices() {
        if ch == ',' {
            changed |=
                rewrite_html_srcset_candidate(&value[candidate_start..index], assets, &mut out)?;
            out.push(ch);
            candidate_start = index + ch.len_utf8();
        }
    }

    changed |= rewrite_html_srcset_candidate(&value[candidate_start..], assets, &mut out)?;
    Ok(changed.then_some(out))
}

fn rewrite_html_srcset_candidate(
    candidate: &str,
    assets: &mut ExportAssetContext<'_>,
    out: &mut String,
) -> Result<bool> {
    let trimmed = candidate.trim_start();
    let leading_len = candidate.len() - trimmed.len();
    let url_end = trimmed
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(trimmed.len());

    if url_end == 0 {
        out.push_str(candidate);
        return Ok(false);
    }

    let url = &trimmed[..url_end];
    let destination = decode_html_entities(url);
    if let Some(destination) = assets.exported_destination(&destination)? {
        out.push_str(&candidate[..leading_len]);
        out.push_str(&escape_attr_value(&destination));
        out.push_str(&trimmed[url_end..]);
        Ok(true)
    } else {
        out.push_str(candidate);
        Ok(false)
    }
}

fn split_image_destination(inner: &str) -> Option<(String, &str)> {
    let trimmed_start = inner.trim_start();
    let leading_ws = inner.len() - trimmed_start.len();
    if let Some(after_open) = trimmed_start.strip_prefix('<') {
        let close = find_unescaped_char(after_open, '>')?;
        let destination = unescape_markdown_destination(&after_open[..close]);
        let suffix_start = leading_ws + 1 + close + 1;
        return Some((destination, &inner[suffix_start..]));
    }

    let relative_end = trimmed_start
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(trimmed_start.len());
    if relative_end == 0 {
        return None;
    }
    let destination = unescape_markdown_destination(&trimmed_start[..relative_end]);
    Some((destination, &trimmed_start[relative_end..]))
}

fn find_unescaped_char(text: &str, needle: char) -> Option<usize> {
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == needle {
            return Some(index);
        }
    }
    None
}

fn find_image_destination_close(text: &str) -> Option<usize> {
    let mut escaped = false;
    let mut angle_depth = 0usize;
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '<' {
            angle_depth += 1;
        } else if ch == '>' && angle_depth > 0 {
            angle_depth -= 1;
        } else if ch == ')' && angle_depth == 0 {
            return Some(index);
        }
    }
    None
}

fn html_asset_tag_len(rest: &str) -> Option<usize> {
    if html_named_tag_matches(rest, "img") || html_named_tag_matches(rest, "source") {
        html_tag_close(rest).map(|close| close + 1)
    } else {
        None
    }
}

fn html_asset_tag_allows_src(rest: &str) -> Option<bool> {
    if html_named_tag_matches(rest, "img") {
        Some(true)
    } else if html_named_tag_matches(rest, "source") {
        Some(false)
    } else {
        None
    }
}

fn html_named_tag_matches(rest: &str, tag_name: &str) -> bool {
    let name_end = 1 + tag_name.len();
    if rest.as_bytes().first() != Some(&b'<') {
        return false;
    }
    if !rest
        .get(1..name_end)
        .is_some_and(|name| name.eq_ignore_ascii_case(tag_name))
    {
        return false;
    }

    matches!(
        rest.as_bytes().get(name_end).copied(),
        Some(b'>') | Some(b'/') | Some(b' ' | b'\t' | b'\r' | b'\n')
    )
}

fn html_tag_close(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in tag.char_indices() {
        match quote {
            Some(active_quote) if ch == active_quote => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '>' => return Some(index),
            None => {}
        }
    }
    None
}

fn html_attr_value_range(tag: &str, attr: &str) -> Option<Range<usize>> {
    let mut index = tag.find(char::is_whitespace).unwrap_or(tag.len());

    while index < tag.len() {
        index = skip_html_attr_space(tag, index);
        if match tag.as_bytes().get(index) {
            Some(byte) => matches!(byte, b'>' | b'/'),
            None => true,
        } {
            index += 1;
            continue;
        }

        let name_start = index;
        while index < tag.len()
            && !tag.as_bytes()[index].is_ascii_whitespace()
            && !matches!(tag.as_bytes()[index], b'=' | b'/' | b'>')
        {
            index += 1;
        }
        if name_start == index {
            index += tag[index..].chars().next()?.len_utf8();
            continue;
        }
        let name = &tag[name_start..index];

        index = skip_html_attr_space(tag, index);
        if tag.as_bytes().get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        index = skip_html_attr_space(tag, index);

        let value_start;
        let value_end;
        if let Some(quote @ (b'"' | b'\'')) = tag.as_bytes().get(index).copied() {
            value_start = index + 1;
            let close = tag[value_start..]
                .bytes()
                .position(|byte| byte == quote)?;
            value_end = value_start + close;
            index = value_end + 1;
        } else {
            value_start = index;
            while index < tag.len()
                && !tag.as_bytes()[index].is_ascii_whitespace()
                && tag.as_bytes()[index] != b'>'
            {
                index += 1;
            }
            value_end = index;
        }

        if name.eq_ignore_ascii_case(attr) {
            return Some(value_start..value_end);
        }
    }

    None
}

fn skip_html_attr_space(tag: &str, mut index: usize) -> usize {
    while index < tag.len() && tag.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn unescape_markdown_destination(destination: &str) -> String {
    let mut out = String::with_capacity(destination.len());
    let mut chars = destination.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn should_leave_image_destination(destination: &str) -> bool {
    let trimmed = destination.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("mailto:")
}

fn local_image_source_path<'a>(
    source_dir: &Path,
    destination: &'a str,
) -> Option<(PathBuf, &'a str)> {
    let source_path = resolve_local_image_path(source_dir, destination);
    if source_path.is_file() {
        return Some((source_path, ""));
    }

    let (path_part, suffix) = split_local_image_reference_suffix(destination)?;
    if path_part.trim().is_empty() {
        return None;
    }
    let source_path = resolve_local_image_path(source_dir, path_part);
    source_path.is_file().then_some((source_path, suffix))
}

fn split_local_image_reference_suffix(destination: &str) -> Option<(&str, &str)> {
    let suffix_start = destination
        .char_indices()
        .find_map(|(index, ch)| matches!(ch, '?' | '#').then_some(index))?;
    Some((&destination[..suffix_start], &destination[suffix_start..]))
}

fn resolve_local_image_path(source_dir: &Path, destination: &str) -> PathBuf {
    let destination = destination.trim();
    let path = Path::new(destination);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        source_dir.join(path)
    }
}

fn export_asset_dir_name(output_path: &Path) -> String {
    let stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document");
    format!("{}_assets", sanitize_asset_name_part(stem))
}

fn sanitize_asset_name_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "document".to_string()
    } else {
        sanitized
    }
}

fn sanitize_asset_extension(ext: &str) -> String {
    let sanitized = ext
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    if sanitized.is_empty() {
        "bin".to_string()
    } else {
        sanitized
    }
}

fn markdown_link_destination(destination: &str) -> String {
    let needs_angle_destination = destination
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '<' | '>' | '\\'));
    if needs_angle_destination {
        format!("<{}>", destination.replace('\\', r"\\").replace('>', r"\>"))
    } else {
        destination.to_string()
    }
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
    let mut mermaid_marker: Option<FenceMarker> = None;

    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let newline = &segment[line.len()..];
        if in_mermaid_block {
            if mermaid_marker.is_some_and(|marker| marker.closes(line)) {
                out.push_str(&render_mermaid_block(&mermaid_block));
                mermaid_block.clear();
                in_mermaid_block = false;
                mermaid_marker = None;
            } else {
                mermaid_block.push_str(line);
                mermaid_block.push_str(newline);
            }
            continue;
        }

        if let Some(marker) = mermaid_fence_open(line) {
            in_mermaid_block = true;
            mermaid_marker = Some(marker);
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

fn mermaid_fence_open(line: &str) -> Option<FenceMarker> {
    let trimmed = line.trim_start();
    let marker = FenceMarker::opening(trimmed)?;
    let rest = &trimmed[marker.len..];
    let info = rest.trim_start();
    let Some(rest) = info.strip_prefix("mermaid") else {
        return None;
    };
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(marker)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FenceMarker {
    ch: char,
    len: usize,
}

impl FenceMarker {
    fn opening(trimmed: &str) -> Option<Self> {
        let ch = trimmed.chars().next()?;
        if ch != '`' && ch != '~' {
            return None;
        }
        let len = trimmed.chars().take_while(|candidate| *candidate == ch).count();
        if len < 3 {
            return None;
        }
        Some(Self { ch, len })
    }

    fn closes(self, line: &str) -> bool {
        let trimmed = line.trim_start();
        let Some(marker) = Self::opening(trimmed) else {
            return false;
        };
        marker.ch == self.ch && marker.len >= self.len
    }
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
    let mut fence_marker: Option<FenceMarker> = None;
    let mut math_block = String::new();
    let mut in_math_block = false;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        let newline = &segment[line.len()..];
        if let Some(marker) = fence_marker {
            out.push_str(segment);
            if marker.closes(line) {
                fence_marker = None;
            }
            continue;
        }

        if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            out.push_str(segment);
            fence_marker = Some(marker);
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
    let fallback = math_fallback_text(source);
    format!(
        "<div class=\"math-block-wrap\"><div class=\"math-fallback math-block-fallback\">{}</div><div class=\"math math-block\">&#92;[{}&#92;]</div></div>\n",
        escape_html(&fallback),
        escape_html(source),
    )
}

fn math_fallback_text(source: &str) -> String {
    let fallback = editor::math_source_to_display_text(source);
    if fallback.trim().is_empty() {
        source.trim().to_string()
    } else {
        fallback
    }
}

fn render_mermaid_block(source: &str) -> String {
    let source = source.trim();
    format!(
        "<figure class=\"mermaid-diagram\">\n<pre class=\"mermaid-source\">{}</pre>\n<div class=\"mermaid-render-target\" aria-hidden=\"true\"></div>\n{}\n</figure>\n",
        escape_html(source),
        render_static_mermaid_fallback(source)
    )
}

fn render_static_mermaid_fallback(source: &str) -> String {
    if let Some(sequence) = parse_static_mermaid_sequence(source) {
        return render_static_sequence_mermaid_svg(sequence, static_mermaid_marker_id(source));
    }

    if let Some(diagram) = parse_static_mermaid_diagram(source) {
        return render_static_mermaid_svg(diagram, static_mermaid_marker_id(source));
    }

    format!(
        "<pre class=\"mermaid-static mermaid-static-source\">{}</pre>",
        escape_html(source)
    )
}

fn static_mermaid_marker_id(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("mermaid-arrow-{hash:x}")
}

fn parse_static_mermaid_diagram(source: &str) -> Option<StaticMermaidDiagram> {
    let mut direction = StaticMermaidDirection::TopDown;
    let mut nodes = Vec::new();
    let mut node_indices = HashMap::new();
    let mut edges = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(parsed_direction) = static_mermaid_direction(line) {
            direction = parsed_direction;
            continue;
        }

        if static_mermaid_non_edge_directive(line) {
            continue;
        }

        let parsed_edges = parse_static_mermaid_edges(line);
        if parsed_edges.is_empty() {
            continue;
        }

        for edge in parsed_edges {
            upsert_static_mermaid_node(&mut nodes, &mut node_indices, edge.from.clone());
            upsert_static_mermaid_node(&mut nodes, &mut node_indices, edge.to.clone());
            edges.push(edge);
        }
    }

    (!nodes.is_empty() && !edges.is_empty()).then_some(StaticMermaidDiagram {
        direction,
        nodes,
        node_indices,
        edges,
    })
}

fn upsert_static_mermaid_node(
    nodes: &mut Vec<StaticMermaidNode>,
    node_indices: &mut HashMap<String, usize>,
    node: StaticMermaidNode,
) {
    if let Some(index) = node_indices.get(&node.id).copied() {
        if nodes[index].label == nodes[index].id && node.label != node.id {
            nodes[index].label = node.label;
        }
        return;
    }

    node_indices.insert(node.id.clone(), nodes.len());
    nodes.push(node);
}

fn render_static_mermaid_svg(diagram: StaticMermaidDiagram, marker_id: String) -> String {
    let node_width = 180usize;
    let node_height = 48usize;
    let gap = 58usize;
    let margin = 28usize;
    let count = diagram.nodes.len().max(1);
    let horizontal = diagram.direction == StaticMermaidDirection::LeftRight;
    let width = if horizontal {
        margin * 2 + node_width * count + gap * count.saturating_sub(1)
    } else {
        margin * 2 + node_width
    };
    let height = if horizontal {
        margin * 2 + node_height
    } else {
        margin * 2 + node_height * count + gap * count.saturating_sub(1)
    };

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">\
<defs><marker id=\"{marker_id}\" viewBox=\"0 0 10 10\" refX=\"8\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"var(--muted)\"/></marker></defs>"
    );

    for edge in &diagram.edges {
        let Some(from_index) = diagram.node_indices.get(&edge.from.id).copied() else {
            continue;
        };
        let Some(to_index) = diagram.node_indices.get(&edge.to.id).copied() else {
            continue;
        };
        let (x1, y1, x2, y2) = static_mermaid_edge_points(
            from_index,
            to_index,
            horizontal,
            node_width,
            node_height,
            gap,
            margin,
        );
        out.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"var(--muted)\" stroke-width=\"2\" marker-end=\"url(#{marker_id})\"/>"
        ));
        if let Some(label) = &edge.label {
            if !label.is_empty() {
                let label_x = (x1 + x2) / 2;
                let label_y = (y1 + y2) / 2 - 8;
                out.push_str(&format!(
                    "<text x=\"{label_x}\" y=\"{label_y}\" text-anchor=\"middle\" font-size=\"12\" fill=\"var(--muted)\">{}</text>",
                    escape_html(label)
                ));
            }
        }
    }

    for (index, node) in diagram.nodes.iter().enumerate() {
        let (x, y) =
            static_mermaid_node_origin(index, horizontal, node_width, node_height, gap, margin);
        out.push_str(&render_static_mermaid_node(
            node,
            x,
            y,
            node_width,
            node_height,
        ));
    }

    out.push_str("</svg>");
    out
}

fn static_mermaid_edge_points(
    from_index: usize,
    to_index: usize,
    horizontal: bool,
    node_width: usize,
    node_height: usize,
    gap: usize,
    margin: usize,
) -> (usize, usize, usize, usize) {
    let (from_x, from_y) =
        static_mermaid_node_origin(from_index, horizontal, node_width, node_height, gap, margin);
    let (to_x, to_y) =
        static_mermaid_node_origin(to_index, horizontal, node_width, node_height, gap, margin);

    if horizontal {
        (
            from_x + node_width,
            from_y + node_height / 2,
            to_x,
            to_y + node_height / 2,
        )
    } else {
        (
            from_x + node_width / 2,
            from_y + node_height,
            to_x + node_width / 2,
            to_y,
        )
    }
}

fn static_mermaid_node_origin(
    index: usize,
    horizontal: bool,
    node_width: usize,
    node_height: usize,
    gap: usize,
    margin: usize,
) -> (usize, usize) {
    if horizontal {
        (margin + index * (node_width + gap), margin)
    } else {
        (margin, margin + index * (node_height + gap))
    }
}

fn render_static_mermaid_node(
    node: &StaticMermaidNode,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    let lines = svg_label_lines(&node.label, 22, 2);
    let center_x = x + width / 2;
    let center_y = y + height / 2;
    let line_height = 15isize;
    let first_y = center_y as isize - ((lines.len().saturating_sub(1) as isize * line_height) / 2);
    let mut out = format!(
        "<g class=\"mermaid-node\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/><text x=\"{center_x}\" y=\"{first_y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-size=\"13\" fill=\"var(--fg)\">"
    );
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { line_height };
        out.push_str(&format!(
            "<tspan x=\"{center_x}\" dy=\"{dy}\">{}</tspan>",
            escape_html(line)
        ));
    }
    out.push_str("</text></g>");
    out
}

fn svg_label_lines(label: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in label.split_whitespace() {
        let pending_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };
        if pending_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(label.to_string());
    }

    lines.truncate(max_lines);
    if let Some(last) = lines.last_mut() {
        if last.chars().count() > max_chars {
            let mut truncated = last.chars().take(max_chars.saturating_sub(3)).collect::<String>();
            truncated.push_str("...");
            *last = truncated;
        }
    }
    lines
}

fn static_mermaid_direction(line: &str) -> Option<StaticMermaidDirection> {
    let mut parts = line.split_whitespace();
    let head = parts.next()?.trim_end_matches(';');
    if !head.eq_ignore_ascii_case("graph") && !head.eq_ignore_ascii_case("flowchart") {
        return None;
    }

    let direction = parts
        .next()
        .unwrap_or("TD")
        .trim_matches(|ch: char| ch == ';' || ch == ',')
        .trim()
        .to_ascii_uppercase();
    match direction.as_str() {
        "LR" | "RL" => Some(StaticMermaidDirection::LeftRight),
        _ => Some(StaticMermaidDirection::TopDown),
    }
}

fn static_mermaid_non_edge_directive(line: &str) -> bool {
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

fn parse_static_mermaid_edges(line: &str) -> Vec<StaticMermaidEdge> {
    let mut edges = Vec::new();
    let mut rest = line.trim();

    while let Some((edge, next_rest)) = parse_static_mermaid_edge_prefix(rest) {
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

fn parse_static_mermaid_edge_prefix(
    line: &str,
) -> Option<(StaticMermaidEdge, Option<&str>)> {
    let (operator_start, operator_end) = find_static_mermaid_edge_operator_from(line, 0)?;
    let operator = &line[operator_start..operator_end];
    let mut from_source = line[..operator_start].trim_end();
    let mut label = None;

    if static_mermaid_labeled_operator(operator)
        && let Some(label_start) = from_source.rfind("--")
    {
        let candidate = clean_static_mermaid_label(&from_source[label_start + 2..]);
        if !candidate.is_empty() {
            from_source = from_source[..label_start].trim_end();
            label = Some(candidate);
        }
    }

    let (to_start, pipe_label) = static_mermaid_pipe_label_after_operator(line, operator_end);
    if pipe_label.is_some() {
        label = pipe_label;
    }

    let (to_end, has_next_edge) = static_mermaid_chained_node_end(line, to_start);
    let from = parse_static_mermaid_node(from_source)?;
    let to = parse_static_mermaid_node(&line[to_start..to_end])?;

    Some((
        StaticMermaidEdge { from, to, label },
        has_next_edge.then_some(&line[to_start..]),
    ))
}

fn static_mermaid_labeled_operator(operator: &str) -> bool {
    matches!(operator, "-->" | "==>" | "-.->" | "--o" | "--x")
}

fn static_mermaid_pipe_label_after_operator(line: &str, start: usize) -> (usize, Option<String>) {
    let rest = line[start..].trim_start();
    let label_start = start + line[start..].len() - rest.len();
    if !rest.starts_with('|') {
        return (label_start, None);
    }

    let after_first = &line[label_start + 1..];
    let Some(second_pipe) = after_first.find('|') else {
        return (label_start, None);
    };

    let label = clean_static_mermaid_label(&after_first[..second_pipe]);
    let after_label = label_start + 1 + second_pipe + 1;
    let rest_after_label = line[after_label..].trim_start();
    let node_start = after_label + line[after_label..].len() - rest_after_label.len();

    (
        node_start,
        if label.is_empty() { None } else { Some(label) },
    )
}

fn static_mermaid_chained_node_end(line: &str, node_start: usize) -> (usize, bool) {
    let Some((next_operator_start, _)) = find_static_mermaid_edge_operator_from(line, node_start)
    else {
        return (line.len(), false);
    };

    let before_next = line[node_start..next_operator_start].trim_end();
    let mut node_end = node_start + before_next.len();
    if let Some(label_start) = before_next.rfind("--") {
        let candidate = clean_static_mermaid_label(&before_next[label_start + 2..]);
        if !candidate.is_empty() {
            node_end = node_start + label_start;
        }
    }

    (node_end, true)
}

fn find_static_mermaid_edge_operator_from(line: &str, start: usize) -> Option<(usize, usize)> {
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

fn parse_static_mermaid_node(source: &str) -> Option<StaticMermaidNode> {
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
        .map(|start| clean_static_mermaid_label(&without_class[..start]))
        .unwrap_or_else(|| clean_static_mermaid_label(without_class));
    let label = label_start
        .map(|start| clean_static_mermaid_label(&without_class[start..]))
        .unwrap_or_default();

    let id = if id.is_empty() { label.clone() } else { id };
    let label = if label.is_empty() { id.clone() } else { label };

    (!id.is_empty() || !label.is_empty()).then_some(StaticMermaidNode { id, label })
}

fn clean_static_mermaid_label(source: &str) -> String {
    source
        .trim()
        .trim_matches(|ch| matches!(ch, '[' | ']' | '(' | ')' | '{' | '}'))
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn parse_static_mermaid_sequence(source: &str) -> Option<StaticMermaidSequence> {
    let mut saw_sequence = false;
    let mut participants = Vec::new();
    let mut participant_indices = HashMap::new();
    let mut messages = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_sequence {
            if line.eq_ignore_ascii_case("sequencediagram") {
                saw_sequence = true;
                continue;
            }
            return None;
        }

        if let Some(participant) = parse_static_sequence_participant(line) {
            upsert_static_sequence_participant(
                &mut participants,
                &mut participant_indices,
                participant,
            );
            continue;
        }

        if static_sequence_non_message_directive(line) {
            continue;
        }

        let Some(message) = parse_static_sequence_message(line) else {
            continue;
        };

        upsert_static_sequence_participant(
            &mut participants,
            &mut participant_indices,
            StaticMermaidParticipant::new(&message.from),
        );
        upsert_static_sequence_participant(
            &mut participants,
            &mut participant_indices,
            StaticMermaidParticipant::new(&message.to),
        );
        messages.push(message);
    }

    (saw_sequence && !participants.is_empty() && !messages.is_empty()).then_some(
        StaticMermaidSequence {
            participants,
            participant_indices,
            messages,
        },
    )
}

fn parse_static_sequence_participant(line: &str) -> Option<StaticMermaidParticipant> {
    let keyword_end = line.find(char::is_whitespace).unwrap_or(line.len());
    let keyword = &line[..keyword_end];
    if !keyword.eq_ignore_ascii_case("participant") && !keyword.eq_ignore_ascii_case("actor") {
        return None;
    }

    let rest = line[keyword_end..].trim();
    if rest.is_empty() {
        return None;
    }

    let (id_source, label_source) = split_static_sequence_alias(rest).unwrap_or((rest, rest));
    let id = clean_static_sequence_participant_id(id_source);
    let label = clean_static_mermaid_label(label_source);
    let label = if label.is_empty() { id.clone() } else { label };

    (!id.is_empty()).then_some(StaticMermaidParticipant { id, label })
}

fn split_static_sequence_alias(source: &str) -> Option<(&str, &str)> {
    let lower = source.to_ascii_lowercase();
    lower
        .find(" as ")
        .map(|index| (&source[..index], &source[index + 4..]))
}

fn static_sequence_non_message_directive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower == "autonumber"
        || lower == "end"
        || lower.starts_with("activate ")
        || lower.starts_with("deactivate ")
        || lower.starts_with("destroy ")
        || lower.starts_with("box ")
        || lower.starts_with("rect ")
        || lower.starts_with("note ")
        || lower.starts_with("loop ")
        || lower.starts_with("alt ")
        || lower.starts_with("else")
        || lower.starts_with("opt ")
        || lower.starts_with("par ")
        || lower.starts_with("and ")
        || lower.starts_with("critical ")
        || lower.starts_with("option ")
        || lower.starts_with("break ")
}

fn parse_static_sequence_message(line: &str) -> Option<StaticMermaidMessage> {
    let (operator_start, operator_end, operator) = find_static_sequence_message_operator(line)?;
    let from = clean_static_sequence_participant_id(&line[..operator_start]);
    let rest = line[operator_end..].trim();
    let (to_source, label_source) = rest.split_once(':').unwrap_or((rest, ""));
    let to = clean_static_sequence_participant_id(to_source);
    let label = clean_static_mermaid_label(label_source);

    (!from.is_empty() && !to.is_empty()).then_some(StaticMermaidMessage {
        from,
        to,
        label,
        dashed: operator.starts_with("--"),
    })
}

fn find_static_sequence_message_operator(line: &str) -> Option<(usize, usize, &'static str)> {
    for (index, _) in line.char_indices() {
        for operator in [
            "-->>+", "-->>-", "-->>", "->>+", "->>-", "->>", "--)+", "--)-", "--)", "-)+",
            "-)-", "-)", "--x+", "--x-", "--x", "-x+", "-x-", "-x", "-->+", "-->-", "-->",
            "->+", "->-", "->",
        ] {
            if line[index..].starts_with(operator) {
                return Some((index, index + operator.len(), operator));
            }
        }
    }
    None
}

fn clean_static_sequence_participant_id(source: &str) -> String {
    clean_static_mermaid_label(source)
        .trim_start_matches(|ch| matches!(ch, '+' | '-'))
        .trim_end_matches(|ch| matches!(ch, '+' | '-'))
        .trim()
        .to_string()
}

fn upsert_static_sequence_participant(
    participants: &mut Vec<StaticMermaidParticipant>,
    participant_indices: &mut HashMap<String, usize>,
    participant: StaticMermaidParticipant,
) {
    if let Some(index) = participant_indices.get(&participant.id).copied() {
        if participants[index].label == participants[index].id
            && participant.label != participant.id
        {
            participants[index].label = participant.label;
        }
        return;
    }

    participant_indices.insert(participant.id.clone(), participants.len());
    participants.push(participant);
}

fn render_static_sequence_mermaid_svg(
    sequence: StaticMermaidSequence,
    marker_id: String,
) -> String {
    let participant_width = 154usize;
    let participant_height = 42usize;
    let participant_gap = 48usize;
    let margin = 28usize;
    let lifeline_top = margin + participant_height + 18;
    let message_start_y = lifeline_top + 40;
    let message_gap = 56usize;
    let participant_count = sequence.participants.len().max(1);
    let message_count = sequence.messages.len().max(1);
    let width = margin * 2
        + participant_width * participant_count
        + participant_gap * participant_count.saturating_sub(1);
    let height = message_start_y + message_gap * message_count + margin;

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid sequence diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">\
<defs><marker id=\"{marker_id}\" viewBox=\"0 0 10 10\" refX=\"8\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"var(--muted)\"/></marker></defs>"
    );

    for (index, participant) in sequence.participants.iter().enumerate() {
        let center_x = static_sequence_participant_center_x(
            index,
            participant_width,
            participant_gap,
            margin,
        );
        let x = center_x - participant_width / 2;
        let y = margin;
        out.push_str(&format!(
            "<g class=\"mermaid-participant\"><rect x=\"{x}\" y=\"{y}\" width=\"{participant_width}\" height=\"{participant_height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/>"
        ));
        out.push_str(&render_static_sequence_label(
            &participant.label,
            center_x,
            y + participant_height / 2,
            18,
            2,
            "var(--fg)",
        ));
        out.push_str(&format!(
            "</g><line x1=\"{center_x}\" y1=\"{lifeline_top}\" x2=\"{center_x}\" y2=\"{}\" stroke=\"var(--rule)\" stroke-width=\"1.5\" stroke-dasharray=\"5 6\"/>",
            height - margin
        ));
    }

    for (index, message) in sequence.messages.iter().enumerate() {
        let Some(from_index) = sequence.participant_indices.get(&message.from).copied() else {
            continue;
        };
        let Some(to_index) = sequence.participant_indices.get(&message.to).copied() else {
            continue;
        };
        let from_x = static_sequence_participant_center_x(
            from_index,
            participant_width,
            participant_gap,
            margin,
        );
        let to_x = static_sequence_participant_center_x(
            to_index,
            participant_width,
            participant_gap,
            margin,
        );
        let y = message_start_y + index * message_gap;
        let dash = if message.dashed {
            " stroke-dasharray=\"6 5\""
        } else {
            ""
        };

        if from_x == to_x {
            let loop_width = 44usize;
            let loop_height = 24usize;
            let x2 = (from_x + loop_width).min(width - margin);
            out.push_str(&format!(
                "<path d=\"M {from_x} {y} H {x2} V {} H {from_x}\" fill=\"none\" stroke=\"var(--muted)\" stroke-width=\"2\"{dash} marker-end=\"url(#{marker_id})\"/>",
                y + loop_height
            ));
            render_static_sequence_message_label(&mut out, &message.label, from_x + 28, y - 10);
        } else {
            out.push_str(&format!(
                "<line x1=\"{from_x}\" y1=\"{y}\" x2=\"{to_x}\" y2=\"{y}\" stroke=\"var(--muted)\" stroke-width=\"2\"{dash} marker-end=\"url(#{marker_id})\"/>"
            ));
            render_static_sequence_message_label(
                &mut out,
                &message.label,
                (from_x + to_x) / 2,
                y - 10,
            );
        }
    }

    out.push_str("</svg>");
    out
}

fn static_sequence_participant_center_x(
    index: usize,
    participant_width: usize,
    participant_gap: usize,
    margin: usize,
) -> usize {
    margin + participant_width / 2 + index * (participant_width + participant_gap)
}

fn render_static_sequence_label(
    label: &str,
    x: usize,
    center_y: usize,
    max_chars: usize,
    max_lines: usize,
    fill: &str,
) -> String {
    let lines = svg_label_lines(label, max_chars, max_lines);
    let line_height = 15isize;
    let first_y = center_y as isize - ((lines.len().saturating_sub(1) as isize * line_height) / 2);
    let mut out = format!(
        "<text x=\"{x}\" y=\"{first_y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-size=\"13\" fill=\"{fill}\">"
    );
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { line_height };
        out.push_str(&format!(
            "<tspan x=\"{x}\" dy=\"{dy}\">{}</tspan>",
            escape_html(line)
        ));
    }
    out.push_str("</text>");
    out
}

fn render_static_sequence_message_label(out: &mut String, label: &str, x: usize, y: usize) {
    if label.is_empty() {
        return;
    }

    out.push_str(&render_static_sequence_label(
        label,
        x,
        y,
        34,
        1,
        "var(--muted)",
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StaticMermaidDirection {
    TopDown,
    LeftRight,
}

struct StaticMermaidDiagram {
    direction: StaticMermaidDirection,
    nodes: Vec<StaticMermaidNode>,
    node_indices: HashMap<String, usize>,
    edges: Vec<StaticMermaidEdge>,
}

#[derive(Clone)]
struct StaticMermaidNode {
    id: String,
    label: String,
}

struct StaticMermaidEdge {
    from: StaticMermaidNode,
    to: StaticMermaidNode,
    label: Option<String>,
}

struct StaticMermaidSequence {
    participants: Vec<StaticMermaidParticipant>,
    participant_indices: HashMap<String, usize>,
    messages: Vec<StaticMermaidMessage>,
}

struct StaticMermaidParticipant {
    id: String,
    label: String,
}

impl StaticMermaidParticipant {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            label: id.to_string(),
        }
    }
}

struct StaticMermaidMessage {
    from: String,
    to: String,
    label: String,
    dashed: bool,
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
            "<span class=\"math-inline-wrap\"><span class=\"math-fallback math-inline-fallback\">{}</span><span class=\"math math-inline\">&#92;({}&#92;)</span></span>",
            escape_html(&math_fallback_text(inner.trim())),
            escape_html(inner.trim()),
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
    let mut fence_marker: Option<FenceMarker> = None;

    for line in markdown.lines() {
        if let Some(marker) = fence_marker {
            if marker.closes(line) {
                fence_marker = None;
            }
            previous_line = None;
            continue;
        }

        if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            fence_marker = Some(marker);
            previous_line = None;
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
    let mut fence_marker: Option<FenceMarker> = None;
    for line in markdown.lines() {
        if let Some(marker) = fence_marker {
            out.push_str(line);
            out.push('\n');
            if marker.closes(line) {
                fence_marker = None;
            }
        } else if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            fence_marker = Some(marker);
            out.push_str(line);
            out.push('\n');
        } else if line.trim().eq_ignore_ascii_case("[toc]") {
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
    let mut fence_marker: Option<FenceMarker> = None;

    while let Some(line) = lines.next() {
        if let Some(marker) = fence_marker {
            out.push_str(line);
            out.push('\n');
            if marker.closes(line) {
                fence_marker = None;
            }
            continue;
        }

        if let Some(marker) = FenceMarker::opening(line.trim_start()) {
            fence_marker = Some(marker);
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
    let scripts = html_enhancement_scripts(body);
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
.math-inline-wrap {{ white-space: nowrap; }}
.math-inline,
.math-inline-fallback {{ font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif; }}
.math {{
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}}
body.math-rendered .math {{
  position: static;
  width: auto;
  height: auto;
  overflow: visible;
  clip-path: none;
}}
body.math-rendered .math-fallback {{ display: none; }}
.math-block-wrap {{ margin: 1.25em 0; }}
.math-block {{
  display: block;
  overflow-x: auto;
  margin: 0;
  padding: .85em 1em;
  text-align: center;
  background: color-mix(in srgb, var(--code-bg) 52%, transparent);
  border-radius: 6px;
}}
.math-block-fallback {{
  display: block;
  overflow-x: auto;
  padding: .85em 1em;
  text-align: center;
  background: color-mix(in srgb, var(--code-bg) 52%, transparent);
  border-radius: 6px;
  font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif;
}}
.mermaid-diagram {{
  overflow-x: auto;
  margin: 1.25em 0;
  padding: 1em;
  background: color-mix(in srgb, var(--code-bg) 52%, transparent);
  border: 1px solid var(--rule);
  border-radius: 6px;
}}
.mermaid-source,
.mermaid-render-target {{ display: none; }}
.mermaid-diagram.mermaid-rendered .mermaid-render-target {{ display: block; }}
.mermaid-diagram.mermaid-rendered .mermaid-static {{ display: none; }}
.mermaid-static {{
  display: block;
  width: 100%;
  max-width: 100%;
  height: auto;
}}
.mermaid-static-source {{
  margin: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
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
@page {{ margin: 18mm 16mm; }}
@media print {{
  :root {{
    color-scheme: light;
    --bg: #fff;
    --fg: #111;
    --muted: #555;
    --rule: #c7c7c7;
    --accent: #111;
    --code-bg: #f2f2f2;
    --mark-bg: #fff2a8;
  }}
  body {{
    background: #fff;
    color: #111;
    font-size: 11pt;
    line-height: 1.55;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }}
  main {{ width: auto; margin: 0; }}
  h1, h2, h3, h4, h5, h6 {{ break-after: avoid; page-break-after: avoid; }}
  p, blockquote, pre, table, ul, ol, .math-block-wrap, .mermaid-diagram, .callout {{
    break-inside: avoid;
    page-break-inside: avoid;
  }}
  pre {{ white-space: pre-wrap; overflow-wrap: anywhere; }}
  img, svg {{ max-width: 100%; break-inside: avoid; page-break-inside: avoid; }}
  a {{ color: inherit; text-decoration: underline; }}
  a[href^="http"]::after {{
    content: " (" attr(href) ")";
    color: var(--muted);
    font-size: .85em;
    overflow-wrap: anywhere;
  }}
  .toc a::after,
  sup a::after,
  .footnotes a::after {{ content: ""; }}
}}
</style>
</head>
<body>
<main>
{}
</main>
{}
</body>
</html>
"#,
        escape_html(title),
        body,
        scripts
    )
}

fn html_enhancement_scripts(body: &str) -> String {
    let mut scripts = String::new();
    if body.contains("class=\"math ") {
        scripts.push_str(
            r#"<script>
window.MathJax = {
  tex: {
    inlineMath: [["\\(", "\\)"]],
    displayMath: [["\\[", "\\]"]]
  },
  svg: { fontCache: "global" },
  startup: {
    pageReady: () => MathJax.startup.defaultPageReady().then(() => {
      document.body.classList.add("math-rendered");
    })
  }
};
</script>
<script defer src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js"></script>
"#,
        );
    }
    if body.contains("class=\"mermaid-diagram\"") {
        scripts.push_str(
            r#"<script type="module">
import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs";
mermaid.initialize({ startOnLoad: false, securityLevel: "strict" });
(async () => {
  for (const [index, figure] of document.querySelectorAll(".mermaid-diagram").entries()) {
    const source = figure.querySelector(".mermaid-source")?.textContent ?? "";
    const target = figure.querySelector(".mermaid-render-target");
    if (!source.trim() || !target) continue;
    try {
      const rendered = await mermaid.render(`vellum-mermaid-${index}`, source);
      target.innerHTML = rendered.svg;
      rendered.bindFunctions?.(target);
      figure.classList.add("mermaid-rendered");
    } catch (_) {
      figure.classList.add("mermaid-runtime-failed");
    }
  }
})();
</script>
"#,
        );
    }
    scripts
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_export_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vellum-{name}-{nonce}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn exports_full_html_document() {
        let html = export_markdown_to_html("# Hello", "Draft <One>").unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>Draft &lt;One&gt;</title>"));
        assert!(html.contains("<h1 id=\"hello\">Hello</h1>"));
        assert!(!html.contains("mathjax@3"));
        assert!(!html.contains("mermaid.esm.min.mjs"));
    }

    #[test]
    fn exported_html_includes_print_styles() {
        let markdown = concat!(
            "# Print\n\n",
            "[link](https://example.com)\n\n",
            "```rust\nfn main() {}\n```\n\n",
            "| A | B |\n| - | - |\n| 1 | 2 |",
        );
        let html = export_markdown_to_html(markdown, "Print").unwrap();

        assert!(html.contains("@page { margin: 18mm 16mm; }"));
        assert!(html.contains("@media print"));
        assert!(html.contains("main { width: auto; margin: 0; }"));
        assert!(html.contains("break-after: avoid"));
        assert!(html.contains("page-break-inside: avoid"));
        assert!(html.contains("a[href^=\"http\"]::after"));
    }

    #[test]
    fn exports_longform_acceptance_sample() {
        let root = temp_export_dir("longform-acceptance");
        let output = root.join("longform.html");
        let sample_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/acceptance");
        let markdown = include_str!("../../../../docs/acceptance/longform.md");

        export_markdown_to_html_file(markdown, "Fallback", Some(&sample_dir), &output).unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<title>Vellum Longform Acceptance</title>"));
        assert!(html.contains("<a href=\"#draft-scope\">Draft Scope</a>"));
        assert!(html.contains("<h2 id=\"editing-blocks\">Editing Blocks</h2>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<code class=\"language-rust\">"));
        assert!(html.contains("src=\"longform_assets/cover.svg#acceptance-cover\""));
        assert!(html.contains("src=\"longform_assets/cover.svg\""));
        assert!(html.contains("src=\"longform_assets/reference-diagram.svg\""));
        assert!(
            html.contains(
                "srcset=\"longform_assets/cover.svg 1x, longform_assets/reference-diagram.svg#workflow 2x\""
            )
        );
        assert!(html.contains("srcset=\"longform_assets/reference-diagram.svg 640w\""));
        assert!(html.contains("title=\"Referenced local diagram\""));
        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">E = mc²</span>"));
        assert!(html.contains("<span class=\"math math-inline\">\\(E = mc^2\\)</span>"));
        assert!(html.contains("a² + b² = c²; E = mc²"));
        assert!(html.contains("\\begin{aligned}"));
        assert!(html.contains("E &amp;= mc^2"));
        assert!(html.contains("mathjax@3"));
        assert!(html.contains("math-rendered"));
        assert!(html.contains("mermaid.esm.min.mjs"));
        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains(">Export HTML<"));
        assert!(html.contains("Mermaid sequence diagram preview"));
        assert!(html.contains(">Save PDF</tspan>"));
        assert!(html.contains("data-footnotes"));
        assert!(html.contains("@media print"));
        assert!(root.join("longform_assets/cover.svg").is_file());
        assert!(root.join("longform_assets/reference-diagram.svg").is_file());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_relative_image_assets() {
        let root = temp_export_dir("html-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/cover.png"), b"cover").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "![Cover](assets/cover.png)",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<img src=\"article_assets/cover.png\" alt=\"Cover\" />"));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover.png")).unwrap(),
            b"cover"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_image_assets() {
        let root = temp_export_dir("html-raw-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/raw.png"), b"raw").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "<img alt=\"Raw\" src='assets/raw.png' class=\"wide\">",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("alt=\"Raw\""));
        assert!(html.contains("src='article_assets/raw.png'"));
        assert!(html.contains("class=\"wide\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/raw.png")).unwrap(),
            b"raw"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_srcset_assets() {
        let root = temp_export_dir("html-raw-srcset-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/cover.png"), b"cover").unwrap();
        std::fs::write(source.join("assets/cover@2x.png"), b"cover2x").unwrap();
        std::fs::write(source.join("assets/fallback.png"), b"fallback").unwrap();
        std::fs::write(source.join("assets/mobile.png"), b"mobile").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "<picture>",
                "<source media=\"(min-width: 600px)\" ",
                "srcset=\"assets/cover.png 1x, ",
                "assets/cover@2x.png?scale=2&amp;theme=print#sharp 2x\">",
                "<img src=\"assets/fallback.png\" ",
                "srcset='assets/mobile.png 480w, https://example.com/remote.png 960w' ",
                "alt=\"Cover\">",
                "</picture>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("src=\"article_assets/fallback.png\""));
        assert!(html.contains(
            "srcset=\"article_assets/cover.png 1x, article_assets/cover@2x.png?scale=2&amp;theme=print#sharp 2x\""
        ));
        assert!(html.contains(
            "srcset='article_assets/mobile.png 480w, https://example.com/remote.png 960w'"
        ));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover.png")).unwrap(),
            b"cover"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover@2x.png")).unwrap(),
            b"cover2x"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/fallback.png")).unwrap(),
            b"fallback"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/mobile.png")).unwrap(),
            b"mobile"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_preserves_local_image_query_and_fragment() {
        let root = temp_export_dir("html-image-suffix-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/cover.png"), b"cover").unwrap();
        std::fs::write(source.join("assets/raw.svg"), b"raw").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "![Cover](assets/cover.png?cache=1#hero)\n\n",
                "<img src=\"assets/raw.svg#icon\" alt=\"Raw\">",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("src=\"article_assets/cover.png?cache=1#hero\""));
        assert!(html.contains("src=\"article_assets/raw.svg#icon\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover.png")).unwrap(),
            b"cover"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/raw.svg")).unwrap(),
            b"raw"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_reference_image_assets() {
        let root = temp_export_dir("html-reference-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/cover.png"), b"cover").unwrap();
        std::fs::write(source.join("assets/diagram.png"), b"diagram").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "![Cover][cover]\n\n",
                "![Diagram]\n\n",
                "[cover]: assets/cover.png \"Cover title\"\n",
                "[Diagram]: assets/diagram.png\n",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("src=\"article_assets/cover.png\""));
        assert!(html.contains("title=\"Cover title\""));
        assert!(html.contains("src=\"article_assets/diagram.png\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover.png")).unwrap(),
            b"cover"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/diagram.png")).unwrap(),
            b"diagram"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_leaves_remote_images_and_code_spans_alone() {
        let root = temp_export_dir("html-remote-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/hidden.png"), b"hidden").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "`![Hidden](assets/hidden.png)`\n\n![Remote](https://example.com/remote.png)",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<code>![Hidden](assets/hidden.png)</code>"));
        assert!(html.contains("src=\"https://example.com/remote.png\""));
        assert!(!export_dir.join("article_assets").exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_leaves_remote_raw_html_images_and_code_spans_alone() {
        let root = temp_export_dir("html-raw-remote-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/hidden.png"), b"hidden").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "`<img src=\"assets/hidden.png\">`\n\n<img src=\"https://example.com/remote.png\" alt=\"Remote\">",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<code>&lt;img src=&quot;assets/hidden.png&quot;&gt;</code>"));
        assert!(html.contains("src=\"https://example.com/remote.png\""));
        assert!(!export_dir.join("article_assets").exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_ignores_reference_assets_without_image_uses() {
        let root = temp_export_dir("html-ignored-reference-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/doc.png"), b"doc").unwrap();
        std::fs::write(source.join("assets/hidden.png"), b"hidden").unwrap();
        std::fs::write(source.join("assets/fenced.png"), b"fenced").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "[Document][doc]\n\n",
                "`![Hidden][hidden]`\n\n",
                "```md\n",
                "![Fenced][fenced]\n",
                "```\n\n",
                "[doc]: assets/doc.png\n",
                "[hidden]: assets/hidden.png\n",
                "[fenced]: assets/fenced.png\n",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<a href=\"assets/doc.png\">Document</a>"));
        assert!(html.contains("<code>![Hidden][hidden]</code>"));
        assert!(html.contains("<code class=\"language-md\">"));
        assert!(!export_dir.join("article_assets").exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_avoids_asset_name_collisions() {
        let root = temp_export_dir("html-asset-collisions");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("one")).unwrap();
        std::fs::create_dir_all(source.join("two")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("one/cover.png"), b"one").unwrap();
        std::fs::write(source.join("two/cover.png"), b"two").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "![One](one/cover.png)\n\n![Two](two/cover.png)",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("src=\"article_assets/cover.png\""));
        assert!(html.contains("src=\"article_assets/cover-2.png\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover.png")).unwrap(),
            b"one"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/cover-2.png")).unwrap(),
            b"two"
        );

        std::fs::remove_dir_all(root).unwrap();
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
    fn toc_marker_inside_longer_code_fence_stays_literal() {
        let html = export_markdown_to_html(
            "````markdown\n```\n[toc]\n```\n````\n\n# Visible",
            "TOC",
        )
        .unwrap();

        assert!(html.contains("[toc]"));
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
    fn callout_marker_inside_longer_code_fence_stays_literal() {
        let html = export_markdown_to_html(
            "````markdown\n```\n> [!note] Literal\n```\n````",
            "Callout",
        )
        .unwrap();

        assert!(html.contains("&gt; [!note] Literal"));
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
    fn inline_extensions_skip_longer_code_fence_contents() {
        let html = export_markdown_to_html("````markdown\n```\n==literal==\n```\n````", "Inline")
            .unwrap();

        assert!(html.contains("==literal=="));
        assert!(!html.contains("<mark>literal</mark>"));
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
        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">E = mc²</span>"));
        assert!(html.contains("<span class=\"math math-inline\">\\(E = mc^2\\)</span>"));
        assert!(html.contains("<div class=\"math-fallback math-block-fallback\">∫₀¹ x² dx</div>"));
        assert!(html.contains("<div class=\"math math-block\">&#92;[\\int_0^1 x^2 dx&#92;]</div>"));
        assert!(html.contains("mathjax@3"));
        assert!(html.contains("math-rendered"));
    }

    #[test]
    fn math_export_skips_code_and_escapes_html() {
        let html = export_markdown_to_html("`$x$` and $<x>$", "Math").unwrap();
        assert!(html.contains("<code>$x$</code>"));
        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">&lt;x&gt;</span>"));
        assert!(html.contains("<span class=\"math math-inline\">\\(&lt;x&gt;\\)</span>"));
    }

    #[test]
    fn math_runtime_is_not_loaded_for_code_only_math_markers() {
        let html = export_markdown_to_html("`$x$`", "Math").unwrap();
        assert!(!html.contains("mathjax@3"));
    }

    #[test]
    fn exports_mermaid_fence_as_diagram_container() {
        let html = export_markdown_to_html(
            "```mermaid\ngraph TD\n  A --> B\n```",
            "Mermaid",
        )
        .unwrap();
        assert!(html.contains("<figure class=\"mermaid-diagram\">"));
        assert!(html.contains("<pre class=\"mermaid-source\">graph TD\n  A --&gt; B</pre>"));
        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains(">A<"));
        assert!(html.contains(">B<"));
        assert!(html.contains(".mermaid-diagram"));
        assert!(html.contains("mermaid.esm.min.mjs"));
    }

    #[test]
    fn mermaid_export_has_static_flowchart_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nflowchart LR\n  Start[Draft] --> Save[Save]\n  Save --> Print[Print or save PDF]\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("viewBox=\"0 0 712 104\""));
        assert!(html.contains(">Draft<"));
        assert!(html.contains(">Save<"));
        assert!(html.contains(">Print or save PDF<"));
        assert!(html.contains("mermaid-render-target"));
        assert!(html.contains("mermaid-rendered"));
    }

    #[test]
    fn mermaid_export_expands_chained_flowchart_edges() {
        let html = export_markdown_to_html(
            "```mermaid\nflowchart LR\n  Draft[Draft] -->|save| Review[Review] -- publish --> Done[Done]\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("viewBox=\"0 0 712 104\""));
        assert!(html.contains(">Draft<"));
        assert!(html.contains(">Review<"));
        assert!(html.contains(">Done<"));
        assert!(html.contains(">save</text>"));
        assert!(html.contains(">publish</text>"));
    }

    #[test]
    fn mermaid_export_has_static_sequence_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nsequenceDiagram\n  participant Alice as Writer\n  actor Bob as Reviewer\n  Alice->>Bob: Hi <draft>\n  Bob-->>Alice: Looks good\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("Mermaid sequence diagram preview"));
        assert!(html.contains(">Writer</tspan>"));
        assert!(html.contains(">Reviewer</tspan>"));
        assert!(html.contains(">Hi &lt;draft&gt;</tspan>"));
        assert!(html.contains(">Looks good</tspan>"));
        assert!(html.contains("stroke-dasharray=\"6 5\""));
        assert!(!html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">sequenceDiagram"
        ));
    }

    #[test]
    fn unsupported_mermaid_export_falls_back_to_source() {
        let html = export_markdown_to_html(
            "```mermaid\npie\n  \"Draft\" : 2\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">pie\n  &quot;Draft&quot; : 2</pre>"
        ));
    }

    #[test]
    fn mermaid_export_escapes_html_and_keeps_regular_code_fences() {
        let html = export_markdown_to_html(
            "```mermaid\ngraph TD\n  A[<tag>] --> B\n```\n\n```rust\nfn main() {}\n```",
            "Mermaid",
        )
        .unwrap();
        assert!(html.contains("A[&lt;tag&gt;] --&gt; B"));
        assert!(html.contains("&lt;tag&gt;</tspan>"));
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

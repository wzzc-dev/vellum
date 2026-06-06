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
    let body = wrap_tables_for_scroll(&body);
    Ok(wrap_html_document(&body, document_title))
}

pub(super) fn export_markdown_to_html_file(
    markdown: &str,
    title: &str,
    source_dir: Option<&Path>,
    output_path: &Path,
) -> Result<()> {
    let markdown = match source_dir {
        Some(source_dir) => rewrite_local_assets(markdown, source_dir, output_path)?,
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
        let base_dir = self.source_dir.to_path_buf();
        self.exported_destination_from(&base_dir, destination)
    }

    fn exported_css_destination_from(
        &mut self,
        base_dir: &Path,
        destination: &str,
    ) -> Result<Option<String>> {
        let Some(destination) = self.exported_destination_from(base_dir, destination)? else {
            return Ok(None);
        };

        let asset_prefix = format!("{}/", self.asset_dir_name);
        Ok(Some(
            destination
                .strip_prefix(&asset_prefix)
                .unwrap_or(&destination)
                .to_string(),
        ))
    }

    fn exported_destination_from(
        &mut self,
        base_dir: &Path,
        destination: &str,
    ) -> Result<Option<String>> {
        if should_leave_local_destination(destination) {
            return Ok(None);
        }

        let Some((source_path, reference_suffix)) = local_asset_source_path(base_dir, destination)
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

        let exported = format!("{}/{}", self.asset_dir_name, file_name);
        self.copied.insert(cache_key, exported.clone());
        self.copy_export_asset(&source_path, &target_path)?;
        Ok(Some(format!("{exported}{reference_suffix}")))
    }

    fn copy_export_asset(&mut self, source_path: &Path, target_path: &Path) -> Result<()> {
        if is_css_asset(source_path) {
            return self.copy_css_asset(source_path, target_path);
        }

        fs::copy(source_path, target_path).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;
        Ok(())
    }

    fn copy_css_asset(&mut self, source_path: &Path, target_path: &Path) -> Result<()> {
        let bytes = fs::read(source_path)
            .with_context(|| format!("failed to read {}", source_path.display()))?;
        let Ok(css) = String::from_utf8(bytes) else {
            fs::copy(source_path, target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
            return Ok(());
        };

        let base_dir = source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.source_dir.to_path_buf());
        let rewritten = rewrite_css_asset_urls(&css, &base_dir, self)?.unwrap_or(css);
        fs::write(target_path, rewritten)
            .with_context(|| format!("failed to write {}", target_path.display()))?;
        Ok(())
    }

    fn unique_asset_file_name(&mut self, source_path: &Path) -> String {
        let stem = source_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("asset");
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

fn rewrite_local_assets(markdown: &str, source_dir: &Path, output_path: &Path) -> Result<String> {
    let mut assets = ExportAssetContext::new(source_dir, output_path);
    let reference_labels = collect_asset_reference_labels(markdown);
    let mut out = String::with_capacity(markdown.len());
    let mut fence_marker: Option<FenceMarker> = None;
    let mut in_style_block = false;

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

        if !in_style_block
            && let Some(marker) = FenceMarker::opening(line.trim_start())
        {
            fence_marker = Some(marker);
            out.push_str(segment);
            continue;
        }

        out.push_str(&rewrite_line_with_style_assets(
            line,
            &mut assets,
            &reference_labels,
            &mut in_style_block,
        )?);
        out.push_str(newline);
    }

    Ok(out)
}

struct AssetReferenceLabels {
    images: HashSet<String>,
    links: HashSet<String>,
}

fn rewrite_line_assets(
    line: &str,
    assets: &mut ExportAssetContext<'_>,
    reference_labels: &AssetReferenceLabels,
) -> Result<String> {
    if let Some(definition) = parse_reference_definition(line) {
        let label = normalize_reference_label(definition.label);
        if reference_labels.images.contains(&label) || reference_labels.links.contains(&label) {
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
        } else if let Some(link) = parse_markdown_link(rest) {
            if let Some(destination) = assets.exported_destination(&link.destination)? {
                out.push('[');
                out.push_str(link.label);
                out.push_str("](");
                out.push_str(&markdown_link_destination(&destination));
                out.push_str(link.title_suffix);
                out.push(')');
            } else {
                out.push_str(link.raw);
            }
            index += link.raw.len();
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

fn rewrite_line_with_style_assets(
    line: &str,
    assets: &mut ExportAssetContext<'_>,
    reference_labels: &AssetReferenceLabels,
    in_style_block: &mut bool,
) -> Result<String> {
    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;

    while index < line.len() {
        let rest = &line[index..];
        if *in_style_block {
            if let Some(close_start) = find_html_end_tag(rest, "style") {
                let style = &rest[..close_start];
                if let Some(rewritten) = rewrite_css_urls(style, assets)? {
                    out.push_str(&rewritten);
                } else {
                    out.push_str(style);
                }

                let close = &rest[close_start..];
                let close_len = html_tag_close(close)
                    .map(|close| close + 1)
                    .unwrap_or(close.len());
                out.push_str(&close[..close_len]);
                index += close_start + close_len;
                *in_style_block = false;
            } else {
                if let Some(rewritten) = rewrite_css_urls(rest, assets)? {
                    out.push_str(&rewritten);
                } else {
                    out.push_str(rest);
                }
                index = line.len();
            }
            continue;
        }

        if let Some(open_start) = find_html_start_tag_outside_code_spans(rest, "style") {
            out.push_str(&rewrite_line_assets(
                &rest[..open_start],
                assets,
                reference_labels,
            )?);

            let open = &rest[open_start..];
            let open_len = html_tag_close(open)
                .map(|close| close + 1)
                .unwrap_or(open.len());
            out.push_str(&open[..open_len]);
            index += open_start + open_len;
            *in_style_block = true;
        } else {
            out.push_str(&rewrite_line_assets(rest, assets, reference_labels)?);
            index = line.len();
        }
    }

    Ok(out)
}

fn collect_asset_reference_labels(markdown: &str) -> AssetReferenceLabels {
    let mut labels = AssetReferenceLabels {
        images: HashSet::new(),
        links: HashSet::new(),
    };
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

        collect_asset_reference_labels_from_line(line, &mut labels);
    }

    labels
}

fn collect_asset_reference_labels_from_line(line: &str, labels: &mut AssetReferenceLabels) {
    if parse_reference_definition(line).is_some() {
        return;
    }

    let mut index = 0usize;
    while index < line.len() {
        let rest = &line[index..];
        if rest.starts_with('`') {
            let len = code_span_len(rest).unwrap_or(1);
            index += len;
        } else if let Some((raw_len, label)) = parse_image_reference_use(rest) {
            labels.images.insert(label);
            index += raw_len;
        } else if let Some((raw_len, label)) = parse_link_reference_use(rest) {
            labels.links.insert(label);
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

fn parse_link_reference_use(rest: &str) -> Option<(usize, String)> {
    if rest.starts_with("![") {
        return None;
    }
    let after_open = rest.strip_prefix('[')?;
    let close_label = find_unescaped_char(after_open, ']')?;
    let label = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];

    if after_label.starts_with('(') {
        return None;
    }

    if let Some(after_ref_open) = after_label.strip_prefix('[') {
        let close_ref = find_unescaped_char(after_ref_open, ']')?;
        let explicit_label = &after_ref_open[..close_ref];
        let label = if explicit_label.is_empty() {
            label
        } else {
            explicit_label
        };
        let normalized = normalize_reference_label(label);
        if normalized.is_empty() {
            return None;
        }
        let raw_len = 1 + close_label + 1 + 1 + close_ref + 1;
        Some((raw_len, normalized))
    } else {
        let normalized = normalize_reference_label(label);
        if normalized.is_empty() {
            return None;
        }
        let raw_len = 1 + close_label + 1;
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

struct MarkdownLink<'a> {
    raw: &'a str,
    label: &'a str,
    destination: String,
    title_suffix: &'a str,
}

struct HtmlAssetTag<'a> {
    raw: &'a str,
    href_range: Option<Range<usize>>,
    src_range: Option<Range<usize>>,
    data_range: Option<Range<usize>>,
    srcset_range: Option<Range<usize>>,
    poster_range: Option<Range<usize>>,
    style_range: Option<Range<usize>>,
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

fn parse_markdown_link(rest: &str) -> Option<MarkdownLink<'_>> {
    if rest.starts_with("![") {
        return None;
    }
    let after_open = rest.strip_prefix('[')?;
    let close_label = find_unescaped_char(after_open, ']')?;
    let label = &after_open[..close_label];
    let after_label = &after_open[close_label + 1..];
    let destination_body = after_label.strip_prefix('(')?;
    let close_destination = find_image_destination_close(destination_body)?;
    let inner = &destination_body[..close_destination];
    let (destination, title_suffix) = split_image_destination(inner)?;
    let raw_len = 1 + close_label + 1 + 1 + close_destination + 1;
    Some(MarkdownLink {
        raw: &rest[..raw_len],
        label,
        destination,
        title_suffix,
    })
}

fn parse_html_asset_tag(rest: &str) -> Option<HtmlAssetTag<'_>> {
    let raw_len = html_asset_tag_len(rest).or_else(|| html_tag_with_style_len(rest))?;
    let raw = &rest[..raw_len];
    let allows_src = html_asset_tag_allows_src(raw);
    let href_range = (html_named_tag_matches(raw, "a") || html_named_tag_matches(raw, "link"))
        .then(|| html_attr_value_range(raw, "href"))
        .flatten();
    let src_range = (allows_src == Some(true))
        .then(|| html_attr_value_range(raw, "src"))
        .flatten();
    let data_range = html_named_tag_matches(raw, "object")
        .then(|| html_attr_value_range(raw, "data"))
        .flatten();
    let srcset_range = html_attr_value_range(raw, "srcset");
    let poster_range = html_named_tag_matches(raw, "video")
        .then(|| html_attr_value_range(raw, "poster"))
        .flatten();
    let style_range = html_attr_value_range(raw, "style");
    if href_range.is_none()
        && src_range.is_none()
        && data_range.is_none()
        && srcset_range.is_none()
        && poster_range.is_none()
        && style_range.is_none()
    {
        return None;
    }

    Some(HtmlAssetTag {
        raw,
        href_range,
        src_range,
        data_range,
        srcset_range,
        poster_range,
        style_range,
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

    if let Some(range) = tag.href_range.clone() {
        let destination = decode_html_entities(&tag.raw[range.clone()]);
        if let Some(destination) = assets.exported_destination(&destination)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: escape_attr_value(&destination),
            });
        }
    }

    if let Some(range) = tag.src_range.clone() {
        let destination = decode_html_entities(&tag.raw[range.clone()]);
        if let Some(destination) = assets.exported_destination(&destination)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: escape_attr_value(&destination),
            });
        }
    }

    if let Some(range) = tag.data_range.clone() {
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

    if let Some(range) = tag.poster_range.clone() {
        let destination = decode_html_entities(&tag.raw[range.clone()]);
        if let Some(destination) = assets.exported_destination(&destination)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: escape_attr_value(&destination),
            });
        }
    }

    if let Some(range) = tag.style_range.clone() {
        let value = decode_html_entities(&tag.raw[range.clone()]);
        if let Some(style) = rewrite_css_urls(&value, assets)? {
            replacements.push(HtmlAttributeReplacement {
                range,
                value: escape_attr_value(&style),
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

struct CssUrlFunction {
    raw_len: usize,
    value_range: Range<usize>,
    quote: Option<char>,
}

fn rewrite_css_urls(value: &str, assets: &mut ExportAssetContext<'_>) -> Result<Option<String>> {
    let base_dir = assets.source_dir.to_path_buf();
    rewrite_css_references_with_base(value, &base_dir, false, assets)
}

fn rewrite_css_asset_urls(
    value: &str,
    base_dir: &Path,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    rewrite_css_references_with_base(value, base_dir, true, assets)
}

fn rewrite_css_references_with_base(
    value: &str,
    base_dir: &Path,
    asset_relative: bool,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    let mut changed = false;
    let mut rewritten = value.to_string();

    if let Some(imports) =
        rewrite_css_imports_with_base(&rewritten, base_dir, asset_relative, assets)?
    {
        rewritten = imports;
        changed = true;
    }

    if let Some(urls) = rewrite_css_urls_with_base(&rewritten, base_dir, asset_relative, assets)? {
        rewritten = urls;
        changed = true;
    }

    Ok(changed.then_some(rewritten))
}

struct CssImportString {
    raw_len: usize,
    value_range: Range<usize>,
    quote: char,
}

fn rewrite_css_imports_with_base(
    value: &str,
    base_dir: &Path,
    asset_relative: bool,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    let mut out = String::with_capacity(value.len());
    let mut changed = false;
    let mut index = 0usize;

    while index < value.len() {
        let rest = &value[index..];
        let Some(relative_start) = find_css_import_start(rest) else {
            break;
        };
        let start = index + relative_start;
        out.push_str(&value[index..start]);

        let Some(import) = parse_css_import_string(&value[start..]) else {
            out.push_str(&value[start..start + 1]);
            index = start + 1;
            continue;
        };

        let raw = &value[start..start + import.raw_len];
        let destination = decode_html_entities(&raw[import.value_range.clone()]);
        let destination = if asset_relative {
            assets.exported_css_destination_from(base_dir, &destination)?
        } else {
            assets.exported_destination_from(base_dir, &destination)?
        };
        if let Some(destination) = destination {
            out.push_str(&raw[..import.value_range.start]);
            out.push_str(&escape_css_string(&destination, import.quote));
            out.push_str(&raw[import.value_range.end..]);
            changed = true;
        } else {
            out.push_str(raw);
        }
        index = start + import.raw_len;
    }

    out.push_str(&value[index..]);
    Ok(changed.then_some(out))
}

fn find_css_import_start(value: &str) -> Option<usize> {
    let mut index = 0usize;
    while index < value.len() {
        let rest = &value[index..];
        if ascii_starts_with_ignore_case(rest, "@import")
            && !value[..index]
                .chars()
                .next_back()
                .is_some_and(is_css_identifier_char)
            && !rest
                .get(7..)
                .and_then(|after| after.chars().next())
                .is_some_and(is_css_identifier_char)
        {
            return Some(index);
        }

        let ch = rest.chars().next()?;
        index += ch.len_utf8();
    }

    None
}

fn parse_css_import_string(rest: &str) -> Option<CssImportString> {
    if !ascii_starts_with_ignore_case(rest, "@import") {
        return None;
    }

    let mut index = 7usize;
    index = skip_ascii_space(rest, index);
    let quote = match rest.as_bytes().get(index).copied() {
        Some(b'"') => '"',
        Some(b'\'') => '\'',
        _ => return None,
    };

    index += quote.len_utf8();
    let value_start = index;
    let close = find_unescaped_char(&rest[value_start..], quote)?;
    let value_end = value_start + close;
    Some(CssImportString {
        raw_len: value_end + quote.len_utf8(),
        value_range: value_start..value_end,
        quote,
    })
}

fn rewrite_css_urls_with_base(
    value: &str,
    base_dir: &Path,
    asset_relative: bool,
    assets: &mut ExportAssetContext<'_>,
) -> Result<Option<String>> {
    let mut out = String::with_capacity(value.len());
    let mut changed = false;
    let mut index = 0usize;

    while index < value.len() {
        let rest = &value[index..];
        let Some(relative_start) = find_css_url_start(rest) else {
            break;
        };
        let start = index + relative_start;
        out.push_str(&value[index..start]);

        let Some(function) = parse_css_url_function(&value[start..]) else {
            out.push_str(&value[start..start + 1]);
            index = start + 1;
            continue;
        };

        let raw = &value[start..start + function.raw_len];
        let destination = decode_html_entities(&raw[function.value_range.clone()]);
        let destination = if asset_relative {
            assets.exported_css_destination_from(base_dir, &destination)?
        } else {
            assets.exported_destination_from(base_dir, &destination)?
        };
        if let Some(destination) = destination {
            out.push_str(&raw[..function.value_range.start]);
            out.push_str(&css_url_replacement(&destination, function.quote));
            out.push_str(&raw[function.value_range.end..]);
            changed = true;
        } else {
            out.push_str(raw);
        }
        index = start + function.raw_len;
    }

    out.push_str(&value[index..]);
    Ok(changed.then_some(out))
}

fn find_css_url_start(value: &str) -> Option<usize> {
    let mut index = 0usize;
    while index < value.len() {
        let rest = &value[index..];
        if ascii_starts_with_ignore_case(rest, "url")
            && !value[..index]
                .chars()
                .next_back()
                .is_some_and(is_css_identifier_char)
            && rest
                .get(3..)
                .is_some_and(|after| skip_ascii_space(after, 0) < after.len())
        {
            return Some(index);
        }

        let ch = rest.chars().next()?;
        index += ch.len_utf8();
    }

    None
}

fn parse_css_url_function(rest: &str) -> Option<CssUrlFunction> {
    if !ascii_starts_with_ignore_case(rest, "url") {
        return None;
    }

    let mut index = 3usize;
    index = skip_ascii_space(rest, index);
    if rest.as_bytes().get(index) != Some(&b'(') {
        return None;
    }
    index += 1;
    index = skip_ascii_space(rest, index);

    let quote = match rest.as_bytes().get(index).copied() {
        Some(b'"') => Some('"'),
        Some(b'\'') => Some('\''),
        _ => None,
    };

    let value_start;
    let value_end;
    if let Some(quote) = quote {
        index += quote.len_utf8();
        value_start = index;
        let close = find_unescaped_char(&rest[value_start..], quote)?;
        value_end = value_start + close;
        index = value_end + quote.len_utf8();
    } else {
        value_start = index;
        let close = rest[value_start..].find(')')?;
        let raw_value_end = value_start + close;
        value_end = value_start + rest[value_start..raw_value_end].trim_end().len();
        index = raw_value_end;
    }

    index = skip_ascii_space(rest, index);
    if rest.as_bytes().get(index) != Some(&b')') {
        return None;
    }

    Some(CssUrlFunction {
        raw_len: index + 1,
        value_range: value_start..value_end,
        quote,
    })
}

fn css_url_replacement(destination: &str, quote: Option<char>) -> String {
    match quote {
        Some(quote) => escape_css_string(destination, quote),
        None if destination.chars().any(css_url_needs_quotes) => {
            format!("\"{}\"", escape_css_string(destination, '"'))
        }
        None => destination.to_string(),
    }
}

fn escape_css_string(value: &str, quote: char) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch == quote || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn css_url_needs_quotes(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '"' | '\'' | '(' | ')' | '\\')
}

fn is_css_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
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
    if html_named_tag_matches(rest, "a")
        || html_named_tag_matches(rest, "img")
        || html_named_tag_matches(rest, "audio")
        || html_named_tag_matches(rest, "video")
        || html_named_tag_matches(rest, "link")
        || html_named_tag_matches(rest, "embed")
        || html_named_tag_matches(rest, "iframe")
        || html_named_tag_matches(rest, "object")
        || html_named_tag_matches(rest, "script")
        || html_named_tag_matches(rest, "source")
        || html_named_tag_matches(rest, "track")
    {
        html_tag_close(rest).map(|close| close + 1)
    } else {
        None
    }
}

fn html_asset_tag_allows_src(rest: &str) -> Option<bool> {
    if html_named_tag_matches(rest, "img")
        || html_named_tag_matches(rest, "audio")
        || html_named_tag_matches(rest, "video")
        || html_named_tag_matches(rest, "embed")
        || html_named_tag_matches(rest, "iframe")
        || html_named_tag_matches(rest, "script")
        || html_named_tag_matches(rest, "source")
        || html_named_tag_matches(rest, "track")
    {
        Some(true)
    } else if html_named_tag_matches(rest, "a") {
        Some(false)
    } else if html_named_tag_matches(rest, "link") {
        Some(false)
    } else if html_named_tag_matches(rest, "object") {
        Some(false)
    } else {
        None
    }
}

fn html_tag_with_style_len(rest: &str) -> Option<usize> {
    if !html_start_tag_candidate(rest) {
        return None;
    }

    let len = html_tag_close(rest).map(|close| close + 1)?;
    html_attr_value_range(&rest[..len], "style")
        .is_some()
        .then_some(len)
}

fn html_start_tag_candidate(rest: &str) -> bool {
    if rest.as_bytes().first() != Some(&b'<') {
        return false;
    }

    let Some(next) = rest.as_bytes().get(1).copied() else {
        return false;
    };
    !matches!(next, b'/' | b'!' | b'?') && next.is_ascii_alphabetic()
}

fn find_html_start_tag_outside_code_spans(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0usize;
    while index < source.len() {
        let rest = &source[index..];
        if rest.starts_with('`') {
            index += code_span_len(rest).unwrap_or(1);
        } else if html_named_tag_matches(rest, tag_name) {
            return Some(index);
        } else if let Some(ch) = rest.chars().next() {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    None
}

fn find_html_end_tag(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0usize;
    while let Some(relative_start) = source[index..].find("</") {
        let start = index + relative_start;
        if html_named_end_tag_matches(&source[start..], tag_name) {
            return Some(start);
        }
        index = start + 2;
    }
    None
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

fn html_named_end_tag_matches(rest: &str, tag_name: &str) -> bool {
    let name_start = 2;
    let name_end = name_start + tag_name.len();
    if !rest.starts_with("</") {
        return false;
    }
    if !rest
        .get(name_start..name_end)
        .is_some_and(|name| name.eq_ignore_ascii_case(tag_name))
    {
        return false;
    }

    matches!(
        rest.as_bytes().get(name_end).copied(),
        Some(b'>') | Some(b' ' | b'\t' | b'\r' | b'\n')
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

fn skip_ascii_space(value: &str, mut index: usize) -> usize {
    while index < value.len() && value.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn ascii_starts_with_ignore_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
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

fn should_leave_local_destination(destination: &str) -> bool {
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

fn local_asset_source_path<'a>(
    source_dir: &Path,
    destination: &'a str,
) -> Option<(PathBuf, &'a str)> {
    let source_path = resolve_local_asset_path(source_dir, destination);
    if source_path.is_file() {
        return Some((source_path, ""));
    }

    let (path_part, suffix) = split_local_asset_reference_suffix(destination)?;
    if path_part.trim().is_empty() {
        return None;
    }
    let source_path = resolve_local_asset_path(source_dir, path_part);
    source_path.is_file().then_some((source_path, suffix))
}

fn split_local_asset_reference_suffix(destination: &str) -> Option<(&str, &str)> {
    let suffix_start = destination
        .char_indices()
        .find_map(|(index, ch)| matches!(ch, '?' | '#').then_some(index))?;
    Some((&destination[..suffix_start], &destination[suffix_start..]))
}

fn resolve_local_asset_path(source_dir: &Path, destination: &str) -> PathBuf {
    let destination = destination.trim();
    let path = Path::new(destination);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        source_dir.join(path)
    }
}

fn is_css_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
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
        format!(
            "<{}>",
            destination
                .replace('\\', r"\\")
                .replace('<', r"\<")
                .replace('>', r"\>")
        )
    } else {
        destination.to_string()
    }
}

fn wrap_tables_for_scroll(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(open_offset) = html[cursor..].find("<table>") {
        let open = cursor + open_offset;
        let after_open = open + "<table>".len();
        let Some(close_offset) = html[after_open..].find("</table>") else {
            break;
        };
        let close_end = after_open + close_offset + "</table>".len();

        out.push_str(&html[cursor..open]);
        out.push_str("<div class=\"table-scroll\">");
        out.push_str(&html[open..close_end]);
        out.push_str("</div>");
        cursor = close_end;
    }

    out.push_str(&html[cursor..]);
    out
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

    if let Some(pie) = parse_static_mermaid_pie_chart(source) {
        return render_static_pie_mermaid_svg(pie);
    }

    if let Some(class_diagram) = parse_static_mermaid_class_diagram(source) {
        return render_static_class_mermaid_svg(class_diagram);
    }

    if let Some(er_diagram) = parse_static_mermaid_er_diagram(source) {
        return render_static_er_mermaid_svg(er_diagram);
    }

    if let Some(state) = parse_static_mermaid_state_diagram(source) {
        return render_static_state_mermaid_svg(state, static_mermaid_marker_id(source));
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
    let mut subgraphs = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if let Some(parsed_direction) = static_mermaid_direction(line) {
            direction = parsed_direction;
            continue;
        }

        if let Some(subgraph) = parse_static_mermaid_subgraph(line) {
            subgraphs.push(subgraph);
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
        subgraphs,
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
    let subgraph_offset = if diagram.subgraphs.is_empty() {
        0
    } else {
        diagram.subgraphs.len() * 34 + 12
    };
    let width = if horizontal {
        margin * 2 + node_width * count + gap * count.saturating_sub(1)
    } else {
        margin * 2 + node_width
    };
    let height = subgraph_offset + if horizontal {
        margin * 2 + node_height
    } else {
        margin * 2 + node_height * count + gap * count.saturating_sub(1)
    };

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">\
<defs><marker id=\"{marker_id}\" viewBox=\"0 0 10 10\" refX=\"8\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"var(--muted)\"/></marker></defs>"
    );

    render_static_mermaid_subgraphs(&mut out, &diagram.subgraphs, width, margin);

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
            subgraph_offset,
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
        let (x, y) = static_mermaid_node_origin(
            index,
            horizontal,
            node_width,
            node_height,
            gap,
            margin,
            subgraph_offset,
        );
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
    subgraph_offset: usize,
) -> (usize, usize, usize, usize) {
    let (from_x, from_y) = static_mermaid_node_origin(
        from_index,
        horizontal,
        node_width,
        node_height,
        gap,
        margin,
        subgraph_offset,
    );
    let (to_x, to_y) = static_mermaid_node_origin(
        to_index,
        horizontal,
        node_width,
        node_height,
        gap,
        margin,
        subgraph_offset,
    );

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
    subgraph_offset: usize,
) -> (usize, usize) {
    if horizontal {
        (
            margin + index * (node_width + gap),
            margin + subgraph_offset,
        )
    } else {
        (
            margin,
            margin + subgraph_offset + index * (node_height + gap),
        )
    }
}

fn render_static_mermaid_subgraphs(
    out: &mut String,
    subgraphs: &[StaticMermaidSubgraph],
    width: usize,
    margin: usize,
) {
    let rect_width = width.saturating_sub(margin * 2);
    for (index, subgraph) in subgraphs.iter().enumerate() {
        let x = margin;
        let y = margin + index * 34;
        let label = if subgraph.id == subgraph.label {
            subgraph.label.clone()
        } else {
            format!("{} ({})", subgraph.label, subgraph.id)
        };
        out.push_str(&format!(
            "<g class=\"mermaid-subgraph\"><rect x=\"{x}\" y=\"{y}\" width=\"{rect_width}\" height=\"28\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\" stroke-dasharray=\"4 5\"/><text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-size=\"12\" fill=\"var(--muted)\">{}</text></g>",
            x + 12,
            y + 18,
            escape_html(&label)
        ));
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

fn parse_static_mermaid_pie_chart(source: &str) -> Option<StaticMermaidPieChart> {
    let mut saw_pie = false;
    let mut title = None;
    let mut slices = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_pie {
            if static_mermaid_pie_header(line) {
                saw_pie = true;
                continue;
            }
            continue;
        }

        if let Some(parsed_title) = parse_static_pie_title(line) {
            title = Some(parsed_title);
            continue;
        }

        if let Some(slice) = parse_static_pie_slice(line) {
            slices.push(slice);
        }
    }

    (!slices.is_empty()).then_some(StaticMermaidPieChart { title, slices })
}

fn render_static_pie_mermaid_svg(chart: StaticMermaidPieChart) -> String {
    let width = 560usize;
    let height = 260usize;
    let center_x = 132f64;
    let center_y = 142f64;
    let radius = 82f64;
    let total = static_pie_total(&chart.slices);
    let title = chart
        .title
        .as_deref()
        .filter(|title| !title.is_empty())
        .unwrap_or("Mermaid pie chart");
    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid pie chart preview\" xmlns=\"http://www.w3.org/2000/svg\">\
<text x=\"28\" y=\"30\" text-anchor=\"start\" font-size=\"14\" font-weight=\"600\" fill=\"var(--fg)\">{}</text>",
        escape_html(title)
    );

    if total > 0. {
        let mut angle = -std::f64::consts::FRAC_PI_2;
        for (index, slice) in chart.slices.iter().enumerate() {
            let sweep = slice.value / total * std::f64::consts::TAU;
            let color = static_pie_slice_color(index);
            if sweep >= std::f64::consts::TAU - 0.0001 {
                out.push_str(&format!(
                    "<circle class=\"mermaid-pie-slice\" cx=\"{center_x:.1}\" cy=\"{center_y:.1}\" r=\"{radius:.1}\" fill=\"{color}\"/>"
                ));
            } else if sweep > 0. {
                let end_angle = angle + sweep;
                let start_x = center_x + radius * angle.cos();
                let start_y = center_y + radius * angle.sin();
                let end_x = center_x + radius * end_angle.cos();
                let end_y = center_y + radius * end_angle.sin();
                let large_arc = usize::from(sweep > std::f64::consts::PI);
                out.push_str(&format!(
                    "<path class=\"mermaid-pie-slice\" d=\"M {center_x:.1} {center_y:.1} L {start_x:.1} {start_y:.1} A {radius:.1} {radius:.1} 0 {large_arc} 1 {end_x:.1} {end_y:.1} Z\" fill=\"{color}\"/>"
                ));
                angle = end_angle;
            }
        }
    }

    out.push_str(&format!(
        "<circle cx=\"{center_x:.1}\" cy=\"{center_y:.1}\" r=\"{radius:.1}\" fill=\"none\" stroke=\"var(--rule)\" stroke-width=\"1.5\"/>"
    ));

    for (index, slice) in chart.slices.iter().enumerate() {
        let y = 64 + index * 26;
        let color = static_pie_slice_color(index);
        let percent = if total > 0. {
            slice.value / total * 100.
        } else {
            0.
        };
        out.push_str(&format!(
            "<g class=\"mermaid-pie-legend\"><rect x=\"280\" y=\"{}\" width=\"14\" height=\"14\" rx=\"3\" fill=\"{color}\"/><text x=\"302\" y=\"{}\" text-anchor=\"start\" font-size=\"13\" fill=\"var(--fg)\">{}</text><text x=\"520\" y=\"{}\" text-anchor=\"end\" font-size=\"12\" fill=\"var(--muted)\">{} ({percent:.1}%)</text></g>",
            y.saturating_sub(11),
            y,
            escape_html(&slice.label),
            y,
            static_pie_value_text(slice.value)
        ));
    }

    out.push_str("</svg>");
    out
}

fn static_mermaid_pie_header(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let Some(head) = parts.next() else {
        return false;
    };
    head.eq_ignore_ascii_case("pie")
}

fn parse_static_pie_title(line: &str) -> Option<String> {
    let title = strip_static_ascii_prefix(line, "title ")?;
    let title = clean_static_mermaid_note_text(title);
    (!title.is_empty()).then_some(title)
}

fn parse_static_pie_slice(line: &str) -> Option<StaticMermaidPieSlice> {
    let (label_source, value_source) = line.split_once(':')?;
    let label = clean_static_mermaid_note_text(label_source);
    let value = value_source.trim().parse::<f64>().ok()?;

    (!label.is_empty() && value.is_finite() && value >= 0.).then_some(StaticMermaidPieSlice {
        label,
        value,
    })
}

fn static_pie_total(slices: &[StaticMermaidPieSlice]) -> f64 {
    slices.iter().map(|slice| slice.value).sum()
}

fn static_pie_value_text(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn static_pie_slice_color(index: usize) -> &'static str {
    const COLORS: &[&str] = &[
        "hsl(214 70% 54%)",
        "hsl(151 55% 41%)",
        "hsl(35 88% 55%)",
        "hsl(344 70% 58%)",
        "hsl(268 55% 62%)",
        "hsl(188 70% 43%)",
    ];
    COLORS[index % COLORS.len()]
}

fn parse_static_mermaid_class_diagram(source: &str) -> Option<StaticMermaidClassDiagram> {
    let mut saw_class = false;
    let mut classes = Vec::new();
    let mut class_indices = HashMap::new();
    let mut relationships = Vec::new();
    let mut current_class: Option<String> = None;

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_class {
            if static_mermaid_class_header(line) {
                saw_class = true;
                continue;
            }
            continue;
        }

        if let Some(class_id) = current_class.clone() {
            if line == "}" {
                current_class = None;
                continue;
            }

            let member = clean_static_mermaid_note_text(line);
            if !member.is_empty() {
                push_static_mermaid_class_member(
                    &mut classes,
                    &mut class_indices,
                    &class_id,
                    member,
                );
            }
            continue;
        }

        if line == "}" {
            continue;
        }

        if let Some(class) = parse_static_mermaid_class_block_start(line) {
            let id = class.id.clone();
            upsert_static_mermaid_class(&mut classes, &mut class_indices, class);
            current_class = Some(id);
            continue;
        }

        if let Some(relationship) = parse_static_mermaid_class_relationship(line) {
            upsert_static_mermaid_class(
                &mut classes,
                &mut class_indices,
                StaticMermaidClass {
                    id: relationship.from.clone(),
                    label: relationship.from.clone(),
                    members: Vec::new(),
                },
            );
            upsert_static_mermaid_class(
                &mut classes,
                &mut class_indices,
                StaticMermaidClass {
                    id: relationship.to.clone(),
                    label: relationship.to.clone(),
                    members: Vec::new(),
                },
            );
            relationships.push(relationship);
            continue;
        }

        if let Some((class_id, member)) = parse_static_mermaid_class_member_line(line) {
            push_static_mermaid_class_member(
                &mut classes,
                &mut class_indices,
                &class_id,
                member,
            );
            continue;
        }

        if let Some(class) = parse_static_mermaid_class_declaration(line) {
            upsert_static_mermaid_class(&mut classes, &mut class_indices, class);
        }
    }

    (!classes.is_empty() || !relationships.is_empty()).then_some(StaticMermaidClassDiagram {
        classes,
        class_indices,
        relationships,
    })
}

fn render_static_class_mermaid_svg(diagram: StaticMermaidClassDiagram) -> String {
    let width = 680usize;
    let margin = 28usize;
    let gap = 16usize;
    let card_width = (width - margin * 2 - gap) / 2;
    let relationship_row_height = 38usize;
    let relationship_area_height = if diagram.relationships.is_empty() {
        0
    } else {
        30 + diagram.relationships.len() * relationship_row_height + 12
    };

    let class_heights = diagram
        .classes
        .iter()
        .map(static_mermaid_class_card_height)
        .collect::<Vec<_>>();
    let mut column_heights = [0usize, 0usize];
    for (index, height) in class_heights.iter().enumerate() {
        let column = index % 2;
        column_heights[column] += height + gap;
    }
    let class_area_height = column_heights
        .into_iter()
        .max()
        .unwrap_or(0)
        .saturating_sub(gap);
    let height = margin * 2 + relationship_area_height + class_area_height.max(1);

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid class diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">"
    );

    let mut y = margin;
    if !diagram.relationships.is_empty() {
        out.push_str(&format!(
            "<text x=\"{margin}\" y=\"{y}\" text-anchor=\"start\" font-size=\"14\" font-weight=\"600\" fill=\"var(--fg)\">Relationships</text>"
        ));
        y += 20;
        for relationship in &diagram.relationships {
            out.push_str(&render_static_class_relationship(
                &diagram,
                relationship,
                margin,
                y,
                width - margin * 2,
                relationship_row_height - 6,
            ));
            y += relationship_row_height;
        }
        y += 12;
    }

    let mut column_y = [y, y];
    for (index, class) in diagram.classes.iter().enumerate() {
        let column = index % 2;
        let x = margin + column * (card_width + gap);
        out.push_str(&render_static_class_card(
            class,
            x,
            column_y[column],
            card_width,
            class_heights[index],
        ));
        column_y[column] += class_heights[index] + gap;
    }

    out.push_str("</svg>");
    out
}

fn render_static_class_relationship(
    diagram: &StaticMermaidClassDiagram,
    relationship: &StaticMermaidClassRelationship,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    let from = static_mermaid_class_label(diagram, &relationship.from);
    let to = static_mermaid_class_label(diagram, &relationship.to);
    let mut label = format!("{} {} {}", from, relationship.operator, to);
    if let Some(text) = relationship.label.as_ref().filter(|text| !text.is_empty()) {
        label.push_str(": ");
        label.push_str(text);
    }

    let text_x = x + 12;
    let center_y = y + height / 2;
    format!(
        "<g class=\"mermaid-class-relationship\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/><text x=\"{text_x}\" y=\"{center_y}\" text-anchor=\"start\" dominant-baseline=\"middle\" font-size=\"13\" fill=\"var(--fg)\">{}</text></g>",
        escape_html(&label)
    )
}

fn render_static_class_card(
    class: &StaticMermaidClass,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    let title = static_mermaid_class_display_text(class);
    let header_height = 34usize;
    let text_x = x + width / 2;
    let mut out = format!(
        "<g class=\"mermaid-class\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{header_height}\" rx=\"8\" fill=\"var(--fg)\" opacity=\"0.06\"/><text x=\"{text_x}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-size=\"13\" font-weight=\"600\" fill=\"var(--fg)\">{}</text>",
        y + header_height / 2,
        escape_html(&title)
    );
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"var(--rule)\"/>",
        y + header_height,
        x + width,
        y + header_height
    ));

    let mut member_y = y + header_height + 20;
    if class.members.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{member_y}\" text-anchor=\"start\" font-size=\"12\" fill=\"var(--muted)\">class</text>",
            x + 12
        ));
    } else {
        for member in &class.members {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{member_y}\" text-anchor=\"start\" font-size=\"12\" fill=\"var(--fg)\">{}</text>",
                x + 12,
                escape_html(member)
            ));
            member_y += 20;
        }
    }

    out.push_str("</g>");
    out
}

fn static_mermaid_class_card_height(class: &StaticMermaidClass) -> usize {
    34 + class.members.len().max(1) * 20 + 14
}

fn static_mermaid_class_display_text(class: &StaticMermaidClass) -> String {
    if class.id == class.label {
        class.label.clone()
    } else {
        format!("{} ({})", class.label, class.id)
    }
}

fn static_mermaid_class_label(diagram: &StaticMermaidClassDiagram, id: &str) -> String {
    diagram
        .class_indices
        .get(id)
        .and_then(|index| diagram.classes.get(*index))
        .map(static_mermaid_class_display_text)
        .unwrap_or_else(|| id.to_string())
}

fn static_mermaid_class_header(line: &str) -> bool {
    line.eq_ignore_ascii_case("classdiagram") || line.eq_ignore_ascii_case("classdiagram-v2")
}

fn parse_static_mermaid_class_block_start(line: &str) -> Option<StaticMermaidClass> {
    if !line.ends_with('{') {
        return None;
    }
    let rest = strip_static_sequence_keyword(line, "class")?;
    parse_static_mermaid_class_from_source(rest.trim_end_matches('{').trim())
}

fn parse_static_mermaid_class_declaration(line: &str) -> Option<StaticMermaidClass> {
    let rest = strip_static_sequence_keyword(line, "class")?;
    parse_static_mermaid_class_from_source(rest.trim())
}

fn parse_static_mermaid_class_from_source(source: &str) -> Option<StaticMermaidClass> {
    let node = parse_static_mermaid_node(source)?;
    Some(StaticMermaidClass {
        id: node.id,
        label: node.label,
        members: Vec::new(),
    })
}

fn parse_static_mermaid_class_member_line(line: &str) -> Option<(String, String)> {
    let (class_source, member_source) = line.split_once(':')?;
    let class_id = clean_static_mermaid_class_id(class_source);
    let member = clean_static_mermaid_note_text(member_source);
    (!class_id.is_empty() && !member.is_empty()).then_some((class_id, member))
}

fn parse_static_mermaid_class_relationship(line: &str) -> Option<StaticMermaidClassRelationship> {
    let (operator_start, operator_end, operator) =
        find_static_mermaid_class_relationship_operator(line)?;
    let from = clean_static_mermaid_class_id(&line[..operator_start]);
    let rest = line[operator_end..].trim();
    let (to_source, label_source) = rest.split_once(':').unwrap_or((rest, ""));
    let to = clean_static_mermaid_class_id(to_source);
    let label = clean_static_mermaid_note_text(label_source);

    (!from.is_empty() && !to.is_empty()).then_some(StaticMermaidClassRelationship {
        from,
        to,
        operator: operator.to_string(),
        label: (!label.is_empty()).then_some(label),
    })
}

fn find_static_mermaid_class_relationship_operator(
    line: &str,
) -> Option<(usize, usize, &'static str)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
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
                "<|--", "--|>", "<|..", "..|>", "<--", "-->", "<..", "..>", "*--", "--*",
                "o--", "--o", "--", "..",
            ] {
                if line[index..].starts_with(operator) {
                    return Some((index, index + operator.len(), operator));
                }
            }
        }
    }

    None
}

fn clean_static_mermaid_class_id(source: &str) -> String {
    let mut source = source
        .trim()
        .trim_matches(|ch: char| ch == ';' || ch == ',')
        .trim();

    loop {
        let stripped = strip_static_mermaid_class_cardinality(source);
        if stripped == source {
            break;
        }
        source = stripped;
    }

    clean_static_mermaid_label(source)
}

fn strip_static_mermaid_class_cardinality(source: &str) -> &str {
    let source = source.trim();
    if let Some(rest) = source.strip_prefix('"')
        && let Some(close) = rest.find('"')
    {
        return rest[close + 1..].trim();
    }
    if let Some(rest) = source.strip_suffix('"')
        && let Some(open) = rest.rfind('"')
    {
        return rest[..open].trim();
    }
    source
}

fn upsert_static_mermaid_class(
    classes: &mut Vec<StaticMermaidClass>,
    class_indices: &mut HashMap<String, usize>,
    class: StaticMermaidClass,
) {
    if let Some(index) = class_indices.get(&class.id).copied() {
        if classes[index].label == classes[index].id && class.label != class.id {
            classes[index].label = class.label;
        }
        for member in class.members {
            if !classes[index].members.contains(&member) {
                classes[index].members.push(member);
            }
        }
        return;
    }

    class_indices.insert(class.id.clone(), classes.len());
    classes.push(class);
}

fn push_static_mermaid_class_member(
    classes: &mut Vec<StaticMermaidClass>,
    class_indices: &mut HashMap<String, usize>,
    class_id: &str,
    member: String,
) {
    upsert_static_mermaid_class(
        classes,
        class_indices,
        StaticMermaidClass {
            id: class_id.to_string(),
            label: class_id.to_string(),
            members: vec![member],
        },
    );
}

fn parse_static_mermaid_er_diagram(source: &str) -> Option<StaticMermaidErDiagram> {
    let mut saw_er = false;
    let mut entities = Vec::new();
    let mut entity_indices = HashMap::new();
    let mut relationships = Vec::new();
    let mut current_entity: Option<String> = None;

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_er {
            if static_mermaid_er_header(line) {
                saw_er = true;
                continue;
            }
            continue;
        }

        if let Some(entity_id) = current_entity.clone() {
            if line == "}" {
                current_entity = None;
                continue;
            }

            let attribute = clean_static_mermaid_note_text(line);
            if !attribute.is_empty() {
                push_static_mermaid_er_attribute(
                    &mut entities,
                    &mut entity_indices,
                    &entity_id,
                    attribute,
                );
            }
            continue;
        }

        if line == "}" {
            continue;
        }

        if let Some(entity_id) = parse_static_mermaid_er_entity_block_start(line) {
            upsert_static_mermaid_er_entity(
                &mut entities,
                &mut entity_indices,
                StaticMermaidErEntity {
                    id: entity_id.clone(),
                    attributes: Vec::new(),
                },
            );
            current_entity = Some(entity_id);
            continue;
        }

        if let Some(relationship) = parse_static_mermaid_er_relationship(line) {
            upsert_static_mermaid_er_entity(
                &mut entities,
                &mut entity_indices,
                StaticMermaidErEntity {
                    id: relationship.from.clone(),
                    attributes: Vec::new(),
                },
            );
            upsert_static_mermaid_er_entity(
                &mut entities,
                &mut entity_indices,
                StaticMermaidErEntity {
                    id: relationship.to.clone(),
                    attributes: Vec::new(),
                },
            );
            relationships.push(relationship);
        }
    }

    (!entities.is_empty() || !relationships.is_empty()).then_some(StaticMermaidErDiagram {
        entities,
        entity_indices,
        relationships,
    })
}

fn render_static_er_mermaid_svg(diagram: StaticMermaidErDiagram) -> String {
    let width = 680usize;
    let margin = 28usize;
    let gap = 16usize;
    let card_width = (width - margin * 2 - gap) / 2;
    let relationship_row_height = 38usize;
    let relationship_area_height = if diagram.relationships.is_empty() {
        0
    } else {
        30 + diagram.relationships.len() * relationship_row_height + 12
    };

    let entity_heights = diagram
        .entities
        .iter()
        .map(static_mermaid_er_entity_card_height)
        .collect::<Vec<_>>();
    let mut column_heights = [0usize, 0usize];
    for (index, height) in entity_heights.iter().enumerate() {
        let column = index % 2;
        column_heights[column] += height + gap;
    }
    let entity_area_height = column_heights
        .into_iter()
        .max()
        .unwrap_or(0)
        .saturating_sub(gap);
    let height = margin * 2 + relationship_area_height + entity_area_height.max(1);

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid ER diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">"
    );

    let mut y = margin;
    if !diagram.relationships.is_empty() {
        out.push_str(&format!(
            "<text x=\"{margin}\" y=\"{y}\" text-anchor=\"start\" font-size=\"14\" font-weight=\"600\" fill=\"var(--fg)\">Relationships</text>"
        ));
        y += 20;
        for relationship in &diagram.relationships {
            out.push_str(&render_static_er_relationship(
                &diagram,
                relationship,
                margin,
                y,
                width - margin * 2,
                relationship_row_height - 6,
            ));
            y += relationship_row_height;
        }
        y += 12;
    }

    let mut column_y = [y, y];
    for (index, entity) in diagram.entities.iter().enumerate() {
        let column = index % 2;
        let x = margin + column * (card_width + gap);
        out.push_str(&render_static_er_entity_card(
            entity,
            x,
            column_y[column],
            card_width,
            entity_heights[index],
        ));
        column_y[column] += entity_heights[index] + gap;
    }

    out.push_str("</svg>");
    out
}

fn render_static_er_relationship(
    diagram: &StaticMermaidErDiagram,
    relationship: &StaticMermaidErRelationship,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    let from = static_mermaid_er_entity_label(diagram, &relationship.from);
    let to = static_mermaid_er_entity_label(diagram, &relationship.to);
    let mut label = format!("{} {} {}", from, relationship.cardinality, to);
    if let Some(text) = relationship.label.as_ref().filter(|text| !text.is_empty()) {
        label.push_str(": ");
        label.push_str(text);
    }

    let text_x = x + 12;
    let center_y = y + height / 2;
    format!(
        "<g class=\"mermaid-er-relationship\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/><text x=\"{text_x}\" y=\"{center_y}\" text-anchor=\"start\" dominant-baseline=\"middle\" font-size=\"13\" fill=\"var(--fg)\">{}</text></g>",
        escape_html(&label)
    )
}

fn render_static_er_entity_card(
    entity: &StaticMermaidErEntity,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    let header_height = 34usize;
    let text_x = x + width / 2;
    let mut out = format!(
        "<g class=\"mermaid-er-entity\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{header_height}\" rx=\"8\" fill=\"var(--fg)\" opacity=\"0.06\"/><text x=\"{text_x}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-size=\"13\" font-weight=\"600\" fill=\"var(--fg)\">{}</text>",
        y + header_height / 2,
        escape_html(&entity.id)
    );
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"var(--rule)\"/>",
        y + header_height,
        x + width,
        y + header_height
    ));

    let mut attribute_y = y + header_height + 20;
    if entity.attributes.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{attribute_y}\" text-anchor=\"start\" font-size=\"12\" fill=\"var(--muted)\">entity</text>",
            x + 12
        ));
    } else {
        for attribute in &entity.attributes {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{attribute_y}\" text-anchor=\"start\" font-size=\"12\" fill=\"var(--fg)\">{}</text>",
                x + 12,
                escape_html(attribute)
            ));
            attribute_y += 20;
        }
    }

    out.push_str("</g>");
    out
}

fn static_mermaid_er_entity_card_height(entity: &StaticMermaidErEntity) -> usize {
    34 + entity.attributes.len().max(1) * 20 + 14
}

fn static_mermaid_er_entity_label(diagram: &StaticMermaidErDiagram, id: &str) -> String {
    diagram
        .entity_indices
        .get(id)
        .and_then(|index| diagram.entities.get(*index))
        .map(|entity| entity.id.clone())
        .unwrap_or_else(|| id.to_string())
}

fn static_mermaid_er_header(line: &str) -> bool {
    line.eq_ignore_ascii_case("erdiagram")
}

fn parse_static_mermaid_er_entity_block_start(line: &str) -> Option<String> {
    if !line.ends_with('{') {
        return None;
    }
    let entity = clean_static_mermaid_er_entity_id(line.trim_end_matches('{'));
    (!entity.is_empty()).then_some(entity)
}

fn parse_static_mermaid_er_relationship(line: &str) -> Option<StaticMermaidErRelationship> {
    let (relationship_source, label_source) = line.split_once(':').unwrap_or((line, ""));
    let parts = relationship_source.split_whitespace().collect::<Vec<_>>();
    let operator_index = parts
        .iter()
        .position(|part| static_mermaid_er_relationship_operator(part))?;
    if operator_index == 0 || operator_index + 1 >= parts.len() {
        return None;
    }

    let from = clean_static_mermaid_er_entity_id(&parts[..operator_index].join(" "));
    let to = clean_static_mermaid_er_entity_id(&parts[operator_index + 1..].join(" "));
    let cardinality = parts[operator_index].trim().to_string();
    let label = clean_static_mermaid_note_text(label_source);

    (!from.is_empty() && !to.is_empty() && !cardinality.is_empty()).then_some(
        StaticMermaidErRelationship {
            from,
            to,
            cardinality,
            label: (!label.is_empty()).then_some(label),
        },
    )
}

fn static_mermaid_er_relationship_operator(part: &str) -> bool {
    (part.contains("--") || part.contains(".."))
        && part
            .chars()
            .all(|ch| matches!(ch, '|' | 'o' | 'O' | '{' | '}' | '-' | '.'))
}

fn clean_static_mermaid_er_entity_id(source: &str) -> String {
    clean_static_mermaid_label(source)
}

fn upsert_static_mermaid_er_entity(
    entities: &mut Vec<StaticMermaidErEntity>,
    entity_indices: &mut HashMap<String, usize>,
    entity: StaticMermaidErEntity,
) {
    if let Some(index) = entity_indices.get(&entity.id).copied() {
        for attribute in entity.attributes {
            if !entities[index].attributes.contains(&attribute) {
                entities[index].attributes.push(attribute);
            }
        }
        return;
    }

    entity_indices.insert(entity.id.clone(), entities.len());
    entities.push(entity);
}

fn push_static_mermaid_er_attribute(
    entities: &mut Vec<StaticMermaidErEntity>,
    entity_indices: &mut HashMap<String, usize>,
    entity_id: &str,
    attribute: String,
) {
    upsert_static_mermaid_er_entity(
        entities,
        entity_indices,
        StaticMermaidErEntity {
            id: entity_id.to_string(),
            attributes: vec![attribute],
        },
    );
}

fn parse_static_mermaid_state_diagram(source: &str) -> Option<StaticMermaidStateDiagram> {
    let mut saw_state = false;
    let mut states = Vec::new();
    let mut state_indices = HashMap::new();
    let mut transitions = Vec::new();
    let mut notes = Vec::new();
    let mut pending_note: Option<(StaticMermaidStateNotePlacement, String, Vec<String>)> = None;

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_state {
            if static_mermaid_is_state_diagram_header(line) {
                saw_state = true;
                continue;
            }
            continue;
        }

        let mut completed_note = None;
        let mut clear_pending_note = false;
        if let Some((placement, state, lines)) = pending_note.as_mut() {
            if static_state_note_end(line) {
                clear_pending_note = true;
                let text = lines
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !state.is_empty() && !text.is_empty() {
                    completed_note = Some(StaticMermaidStateNote {
                        placement: *placement,
                        state: state.clone(),
                        text,
                    });
                }
            } else {
                let text = clean_static_mermaid_note_text(line);
                if !text.is_empty() {
                    lines.push(text);
                }
            }
        }
        if let Some(note) = completed_note {
            upsert_static_state(
                &mut states,
                &mut state_indices,
                StaticMermaidState {
                    id: note.state.clone(),
                    label: note.state.clone(),
                },
            );
            notes.push(note);
        }
        if clear_pending_note {
            pending_note = None;
        }
        if clear_pending_note || pending_note.is_some() {
            continue;
        }

        if let Some((placement, state, text)) = parse_static_state_note_start(line) {
            upsert_static_state(
                &mut states,
                &mut state_indices,
                StaticMermaidState {
                    id: state.clone(),
                    label: state.clone(),
                },
            );
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                notes.push(StaticMermaidStateNote {
                    placement,
                    state,
                    text,
                });
            } else {
                pending_note = Some((placement, state, Vec::new()));
            }
            continue;
        }

        if let Some(transition) = parse_static_state_transition(line) {
            upsert_static_state_endpoint(
                &mut states,
                &mut state_indices,
                &transition.from,
                true,
            );
            upsert_static_state_endpoint(
                &mut states,
                &mut state_indices,
                &transition.to,
                false,
            );
            transitions.push(transition);
            continue;
        }

        if let Some(state) = parse_static_state_declaration(line) {
            upsert_static_state(&mut states, &mut state_indices, state);
            continue;
        }

        if static_state_non_transition_directive(line) {
            continue;
        }
    }

    (!states.is_empty() && !transitions.is_empty()).then_some(StaticMermaidStateDiagram {
        states,
        state_indices,
        transitions,
        notes,
    })
}

fn render_static_state_mermaid_svg(
    diagram: StaticMermaidStateDiagram,
    marker_id: String,
) -> String {
    let state_width = 170usize;
    let state_height = 48usize;
    let gap = 54usize;
    let margin = 28usize;
    let count = diagram.states.len().max(1);
    let width = margin * 2 + state_width * count + gap * count.saturating_sub(1);
    let note_top = margin + state_height + 42;
    let note_height = 34usize;
    let height = note_top + diagram.notes.len() * (note_height + 10) + margin;

    let mut out = format!(
        "<svg class=\"mermaid-static\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"Mermaid state diagram preview\" xmlns=\"http://www.w3.org/2000/svg\">\
<defs><marker id=\"{marker_id}\" viewBox=\"0 0 10 10\" refX=\"8\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"var(--muted)\"/></marker></defs>"
    );

    for transition in &diagram.transitions {
        render_static_state_transition(
            &mut out,
            &diagram,
            transition,
            &marker_id,
            state_width,
            state_height,
            gap,
            margin,
        );
    }

    for (index, state) in diagram.states.iter().enumerate() {
        let x = margin + index * (state_width + gap);
        let y = margin;
        render_static_state_node(&mut out, state, x, y, state_width, state_height);
    }

    for (index, note) in diagram.notes.iter().enumerate() {
        render_static_state_note(
            &mut out,
            &diagram,
            note,
            margin,
            note_top + index * (note_height + 10),
            width.saturating_sub(margin * 2),
            note_height,
        );
    }

    out.push_str("</svg>");
    out
}

fn render_static_state_transition(
    out: &mut String,
    diagram: &StaticMermaidStateDiagram,
    transition: &StaticMermaidStateTransition,
    marker_id: &str,
    state_width: usize,
    state_height: usize,
    gap: usize,
    margin: usize,
) {
    let Some(from_index) = diagram.state_indices.get(&transition.from).copied() else {
        return;
    };
    let Some(to_index) = diagram.state_indices.get(&transition.to).copied() else {
        return;
    };

    let y = margin + state_height / 2;
    if from_index == to_index {
        let x = margin + from_index * (state_width + gap);
        let right = x + state_width;
        let loop_top = margin.saturating_sub(14);
        out.push_str(&format!(
            "<path d=\"M {right} {y} C {} {loop_top}, {} {loop_top}, {right} {y}\" fill=\"none\" stroke=\"var(--muted)\" stroke-width=\"2\" marker-end=\"url(#{marker_id})\"/>",
            right + 34,
            right + 34
        ));
        if let Some(label) = &transition.label {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"var(--muted)\">{}</text>",
                right + 36,
                loop_top.saturating_sub(4),
                escape_html(label)
            ));
        }
        return;
    }

    let from_x = margin + from_index * (state_width + gap);
    let to_x = margin + to_index * (state_width + gap);
    let (x1, x2) = if from_index < to_index {
        (from_x + state_width, to_x)
    } else {
        (from_x, to_x + state_width)
    };
    out.push_str(&format!(
        "<line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"var(--muted)\" stroke-width=\"2\" marker-end=\"url(#{marker_id})\"/>"
    ));
    if let Some(label) = &transition.label {
        let label_x = (x1 + x2) / 2;
        let label_y = y.saturating_sub(10);
        out.push_str(&format!(
            "<text x=\"{label_x}\" y=\"{label_y}\" text-anchor=\"middle\" font-size=\"12\" fill=\"var(--muted)\">{}</text>",
            escape_html(label)
        ));
    }
}

fn render_static_state_node(
    out: &mut String,
    state: &StaticMermaidState,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    let label = static_state_display_text(state);
    out.push_str(&format!(
        "<g class=\"mermaid-state\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\"/>"
    ));
    out.push_str(&render_static_sequence_label(
        &label,
        x + width / 2,
        y + height / 2,
        24,
        2,
        "var(--fg)",
    ));
    out.push_str("</g>");
}

fn render_static_state_note(
    out: &mut String,
    diagram: &StaticMermaidStateDiagram,
    note: &StaticMermaidStateNote,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    let placement = match note.placement {
        StaticMermaidStateNotePlacement::LeftOf => "note left of",
        StaticMermaidStateNotePlacement::RightOf => "note right of",
    };
    let state = diagram
        .state_indices
        .get(&note.state)
        .and_then(|index| diagram.states.get(*index))
        .map(static_state_display_text)
        .unwrap_or_else(|| note.state.clone());
    let label = format!("{placement} {state}: {}", note.text);

    out.push_str(&format!(
        "<g class=\"mermaid-state-note\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\" stroke-dasharray=\"4 5\"/>"
    ));
    out.push_str(&render_static_sequence_label(
        &label,
        x + width / 2,
        y + height / 2,
        56,
        1,
        "var(--muted)",
    ));
    out.push_str("</g>");
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

fn static_mermaid_is_state_diagram_header(line: &str) -> bool {
    line.eq_ignore_ascii_case("statediagram") || line.eq_ignore_ascii_case("statediagram-v2")
}

fn parse_static_state_transition(line: &str) -> Option<StaticMermaidStateTransition> {
    let operator_start = line.find("-->")?;
    let from = static_state_transition_endpoint(&clean_static_state_id(&line[..operator_start]), true);
    let rest = line[operator_start + 3..].trim();
    let (to_source, label_source) = rest.split_once(':').unwrap_or((rest, ""));
    let to = static_state_transition_endpoint(&clean_static_state_id(to_source), false);
    let label = clean_static_mermaid_note_text(label_source);

    (!from.is_empty() && !to.is_empty()).then_some(StaticMermaidStateTransition {
        from,
        to,
        label: (!label.is_empty()).then_some(label),
    })
}

fn static_state_transition_endpoint(id: &str, is_from: bool) -> String {
    if id == "[*]" {
        static_state_terminal_id(is_from).to_string()
    } else {
        id.to_string()
    }
}

fn parse_static_state_declaration(line: &str) -> Option<StaticMermaidState> {
    let rest = if let Some(rest) = strip_static_sequence_keyword(line, "state") {
        rest.trim()
    } else if line.contains("-->") || line.starts_with("note ") {
        return None;
    } else {
        line
    };
    let rest = rest.trim_end_matches('{').trim();
    if rest.is_empty() || rest == "}" || static_state_is_terminal(rest) {
        return None;
    }

    if let Some((left, right)) = split_static_sequence_alias(rest) {
        let left = left.trim();
        let right = right.trim();
        let (id, label) = if left.starts_with('"') || left.starts_with('\'') {
            (clean_static_state_id(right), clean_static_mermaid_label(left))
        } else {
            (clean_static_state_id(left), clean_static_mermaid_label(right))
        };
        return (!id.is_empty()).then_some(StaticMermaidState {
            label: if label.is_empty() { id.clone() } else { label },
            id,
        });
    }

    if let Some((id_source, label_source)) = rest.split_once(':') {
        let id = clean_static_state_id(id_source);
        let label = clean_static_mermaid_note_text(label_source);
        return (!id.is_empty()).then_some(StaticMermaidState {
            label: if label.is_empty() { id.clone() } else { label },
            id,
        });
    }

    let id = clean_static_state_id(rest);
    (!id.is_empty()).then_some(StaticMermaidState {
        label: id.clone(),
        id,
    })
}

fn parse_static_state_note_start(
    line: &str,
) -> Option<(StaticMermaidStateNotePlacement, String, Option<String>)> {
    let rest = strip_static_ascii_prefix(line, "note ")?;
    let (placement, rest) = if let Some(rest) = strip_static_ascii_prefix(rest.trim(), "left of ") {
        (StaticMermaidStateNotePlacement::LeftOf, rest)
    } else if let Some(rest) = strip_static_ascii_prefix(rest.trim(), "right of ") {
        (StaticMermaidStateNotePlacement::RightOf, rest)
    } else {
        return None;
    };
    let (state_source, text) = rest.split_once(':').unwrap_or((rest, ""));
    let state = clean_static_state_id(state_source);
    let text = clean_static_mermaid_note_text(text);

    (!state.is_empty()).then_some((
        placement,
        state,
        (!text.is_empty()).then_some(text),
    ))
}

fn static_state_note_end(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower == "end note" || lower == "endnote"
}

fn static_state_non_transition_directive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower == "}"
        || lower == "end"
        || lower.starts_with("direction ")
        || lower.starts_with("classdef ")
        || lower.starts_with("class ")
        || lower.starts_with("style ")
        || lower.starts_with("hide empty description")
        || lower.starts_with("accdescr")
        || lower.starts_with("acctitle")
}

fn upsert_static_state_endpoint(
    states: &mut Vec<StaticMermaidState>,
    state_indices: &mut HashMap<String, usize>,
    endpoint: &str,
    is_from: bool,
) {
    let state = if static_state_is_terminal(endpoint) {
        StaticMermaidState {
            id: static_state_terminal_id(is_from).to_string(),
            label: if is_from { "start" } else { "end" }.to_string(),
        }
    } else {
        StaticMermaidState {
            id: endpoint.to_string(),
            label: endpoint.to_string(),
        }
    };
    upsert_static_state(states, state_indices, state);
}

fn upsert_static_state(
    states: &mut Vec<StaticMermaidState>,
    state_indices: &mut HashMap<String, usize>,
    state: StaticMermaidState,
) {
    if let Some(index) = state_indices.get(&state.id).copied() {
        if states[index].label == states[index].id && state.label != state.id {
            states[index].label = state.label;
        }
        return;
    }

    state_indices.insert(state.id.clone(), states.len());
    states.push(state);
}

fn static_state_display_text(state: &StaticMermaidState) -> String {
    if state.id == state.label
        || state.id == static_state_terminal_id(true)
        || state.id == static_state_terminal_id(false)
    {
        state.label.clone()
    } else {
        format!("{} ({})", state.label, state.id)
    }
}

fn static_state_is_terminal(id: &str) -> bool {
    id.trim() == "[*]"
        || id == static_state_terminal_id(true)
        || id == static_state_terminal_id(false)
}

fn static_state_terminal_id(is_from: bool) -> &'static str {
    if is_from {
        "__state_start"
    } else {
        "__state_end"
    }
}

fn clean_static_state_id(source: &str) -> String {
    let trimmed = source
        .trim()
        .trim_matches(|ch: char| ch == ';' || ch == ',')
        .trim();
    if trimmed == "[*]" {
        return "[*]".to_string();
    }

    clean_static_mermaid_label(source)
        .split("<<")
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim()
        .to_string()
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

fn parse_static_mermaid_subgraph(line: &str) -> Option<StaticMermaidSubgraph> {
    let rest = strip_static_ascii_prefix(line, "subgraph ")?.trim();
    let node = parse_static_mermaid_node(rest)?;
    (!node.id.is_empty() || !node.label.is_empty()).then_some(StaticMermaidSubgraph {
        id: node.id,
        label: node.label,
    })
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

fn clean_static_mermaid_note_text(source: &str) -> String {
    source
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
    let mut fragment_depth = 0usize;
    let mut autonumber: Option<(usize, usize)> = None;
    let mut items = Vec::new();

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

        if let Some(numbering) = parse_static_sequence_autonumber(line) {
            autonumber = Some(numbering);
            continue;
        }

        if let Some(participant) = parse_static_sequence_participant(line) {
            upsert_static_sequence_participant(
                &mut participants,
                &mut participant_indices,
                participant,
            );
            continue;
        }

        if let Some(note) = parse_static_sequence_note(line) {
            for participant_id in &note.participants {
                upsert_static_sequence_participant(
                    &mut participants,
                    &mut participant_indices,
                    StaticMermaidParticipant::new(participant_id),
                );
            }
            items.push(StaticMermaidSequenceItem::Note(note.clone()));
            continue;
        }

        if let Some(fragment) = parse_static_sequence_fragment(line) {
            if fragment.kind == StaticMermaidFragmentKind::End {
                if fragment_depth == 0 {
                    continue;
                }
                fragment_depth = fragment_depth.saturating_sub(1);
            } else if static_sequence_fragment_opens(fragment.kind) {
                fragment_depth += 1;
            }
            items.push(StaticMermaidSequenceItem::Fragment(fragment));
            continue;
        }

        if let Some(lifecycle) = parse_static_sequence_lifecycle(line) {
            upsert_static_sequence_participant(
                &mut participants,
                &mut participant_indices,
                StaticMermaidParticipant::new(&lifecycle.participant),
            );
            items.push(StaticMermaidSequenceItem::Lifecycle(lifecycle));
            continue;
        }

        if static_sequence_non_message_directive(line) {
            continue;
        }

        let Some(message) = parse_static_sequence_message(line) else {
            continue;
        };
        let mut message = message;
        if let Some((next, step)) = autonumber.as_mut() {
            message.number = Some(*next);
            *next = next.saturating_add(*step);
        }

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
        items.push(StaticMermaidSequenceItem::Message(message.clone()));
    }

    (saw_sequence && !participants.is_empty() && !items.is_empty()).then_some(
        StaticMermaidSequence {
            participants,
            participant_indices,
            items,
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

fn parse_static_sequence_note(line: &str) -> Option<StaticMermaidNote> {
    let rest = strip_static_ascii_prefix(line, "note ")?.trim();
    let (placement, rest) = if let Some(rest) = strip_static_ascii_prefix(rest, "left of ") {
        (StaticMermaidNotePlacement::LeftOf, rest)
    } else if let Some(rest) = strip_static_ascii_prefix(rest, "right of ") {
        (StaticMermaidNotePlacement::RightOf, rest)
    } else if let Some(rest) = strip_static_ascii_prefix(rest, "over ") {
        (StaticMermaidNotePlacement::Over, rest)
    } else {
        return None;
    };
    let (participants_source, text_source) = rest.split_once(':')?;
    let participants = participants_source
        .split(',')
        .map(clean_static_sequence_participant_id)
        .filter(|participant| !participant.is_empty())
        .collect::<Vec<_>>();
    let text = clean_static_mermaid_note_text(text_source);

    (!participants.is_empty() && !text.is_empty()).then_some(StaticMermaidNote {
        placement,
        participants,
        text,
    })
}

fn parse_static_sequence_fragment(line: &str) -> Option<StaticMermaidFragment> {
    for (keyword, kind) in [
        ("loop", StaticMermaidFragmentKind::Loop),
        ("alt", StaticMermaidFragmentKind::Alt),
        ("else", StaticMermaidFragmentKind::Else),
        ("opt", StaticMermaidFragmentKind::Opt),
        ("par", StaticMermaidFragmentKind::Par),
        ("and", StaticMermaidFragmentKind::And),
        ("critical", StaticMermaidFragmentKind::Critical),
        ("option", StaticMermaidFragmentKind::Option),
        ("break", StaticMermaidFragmentKind::Break),
        ("end", StaticMermaidFragmentKind::End),
    ] {
        if let Some(label) = strip_static_sequence_keyword(line, keyword) {
            return Some(StaticMermaidFragment {
                kind,
                label: clean_static_mermaid_note_text(label),
            });
        }
    }

    None
}

fn parse_static_sequence_autonumber(line: &str) -> Option<(usize, usize)> {
    let rest = strip_static_sequence_keyword(line, "autonumber")?;
    let mut numbers = rest
        .split_whitespace()
        .filter_map(|part| part.parse::<usize>().ok());
    let start = numbers.next().unwrap_or(1);
    let step = numbers.next().unwrap_or(1).max(1);
    Some((start, step))
}

fn parse_static_sequence_lifecycle(line: &str) -> Option<StaticMermaidLifecycle> {
    for (keyword, kind) in [
        ("activate", StaticMermaidLifecycleKind::Activate),
        ("deactivate", StaticMermaidLifecycleKind::Deactivate),
        ("destroy", StaticMermaidLifecycleKind::Destroy),
    ] {
        if let Some(participant) = strip_static_sequence_keyword(line, keyword) {
            let participant = clean_static_sequence_participant_id(participant);
            return (!participant.is_empty()).then_some(StaticMermaidLifecycle {
                kind,
                participant,
            });
        }
    }

    None
}

fn static_sequence_fragment_opens(kind: StaticMermaidFragmentKind) -> bool {
    matches!(
        kind,
        StaticMermaidFragmentKind::Loop
            | StaticMermaidFragmentKind::Alt
            | StaticMermaidFragmentKind::Opt
            | StaticMermaidFragmentKind::Par
            | StaticMermaidFragmentKind::Critical
            | StaticMermaidFragmentKind::Break
    )
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

fn strip_static_ascii_prefix<'a>(source: &'a str, prefix: &str) -> Option<&'a str> {
    let head = source.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then_some(&source[prefix.len()..])
}

fn strip_static_sequence_keyword<'a>(source: &'a str, keyword: &str) -> Option<&'a str> {
    let head = source.get(..keyword.len())?;
    if !head.eq_ignore_ascii_case(keyword) {
        return None;
    }

    let rest = &source[keyword.len()..];
    if rest.is_empty() {
        Some("")
    } else if rest.chars().next().is_some_and(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
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
        number: None,
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
    let item_gap = 60usize;
    let participant_count = sequence.participants.len().max(1);
    let item_count = sequence.items.len().max(1);
    let width = margin * 2
        + participant_width * participant_count
        + participant_gap * participant_count.saturating_sub(1);
    let height = message_start_y + item_gap * item_count + margin;

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

    for (index, item) in sequence.items.iter().enumerate() {
        let y = message_start_y + index * item_gap;
        match item {
            StaticMermaidSequenceItem::Message(message) => render_static_sequence_message(
                &mut out,
                &sequence,
                message,
                y,
                width,
                margin,
                participant_width,
                participant_gap,
                &marker_id,
            ),
            StaticMermaidSequenceItem::Note(note) => render_static_sequence_note(
                &mut out,
                &sequence,
                note,
                y,
                width,
                margin,
                participant_width,
                participant_gap,
            ),
            StaticMermaidSequenceItem::Fragment(fragment) => {
                render_static_sequence_fragment(&mut out, fragment, y, width, margin)
            }
            StaticMermaidSequenceItem::Lifecycle(lifecycle) => render_static_sequence_lifecycle(
                &mut out,
                &sequence,
                lifecycle,
                y,
                margin,
                participant_width,
                participant_gap,
            ),
        }
    }

    out.push_str("</svg>");
    out
}

fn render_static_sequence_message(
    out: &mut String,
    sequence: &StaticMermaidSequence,
    message: &StaticMermaidMessage,
    y: usize,
    width: usize,
    margin: usize,
    participant_width: usize,
    participant_gap: usize,
    marker_id: &str,
) {
    let Some(from_index) = sequence.participant_indices.get(&message.from).copied() else {
        return;
    };
    let Some(to_index) = sequence.participant_indices.get(&message.to).copied() else {
        return;
    };
    let from_x =
        static_sequence_participant_center_x(from_index, participant_width, participant_gap, margin);
    let to_x =
        static_sequence_participant_center_x(to_index, participant_width, participant_gap, margin);
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
        render_static_sequence_message_label(out, message, from_x + 28, y - 10);
    } else {
        out.push_str(&format!(
            "<line x1=\"{from_x}\" y1=\"{y}\" x2=\"{to_x}\" y2=\"{y}\" stroke=\"var(--muted)\" stroke-width=\"2\"{dash} marker-end=\"url(#{marker_id})\"/>"
        ));
        render_static_sequence_message_label(out, message, (from_x + to_x) / 2, y - 10);
    }
}

fn render_static_sequence_note(
    out: &mut String,
    sequence: &StaticMermaidSequence,
    note: &StaticMermaidNote,
    y: usize,
    width: usize,
    margin: usize,
    participant_width: usize,
    participant_gap: usize,
) {
    let centers = note
        .participants
        .iter()
        .filter_map(|participant| sequence.participant_indices.get(participant).copied())
        .map(|index| {
            static_sequence_participant_center_x(index, participant_width, participant_gap, margin)
        })
        .collect::<Vec<_>>();
    if centers.is_empty() {
        return;
    }

    let note_height = 42usize;
    let min_x = *centers.iter().min().unwrap();
    let max_x = *centers.iter().max().unwrap();
    let range_width = max_x.saturating_sub(min_x) + participant_width;
    let note_width = range_width.clamp(154, 260);
    let center_x = match note.placement {
        StaticMermaidNotePlacement::LeftOf => min_x.saturating_sub(participant_width / 2 + 24),
        StaticMermaidNotePlacement::RightOf => max_x + participant_width / 2 + 24,
        StaticMermaidNotePlacement::Over => (min_x + max_x) / 2,
    };
    let min_left = margin;
    let max_left = width.saturating_sub(margin + note_width);
    let x = center_x
        .saturating_sub(note_width / 2)
        .clamp(min_left, max_left);
    let rect_y = y.saturating_sub(note_height / 2);

    out.push_str(&format!(
        "<g class=\"mermaid-note\"><rect x=\"{x}\" y=\"{rect_y}\" width=\"{note_width}\" height=\"{note_height}\" rx=\"8\" fill=\"var(--mark-bg)\" stroke=\"var(--rule)\"/>"
    ));
    out.push_str(&render_static_sequence_label(
        &note.text,
        x + note_width / 2,
        rect_y + note_height / 2,
        30,
        2,
        "var(--fg)",
    ));
    out.push_str("</g>");
}

fn render_static_sequence_fragment(
    out: &mut String,
    fragment: &StaticMermaidFragment,
    y: usize,
    width: usize,
    margin: usize,
) {
    let fragment_height = 34usize;
    let x = margin;
    let rect_y = y.saturating_sub(fragment_height / 2);
    let rect_width = width.saturating_sub(margin * 2);
    let label = static_mermaid_fragment_display_text(fragment);

    out.push_str(&format!(
        "<g class=\"mermaid-fragment\"><rect x=\"{x}\" y=\"{rect_y}\" width=\"{rect_width}\" height=\"{fragment_height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--rule)\" stroke-dasharray=\"4 5\"/>"
    ));
    out.push_str(&render_static_sequence_label(
        &label,
        x + rect_width / 2,
        rect_y + fragment_height / 2,
        42,
        1,
        "var(--muted)",
    ));
    out.push_str("</g>");
}

fn render_static_sequence_lifecycle(
    out: &mut String,
    sequence: &StaticMermaidSequence,
    lifecycle: &StaticMermaidLifecycle,
    y: usize,
    margin: usize,
    participant_width: usize,
    participant_gap: usize,
) {
    let Some(index) = sequence
        .participant_indices
        .get(&lifecycle.participant)
        .copied()
    else {
        return;
    };

    let center_x =
        static_sequence_participant_center_x(index, participant_width, participant_gap, margin);
    let width = 128usize;
    let height = 30usize;
    let x = center_x.saturating_sub(width / 2);
    let rect_y = y.saturating_sub(height / 2);
    let label = static_sequence_lifecycle_display_text(sequence, lifecycle);

    out.push_str(&format!(
        "<g class=\"mermaid-lifecycle\"><rect x=\"{x}\" y=\"{rect_y}\" width=\"{width}\" height=\"{height}\" rx=\"8\" fill=\"var(--code-bg)\" stroke=\"var(--muted)\"/>"
    ));
    out.push_str(&render_static_sequence_label(
        &label,
        x + width / 2,
        rect_y + height / 2,
        28,
        1,
        "var(--muted)",
    ));

    if lifecycle.kind == StaticMermaidLifecycleKind::Destroy {
        let cross_left = center_x.saturating_sub(9);
        let cross_right = center_x + 9;
        let cross_top = rect_y + height + 6;
        let cross_bottom = cross_top + 18;
        out.push_str(&format!(
            "<line x1=\"{cross_left}\" y1=\"{cross_top}\" x2=\"{cross_right}\" y2=\"{cross_bottom}\" stroke=\"var(--muted)\" stroke-width=\"2\"/><line x1=\"{cross_right}\" y1=\"{cross_top}\" x2=\"{cross_left}\" y2=\"{cross_bottom}\" stroke=\"var(--muted)\" stroke-width=\"2\"/>"
        ));
    }

    out.push_str("</g>");
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

fn render_static_sequence_message_label(
    out: &mut String,
    message: &StaticMermaidMessage,
    x: usize,
    y: usize,
) {
    let label = static_sequence_message_display_text(message);
    if label.is_empty() {
        return;
    }

    out.push_str(&render_static_sequence_label(
        &label,
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
    subgraphs: Vec<StaticMermaidSubgraph>,
}

#[derive(Clone)]
struct StaticMermaidNode {
    id: String,
    label: String,
}

#[derive(Clone)]
struct StaticMermaidSubgraph {
    id: String,
    label: String,
}

struct StaticMermaidEdge {
    from: StaticMermaidNode,
    to: StaticMermaidNode,
    label: Option<String>,
}

struct StaticMermaidPieChart {
    title: Option<String>,
    slices: Vec<StaticMermaidPieSlice>,
}

struct StaticMermaidPieSlice {
    label: String,
    value: f64,
}

struct StaticMermaidClassDiagram {
    classes: Vec<StaticMermaidClass>,
    class_indices: HashMap<String, usize>,
    relationships: Vec<StaticMermaidClassRelationship>,
}

#[derive(Clone)]
struct StaticMermaidClass {
    id: String,
    label: String,
    members: Vec<String>,
}

struct StaticMermaidClassRelationship {
    from: String,
    to: String,
    operator: String,
    label: Option<String>,
}

struct StaticMermaidErDiagram {
    entities: Vec<StaticMermaidErEntity>,
    entity_indices: HashMap<String, usize>,
    relationships: Vec<StaticMermaidErRelationship>,
}

#[derive(Clone)]
struct StaticMermaidErEntity {
    id: String,
    attributes: Vec<String>,
}

struct StaticMermaidErRelationship {
    from: String,
    to: String,
    cardinality: String,
    label: Option<String>,
}

struct StaticMermaidStateDiagram {
    states: Vec<StaticMermaidState>,
    state_indices: HashMap<String, usize>,
    transitions: Vec<StaticMermaidStateTransition>,
    notes: Vec<StaticMermaidStateNote>,
}

struct StaticMermaidState {
    id: String,
    label: String,
}

struct StaticMermaidStateTransition {
    from: String,
    to: String,
    label: Option<String>,
}

struct StaticMermaidStateNote {
    placement: StaticMermaidStateNotePlacement,
    state: String,
    text: String,
}

#[derive(Clone, Copy)]
enum StaticMermaidStateNotePlacement {
    LeftOf,
    RightOf,
}

struct StaticMermaidSequence {
    participants: Vec<StaticMermaidParticipant>,
    participant_indices: HashMap<String, usize>,
    items: Vec<StaticMermaidSequenceItem>,
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

#[derive(Clone)]
struct StaticMermaidMessage {
    from: String,
    to: String,
    label: String,
    dashed: bool,
    number: Option<usize>,
}

#[derive(Clone)]
struct StaticMermaidNote {
    placement: StaticMermaidNotePlacement,
    participants: Vec<String>,
    text: String,
}

#[derive(Clone, Copy)]
enum StaticMermaidNotePlacement {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Clone)]
struct StaticMermaidFragment {
    kind: StaticMermaidFragmentKind,
    label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StaticMermaidFragmentKind {
    Loop,
    Alt,
    Else,
    Opt,
    Par,
    And,
    Critical,
    Option,
    Break,
    End,
}

#[derive(Clone)]
struct StaticMermaidLifecycle {
    kind: StaticMermaidLifecycleKind,
    participant: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StaticMermaidLifecycleKind {
    Activate,
    Deactivate,
    Destroy,
}

#[derive(Clone)]
enum StaticMermaidSequenceItem {
    Message(StaticMermaidMessage),
    Note(StaticMermaidNote),
    Fragment(StaticMermaidFragment),
    Lifecycle(StaticMermaidLifecycle),
}

fn static_mermaid_fragment_display_text(fragment: &StaticMermaidFragment) -> String {
    let kind = static_mermaid_fragment_label(fragment.kind);
    if fragment.label.is_empty() {
        kind.to_string()
    } else {
        format!("{kind}: {}", fragment.label)
    }
}

fn static_mermaid_fragment_label(kind: StaticMermaidFragmentKind) -> &'static str {
    match kind {
        StaticMermaidFragmentKind::Loop => "loop",
        StaticMermaidFragmentKind::Alt => "alt",
        StaticMermaidFragmentKind::Else => "else",
        StaticMermaidFragmentKind::Opt => "opt",
        StaticMermaidFragmentKind::Par => "par",
        StaticMermaidFragmentKind::And => "and",
        StaticMermaidFragmentKind::Critical => "critical",
        StaticMermaidFragmentKind::Option => "option",
        StaticMermaidFragmentKind::Break => "break",
        StaticMermaidFragmentKind::End => "end",
    }
}

fn static_sequence_lifecycle_display_text(
    sequence: &StaticMermaidSequence,
    lifecycle: &StaticMermaidLifecycle,
) -> String {
    let action = static_sequence_lifecycle_label(lifecycle.kind);
    let participant = sequence
        .participant_indices
        .get(&lifecycle.participant)
        .and_then(|index| sequence.participants.get(*index))
        .map(|participant| participant.label.as_str())
        .unwrap_or(lifecycle.participant.as_str());

    format!("{action} {participant}")
}

fn static_sequence_lifecycle_label(kind: StaticMermaidLifecycleKind) -> &'static str {
    match kind {
        StaticMermaidLifecycleKind::Activate => "activate",
        StaticMermaidLifecycleKind::Deactivate => "deactivate",
        StaticMermaidLifecycleKind::Destroy => "destroy",
    }
}

fn static_sequence_message_display_text(message: &StaticMermaidMessage) -> String {
    match (message.number, message.label.trim()) {
        (Some(number), "") => format!("{number}."),
        (Some(number), label) => format!("{number}. {label}"),
        (None, label) => label.to_string(),
    }
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
  overflow-wrap: break-word;
}}
h1, h2, h3, h4, h5, h6 {{ line-height: 1.25; margin: 1.8em 0 .65em; }}
p, ul, ol, blockquote, pre, .table-scroll {{ margin: 1em 0; }}
a {{ color: var(--accent); }}
picture {{ display: block; }}
img, svg {{ max-width: 100%; height: auto; }}
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
pre code {{ padding: 0; white-space: inherit; background: transparent; }}
.table-scroll {{
  max-width: 100%;
  overflow-x: auto;
}}
table {{ border-collapse: collapse; width: 100%; min-width: 100%; }}
th, td {{
  border: 1px solid var(--rule);
  padding: .45em .65em;
  vertical-align: top;
  overflow-wrap: anywhere;
}}
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
  p, blockquote, pre, .table-scroll, table, ul, ol, .math-block-wrap, .mermaid-diagram, .callout {{
    break-inside: avoid;
    page-break-inside: avoid;
  }}
  pre {{ white-space: pre-wrap; overflow-wrap: anywhere; }}
  .table-scroll {{ overflow: visible; }}
  table {{ table-layout: fixed; width: 100%; min-width: 0; }}
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
        assert!(html.contains(".table-scroll {\n  max-width: 100%;\n  overflow-x: auto;"));
        assert!(html.contains(".table-scroll { overflow: visible; }"));
        assert!(html.contains("table { table-layout: fixed; width: 100%; min-width: 0; }"));
        assert!(html.contains("overflow-wrap: anywhere;"));
        assert!(html.contains("break-after: avoid"));
        assert!(html.contains("page-break-inside: avoid"));
        assert!(html.contains("a[href^=\"http\"]::after"));
    }

    #[test]
    fn exported_tables_are_wrapped_for_scroll_and_print() {
        let html = export_markdown_to_html(
            "| Path | Status |\n| --- | --- |\n| workspace/assets/very-long-unbroken-name.md | Ready |",
            "Table",
        )
        .unwrap();

        assert!(html.contains("<div class=\"table-scroll\"><table>"));
        assert!(html.contains("</table></div>"));
        assert!(html.contains("workspace/assets/very-long-unbroken-name.md"));
    }

    #[test]
    fn raw_html_tables_with_attributes_are_not_partially_wrapped() {
        let html = export_markdown_to_html(
            "<table class=\"raw\"><tr><td>Raw</td></tr></table>",
            "Raw Table",
        )
        .unwrap();

        assert!(html.contains("<table class=\"raw\"><tr><td>Raw</td></tr></table>"));
        assert!(!html.contains("</table></div>"));
    }

    #[test]
    fn exported_markdown_destinations_escape_angle_brackets_when_wrapped() {
        assert_eq!(
            markdown_link_destination("./assets/a<b>.png"),
            r"<./assets/a\<b\>.png>"
        );
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
        assert!(html.contains("<a href=\"#layout-stress\">Layout Stress</a>"));
        assert!(html.contains("<h2 id=\"editing-blocks\">Editing Blocks</h2>"));
        assert!(html.contains("<h2 id=\"layout-stress\">Layout Stress</h2>"));
        assert!(html.contains("<div class=\"table-scroll\"><table>"));
        assert!(html.contains("very-long-reference-name-without-natural-breaks.md"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<code class=\"language-rust\">"));
        assert!(html.contains("<code class=\"language-text\">"));
        assert!(html.contains("src=\"longform_assets/cover.svg#acceptance-cover\""));
        assert!(html.contains("src=\"longform_assets/cover.svg\""));
        assert!(html.contains("src=\"longform_assets/reference-diagram.svg\""));
        assert!(html.contains("href=\"longform_assets/appendix.md#notes\""));
        assert!(html.contains("href=\"longform_assets/appendix.md?raw=1#notes\""));
        assert!(html.contains("href=\"longform_assets/acceptance.css\""));
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
        assert!(html.contains("C(n, k) = C(n, k) = n!/k!(n-k)!"));
        assert!(
            html.contains("x\u{0302} + v\u{20D7} + y\u{0307} + ⏟(a + b)_group")
        );
        assert!(html.contains("<div class=\"math-fallback math-block-fallback\">a b; c d</div>"));
        assert!(html.contains("x = y + 1 z = x - 1"));
        assert!(!html.contains("<div class=\"math-fallback math-block-fallback\">cc"));
        assert!(!html.contains("<div class=\"math-fallback math-block-fallback\">2 x"));
        assert!(html.contains("\\begin{aligned}"));
        assert!(html.contains("E &amp;= mc^2"));
        assert!(html.contains("mathjax@3"));
        assert!(html.contains("math-rendered"));
        assert!(html.contains("mermaid.esm.min.mjs"));
        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("class=\"mermaid-subgraph\""));
        assert!(html.contains(">Writing workflow (Writing)</text>"));
        assert!(html.contains(">Export HTML<"));
        assert!(html.contains("Mermaid sequence diagram preview"));
        assert!(html.contains("class=\"mermaid-lifecycle\""));
        assert!(html.contains(">activate Vellum</tspan>"));
        assert!(html.contains(">destroy Browser</tspan>"));
        assert!(html.contains(">1. Export print HTML</tspan>"));
        assert!(html.contains(">3. Save PDF</tspan>"));
        assert!(html.contains("Mermaid state diagram preview"));
        assert!(html.contains(">Review queue (Review)</tspan>"));
        assert!(html.contains(">Save changes</text>"));
        assert!(html.contains(">note right of Review queue"));
        assert!(html.contains("Mermaid pie chart preview"));
        assert!(html.contains(">Export coverage</text>"));
        assert!(html.contains("class=\"mermaid-pie-legend\""));
        assert!(html.contains(">HTML export</text>"));
        assert!(html.contains("Mermaid class diagram preview"));
        assert!(html.contains("class=\"mermaid-class\""));
        assert!(html.contains("class=\"mermaid-class-relationship\""));
        assert!(html.contains(">Markdown document (Document)</text>"));
        assert!(html.contains(">+export_html()</text>"));
        assert!(html.contains("LongformNote --&gt; AssetStore: writes assets"));
        assert!(html.contains("Mermaid ER diagram preview"));
        assert!(html.contains("class=\"mermaid-er-entity\""));
        assert!(html.contains("class=\"mermaid-er-relationship\""));
        assert!(html.contains(">DOCUMENT</text>"));
        assert!(html.contains(">string title PK</text>"));
        assert!(html.contains("DOCUMENT ||--o{ ASSET: owns"));
        assert!(html.contains("DOCUMENT }o..|| WORKSPACE: belongs_to"));
        assert!(html.contains("data-footnotes"));
        assert!(html.contains("href=\"#user-content-fn-acceptance\""));
        assert!(html.contains("id=\"user-content-fn-acceptance\""));
        assert!(html.contains("href=\"#user-content-fnref-acceptance\""));
        assert!(html.contains("This footnote checks GFM footnote rendering"));
        assert!(html.contains("@media print"));
        let css = std::fs::read_to_string(root.join("longform_assets/acceptance.css")).unwrap();
        assert!(css.contains("@import \"acceptance-theme.css?print=1#screen\""));
        assert!(css.contains("url(\"cover.svg#style-cover\")"));
        assert!(!css.contains("longform_assets/cover.svg"));
        let imported_css =
            std::fs::read_to_string(root.join("longform_assets/acceptance-theme.css")).unwrap();
        assert!(imported_css.contains("url(reference-diagram.svg#theme-diagram)"));
        assert!(root.join("longform_assets/cover.svg").is_file());
        assert!(root.join("longform_assets/reference-diagram.svg").is_file());
        assert!(root.join("longform_assets/appendix.md").is_file());
        assert!(root.join("longform_assets/acceptance.css").is_file());
        assert!(root.join("longform_assets/acceptance-theme.css").is_file());

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
    fn html_file_export_copies_raw_html_media_assets() {
        let root = temp_export_dir("html-raw-media-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/poster.png"), b"poster").unwrap();
        std::fs::write(source.join("assets/clip.mp4"), b"clip").unwrap();
        std::fs::write(source.join("assets/clip.webm"), b"webm").unwrap();
        std::fs::write(source.join("assets/captions.vtt"), b"captions").unwrap();
        std::fs::write(source.join("assets/audio.mp3"), b"audio").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "<video controls poster=\"assets/poster.png\" src=\"assets/clip.mp4\">",
                "<source src='assets/clip.webm' type=\"video/webm\">",
                "<track kind=\"captions\" src=\"assets/captions.vtt\">",
                "</video>\n",
                "<audio controls src=\"assets/audio.mp3\"></audio>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("poster=\"article_assets/poster.png\""));
        assert!(html.contains("src=\"article_assets/clip.mp4\""));
        assert!(html.contains("src='article_assets/clip.webm'"));
        assert!(html.contains("src=\"article_assets/captions.vtt\""));
        assert!(html.contains("src=\"article_assets/audio.mp3\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/poster.png")).unwrap(),
            b"poster"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/clip.mp4")).unwrap(),
            b"clip"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/clip.webm")).unwrap(),
            b"webm"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/captions.vtt")).unwrap(),
            b"captions"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/audio.mp3")).unwrap(),
            b"audio"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_embedded_document_assets() {
        let root = temp_export_dir("html-raw-embedded-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(source.join("assets/fonts")).unwrap();
        std::fs::create_dir_all(source.join("assets/images")).unwrap();
        std::fs::create_dir_all(source.join("assets/theme")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(
            source.join("assets/article.css"),
            concat!(
                "@import \"theme/base.css?theme=print#screen\";\n",
                ".hero { background: url(\"images/bg.png#hero\"); }\n",
                "@font-face { src: url(fonts/body.woff2?cache=1) format(\"woff2\"); }\n",
                ".remote { background: url(https://example.com/remote.png); }\n",
            )
            .as_bytes(),
        )
        .unwrap();
        std::fs::write(
            source.join("assets/theme/base.css"),
            ".paper { background: url(../images/paper.png); }",
        )
        .unwrap();
        std::fs::write(source.join("assets/images/bg.png"), b"bg").unwrap();
        std::fs::write(source.join("assets/images/paper.png"), b"paper").unwrap();
        std::fs::write(source.join("assets/fonts/body.woff2"), b"font").unwrap();
        std::fs::write(source.join("assets/frame.html"), b"frame").unwrap();
        std::fs::write(source.join("assets/widget.svg"), b"widget").unwrap();
        std::fs::write(source.join("assets/report.pdf"), b"report").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "<link rel=\"stylesheet\" href=\"assets/article.css\">\n",
                "<iframe src=\"assets/frame.html#draft\"></iframe>\n",
                "<embed src='assets/widget.svg' type=\"image/svg+xml\">\n",
                "<object data=\"assets/report.pdf?download=1#page-2\"></object>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("href=\"article_assets/article.css\""));
        assert!(html.contains("src=\"article_assets/frame.html#draft\""));
        assert!(html.contains("src='article_assets/widget.svg'"));
        assert!(html.contains("data=\"article_assets/report.pdf?download=1#page-2\""));
        let css = std::fs::read_to_string(export_dir.join("article_assets/article.css")).unwrap();
        assert!(css.contains("@import \"base.css?theme=print#screen\""));
        assert!(css.contains("url(\"bg.png#hero\")"));
        assert!(css.contains("url(body.woff2?cache=1) format(\"woff2\")"));
        assert!(css.contains("url(https://example.com/remote.png)"));
        assert!(!css.contains("article_assets/bg.png"));
        let base_css = std::fs::read_to_string(export_dir.join("article_assets/base.css")).unwrap();
        assert!(base_css.contains("url(paper.png)"));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/bg.png")).unwrap(),
            b"bg"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/paper.png")).unwrap(),
            b"paper"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/body.woff2")).unwrap(),
            b"font"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/frame.html")).unwrap(),
            b"frame"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/widget.svg")).unwrap(),
            b"widget"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/report.pdf")).unwrap(),
            b"report"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_script_assets() {
        let root = temp_export_dir("html-raw-script-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/app.js"), b"console.log('ok')").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            "<script defer src=\"assets/app.js?cache=1#boot\"></script>",
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("src=\"article_assets/app.js?cache=1#boot\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/app.js")).unwrap(),
            b"console.log('ok')"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_style_attribute_assets() {
        let root = temp_export_dir("html-raw-style-attribute-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/bg image.png"), b"background").unwrap();
        std::fs::write(source.join("assets/mask.svg"), b"mask").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "<section class=\"hero\" style=\"",
                "background: url('assets/bg image.png?cache=1&amp;theme=print#hero'); ",
                "mask-image: url(assets/mask.svg);",
                "\">Hero</section>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains(
            "background: url('article_assets/bg image.png?cache=1&amp;theme=print#hero')"
        ));
        assert!(html.contains("mask-image: url(article_assets/mask.svg)"));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/bg image.png")).unwrap(),
            b"background"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/mask.svg")).unwrap(),
            b"mask"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_style_block_assets() {
        let root = temp_export_dir("html-raw-style-block-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/print.css"), b"print").unwrap();
        std::fs::write(source.join("assets/hero.png"), b"hero").unwrap();
        std::fs::write(source.join("assets/font.woff2"), b"font").unwrap();
        std::fs::write(source.join("assets/hidden.png"), b"hidden").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "`<style>.hidden { background: url(assets/hidden.png); }</style>`\n",
                "<style>\n",
                "@import \"assets/print.css?mode=screen\";\n",
                ".hero { background-image: url(\"assets/hero.png#top\"); }\n",
                "@font-face { src: url(assets/font.woff2) format(\"woff2\"); }\n",
                ".remote { background: url(https://example.com/remote.png); }\n",
                "</style>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("@import \"article_assets/print.css?mode=screen\""));
        assert!(html.contains("url(\"article_assets/hero.png#top\")"));
        assert!(html.contains("url(article_assets/font.woff2) format(\"woff2\")"));
        assert!(html.contains("url(https://example.com/remote.png)"));
        assert!(html.contains(
            "<code>&lt;style&gt;.hidden { background: url(assets/hidden.png); }&lt;/style&gt;</code>"
        ));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/print.css")).unwrap(),
            b"print"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/hero.png")).unwrap(),
            b"hero"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/font.woff2")).unwrap(),
            b"font"
        );
        assert!(!export_dir.join("article_assets/hidden.png").exists());

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
    fn html_file_export_copies_local_attachment_links() {
        let root = temp_export_dir("html-link-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(source.join("notes")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/report.pdf"), b"report").unwrap();
        std::fs::write(source.join("notes/appendix.md"), b"# Appendix").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "[Report](assets/report.pdf \"Report\")\n\n",
                "[Appendix](notes/appendix.md#summary)\n\n",
                "[Remote](https://example.com/report.pdf)\n\n",
                "[Same page](#draft)",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("href=\"article_assets/report.pdf\""));
        assert!(html.contains("title=\"Report\""));
        assert!(html.contains("href=\"article_assets/appendix.md#summary\""));
        assert!(html.contains("href=\"https://example.com/report.pdf\""));
        assert!(html.contains("href=\"#draft\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/report.pdf")).unwrap(),
            b"report"
        );
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/appendix.md")).unwrap(),
            b"# Appendix"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_reference_attachment_links() {
        let root = temp_export_dir("html-reference-link-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/guide.pdf"), b"guide").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "[Guide][guide]\n\n",
                "[Guide]: assets/guide.pdf?download=1#page-2 \"Guide PDF\"\n",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("href=\"article_assets/guide.pdf?download=1#page-2\""));
        assert!(html.contains("title=\"Guide PDF\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/guide.pdf")).unwrap(),
            b"guide"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_file_export_copies_raw_html_anchor_assets() {
        let root = temp_export_dir("html-raw-link-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/report.pdf"), b"report").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "<a class=\"download\" href='assets/report.pdf?download=1&amp;theme=print#p2'>Report</a>\n",
                "<a href=\"#section\">Same page</a>\n",
                "<a href=\"https://example.com/report.pdf\">Remote</a>",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("class=\"download\""));
        assert!(
            html.contains("href='article_assets/report.pdf?download=1&amp;theme=print#p2'")
        );
        assert!(html.contains("href=\"#section\""));
        assert!(html.contains("href=\"https://example.com/report.pdf\""));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/report.pdf")).unwrap(),
            b"report"
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
    fn html_file_export_leaves_attachment_links_inside_code_alone() {
        let root = temp_export_dir("html-link-code-assets");
        let source = root.join("source");
        let export_dir = root.join("export");
        std::fs::create_dir_all(source.join("assets")).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        std::fs::write(source.join("assets/report.pdf"), b"report").unwrap();

        let output = export_dir.join("article.html");
        export_markdown_to_html_file(
            concat!(
                "`[Hidden](assets/report.pdf)`\n\n",
                "```md\n",
                "[Hidden](assets/report.pdf)\n",
                "```\n",
            ),
            "Article",
            Some(&source),
            &output,
        )
        .unwrap();

        let html = std::fs::read_to_string(&output).unwrap();
        assert!(html.contains("<code>[Hidden](assets/report.pdf)</code>"));
        assert!(html.contains("<code class=\"language-md\">"));
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
    fn html_file_export_copies_reference_link_assets_without_image_uses() {
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
        assert!(html.contains("<a href=\"article_assets/doc.png\">Document</a>"));
        assert!(html.contains("<code>![Hidden][hidden]</code>"));
        assert!(html.contains("<code class=\"language-md\">"));
        assert_eq!(
            std::fs::read(export_dir.join("article_assets/doc.png")).unwrap(),
            b"doc"
        );
        assert!(!export_dir.join("article_assets/hidden.png").exists());
        assert!(!export_dir.join("article_assets/fenced.png").exists());

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
    fn math_export_includes_cases_fallback() {
        let html = export_markdown_to_html(
            "$$\n\\begin{cases}\nx^2 & x > 0 \\\\\n0 & \\text{otherwise}\n\\end{cases}\n$$",
            "Math",
        )
        .unwrap();

        assert!(html.contains(
            "<div class=\"math-fallback math-block-fallback\">{ x² if x &gt; 0; 0 if otherwise }</div>"
        ));
        assert!(html.contains("\\begin{cases}"));
        assert!(html.contains("mathjax@3"));
    }

    #[test]
    fn math_export_supports_fraction_style_aliases() {
        let html =
            export_markdown_to_html("Inline $\\dfrac{x^2}{y}$ and $\\tfrac{1}{n+1}$", "Math")
                .unwrap();

        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">x²/y</span>"));
        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">1/n+1</span>"));
        assert!(html.contains("\\(\\dfrac{x^2}{y}\\)"));
        assert!(html.contains("\\(\\tfrac{1}{n+1}\\)"));
    }

    #[test]
    fn math_export_supports_over_fraction_fallback() {
        let html = export_markdown_to_html("Inline ${x^2 \\over y}$", "Math").unwrap();

        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">x²/y</span>"));
        assert!(html.contains("\\({x^2 \\over y}\\)"));
    }

    #[test]
    fn math_export_supports_text_style_and_operator_commands() {
        let html = export_markdown_to_html(
            "Inline $\\mathbf{x} + \\mathcal{F}$ and $\\operatorname*{argmax}_x f(x)$",
            "Math",
        )
        .unwrap();

        assert!(html.contains("<span class=\"math-fallback math-inline-fallback\">x + F</span>"));
        assert!(html.contains(
            "<span class=\"math-fallback math-inline-fallback\">argmaxₓ f(x)</span>"
        ));
        assert!(html.contains("\\(\\mathbf{x} + \\mathcal{F}\\)"));
        assert!(html.contains("\\(\\operatorname*{argmax}_x f(x)\\)"));
    }

    #[test]
    fn math_export_skips_environment_preambles_in_fallback() {
        let html = export_markdown_to_html(
            "$$\n\\begin{array}{cc}a & b \\\\ c & d\\end{array}\n$$\n\n$$\n\\begin{alignat}{2}x &= y + 1 & z &= x - 1\\end{alignat}\n$$",
            "Math",
        )
        .unwrap();

        assert!(html.contains("<div class=\"math-fallback math-block-fallback\">a b; c d</div>"));
        assert!(html.contains("<div class=\"math-fallback math-block-fallback\">x = y + 1 z = x - 1</div>"));
        assert!(!html.contains("<div class=\"math-fallback math-block-fallback\">cc"));
        assert!(!html.contains("<div class=\"math-fallback math-block-fallback\">2 x"));
        assert!(html.contains("\\begin{array}{cc}"));
        assert!(html.contains("\\begin{alignat}{2}"));
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
    fn mermaid_export_has_static_flowchart_subgraph_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nflowchart LR\n  subgraph Writing[Writing workflow]\n    Draft[Draft] --> Save[Save]\n  end\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("class=\"mermaid-subgraph\""));
        assert!(html.contains(">Writing workflow (Writing)</text>"));
        assert!(html.contains(">Draft<"));
        assert!(html.contains(">Save<"));
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
    fn mermaid_export_has_static_sequence_autonumber_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nsequenceDiagram\n  autonumber 10 5\n  Writer->>App: Start\n  App-->>Writer: Ready\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains(">10. Start</tspan>"));
        assert!(html.contains(">15. Ready</tspan>"));
    }

    #[test]
    fn mermaid_export_has_static_sequence_note_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nsequenceDiagram\n  participant Alice as Writer\n  Alice->>Bob: Start\n  Note over Alice,Bob: Shared context <ok>\n  Note right of Bob: Follow up\n  Bob-->>Alice: Done\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("class=\"mermaid-note\""));
        assert!(html.contains(">Shared context &lt;ok&gt;</tspan>"));
        assert!(html.contains(">Follow up</tspan>"));
        assert!(html.contains(">Start</tspan>"));
        assert!(html.contains(">Done</tspan>"));
    }

    #[test]
    fn mermaid_export_has_static_sequence_fragment_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nsequenceDiagram\n  Alice->>App: Start\n  loop Every day <check>\n  App-->>Alice: Reminder\n  alt Approved\n  Alice->>App: Publish\n  else Needs work\n  Alice->>App: Revise\n  end\n  end\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("class=\"mermaid-fragment\""));
        assert!(html.contains(">loop: Every day &lt;check&gt;</tspan>"));
        assert!(html.contains(">alt: Approved</tspan>"));
        assert!(html.contains(">else: Needs work</tspan>"));
        assert!(html.contains(">end</tspan>"));
        assert!(html.contains(">Reminder</tspan>"));
        assert!(html.contains(">Revise</tspan>"));
    }

    #[test]
    fn mermaid_export_has_static_sequence_lifecycle_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nsequenceDiagram\n  participant App as Vellum\n  Writer->>App: Start\n  activate App\n  App-->>Writer: Ready\n  deactivate App\n  destroy App\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("class=\"mermaid-lifecycle\""));
        assert!(html.contains(">activate Vellum</tspan>"));
        assert!(html.contains(">deactivate Vellum</tspan>"));
        assert!(html.contains(">destroy Vellum</tspan>"));
        assert!(html.contains(">Ready</tspan>"));
    }

    #[test]
    fn mermaid_export_has_static_state_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nstateDiagram-v2\n  [*] --> Draft\n  state \"Review queue\" as Review\n  Draft --> Review: save <draft>\n  note right of Review: External change check\n  Review --> [*]\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("Mermaid state diagram preview"));
        assert!(html.contains("class=\"mermaid-state\""));
        assert!(html.contains("class=\"mermaid-state-note\""));
        assert!(html.contains(">Review queue (Review)</tspan>"));
        assert!(html.contains(">save &lt;draft&gt;</text>"));
        assert!(html.contains(">note right of Review queue"));
        assert!(!html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">stateDiagram-v2"
        ));
    }

    #[test]
    fn mermaid_export_has_static_pie_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\npie showData\n  title Export coverage\n  \"HTML <export>\" : 65\n  \"Print PDF\" : 35\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("Mermaid pie chart preview"));
        assert!(html.contains("class=\"mermaid-pie-slice\""));
        assert!(html.contains("class=\"mermaid-pie-legend\""));
        assert!(html.contains(">Export coverage</text>"));
        assert!(html.contains(">HTML &lt;export&gt;</text>"));
        assert!(html.contains(">65 (65.0%)</text>"));
        assert!(!html.contains("<pre class=\"mermaid-static mermaid-static-source\">pie"));
    }

    #[test]
    fn mermaid_export_has_static_class_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nclassDiagram\n  class Document[Markdown Document] {\n    +String title\n    +save()\n  }\n  Document : +render_html()\n  Document <|-- LongformNote : extends <draft>\n  LongformNote --> AssetStore : writes\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("Mermaid class diagram preview"));
        assert!(html.contains("class=\"mermaid-class\""));
        assert!(html.contains("class=\"mermaid-class-relationship\""));
        assert!(html.contains(">Markdown Document (Document)</text>"));
        assert!(html.contains(">+String title</text>"));
        assert!(html.contains(">+render_html()</text>"));
        assert!(html.contains(
            "Markdown Document (Document) &lt;|-- LongformNote: extends &lt;draft&gt;"
        ));
        assert!(html.contains("LongformNote --&gt; AssetStore: writes"));
        assert!(!html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">classDiagram"
        ));
    }

    #[test]
    fn mermaid_export_has_static_er_fallback() {
        let html = export_markdown_to_html(
            "```mermaid\nerDiagram\n  DOCUMENT ||--o{ ASSET : owns <local>\n  DOCUMENT }o..|| WORKSPACE : belongs_to\n  DOCUMENT {\n    string title PK\n    datetime updated_at\n  }\n  ASSET {\n    string path\n  }\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains("<svg class=\"mermaid-static\""));
        assert!(html.contains("Mermaid ER diagram preview"));
        assert!(html.contains("class=\"mermaid-er-entity\""));
        assert!(html.contains("class=\"mermaid-er-relationship\""));
        assert!(html.contains(">DOCUMENT</text>"));
        assert!(html.contains(">string title PK</text>"));
        assert!(html.contains(">datetime updated_at</text>"));
        assert!(html.contains("DOCUMENT ||--o{ ASSET: owns &lt;local&gt;"));
        assert!(html.contains("DOCUMENT }o..|| WORKSPACE: belongs_to"));
        assert!(!html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">erDiagram"
        ));
    }

    #[test]
    fn unsupported_mermaid_export_falls_back_to_source() {
        let html = export_markdown_to_html(
            "```mermaid\njourney\n  title Drafting\n  section Write\n    Draft: 5: Writer\n```",
            "Mermaid",
        )
        .unwrap();

        assert!(html.contains(
            "<pre class=\"mermaid-static mermaid-static-source\">journey\n  title Drafting\n  section Write\n    Draft: 5: Writer</pre>"
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

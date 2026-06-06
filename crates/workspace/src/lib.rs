use std::{
    collections::BTreeSet,
    fs,
    ops::Range,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::SystemTime,
};

use anyhow::{Context as _, Result};
use gpui_component::tree::TreeItem;
use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEvent {
    Changed(PathBuf),
    Removed(PathBuf),
    Relocated { from: PathBuf, to: PathBuf },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSortMode {
    Name,
    Natural,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeSort {
    pub mode: TreeSortMode,
    pub directories_first: bool,
}

impl Default for TreeSort {
    fn default() -> Self {
        Self {
            mode: TreeSortMode::Name,
            directories_first: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDocument {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub title: Option<String>,
    pub headings: Vec<WorkspaceHeading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHeading {
    pub title: String,
    pub source_offset: usize,
    pub depth: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickOpenItem {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub title: Option<String>,
    pub heading: Option<WorkspaceHeading>,
    pub score: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkspaceSearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSearchResult {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub matches: Vec<WorkspaceTextMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTextMatch {
    pub range: Range<usize>,
    pub line_number: usize,
    pub snippet: String,
    pub snippet_match: Range<usize>,
}

pub struct WorkspaceState {
    pub root: Option<PathBuf>,
    pub expanded_dirs: BTreeSet<PathBuf>,
    pub selected_file: Option<PathBuf>,
    tree_sort: TreeSort,
    watcher: Option<RecommendedWatcher>,
    rx: Option<Receiver<WorkspaceEvent>>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            root: None,
            expanded_dirs: BTreeSet::new(),
            selected_file: None,
            tree_sort: TreeSort::default(),
            watcher: None,
            rx: None,
        }
    }

    pub fn set_root(&mut self, root: Option<PathBuf>) -> Result<()> {
        self.root = root.clone();
        self.expanded_dirs.clear();
        self.selected_file = None;
        self.watcher = None;
        self.rx = None;

        if let Some(root) = root {
            self.expanded_dirs.insert(root.clone());

            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
                let event = match result {
                    Ok(event) => map_workspace_event(event),
                    Err(_) => WorkspaceEvent::Unknown,
                };
                let _ = tx.send(event);
            })
            .context("failed to create file watcher")?;
            watcher
                .watch(&root, RecursiveMode::Recursive)
                .with_context(|| format!("failed to watch {}", root.display()))?;
            self.watcher = Some(watcher);
            self.rx = Some(rx);
        }

        Ok(())
    }

    pub fn tree_sort(&self) -> TreeSort {
        self.tree_sort
    }

    pub fn set_tree_sort(&mut self, sort: TreeSort) {
        self.tree_sort = sort;
    }

    pub fn poll_events(&mut self) -> Vec<WorkspaceEvent> {
        let mut events = Vec::new();
        let Some(rx) = &self.rx else {
            return events;
        };

        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        events
    }

    pub fn select_file(&mut self, path: PathBuf) {
        self.expand_ancestors(&path);
        self.selected_file = Some(path);
    }

    pub fn clear_selection(&mut self) {
        self.selected_file = None;
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if !self.expanded_dirs.remove(path) {
            self.expanded_dirs.insert(path.to_path_buf());
        }
    }

    pub fn tree_items(&self) -> Result<Vec<TreeItem>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        Ok(vec![build_tree_item(
            root,
            &self.expanded_dirs,
            &self.tree_sort,
        )?])
    }

    pub fn tree_items_matching(&self, query: &str) -> Result<Vec<TreeItem>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        let query = normalize_query(query);
        if query.is_empty() {
            return self.tree_items();
        }
        Ok(
            build_filtered_tree_item(root, root, &query, &self.tree_sort)?
                .into_iter()
                .collect(),
        )
    }

    pub fn markdown_documents(&self) -> Result<Vec<WorkspaceDocument>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        markdown_documents(root, &self.tree_sort)
    }

    pub fn quick_open_items(&self, query: &str, limit: usize) -> Result<Vec<QuickOpenItem>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        quick_open_items(root, query, limit, &self.tree_sort)
    }

    pub fn search_markdown(
        &self,
        query: &str,
        options: WorkspaceSearchOptions,
    ) -> Result<Vec<WorkspaceSearchResult>> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        search_markdown(root, query, options, &self.tree_sort)
    }

    fn expand_ancestors(&mut self, path: &Path) {
        let Some(root) = &self.root else {
            return;
        };
        if !path.starts_with(root) {
            return;
        }

        let mut parent = path.parent();
        while let Some(dir) = parent {
            if dir.starts_with(root) {
                self.expanded_dirs.insert(dir.to_path_buf());
            }
            if dir == root {
                break;
            }
            parent = dir.parent();
        }
    }
}

fn map_workspace_event(event: Event) -> WorkspaceEvent {
    if let EventKind::Modify(ModifyKind::Name(mode)) = event.kind {
        return match (mode, event.paths.as_slice()) {
            (RenameMode::Both, [from, to, ..]) => WorkspaceEvent::Relocated {
                from: from.clone(),
                to: to.clone(),
            },
            (RenameMode::From, [path, ..]) => WorkspaceEvent::Removed(path.clone()),
            (RenameMode::To, [path, ..]) => WorkspaceEvent::Changed(path.clone()),
            (_, [path, ..]) => WorkspaceEvent::Changed(path.clone()),
            _ => WorkspaceEvent::Unknown,
        };
    }

    let path = event.paths.first().cloned();
    match (event.kind, path) {
        (EventKind::Modify(_), Some(path))
        | (EventKind::Create(_), Some(path))
        | (EventKind::Any, Some(path)) => WorkspaceEvent::Changed(path),
        (EventKind::Remove(_), Some(path)) => WorkspaceEvent::Removed(path),
        _ => WorkspaceEvent::Unknown,
    }
}

fn build_tree_item(
    path: &Path,
    expanded_dirs: &BTreeSet<PathBuf>,
    sort: &TreeSort,
) -> Result<TreeItem> {
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let entry_path = entry.path();
                entry_path.is_dir() || is_markdown_path(&entry_path)
            })
            .collect::<Vec<_>>();
        sort_entries(&mut entries, sort);

        let children = entries
            .iter()
            .map(|entry| build_tree_item(&entry.path(), expanded_dirs, sort))
            .collect::<Result<Vec<_>>>()?;

        Ok(TreeItem::new(id, label)
            .expanded(expanded_dirs.contains(path))
            .children(children))
    } else {
        Ok(TreeItem::new(id, label))
    }
}

fn build_filtered_tree_item(
    path: &Path,
    root: &Path,
    query: &str,
    sort: &TreeSort,
) -> Result<Option<TreeItem>> {
    let label = path_label(path);
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = markdown_tree_entries(path)?;
        sort_entries(&mut entries, sort);

        let direct_match = path_matches_query(path, root, &label, query);
        let children = if direct_match {
            entries
                .iter()
                .map(|entry| build_expanded_tree_item(&entry.path(), sort))
                .collect::<Result<Vec<_>>>()?
        } else {
            entries
                .iter()
                .filter_map(|entry| {
                    match build_filtered_tree_item(&entry.path(), root, query, sort) {
                        Ok(Some(item)) => Some(Ok(item)),
                        Ok(None) => None,
                        Err(err) => Some(Err(err)),
                    }
                })
                .collect::<Result<Vec<_>>>()?
        };

        if direct_match || !children.is_empty() {
            Ok(Some(
                TreeItem::new(id, label).expanded(true).children(children),
            ))
        } else {
            Ok(None)
        }
    } else if path_matches_query(path, root, &label, query) {
        Ok(Some(TreeItem::new(id, label)))
    } else {
        Ok(None)
    }
}

fn build_expanded_tree_item(path: &Path, sort: &TreeSort) -> Result<TreeItem> {
    let label = path_label(path);
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        let mut entries = markdown_tree_entries(path)?;
        sort_entries(&mut entries, sort);
        let children = entries
            .iter()
            .map(|entry| build_expanded_tree_item(&entry.path(), sort))
            .collect::<Result<Vec<_>>>()?;
        Ok(TreeItem::new(id, label).expanded(true).children(children))
    } else {
        Ok(TreeItem::new(id, label))
    }
}

fn markdown_tree_entries(path: &Path) -> Result<Vec<fs::DirEntry>> {
    Ok(fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let entry_path = entry.path();
            entry_path.is_dir() || is_markdown_path(&entry_path)
        })
        .collect::<Vec<_>>())
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn path_matches_query(path: &Path, root: &Path, label: &str, query: &str) -> bool {
    label.to_ascii_lowercase().contains(query)
        || path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
            .contains(query)
}

fn normalize_query(query: &str) -> String {
    query.trim().replace('\\', "/").to_ascii_lowercase()
}

fn sort_entries(entries: &mut [fs::DirEntry], sort: &TreeSort) {
    entries.sort_by(|left, right| compare_paths(&left.path(), &right.path(), sort));
}

fn compare_paths(left: &Path, right: &Path, sort: &TreeSort) -> std::cmp::Ordering {
    if sort.directories_first {
        match (left.is_dir(), right.is_dir()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
    }

    let left_name = path_label(left);
    let right_name = path_label(right);
    match sort.mode {
        TreeSortMode::Name => normalized_name(&left_name).cmp(&normalized_name(&right_name)),
        TreeSortMode::Natural => natural_cmp(&left_name, &right_name),
        TreeSortMode::Modified => modified_at(right)
            .cmp(&modified_at(left))
            .then_with(|| natural_cmp(&left_name, &right_name)),
    }
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("markdown")
                || ext.eq_ignore_ascii_case("mdown")
        })
        .unwrap_or(false)
}

pub fn markdown_documents(root: &Path, sort: &TreeSort) -> Result<Vec<WorkspaceDocument>> {
    let mut paths = Vec::new();
    collect_markdown_paths(root, &mut paths)?;
    paths.sort_by(|left, right| compare_paths(left, right, sort));

    Ok(paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let headings = extract_headings(&text);
            let title = headings.first().map(|heading| heading.title.clone());
            WorkspaceDocument {
                file_name: path_label(&path),
                relative_path: relative_path(root, &path),
                path,
                title,
                headings,
            }
        })
        .collect())
}

pub fn quick_open_items(
    root: &Path,
    query: &str,
    limit: usize,
    sort: &TreeSort,
) -> Result<Vec<QuickOpenItem>> {
    let normalized_query = normalize_search_query(query);
    let mut items = Vec::new();

    for doc in markdown_documents(root, sort)? {
        push_quick_open_candidate(&mut items, &normalized_query, &doc, None);
        for heading in &doc.headings {
            push_quick_open_candidate(&mut items, &normalized_query, &doc, Some(heading.clone()));
        }
    }

    items.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| natural_cmp(&left.relative_path, &right.relative_path))
            .then_with(|| {
                left.heading
                    .as_ref()
                    .map(|heading| heading.source_offset)
                    .cmp(&right.heading.as_ref().map(|heading| heading.source_offset))
            })
    });
    if limit > 0 && items.len() > limit {
        items.truncate(limit);
    }
    Ok(items)
}

pub fn search_markdown(
    root: &Path,
    query: &str,
    options: WorkspaceSearchOptions,
    sort: &TreeSort,
) -> Result<Vec<WorkspaceSearchResult>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for doc in markdown_documents(root, sort)? {
        let text = fs::read_to_string(&doc.path).unwrap_or_default();
        let matches = find_text_matches(&text, query, options)
            .into_iter()
            .map(|range| text_match_for_range(&text, range))
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            results.push(WorkspaceSearchResult {
                path: doc.path,
                relative_path: doc.relative_path,
                file_name: doc.file_name,
                matches,
            });
        }
    }
    Ok(results)
}

fn collect_markdown_paths(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_paths(&path, paths)?;
        } else if is_markdown_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn push_quick_open_candidate(
    items: &mut Vec<QuickOpenItem>,
    normalized_query: &str,
    doc: &WorkspaceDocument,
    heading: Option<WorkspaceHeading>,
) {
    let title = heading
        .as_ref()
        .map(|heading| heading.title.as_str())
        .or(doc.title.as_deref());
    let haystacks = [
        doc.file_name.as_str(),
        doc.relative_path.as_str(),
        title.unwrap_or_default(),
    ];
    let score = if normalized_query.is_empty() {
        if heading.is_some() { 40 } else { 50 }
    } else {
        haystacks
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                quick_match_score(candidate, normalized_query).map(|score| {
                    let weight = match index {
                        0 => 30,
                        1 => 20,
                        _ => 25,
                    };
                    score + weight
                })
            })
            .max()
            .unwrap_or(0)
    };

    if score <= 0 {
        return;
    }

    items.push(QuickOpenItem {
        path: doc.path.clone(),
        relative_path: doc.relative_path.clone(),
        file_name: doc.file_name.clone(),
        title: doc.title.clone(),
        heading,
        score,
    });
}

fn quick_match_score(candidate: &str, normalized_query: &str) -> Option<i64> {
    let candidate = normalize_search_query(candidate);
    if candidate.is_empty() {
        return None;
    }
    if candidate == normalized_query {
        return Some(1_000);
    }
    if candidate.starts_with(normalized_query) {
        return Some(850 - candidate.len() as i64);
    }
    if candidate.contains(normalized_query) {
        return Some(700 - candidate.find(normalized_query).unwrap_or(0) as i64);
    }
    fuzzy_subsequence_score(&candidate, normalized_query)
}

fn fuzzy_subsequence_score(candidate: &str, query: &str) -> Option<i64> {
    let mut score = 250;
    let mut search_start = 0;
    let mut previous_match: Option<usize> = None;
    for ch in query.chars() {
        let haystack = &candidate[search_start..];
        let Some(pos) = haystack.find(ch) else {
            return None;
        };
        let absolute = search_start + pos;
        if let Some(previous) = previous_match {
            if absolute == previous + 1 {
                score += 20;
            } else {
                score -= (absolute - previous).min(20) as i64;
            }
        } else {
            score -= absolute.min(40) as i64;
        }
        previous_match = Some(absolute);
        search_start = absolute + ch.len_utf8();
    }
    Some(score.max(1))
}

fn find_text_matches(
    text: &str,
    query: &str,
    options: WorkspaceSearchOptions,
) -> Vec<Range<usize>> {
    if options.use_regex {
        let pattern = if options.whole_word {
            format!(r"\b(?:{})\b", query)
        } else {
            query.to_string()
        };
        let regex = regex::RegexBuilder::new(&pattern)
            .case_insensitive(!options.case_sensitive)
            .build();
        return match regex {
            Ok(regex) => regex
                .find_iter(text)
                .map(|item| item.start()..item.end())
                .collect(),
            Err(_) => Vec::new(),
        };
    }

    let pattern = regex::escape(query);
    let regex = regex::RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .build();
    match regex {
        Ok(regex) => regex
            .find_iter(text)
            .filter_map(|item| {
                let range = item.start()..item.end();
                if options.whole_word && !is_whole_word(text, range.start, range.end) {
                    None
                } else {
                    Some(range)
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn text_match_for_range(text: &str, range: Range<usize>) -> WorkspaceTextMatch {
    let line_start = text[..range.start]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = text[range.end..]
        .find('\n')
        .map(|index| range.end + index)
        .unwrap_or(text.len());
    let raw_line = &text[line_start..line_end];
    let leading_trim = raw_line.len() - raw_line.trim_start().len();
    let trailing_trim = raw_line.trim_end().len();
    let snippet_start = line_start + leading_trim;
    let snippet_end = line_start + trailing_trim;
    let snippet = text[snippet_start..snippet_end].to_string();
    let match_start = range.start.saturating_sub(snippet_start).min(snippet.len());
    let match_end = range.end.saturating_sub(snippet_start).min(snippet.len());

    WorkspaceTextMatch {
        range,
        line_number: text[..line_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1,
        snippet,
        snippet_match: match_start..match_end,
    }
}

fn extract_headings(text: &str) -> Vec<WorkspaceHeading> {
    let mut headings = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        if let Some((depth, title)) = parse_atx_heading(line_without_newline) {
            headings.push(WorkspaceHeading {
                title,
                source_offset: offset,
                depth,
            });
        }
        offset += line.len();
    }
    headings
}

fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let marker_len = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if marker_len == 0 || marker_len > 6 {
        return None;
    }
    let after_marker = trimmed.get(marker_len..)?;
    if !after_marker.starts_with(char::is_whitespace) {
        return None;
    }
    let title = after_marker.trim().trim_end_matches('#').trim().to_string();
    (!title.is_empty()).then_some((marker_len as u8, title))
}

fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before_is_boundary = start == 0
        || !text.as_bytes()[start - 1].is_ascii_alphanumeric()
            && text.as_bytes()[start - 1] != b'_';
    let after_is_boundary = end >= text.len()
        || !text.as_bytes()[end].is_ascii_alphanumeric() && text.as_bytes()[end] != b'_';
    before_is_boundary && after_is_boundary
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_search_query(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

fn normalized_name(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn modified_at(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let mut left_iter = left.char_indices().peekable();
    let mut right_iter = right.char_indices().peekable();

    while left_iter.peek().is_some() && right_iter.peek().is_some() {
        let left_char = left_iter.peek().map(|(_, ch)| *ch).unwrap();
        let right_char = right_iter.peek().map(|(_, ch)| *ch).unwrap();
        if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
            let left_number = take_ascii_number(left, &mut left_iter);
            let right_number = take_ascii_number(right, &mut right_iter);
            let ordering = compare_number_strings(left_number, right_number);
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        } else {
            let ordering = left_char
                .to_ascii_lowercase()
                .cmp(&right_char.to_ascii_lowercase());
            left_iter.next();
            right_iter.next();
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
    }

    left.len().cmp(&right.len())
}

fn take_ascii_number<'a>(
    source: &'a str,
    iter: &mut std::iter::Peekable<std::str::CharIndices<'a>>,
) -> &'a str {
    let start = iter.peek().map(|(index, _)| *index).unwrap_or(source.len());
    let mut end = start;
    while let Some((index, ch)) = iter.peek().copied() {
        if !ch.is_ascii_digit() {
            break;
        }
        end = index + ch.len_utf8();
        iter.next();
    }
    &source[start..end]
}

fn compare_number_strings(left: &str, right: &str) -> std::cmp::Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    let left_cmp = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_cmp = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };
    left_cmp
        .len()
        .cmp(&right_cmp.len())
        .then_with(|| left_cmp.cmp(right_cmp))
        .then_with(|| left.len().cmp(&right.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vellum-workspace-filter-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp workspace should be created");
            Self { root }
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temp workspace parent should be created");
            }
            fs::write(path, contents).expect("temp workspace file should be written");
        }

        fn state(&self) -> WorkspaceState {
            WorkspaceState {
                root: Some(self.root.clone()),
                expanded_dirs: BTreeSet::new(),
                selected_file: None,
                tree_sort: TreeSort::default(),
                watcher: None,
                rx: None,
            }
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn child_labels(item: &TreeItem) -> Vec<String> {
        item.children
            .iter()
            .map(|child| child.label.to_string())
            .collect()
    }

    fn child_paths(item: &TreeItem) -> Vec<String> {
        item.children
            .iter()
            .map(|child| PathBuf::from(child.id.as_ref()))
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect()
    }

    fn name_event(mode: RenameMode, paths: &[&str]) -> Event {
        paths.iter().fold(
            Event::new(EventKind::Modify(ModifyKind::Name(mode))),
            |event, path| event.add_path(PathBuf::from(path)),
        )
    }

    #[test]
    fn maps_paired_rename_to_relocation() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::Both, &["old.md", "new.md"])),
            WorkspaceEvent::Relocated {
                from: PathBuf::from("old.md"),
                to: PathBuf::from("new.md"),
            }
        );
    }

    #[test]
    fn maps_single_rename_from_to_removed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::From, &["old.md"])),
            WorkspaceEvent::Removed(PathBuf::from("old.md"))
        );
    }

    #[test]
    fn maps_single_rename_to_to_changed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::To, &["new.md"])),
            WorkspaceEvent::Changed(PathBuf::from("new.md"))
        );
    }

    #[test]
    fn maps_unclassified_rename_with_path_to_changed() {
        assert_eq!(
            map_workspace_event(name_event(RenameMode::Any, &["note.md"])),
            WorkspaceEvent::Changed(PathBuf::from("note.md"))
        );
    }

    #[test]
    fn selecting_file_expands_ancestor_directories() {
        let workspace = TempWorkspace::new();
        workspace.write("chapters/drafts/outline.md", "# Outline");

        let mut state = workspace.state();
        let selected_file = workspace.root.join("chapters/drafts/outline.md");

        state.select_file(selected_file.clone());

        assert_eq!(state.selected_file.as_ref(), Some(&selected_file));
        assert!(state.expanded_dirs.contains(&workspace.root));
        assert!(
            state
                .expanded_dirs
                .contains(&workspace.root.join("chapters"))
        );
        assert!(
            state
                .expanded_dirs
                .contains(&workspace.root.join("chapters/drafts"))
        );

        let items = state.tree_items().expect("tree should build");
        assert_eq!(child_labels(&items[0]), vec!["chapters"]);
        assert!(items[0].children[0].is_expanded());
        assert!(items[0].children[0].children[0].is_expanded());
    }

    #[test]
    fn filters_markdown_files_across_collapsed_directories() {
        let workspace = TempWorkspace::new();
        workspace.write("Draft.md", "# Draft");
        workspace.write("notes/Meeting Notes.markdown", "# Meeting");
        workspace.write("notes/ignore.txt", "not shown");

        let items = workspace
            .state()
            .tree_items_matching("meeting")
            .expect("filtered tree should build");

        assert_eq!(items.len(), 1);
        assert!(items[0].is_expanded());
        assert_eq!(child_labels(&items[0]), vec!["notes"]);
        let notes = &items[0].children[0];
        assert!(notes.is_expanded());
        assert_eq!(child_labels(notes), vec!["Meeting Notes.markdown"]);
    }

    #[test]
    fn file_filter_matches_case_insensitive_relative_paths() {
        let workspace = TempWorkspace::new();
        workspace.write("deep/sub/plan.mdown", "# Plan");
        workspace.write("deep/sub/other.md", "# Other");
        workspace.write("root.md", "# Root");

        let items = workspace
            .state()
            .tree_items_matching("SUB/PLAN")
            .expect("filtered tree should build");

        let root = &items[0];
        assert_eq!(child_labels(root), vec!["deep"]);
        let deep = &root.children[0];
        assert_eq!(child_labels(deep), vec!["sub"]);
        let sub = &deep.children[0];
        assert_eq!(child_labels(sub), vec!["plan.mdown"]);
    }

    #[test]
    fn matching_folder_expands_markdown_subtree() {
        let workspace = TempWorkspace::new();
        workspace.write("chapters/one.md", "# One");
        workspace.write("chapters/nested/two.md", "# Two");
        workspace.write("outside.md", "# Outside");

        let items = workspace
            .state()
            .tree_items_matching("chapters")
            .expect("filtered tree should build");

        let root = &items[0];
        assert_eq!(child_labels(root), vec!["chapters"]);
        let chapters = &root.children[0];
        assert!(chapters.is_expanded());
        assert_eq!(child_labels(chapters), vec!["nested", "one.md"]);
        assert_eq!(child_labels(&chapters.children[0]), vec!["two.md"]);
    }

    #[test]
    fn tree_sort_supports_natural_order_and_directory_toggle() {
        let workspace = TempWorkspace::new();
        workspace.write("chapter10.md", "# Ten");
        workspace.write("chapter2.md", "# Two");
        workspace.write("folder/note.md", "# Folder");

        let mut state = workspace.state();
        state.set_tree_sort(TreeSort {
            mode: TreeSortMode::Natural,
            directories_first: false,
        });

        let items = state.tree_items().expect("tree should build");
        assert_eq!(
            child_paths(&items[0]),
            vec!["chapter2.md", "chapter10.md", "folder"]
        );
    }

    #[test]
    fn markdown_documents_extract_titles_and_ignore_non_markdown() {
        let workspace = TempWorkspace::new();
        workspace.write("draft.md", "body\n## Later\n");
        workspace.write("guide.markdown", "# Guide\n\nText");
        workspace.write("ignore.txt", "# Ignore");

        let documents = markdown_documents(&workspace.root, &TreeSort::default())
            .expect("documents should scan");

        assert_eq!(documents.len(), 2);
        assert!(documents.iter().any(|doc| doc.relative_path == "draft.md"
            && doc.title.as_deref() == Some("Later")));
        assert!(
            documents
                .iter()
                .any(|doc| doc.relative_path == "guide.markdown"
                    && doc.title.as_deref() == Some("Guide"))
        );
    }

    #[test]
    fn quick_open_matches_file_paths_and_headings() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "notes/project-plan.md",
            "# Project Plan\n\n## Shipping Checklist\n\n- item",
        );
        workspace.write("archive/day.md", "# Daily Log");

        let items = quick_open_items(&workspace.root, "ship", 10, &TreeSort::default())
            .expect("quick open should scan");

        assert!(!items.is_empty());
        assert_eq!(items[0].relative_path, "notes/project-plan.md");
        assert_eq!(
            items[0]
                .heading
                .as_ref()
                .map(|heading| heading.title.as_str()),
            Some("Shipping Checklist")
        );
        assert!(items[0].heading.as_ref().unwrap().source_offset > 0);
    }

    #[test]
    fn search_markdown_finds_tags_and_returns_snippet_ranges() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "notes/tags.md",
            "# Tags\n\nKeep #rust-notes near the top.\nIgnore #other.\n",
        );

        let results = search_markdown(
            &workspace.root,
            "#rust-notes",
            WorkspaceSearchOptions::default(),
            &TreeSort::default(),
        )
        .expect("search should scan");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relative_path, "notes/tags.md");
        assert_eq!(results[0].matches.len(), 1);
        let first = &results[0].matches[0];
        assert_eq!(first.line_number, 3);
        assert_eq!(&first.snippet[first.snippet_match.clone()], "#rust-notes");
    }

    #[test]
    fn search_markdown_honors_case_word_and_regex_options() {
        let workspace = TempWorkspace::new();
        workspace.write("notes/search.md", "alpha alphabet ALPHA\nbeta-42\n");

        let whole_word = search_markdown(
            &workspace.root,
            "alpha",
            WorkspaceSearchOptions {
                case_sensitive: false,
                whole_word: true,
                use_regex: false,
            },
            &TreeSort::default(),
        )
        .expect("search should scan");
        assert_eq!(whole_word[0].matches.len(), 2);

        let case_sensitive = search_markdown(
            &workspace.root,
            "alpha",
            WorkspaceSearchOptions {
                case_sensitive: true,
                whole_word: true,
                use_regex: false,
            },
            &TreeSort::default(),
        )
        .expect("search should scan");
        assert_eq!(case_sensitive[0].matches.len(), 1);

        let regex = search_markdown(
            &workspace.root,
            r"beta-\d+",
            WorkspaceSearchOptions {
                case_sensitive: true,
                whole_word: false,
                use_regex: true,
            },
            &TreeSort::default(),
        )
        .expect("search should scan");
        assert_eq!(regex[0].matches[0].snippet, "beta-42");
    }
}

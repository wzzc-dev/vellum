use std::ops::Range;

use super::document::SelectionAffinity;

pub(crate) const TABLE_COLUMN_GAP: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableNavDirection {
    Forward,
    Backward,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TableCellRef {
    pub(crate) visible_row: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TableModel {
    source: String,
    rows: Vec<TableRow>,
    visible_row_indices: Vec<usize>,
    column_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TableRow {
    pub(crate) line_start: usize,
    pub(crate) line_end: usize,
    pub(crate) end_with_newline: usize,
    pub(crate) cells: Vec<TableCell>,
    pub(crate) is_delimiter: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TableCell {
    pub(crate) source_range: Range<usize>,
}

impl TableModel {
    pub(crate) fn parse(text: &str) -> Self {
        let mut rows = Vec::new();
        let mut offset = 0usize;

        for segment in split_inclusive_lines(text) {
            let line = segment.trim_end_matches(['\r', '\n']);
            let line_len = line.len();
            rows.push(TableRow {
                line_start: offset,
                line_end: offset + line_len,
                end_with_newline: offset + segment.len(),
                cells: parse_pipe_table_cells(line, offset),
                is_delimiter: is_pipe_table_delimiter_row(line),
            });
            offset += segment.len();
            if line_len == 0 && segment.is_empty() {
                break;
            }
        }

        if rows.is_empty() && !text.is_empty() {
            rows.push(TableRow {
                line_start: 0,
                line_end: text.len(),
                end_with_newline: text.len(),
                cells: parse_pipe_table_cells(text, 0),
                is_delimiter: false,
            });
        }

        let visible_row_indices = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (!row.is_delimiter).then_some(index))
            .collect::<Vec<_>>();
        let column_count = visible_row_indices
            .iter()
            .filter_map(|index| rows.get(*index))
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0);

        Self {
            source: text.to_string(),
            rows,
            visible_row_indices,
            column_count,
        }
    }

    pub(crate) fn rows(&self) -> &[TableRow] {
        &self.rows
    }

    pub(crate) fn column_count(&self) -> usize {
        self.column_count
    }

    pub(crate) fn visible_row_count(&self) -> usize {
        self.visible_row_indices.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.visible_row_count() == 0 || self.column_count == 0
    }

    pub(crate) fn visible_row(&self, visible_row: usize) -> Option<&TableRow> {
        self.visible_row_indices
            .get(visible_row)
            .and_then(|index| self.rows.get(*index))
    }

    pub(crate) fn cell(&self, cell_ref: TableCellRef) -> Option<&TableCell> {
        self.visible_row(cell_ref.visible_row)
            .and_then(|row| row.cells.get(cell_ref.column))
    }

    pub(crate) fn cell_source_range(&self, cell_ref: TableCellRef) -> Option<Range<usize>> {
        self.cell(cell_ref).map(|cell| cell.source_range.clone())
    }

    pub(crate) fn cell_source_text(&self, cell_ref: TableCellRef) -> Option<&str> {
        let range = self.cell_source_range(cell_ref)?;
        self.source.get(range)
    }

    pub(crate) fn first_cell(&self) -> Option<TableCellRef> {
        (!self.is_empty()).then_some(TableCellRef {
            visible_row: 0,
            column: 0,
        })
    }

    pub(crate) fn last_cell(&self) -> Option<TableCellRef> {
        let visible_row = self.visible_row_count().checked_sub(1)?;
        let column = self.column_count.checked_sub(1)?;
        Some(TableCellRef {
            visible_row,
            column,
        })
    }

    pub(crate) fn cell_ref_for_source_offset(
        &self,
        source_offset: usize,
        affinity: SelectionAffinity,
    ) -> Option<TableCellRef> {
        let source_offset = source_offset.min(self.source.len());
        let mut last_cell = None;

        for visible_row in 0..self.visible_row_count() {
            let row = self.visible_row(visible_row)?;
            if row.cells.is_empty() {
                continue;
            }

            let first = row.cells.first()?;
            if source_offset < first.source_range.start {
                return Some(TableCellRef {
                    visible_row,
                    column: 0,
                });
            }

            for (column, cell) in row.cells.iter().enumerate() {
                let current = TableCellRef {
                    visible_row,
                    column,
                };
                if source_offset <= cell.source_range.end {
                    return Some(current);
                }

                let Some(next) = row.cells.get(column + 1) else {
                    last_cell = Some(current);
                    break;
                };
                if source_offset < next.source_range.start {
                    return Some(match affinity {
                        SelectionAffinity::Upstream => current,
                        SelectionAffinity::Downstream => TableCellRef {
                            visible_row,
                            column: column + 1,
                        },
                    });
                }
            }

            if source_offset <= row.end_with_newline {
                return last_cell;
            }
        }

        last_cell.or_else(|| self.last_cell())
    }

    pub(crate) fn next_cell_ref(
        &self,
        current: TableCellRef,
        direction: TableNavDirection,
    ) -> Option<TableCellRef> {
        if self.is_empty() {
            return None;
        }

        match direction {
            TableNavDirection::Forward => {
                if current.column + 1 < self.column_count {
                    Some(TableCellRef {
                        visible_row: current.visible_row,
                        column: current.column + 1,
                    })
                } else if current.visible_row + 1 < self.visible_row_count() {
                    Some(TableCellRef {
                        visible_row: current.visible_row + 1,
                        column: 0,
                    })
                } else {
                    None
                }
            }
            TableNavDirection::Backward => {
                if current.column > 0 {
                    Some(TableCellRef {
                        visible_row: current.visible_row,
                        column: current.column - 1,
                    })
                } else if current.visible_row > 0 {
                    Some(TableCellRef {
                        visible_row: current.visible_row - 1,
                        column: self.column_count.saturating_sub(1),
                    })
                } else {
                    None
                }
            }
            TableNavDirection::Down => {
                if current.visible_row + 1 < self.visible_row_count() {
                    Some(TableCellRef {
                        visible_row: current.visible_row + 1,
                        column: current.column.min(self.column_count.saturating_sub(1)),
                    })
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn delimiter_row_text(&self) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.is_delimiter)
            .and_then(|row| self.source.get(row.line_start..row.line_end))
    }

    pub(crate) fn empty_row_markdown(&self) -> String {
        format!("| {} |", vec![""; self.column_count.max(1)].join(" | "))
    }

    pub(crate) fn delimiter_row_markdown(&self) -> String {
        self.delimiter_row_text()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("| {} |", vec!["---"; self.column_count.max(1)].join(" | ")))
    }

    pub(crate) fn rebuild_markdown_with_override(
        &self,
        cell_ref: TableCellRef,
        cell_source: String,
    ) -> String {
        let mut lines = Vec::with_capacity(self.visible_row_count().saturating_add(1));
        for visible_row in 0..self.visible_row_count() {
            let cells = (0..self.column_count.max(1))
                .map(|column| {
                    let current = TableCellRef {
                        visible_row,
                        column,
                    };
                    if current == cell_ref {
                        cell_source.clone()
                    } else {
                        self.cell_source_text(current).unwrap_or("").to_string()
                    }
                })
                .map(|cell| escape_markdown_table_cell(&cell))
                .collect::<Vec<_>>();

            lines.push(format_pipe_row(&cells));
            if visible_row == 0 {
                lines.push(self.delimiter_row_markdown());
            }
        }

        lines.join("\n")
    }

    pub(crate) fn rebuild_markdown_without_row(
        &self,
        visible_row_to_remove: usize,
    ) -> Option<String> {
        if visible_row_to_remove >= self.visible_row_count() || self.visible_row_count() <= 1 {
            return None;
        }

        let remaining_rows = (0..self.visible_row_count())
            .filter(|visible_row| *visible_row != visible_row_to_remove)
            .collect::<Vec<_>>();
        let mut lines = Vec::with_capacity(remaining_rows.len().saturating_add(1));

        for (rebuilt_row, visible_row) in remaining_rows.into_iter().enumerate() {
            let cells = self.visible_row_cells(visible_row);
            lines.push(format_pipe_row(&cells));
            if rebuilt_row == 0 {
                lines.push(self.delimiter_row_markdown());
            }
        }

        Some(lines.join("\n"))
    }

    pub(crate) fn rebuild_markdown_with_inserted_row_after(
        &self,
        visible_row: usize,
    ) -> Option<String> {
        if visible_row >= self.visible_row_count() {
            return None;
        }

        let mut lines = Vec::with_capacity(self.visible_row_count().saturating_add(2));
        for current_row in 0..self.visible_row_count() {
            let cells = self.visible_row_cells(current_row);
            lines.push(format_pipe_row(&cells));
            if current_row == 0 {
                lines.push(self.delimiter_row_markdown());
            }
            if current_row == visible_row {
                lines.push(self.empty_row_markdown());
            }
        }

        Some(lines.join("\n"))
    }

    pub(crate) fn rebuild_markdown_with_inserted_column_after(
        &self,
        column_to_insert_after: usize,
    ) -> Option<String> {
        if column_to_insert_after >= self.column_count() {
            return None;
        }

        let mut lines = Vec::with_capacity(self.visible_row_count().saturating_add(1));
        let delimiter_cells = self.delimiter_row_cells();
        for visible_row in 0..self.visible_row_count() {
            let mut cells = self.visible_row_cells(visible_row);
            cells.insert(column_to_insert_after + 1, String::new());
            lines.push(format_pipe_row(&cells));

            if visible_row == 0 {
                let mut delimiter = delimiter_cells.clone();
                delimiter.insert(column_to_insert_after + 1, "---".to_string());
                lines.push(format_pipe_row(&delimiter));
            }
        }

        Some(lines.join("\n"))
    }

    pub(crate) fn rebuild_markdown_without_column(
        &self,
        column_to_remove: usize,
    ) -> Option<String> {
        if column_to_remove >= self.column_count() || self.column_count() <= 1 {
            return None;
        }

        let mut lines = Vec::with_capacity(self.visible_row_count().saturating_add(1));
        let mut delimiter_cells = self.delimiter_row_cells();
        delimiter_cells.remove(column_to_remove);

        for visible_row in 0..self.visible_row_count() {
            let mut cells = self.visible_row_cells(visible_row);
            cells.remove(column_to_remove);
            lines.push(format_pipe_row(&cells));

            if visible_row == 0 {
                lines.push(format_pipe_row(&delimiter_cells));
            }
        }

        Some(lines.join("\n"))
    }

    pub(crate) fn append_empty_row(&self) -> String {
        if self.source.is_empty() {
            return self.empty_row_markdown();
        }

        format!("{}\n{}", self.source, self.empty_row_markdown())
    }

    fn visible_row_cells(&self, visible_row: usize) -> Vec<String> {
        (0..self.column_count.max(1))
            .map(|column| {
                self.cell_source_text(TableCellRef {
                    visible_row,
                    column,
                })
                .unwrap_or("")
                .to_string()
            })
            .map(|cell| escape_markdown_table_cell(&cell))
            .collect()
    }

    fn delimiter_row_cells(&self) -> Vec<String> {
        let Some(row) = self.rows.iter().find(|row| row.is_delimiter) else {
            return vec!["---".to_string(); self.column_count.max(1)];
        };

        row.cells
            .iter()
            .map(|cell| self.source[cell.source_range.clone()].to_string())
            .collect()
    }
}

pub(crate) fn pipe_row_cell_count(line: &str) -> usize {
    unescaped_pipe_indices(line).windows(2).count()
}

fn parse_pipe_table_cells(line: &str, line_start: usize) -> Vec<TableCell> {
    pipe_table_segments(line)
        .into_iter()
        .map(|segment| trim_horizontal_whitespace_range(line, segment))
        .map(|source_range| TableCell {
            source_range: line_start + source_range.start..line_start + source_range.end,
        })
        .collect()
}

fn pipe_table_segments(line: &str) -> Vec<Range<usize>> {
    let pipes = unescaped_pipe_indices(line);
    if pipes.is_empty() {
        return vec![0..line.len()];
    }

    let mut segments = Vec::new();
    if pipes[0] > 0 {
        segments.push(0..pipes[0]);
    }
    for window in pipes.windows(2) {
        segments.push(window[0] + 1..window[1]);
    }
    if let Some(last) = pipes.last().copied()
        && last + 1 < line.len()
    {
        segments.push(last + 1..line.len());
    }

    segments
}

fn unescaped_pipe_indices(text: &str) -> Vec<usize> {
    text.char_indices()
        .filter_map(|(index, ch)| (ch == '|' && !is_escaped_pipe(text, index)).then_some(index))
        .collect()
}

fn is_escaped_pipe(text: &str, pipe_index: usize) -> bool {
    let bytes = text.as_bytes();
    let mut slash_count = 0usize;
    let mut index = pipe_index;
    while index > 0 && bytes[index - 1] == b'\\' {
        slash_count += 1;
        index -= 1;
    }

    slash_count % 2 == 1
}

fn trim_horizontal_whitespace_range(text: &str, range: Range<usize>) -> Range<usize> {
    let slice = &text[range.clone()];
    let bytes = slice.as_bytes();
    let leading = bytes
        .iter()
        .take_while(|byte| matches!(**byte, b' ' | b'\t'))
        .count();
    let trailing = bytes
        .iter()
        .rev()
        .take_while(|byte| matches!(**byte, b' ' | b'\t'))
        .count();
    let end = range.end.saturating_sub(trailing);
    let start = (range.start + leading).min(end);
    start..end
}

fn is_pipe_table_delimiter_row(line: &str) -> bool {
    let cells = pipe_table_segments(line);
    !cells.is_empty()
        && cells.into_iter().all(|cell| {
            let trimmed = trim_horizontal_whitespace_range(line, cell);
            if trimmed.is_empty() {
                return false;
            }

            let marker = &line[trimmed];
            marker.bytes().all(|byte| matches!(byte, b'-' | b':'))
                && marker.bytes().any(|byte| byte == b'-')
        })
}

fn format_pipe_row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

fn escape_markdown_table_cell(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for (index, ch) in text.char_indices() {
        if ch == '|' && !is_escaped_pipe(text, index) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_visible_rows_and_columns() {
        let model = TableModel::parse("| Name | Role |\n| --- | --- |\n| Ada | Eng |");

        assert_eq!(model.visible_row_count(), 2);
        assert_eq!(model.column_count(), 2);
        assert_eq!(
            model.cell_source_text(TableCellRef {
                visible_row: 1,
                column: 1,
            }),
            Some("Eng")
        );
    }

    #[test]
    fn finds_current_cell_from_hidden_boundary() {
        let model = TableModel::parse("| Ada | Eng |\n| --- | --- |\n| Bob | CTO |");
        let first = model.visible_row(0).expect("first row");
        let second_cell = first.cells.get(1).expect("second cell");

        assert_eq!(
            model.cell_ref_for_source_offset(
                second_cell.source_range.start,
                SelectionAffinity::Downstream
            ),
            Some(TableCellRef {
                visible_row: 0,
                column: 1,
            })
        );
    }

    #[test]
    fn appending_empty_row_keeps_existing_table() {
        let model = TableModel::parse("| Name | Role |\n| --- | --- |\n| Ada | Eng |");

        assert_eq!(
            model.append_empty_row(),
            "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n|  |  |"
        );
    }

    #[test]
    fn deleting_visible_row_rebuilds_markdown_and_preserves_delimiter() {
        let model = TableModel::parse("| H1 | H2 |\n| :--- | ---: |\n| A | B |\n| C | D |");

        assert_eq!(
            model.rebuild_markdown_without_row(1),
            Some("| H1 | H2 |\n| :--- | ---: |\n| C | D |".to_string())
        );
    }

    #[test]
    fn inserting_visible_row_after_current_rebuilds_markdown() {
        let model = TableModel::parse("| H1 | H2 |\n| --- | --- |\n| A | B |");

        assert_eq!(
            model.rebuild_markdown_with_inserted_row_after(0),
            Some("| H1 | H2 |\n| --- | --- |\n|  |  |\n| A | B |".to_string())
        );
    }

    #[test]
    fn inserting_column_preserves_existing_delimiter_alignment() {
        let model = TableModel::parse("| H1 | H2 |\n| :--- | ---: |\n| A | B |");

        assert_eq!(
            model.rebuild_markdown_with_inserted_column_after(0),
            Some("| H1 |  | H2 |\n| :--- | --- | ---: |\n| A |  | B |".to_string())
        );
    }

    #[test]
    fn deleting_column_rebuilds_markdown_and_keeps_alignment() {
        let model = TableModel::parse("| H1 | H2 | H3 |\n| :--- | --- | ---: |\n| A | B | C |");

        assert_eq!(
            model.rebuild_markdown_without_column(1),
            Some("| H1 | H3 |\n| :--- | ---: |\n| A | C |".to_string())
        );
    }
}

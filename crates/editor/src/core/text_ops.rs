use std::cmp;

pub(crate) fn count_document_words(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_word = false;

    for ch in text.chars() {
        if is_cjk_character(ch) {
            if in_word {
                count += 1;
                in_word = false;
            }
            count += 1;
        } else if ch.is_alphanumeric() {
            in_word = true;
        } else if in_word {
            count += 1;
            in_word = false;
        }
    }

    if in_word {
        count += 1;
    }

    count
}

pub(crate) fn adjust_block_markup(text: &str, deepen: bool) -> Option<String> {
    let mut lines = text.lines();
    let first = lines.next()?;
    let rest = if text.contains('\n') {
        text[first.len()..].to_string()
    } else {
        String::new()
    };

    let trimmed = first.trim_start();
    let indent = &first[..first.len().saturating_sub(trimmed.len())];

    if let Some(space_ix) = trimmed.find(' ') {
        let marker = &trimmed[..space_ix];
        if marker.chars().all(|ch| ch == '#') && !marker.is_empty() {
            let current = marker.len();
            let updated = if deepen {
                cmp::min(current + 1, 6)
            } else {
                current.saturating_sub(1)
            };
            let head = if updated == 0 {
                format!("{indent}{}", &trimmed[space_ix + 1..])
            } else {
                format!(
                    "{indent}{} {}",
                    "#".repeat(updated),
                    &trimmed[space_ix + 1..]
                )
            };
            return Some(format!("{head}{rest}"));
        }
    }

    let list_markers = ["- ", "* ", "+ ", "- [ ] ", "- [x] ", "* [ ] ", "* [x] "];
    if list_markers.iter().any(|marker| trimmed.starts_with(marker))
        || trimmed
            .split_once(". ")
            .map(|(n, _)| n.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
    {
        let updated_indent = if deepen {
            format!("{indent}  ")
        } else if indent.len() >= 2 {
            indent[..indent.len() - 2].to_string()
        } else {
            String::new()
        };

        let updated = text
            .lines()
            .map(|line| format!("{updated_indent}{}", line.trim_start()))
            .collect::<Vec<_>>()
            .join("\n");
        return Some(updated);
    }

    if deepen {
        Some(format!("# {text}"))
    } else {
        None
    }
}

pub(crate) fn byte_offset_for_line_column(text: &str, target_line: usize, target_column: usize) -> usize {
    let mut offset = 0usize;

    for (line_ix, segment) in text.split('\n').enumerate() {
        if line_ix == target_line {
            return offset + byte_offset_for_char_column(segment, target_column);
        }

        offset += segment.len();
        if offset < text.len() {
            offset += 1;
        }
    }

    text.len()
}

fn byte_offset_for_char_column(text: &str, target_column: usize) -> usize {
    match text.char_indices().nth(target_column) {
        Some((offset, _)) => offset,
        None => text.len(),
    }
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0x3040..=0x30FF
            | 0x31F0..=0x31FF
            | 0xAC00..=0xD7AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_across_cjk_and_ascii() {
        assert_eq!(count_document_words("hello world"), 2);
        assert_eq!(count_document_words("你好 world"), 3);
    }

    #[test]
    fn adjusts_heading_markup() {
        assert_eq!(
            adjust_block_markup("# Title", true),
            Some("## Title".to_string())
        );
        assert_eq!(
            adjust_block_markup("## Title", false),
            Some("# Title".to_string())
        );
    }

    #[test]
    fn deepens_plain_text_into_heading() {
        assert_eq!(
            adjust_block_markup("Title", true),
            Some("# Title".to_string())
        );
    }

    #[test]
    fn maps_line_and_column_back_to_utf8_offset() {
        assert_eq!(byte_offset_for_line_column("abc\ndef", 0, 0), 0);
        assert_eq!(byte_offset_for_line_column("abc\ndef", 0, 2), 2);
        assert_eq!(byte_offset_for_line_column("abc\ndef", 1, 1), 5);
        assert_eq!(byte_offset_for_line_column("a\nworld", 1, 3), 5);
    }
}

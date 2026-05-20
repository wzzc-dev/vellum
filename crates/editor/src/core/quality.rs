use std::time::Instant;

use super::{DisplayMap, DocumentBuffer, HiddenSyntaxPolicy, RenderSpanKind, SelectionModel};

const QUALITY_CORPUS: &[(&str, &str)] = &[
    (
        "daily_note",
        "# Daily Note\n\n- [x] Ship HTML export\n- [ ] Review outline\n\n> [!note] Writing\n> Keep the editor quiet.\n",
    ),
    (
        "research_note",
        "[toc]\n\n# Paper\n\n## Summary\n\nA link to [Vellum](https://example.com) and `inline code`.\n\n[^1]: Footnote text.\n",
    ),
    (
        "technical_note",
        "# API\n\n| Name | Value |\n| --- | --- |\n| alpha | `1` |\n\n```rust\nfn main() {}\n```\n",
    ),
    (
        "math_note",
        "# Math\n\nInline $E = mc^2$ and block math:\n\n$$\n\\int_0^1 x^2 dx\n$$\n",
    ),
    (
        "front_matter",
        "---\ntitle: Draft\nstatus: review\n---\n\n# Draft\n\nContent with ==highlight== and ^sup^ text.\n",
    ),
];

#[test]
fn quality_corpus_builds_stable_display_maps() {
    for (name, source) in QUALITY_CORPUS {
        let document = DocumentBuffer::from_text(source);
        let map = DisplayMap::from_document(&document, None, HiddenSyntaxPolicy::SelectionAware);

        assert!(
            !map.blocks.is_empty(),
            "corpus case {name} should produce render blocks"
        );
        assert!(
            map.visible_text.is_char_boundary(map.visible_text.len()),
            "corpus case {name} should keep visible text on UTF-8 boundaries"
        );
        assert!(
            map.blocks.iter().all(|block| block.visible_range.end <= map.visible_text.len()),
            "corpus case {name} should keep block ranges inside visible text"
        );
    }
}

#[test]
fn quality_corpus_keeps_visible_to_source_mapping_monotonic() {
    for (name, source) in QUALITY_CORPUS {
        let document = DocumentBuffer::from_text(source);
        let map = DisplayMap::from_document(&document, None, HiddenSyntaxPolicy::SelectionAware);
        let mut last_source = 0usize;

        for visible_offset in map.visible_text.char_indices().map(|(offset, _)| offset) {
            let hit = map.visible_to_source(visible_offset);
            assert!(
                hit.source_offset >= last_source,
                "corpus case {name} moved source mapping backward at visible offset {visible_offset}"
            );
            last_source = hit.source_offset;
        }
    }
}

#[test]
fn quality_corpus_reveals_selected_markup_without_losing_text() {
    let source = "# Title\n\nA [link](https://example.com) and **strong** text.";
    let link_start = source.find("[link]").expect("link fixture") + 2;
    let selection = SelectionModel::collapsed(link_start);

    let document = DocumentBuffer::from_text(source);
    let hidden = DisplayMap::from_document(&document, None, HiddenSyntaxPolicy::SelectionAware);
    let revealed =
        DisplayMap::from_document(&document, Some(&selection), HiddenSyntaxPolicy::SelectionAware);

    assert!(hidden.visible_text.contains("link"));
    assert!(revealed.visible_text.contains("[link](https://example.com)"));
    assert!(revealed.visible_text.len() > hidden.visible_text.len());
}

#[test]
fn quality_corpus_marks_embedded_surfaces() {
    let source =
        "![alt](./image.png)\n\n```moonbit\nfn main { () }\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n";
    let document = DocumentBuffer::from_text(source);
    let map = DisplayMap::from_document(&document, None, HiddenSyntaxPolicy::SelectionAware);

    assert!(
        map.blocks.iter().any(|block| block.embedded.is_some()),
        "image, code, or table blocks should be marked as embedded surfaces"
    );
    assert!(
        map.blocks
            .iter()
            .flat_map(|block| &block.spans)
            .any(|span| span.kind == RenderSpanKind::Text),
        "embedded-heavy documents still need text spans for mapping"
    );
}

#[test]
#[ignore = "manual performance smoke test for editor display-map work"]
fn performance_smoke_builds_large_display_map_quickly() {
    let mut source = String::new();
    for index in 0..250 {
        source.push_str(&format!(
            "# Heading {index}\n\nParagraph with **bold** and [link](https://example.com/{index}).\n\n- [ ] task\n- item\n\n| A | B |\n| - | - |\n| {index} | value |\n\n"
        ));
    }

    let started = Instant::now();
    let document = DocumentBuffer::from_text(&source);
    let map = DisplayMap::from_document(&document, None, HiddenSyntaxPolicy::SelectionAware);
    let elapsed = started.elapsed();

    assert!(!map.visible_text.is_empty());
    eprintln!(
        "built {} source bytes into {} visible bytes and {} blocks in {:?}",
        source.len(),
        map.visible_text.len(),
        map.blocks.len(),
        elapsed
    );
}

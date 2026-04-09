use std::time::Instant;

use editor::DocumentBuffer;

const SECTIONS: usize = 400;
const ITERATIONS: usize = 2_000;

fn main() {
    let sample = build_sample(SECTIONS);
    let marker = format!("section-{}/middle-anchor", SECTIONS / 2);
    let mut cursor = sample
        .find(&marker)
        .expect("sample marker should exist")
        + marker.len();
    let mut document = DocumentBuffer::from_text(&sample);

    let start = Instant::now();
    for iteration in 0..ITERATIONS {
        if iteration % 2 == 0 {
            document.replace_range(cursor..cursor, "x");
            cursor += 1;
        } else {
            document.replace_range(cursor - 1..cursor, "");
            cursor -= 1;
        }
    }
    let elapsed = start.elapsed();

    assert_eq!(
        document.text(),
        sample,
        "alternating insert/delete loop should restore the original sample",
    );

    println!("sample_bytes={}", sample.len());
    println!("blocks={}", document.blocks().len());
    println!("iterations={ITERATIONS}");
    println!("parse_version={}", document.parse_version());
    println!("total_ms={}", elapsed.as_millis());
    println!(
        "avg_us_per_edit={:.2}",
        elapsed.as_secs_f64() * 1_000_000.0 / ITERATIONS as f64
    );
}

fn build_sample(section_count: usize) -> String {
    let mut text = String::new();
    for index in 0..section_count {
        text.push_str(&format!("# Section {index}\n\n"));
        text.push_str(&format!(
            "Paragraph {index} starts here with some body text and {}.\n\n",
            if index == section_count / 2 {
                format!("section-{index}/middle-anchor")
            } else {
                "a stable marker".to_string()
            }
        ));
        text.push_str("- item one\n- item two\n\n");
        text.push_str("> quoted line\n> continued quote\n\n");
        text.push_str("```rust\nfn section_{index}() {{}}\n```\n\n");
    }

    text
}

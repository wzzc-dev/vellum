---
title: Vellum Longform Acceptance
tags: [acceptance, export]
---

# Vellum Longform Acceptance

[toc]

## Draft Scope

This document is a compact end-to-end sample for longform writing. It exercises headings, outline entries, lists, tables, code, local images, inline math, block math, diagrams, footnotes, and a generated table of contents.

The exported article should keep local companion links portable, such as the [acceptance appendix](assets/appendix.md#notes).

<p>Raw HTML links should stay portable too: <a href="assets/appendix.md?raw=1#notes">appendix source</a>.</p>

![Acceptance cover](assets/cover.svg#acceptance-cover)

![Referenced local diagram][asset-diagram]

[asset-diagram]: assets/reference-diagram.svg "Referenced local diagram"

<picture>
  <source media="(min-width: 720px)" srcset="assets/cover.svg 1x, assets/reference-diagram.svg#workflow 2x">
  <img src="assets/cover.svg" srcset="assets/reference-diagram.svg 640w" alt="Responsive local asset export">
</picture>

## Editing Blocks

- Draft the introduction.
- Review the outline.
- Export the finished article.

| Area | Expected behavior | Status |
| --- | --- | --- |
| Editor | Live preview keeps Markdown structure editable | Ready |
| Assets | Local image paths remain document-relative | Ready |
| Export | HTML includes copied local assets and print styles | Ready |

## Code Sample

```rust
fn export_status(ok: bool) -> &'static str {
    if ok {
        "ready"
    } else {
        "needs review"
    }
}
```

## Layout Stress

The export should keep wide content readable without clipping in browser print preview.

| Surface | Long content that should wrap or scroll | Print expectation |
| --- | --- | --- |
| Table | `workspace/assets/2026/research-notes/very-long-reference-name-without-natural-breaks.md#portable-export-validation` | Cells stay inside the printable page. |
| Code | `cargo test --workspace --all-targets --features acceptance-layout-print-checks` | Long commands wrap in print output. |
| Image | The responsive cover and diagram above should scale down instead of overflowing. | Images fit within page margins. |

```text
export --input docs/acceptance/longform.md --output /tmp/vellum-longform-export-with-a-very-long-file-name-that-must-wrap-in-print-preview.html --include-assets --verify-local-links
```

## Math Notes

Inline math should render in exported HTML: $E = mc^2$.

$$
\int_0^1 x^2 dx = \frac{1}{3}
$$

Aligned math should keep rows and alignment points readable in preview and export:

$$
\begin{aligned}
a^2 + b^2 &= c^2 \\
E &= mc^2
\end{aligned}
$$

Piecewise math should keep conditions readable in preview and export:

$$
\begin{cases}
x^2 & x > 0 \\
0 & \text{otherwise}
\end{cases}
$$

Binomial math should keep combination notation readable in preview and export:

$$
\binom{n}{k} = {n \choose k} = \frac{n!}{k!(n-k)!}
$$

Accent and brace math should keep annotated variables readable in preview and export:

$$
\hat{x} + \vec{v} + \dot{y} + \underbrace{a + b}_{group}
$$

## Mermaid Diagrams

```mermaid
flowchart TD
  Draft[Draft note] --> Save[Save and reopen]
  Save --> Export[Export HTML]
  Export --> Print[Print or save PDF]
```

```mermaid
sequenceDiagram
  participant Writer
  participant App as Vellum
  participant Browser
  Writer->>App: Export print HTML
  Note over Writer,App: Sequence note remains visible
  loop Print review
  App-->>Browser: Open exported document
  end
  Browser-->>Writer: Save PDF
```

## Footnote

The acceptance pass should verify that footnotes survive export and remain linked.[^acceptance]

[^acceptance]: This footnote checks GFM footnote rendering in the exported document.

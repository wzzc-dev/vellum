# Manual Acceptance Checklist

Use `docs/acceptance/longform.md` for this pass. The checklist is intentionally short enough to run before a release, but broad enough to catch regressions in the main writing flow.

## Editing

- Open `docs/acceptance/longform.md` from the workspace file tree.
- Confirm the editor shows a quiet live preview view and the document remains editable.
- Confirm the front matter renders as a metadata preview and returns to source editing when clicked.
- Confirm the outline sidebar lists the top-level title and each section heading.
- Confirm the `[toc]` block renders a table of contents from the document headings and returns to source editing when clicked.
- Confirm clicking a heading inside the rendered `[toc]` block moves the cursor to that heading.
- Edit text inside a paragraph, a table cell, the code block, and the math block.
- Confirm the status bar shows words, characters, lines, and reading time, and updates after editing text.
- Confirm the aligned, piecewise, binomial, accent, array, and alignat math blocks remain readable when the cursor leaves them.
- Confirm the Mermaid blocks show structured previews when the cursor leaves them and return to source editing when clicked.
- Confirm the Mermaid flowchart subgraph label remains visible in preview.
- Confirm the Mermaid sequence numbering, note, loop fragment, and lifecycle events remain visible in preview.
- Confirm the Mermaid state diagram transitions and note remain visible in preview.
- Confirm the Mermaid class diagram classes, members, and relationships remain visible in preview.
- Confirm the Mermaid ER diagram entities, attributes, and relationships remain visible in preview.
- Confirm the Mermaid pie chart title, slices, and values remain visible in preview.
- Confirm footnote references render as superscript markers and footnote definitions show a preview when the cursor leaves them.
- Save, close the tab, reopen the file, and confirm the edits persist.
- Use the file tree filter to find a nested Markdown file, open it, then clear the filter.
- Open a Markdown file from a nested workspace folder and confirm the sidebar remains rooted at the original workspace folder.

## Assets

- Confirm the local cover image renders from `docs/acceptance/assets/cover.svg`.
- Confirm the referenced local diagram renders from `docs/acceptance/assets/reference-diagram.svg`.
- Confirm the responsive raw HTML image renders and uses local `srcset` candidates.
- Paste or drop a test image into the document.
- Confirm the inserted image is written under the configured image asset folder and the Markdown path is document-relative.
- Drop multiple local image or attachment files and confirm they are inserted as separate paragraphs.
- Select text, paste a local file path over it, and confirm the selection becomes a Markdown link with a document-relative destination when the path is under the same workspace.

## External File Changes

- With the document open, edit the Markdown file in another editor and save it.
- Confirm Vellum detects the external change and offers the expected reload or keep-current conflict path when local edits exist.
- Rename the file in the workspace tree and confirm the open tab follows the new path.
- Try renaming a file with a path separator or `..` and confirm the rename is rejected without moving it.
- Restart the app after the rename and confirm the last-opened document restores from the new path.
- Try opening a deleted recent file and confirm it is removed from the recent files menu.
- Open another Markdown file from the workspace tree, use Save As for a copied file, and confirm both paths appear at the top of the recent files menu.
- Delete a copied test file from the workspace tree and confirm the editor shows a missing-file banner or status.

## Export And Print

- Export HTML from the app.
- Use Export and Open for Print from the app.
- Confirm the exported HTML opens in a browser with the table of contents, table, code block, images, math, Mermaid diagrams, and footnote present.
- Confirm the exported flowchart includes the subgraph label when browser Mermaid rendering is unavailable.
- Confirm the exported sequence diagram includes numbering, the note, loop fragment, and lifecycle events when browser Mermaid rendering is unavailable.
- Confirm the exported state diagram includes transitions and the note when browser Mermaid rendering is unavailable.
- Confirm the exported class diagram includes classes, members, and relationships when browser Mermaid rendering is unavailable.
- Confirm the exported ER diagram includes entities, attributes, and relationships when browser Mermaid rendering is unavailable.
- Confirm the exported pie chart includes the title, slices, and values when browser Mermaid rendering is unavailable.
- Confirm local images are copied beside the exported HTML in the generated assets folder.
- Confirm the local appendix links, including the raw HTML link, point into the generated assets folder and open from the exported HTML.
- Confirm responsive image `srcset` entries point at the generated assets folder and keep any fragment suffixes.
- Confirm the linked local stylesheet and its imported stylesheet point at the generated assets folder and keep their local image references portable.
- Open the browser print dialog for the exported HTML and verify the preview uses a white page, document margins, wrapped code, and no clipped table or image.
- Save the browser print preview as PDF and confirm the PDF contains the same visible content.

## Preferences

- Open Preferences from the app.
- Change the syntax theme, editor font size, sidebar visibility, status bar, focus mode, typewriter mode, focus highlight, and image asset folder.
- Restart the app and confirm the preferences persist.

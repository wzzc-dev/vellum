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
- Confirm the Mermaid blocks show structured previews when the cursor leaves them and return to source editing when clicked.
- Confirm footnote references render as superscript markers and footnote definitions show a preview when the cursor leaves them.
- Save, close the tab, reopen the file, and confirm the edits persist.

## Assets

- Confirm the local cover image renders from `docs/acceptance/assets/cover.svg`.
- Confirm the referenced local diagram renders from `docs/acceptance/assets/reference-diagram.svg`.
- Confirm the responsive raw HTML image renders and uses local `srcset` candidates.
- Paste or drop a test image into the document.
- Confirm the inserted image is written under the configured image asset folder and the Markdown path is document-relative.

## External File Changes

- With the document open, edit the Markdown file in another editor and save it.
- Confirm Vellum detects the external change and offers the expected reload or keep-current conflict path when local edits exist.
- Rename the file in the workspace tree and confirm the open tab follows the new path.
- Restart the app after the rename and confirm the last-opened document restores from the new path.
- Delete a copied test file from the workspace tree and confirm the editor shows a missing-file banner or status.

## Export And Print

- Export HTML from the app.
- Use Export and Open for Print from the app.
- Confirm the exported HTML opens in a browser with the table of contents, table, code block, images, math, Mermaid diagrams, and footnote present.
- Confirm local images are copied beside the exported HTML in the generated assets folder.
- Confirm responsive image `srcset` entries point at the generated assets folder and keep any fragment suffixes.
- Open the browser print dialog for the exported HTML and verify the preview uses a white page, document margins, wrapped code, and no clipped table or image.
- Save the browser print preview as PDF and confirm the PDF contains the same visible content.

## Preferences

- Open Preferences from the app.
- Change the syntax theme, editor font size, sidebar visibility, status bar, focus mode, typewriter mode, focus highlight, and image asset folder.
- Restart the app and confirm the preferences persist.

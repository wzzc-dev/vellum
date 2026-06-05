# Manual Acceptance Checklist

Use `docs/acceptance/longform.md` for this pass. The checklist is intentionally short enough to run before a release, but broad enough to catch regressions in the main writing flow.

## Editing

- Open `docs/acceptance/longform.md` from the workspace file tree.
- Confirm the editor shows a quiet live preview view and the document remains editable.
- Confirm the outline sidebar lists the top-level title and each section heading.
- Confirm the `[toc]` block renders a table of contents from the document headings and returns to source editing when clicked.
- Edit text inside a paragraph, a table cell, the code block, and the math block.
- Confirm the Mermaid block shows a structured preview when the cursor leaves it and returns to source editing when clicked.
- Save, close the tab, reopen the file, and confirm the edits persist.

## Assets

- Confirm the local cover image renders from `docs/acceptance/assets/cover.svg`.
- Paste or drop a test image into the document.
- Confirm the inserted image is written under the configured image asset folder and the Markdown path is document-relative.

## External File Changes

- With the document open, edit the Markdown file in another editor and save it.
- Confirm Vellum detects the external change and offers the expected reload or keep-current conflict path when local edits exist.
- Rename the file in the workspace tree and confirm the open tab follows the new path.
- Delete a copied test file from the workspace tree and confirm the missing-file status is visible.

## Export And Print

- Export HTML from the app.
- Use Export and Open for Print from the app.
- Confirm the exported HTML opens in a browser with the table of contents, table, code block, image, math, Mermaid diagram, and footnote present.
- Confirm local images are copied beside the exported HTML in the generated assets folder.
- Open the browser print dialog for the exported HTML and verify the preview uses a white page, document margins, wrapped code, and no clipped table or image.
- Save the browser print preview as PDF and confirm the PDF contains the same visible content.

## Preferences

- Open Preferences from the app.
- Change the syntax theme, sidebar visibility, status bar, focus mode, typewriter mode, focus highlight, and image asset folder.
- Restart the app and confirm the preferences persist.

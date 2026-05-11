# Vellum Plugin SDK

This package contains plugin-focused helpers on top of `vellum/app-sdk`.

Current helpers:

- New plugins will use `vellum.toml` with `kind = "plugin"`.
- Plugin contributions are represented by `commands` and `panels` in the unified manifest.
- `src/lib.mbt` exposes panel-oriented helpers such as `panel_column`, `panel_title`, `panel_hint`, and `show_status`.
- The runnable reference lives in `../../examples/plugins/counter`.

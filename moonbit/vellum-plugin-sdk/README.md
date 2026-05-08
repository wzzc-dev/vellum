# Vellum Plugin SDK

This is the staging package for MoonBit plugin components that will use the same typed `vellum:app/app-world` UI and event protocol as apps.

Current status:

- Legacy extension-world examples live in `../legacy-extensions`.
- New plugins will use `vellum.toml` with `kind = "plugin"`.
- Plugin contributions are represented by `commands` and `panels` in the unified manifest.

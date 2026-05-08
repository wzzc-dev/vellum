# Vellum App SDK

This is the staging package for the MoonBit DSL that targets `vellum:app/app-world`.

Current status:

- The canonical WIT lives at `../../wit/vellum-app.wit`.
- `../demos/markdown-editor` is the first runnable app component.
- The demo still keeps generated WIT bindings locally because MoonBit bindings are component-package specific.

The intended public API is a typed DSL around `Column`, `Row`, `Text`, `Button`, `Input`, `Tabs`, `SplitView`, `ScrollView`, and `NativeView`.

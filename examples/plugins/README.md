# Vellum Plugin Examples

This directory contains typed plugin component examples for `vellum:app/app-world`.

Current example:

- `counter/` builds a minimal plugin panel component with `vellum.toml`, a typed `ViewTree`, and a `Ping Host` action.

The handwritten plugin logic lives in `src/`, while `gen/world/appWorld/` only forwards the WIT exports to that implementation.

Build it with:

```bash
cd examples/plugins/counter
./build.sh
```

Or build every plugin example:

```bash
cd examples/plugins
./build-all.sh
```

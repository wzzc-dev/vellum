# Week 1-6 examples

The old week 1-6 MoonBit GUI snippets used the obsolete `@gui.*` pseudo API.
They have been replaced by standalone Vellum extension projects:

- `counter_demo.mbt` -> `../counter`
- `form_demo.mbt` -> `../form`
- `todo_list_demo.mbt` -> `../todo-list`
- `timer_demo.mbt` -> `../timer`
- `navigation_demo.mbt` -> `../navigation`
- `gesture_demo.mbt` -> `../gesture`

Run `../build-all.sh` to build the current examples as WASM Component Model
extensions for Vellum's `vellum:extension/extension-world` host.

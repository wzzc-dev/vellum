fn str_to_bytes(s : String) -> FixedArray[Byte] {
  let buf = @encoding/utf8.encode(s)
  let arr : FixedArray[Byte] = FixedArray::makei(buf.length(), fn(i) { buf[i] })
  arr
}

fn versioned_json(json_str : String) -> FixedArray[Byte] {
  let payload = "{\"version\":1,\"data\":" + json_str + "}"
  str_to_bytes(payload)
}

fn ui_column(children : Array[String]) -> String {
  "{\"Column\":{\"children\":[" + children.iter().join(",") + "],\"gap\":8.0,\"padding\":null,\"scrollable\":false}}"
}

fn ui_row(children : Array[String]) -> String {
  "{\"Row\":{\"children\":[" + children.iter().join(",") + "],\"gap\":8.0,\"padding\":null}}"
}

fn ui_text(content : String) -> String {
  "{\"Text\":{\"content\":\"" + content + "\",\"style\":{\"size\":null,\"color\":null,\"bold\":null,\"italic\":null,\"monospace\":null}}}"
}

fn ui_large_text(content : String) -> String {
  "{\"Text\":{\"content\":\"" + content + "\",\"style\":{\"size\":36.0,\"color\":null,\"bold\":true,\"italic\":null,\"monospace\":null}}}"
}

fn ui_heading(content : String, level : Int) -> String {
  "{\"Heading\":{\"content\":\"" + content + "\",\"level\":" + level.to_string() + "}}"
}

fn ui_button(id : String, label : String, variant : String) -> String {
  "{\"Button\":{\"id\":\"" + id + "\",\"label\":\"" + label + "\",\"variant\":\"" + variant + "\",\"icon\":null,\"disabled\":false}}"
}

fn ui_separator() -> String {
  "\"Separator\""
}

fn refresh_ui() -> Unit {
  let root = build_panel_ui()
  ignore(@ui.set_panel_view("moonbit-gui", versioned_json(root)))
}

fn build_panel_ui() -> String {
  ui_column([
    ui_heading("MoonBit GUI Demo", 2),
    ui_separator(),
    ui_large_text(state.counter.to_string()),
    ui_text("Click buttons to update the counter!"),
    ui_row([
      ui_button("moonbit-gui-decrement", "-", "Secondary"),
      ui_button("moonbit-gui-reset", "Reset", "Ghost"),
      ui_button("moonbit-gui-increment", "+", "Primary"),
    ]),
    ui_separator(),
    ui_text("This is a simple MoonBit extension UI demo."),
  ])
}

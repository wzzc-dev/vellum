fn json_escape(s : String) -> String {
  let mut out = ""
  for i in 0..<s.length() {
    let code = s.code_unit_at(i)
    out = out + (match code {
      8 => "\\b"
      9 => "\\t"
      10 => "\\n"
      12 => "\\f"
      13 => "\\r"
      34 => "\\\""
      92 => "\\\\"
      _ => if code < 32 { "" } else { code.unsafe_to_char().to_string() }
    })
  };
  out
}

fn json_string(s : String) -> String {
  "\"" + json_escape(s) + "\""
}

fn json_string_opt(value : String?) -> String {
  match value {
    Some(v) => json_string(v)
    None => "null"
  }
}

fn json_bool_opt(value : Bool?) -> String {
  match value {
    Some(v) => v.to_string()
    None => "null"
  }
}

fn json_double_opt(value : Double?) -> String {
  match value {
    Some(v) => v.to_string()
    None => "null"
  }
}

fn str_to_bytes(s : String) -> FixedArray[Byte] {
  let buf = @encoding/utf8.encode(s)
  FixedArray::makei(buf.length(), fn(i) { buf[i] })
}

fn versioned_json(json_str : String) -> FixedArray[Byte] {
  str_to_bytes("{\"version\":1,\"data\":" + json_str + "}")
}

fn ui_style(size : Double?, color : String?, bold : Bool?, italic : Bool?, monospace : Bool?) -> String {
  "{\"size\":" + json_double_opt(size) +
  ",\"color\":" + json_string_opt(color) +
  ",\"bold\":" + json_bool_opt(bold) +
  ",\"italic\":" + json_bool_opt(italic) +
  ",\"monospace\":" + json_bool_opt(monospace) + "}"
}

fn ui_column(children : Array[String]) -> String {
  "{\"Column\":{\"children\":[" + children.iter().join(",") +
  "],\"gap\":8.0,\"padding\":null,\"scrollable\":false}}"
}

fn ui_row(children : Array[String]) -> String {
  "{\"Row\":{\"children\":[" + children.iter().join(",") +
  "],\"gap\":8.0,\"padding\":null}}"
}

fn ui_text(content : String) -> String {
  "{\"Text\":{\"content\":" + json_string(content) +
  ",\"style\":" + ui_style(None, None, None, None, None) + "}}"
}

fn ui_small_text(content : String) -> String {
  "{\"Text\":{\"content\":" + json_string(content) +
  ",\"style\":" + ui_style(Some(11.0), Some("muted-foreground"), None, None, None) + "}}"
}

fn ui_large_text(content : String) -> String {
  "{\"Text\":{\"content\":" + json_string(content) +
  ",\"style\":" + ui_style(Some(36.0), None, Some(true), None, None) + "}}"
}

fn ui_mono_text(content : String) -> String {
  "{\"Text\":{\"content\":" + json_string(content) +
  ",\"style\":" + ui_style(None, None, None, None, Some(true)) + "}}"
}

fn ui_heading(content : String, level : Int) -> String {
  "{\"Heading\":{\"content\":" + json_string(content) + ",\"level\":" + level.to_string() + "}}"
}

fn ui_button(id : String, label : String, variant : String) -> String {
  "{\"Button\":{\"id\":" + json_string(id) +
  ",\"label\":" + json_string(label) +
  ",\"variant\":" + json_string(variant) +
  ",\"icon\":null,\"disabled\":false}}"
}

fn ui_disabled_button(id : String, label : String, variant : String) -> String {
  "{\"Button\":{\"id\":" + json_string(id) +
  ",\"label\":" + json_string(label) +
  ",\"variant\":" + json_string(variant) +
  ",\"icon\":null,\"disabled\":true}}"
}

fn ui_text_input(id : String, placeholder : String, value : String) -> String {
  "{\"TextInput\":{\"id\":" + json_string(id) +
  ",\"placeholder\":" + json_string(placeholder) +
  ",\"value\":" + json_string(value) +
  ",\"single_line\":true}}"
}

fn ui_checkbox(id : String, label : String, checked : Bool) -> String {
  "{\"Checkbox\":{\"id\":" + json_string(id) +
  ",\"label\":" + json_string(label) +
  ",\"checked\":" + checked.to_string() + "}}"
}

fn ui_select(id : String, options : Array[String], selected : Int) -> String {
  let selected_json = if selected < 0 { "null" } else { selected.to_string() }
  "{\"Select\":{\"id\":" + json_string(id) +
  ",\"options\":[" + options.map(fn(opt) { json_string(opt) }).iter().join(",") +
  "],\"selected\":" + selected_json + "}}"
}

fn ui_badge(label : String, severity : String?) -> String {
  "{\"Badge\":{\"label\":" + json_string(label) +
  ",\"severity\":" + json_string_opt(severity) + "}}"
}

fn ui_progress(value : Double, label : String?) -> String {
  "{\"Progress\":{\"value\":" + value.to_string() + ",\"label\":" + json_string_opt(label) + "}}"
}

fn ui_separator() -> String {
  "\"Separator\""
}

fn ui_list_item(id : String, label : String, description : String?, severity : String?) -> String {
  "{\"id\":" + json_string(id) +
  ",\"label\":" + json_string(label) +
  ",\"description\":" + json_string_opt(description) +
  ",\"icon\":null,\"severity\":" + json_string_opt(severity) +
  ",\"children\":[]}"
}

fn ui_list(items : Array[String]) -> String {
  "{\"List\":{\"items\":[" + items.iter().join(",") + "]}}"
}

fn ui_disclosure(label : String, open : Bool, children : Array[String]) -> String {
  "{\"Disclosure\":{\"label\":" + json_string(label) +
  ",\"open\":" + open.to_string() +
  ",\"children\":[" + children.iter().join(",") + "]}}"
}

fn set_panel(panel_id : String, root : String) -> Unit {
  ignore(@ui.set_panel_view(panel_id, versioned_json(root)))
}

fn status(message : String) -> Unit {
  ignore(@host.show_status_message(message))
}

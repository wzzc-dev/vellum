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

fn ui_badge(label : String) -> String {
  "{\"Badge\":{\"label\":\"" + label + "\",\"severity\":null}}"
}

fn ui_progress(value : Double) -> String {
  "{\"Progress\":{\"value\":" + value.to_string() + ",\"label\":null}}"
}

fn ui_separator() -> String {
  "\"Separator\""
}

fn format_remaining(ms : UInt64) -> String {
  let total_secs : Int = UInt64::to_int(ms / 1000UL)
  let mins = total_secs / 60
  let secs = total_secs % 60
  mins.to_string() + ":" + pad_zero(secs)
}

fn pad_zero(n : Int) -> String {
  if n < 10 { "0" + n.to_string() } else { n.to_string() }
}

fn refresh_ui() -> Unit {
  let root = build_panel_ui()
  ignore(@ui.set_panel_view("pomodoro", versioned_json(root)))
}

fn uint64_to_double(v : UInt64) -> Double {
  Int64::to_double(v.reinterpret_as_int64())
}

fn build_panel_ui() -> String {
  let (remaining_text, mode_text, progress_value, start_pause_label, badge_label) =
    match state.timer_state {
      Idle => ("25:00", "Focus", 0.0, "Start", "Ready")
      Running(data) => {
        let now = @timer.now_ms()
        let elapsed = now - data.started_at_ms
        let remaining = if data.total_duration_ms > elapsed { data.total_duration_ms - elapsed } else { 0UL }
        let progress : Double = 1.0 - uint64_to_double(remaining) / uint64_to_double(data.total_duration_ms)
        (format_remaining(remaining), data.mode.label(), progress, "Pause", "Running")
      }
      Paused(data) => {
        let progress : Double = 1.0 - uint64_to_double(data.remaining_ms) / uint64_to_double(data.total_duration_ms)
        (format_remaining(data.remaining_ms), data.mode.label(), progress, "Start", "Paused")
      }
      Completed => ("00:00", "Done", 1.0, "Restart", "Done")
    }

  ui_column([
    ui_heading(mode_text, 2),
    ui_large_text(remaining_text),
    ui_progress(progress_value),
    ui_badge(badge_label),
    ui_row([
      ui_button("pomodoro-start-pause", start_pause_label, "Primary"),
      ui_button("pomodoro-reset", "Reset", "Secondary"),
      ui_button("pomodoro-skip", "Skip", "Ghost"),
    ]),
    ui_separator(),
    ui_text("Focus rounds: " + state.completed_focus_count.to_string()),
  ])
}

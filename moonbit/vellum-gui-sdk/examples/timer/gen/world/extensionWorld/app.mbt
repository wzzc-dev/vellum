pub(all) struct TimerState {
  mut running : Bool
  mut remaining_ms : UInt64
  mut total_ms : UInt64
  mut mode : String
  mut completed_focus_count : Int
}

let state : TimerState = {
  running: false,
  remaining_ms: 1500000UL,
  total_ms: 1500000UL,
  mode: "Focus",
  completed_focus_count: 0,
}

fn uint64_to_double(v : UInt64) -> Double {
  Int64::to_double(v.reinterpret_as_int64())
}

fn pad_zero(n : Int) -> String {
  if n < 10 { "0" + n.to_string() } else { n.to_string() }
}

fn format_remaining(ms : UInt64) -> String {
  let total_secs = UInt64::to_int(ms / 1000UL)
  let mins = total_secs / 60
  let secs = total_secs % 60
  mins.to_string() + ":" + pad_zero(secs)
}

fn progress_value() -> Double {
  if state.total_ms == 0UL {
    1.0
  } else {
    1.0 - uint64_to_double(state.remaining_ms) / uint64_to_double(state.total_ms)
  }
}

fn refresh_ui() -> Unit {
  set_panel("timer", build_panel_ui())
}

fn build_panel_ui() -> String {
  let running_label = if state.running { "Pause" } else { "Start" }
  let status_label = if state.running { "Running" } else { "Ready" }
  ui_column([
    ui_heading(state.mode, 2),
    ui_large_text(format_remaining(state.remaining_ms)),
    ui_progress(progress_value(), Some(status_label)),
    ui_badge(status_label, if state.running { Some("Info") } else { Some("Hint") }),
    ui_row([
      ui_button("timer-start-pause", running_label, "Primary"),
      ui_button("timer-reset", "Reset", "Secondary"),
      ui_button("timer-skip", "Skip", "Ghost"),
    ]),
    ui_separator(),
    ui_text("Focus rounds: " + state.completed_focus_count.to_string()),
    ui_small_text("Uses vellum:extension/timer request-tick"),
  ])
}

fn start_pause() -> Unit {
  if state.running {
    state.running = false
    ignore(@timer.cancel_tick())
    status("Timer paused")
  } else {
    state.running = true
    ignore(@timer.request_tick(1000U))
    status("Timer started")
  }
  refresh_ui()
}

fn reset_timer() -> Unit {
  state.running = false
  state.mode = "Focus"
  state.total_ms = 1500000UL
  state.remaining_ms = state.total_ms
  state.completed_focus_count = 0
  ignore(@timer.cancel_tick())
  status("Timer reset")
  refresh_ui()
}

fn skip_timer() -> Unit {
  if state.mode == "Focus" {
    state.completed_focus_count = state.completed_focus_count + 1
    state.mode = "Break"
    state.total_ms = 300000UL
  } else {
    state.mode = "Focus"
    state.total_ms = 1500000UL
  }
  state.remaining_ms = state.total_ms
  state.running = false
  ignore(@timer.cancel_tick())
  status("Skipped to " + state.mode)
  refresh_ui()
}

fn timer_tick() -> Unit {
  if state.running {
    if state.remaining_ms > 1000UL {
      state.remaining_ms = state.remaining_ms - 1000UL
      refresh_ui()
    } else {
      skip_timer()
    }
  } else {
    ignore(@timer.cancel_tick())
  }
}

pub fn activate(_ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  refresh_ui()
  Ok(())
}

pub fn deactivate() -> Result[Unit, @types.ExtensionError] {
  ignore(@timer.cancel_tick())
  Ok(())
}

pub fn handle_event(event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_type == "timer.tick" {
    timer_tick()
  }
  Ok(())
}

pub fn execute_command(command_id : String) -> Result[Unit, @types.ExtensionError] {
  match command_id {
    "timer.start-pause" | "vellum.sdk-example.timer.timer.start-pause" => start_pause()
    "timer.reset" | "vellum.sdk-example.timer.timer.reset" => reset_timer()
    "timer.skip" | "vellum.sdk-example.timer.timer.skip" => skip_timer()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "timer-start-pause" => start_pause()
      "timer-reset" => reset_timer()
      "timer-skip" => skip_timer()
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


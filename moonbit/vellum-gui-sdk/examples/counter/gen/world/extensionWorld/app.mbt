pub(all) struct CounterState {
  mut count : Int
  mut step : Int
}

let state : CounterState = {
  count: 0,
  step: 1,
}

fn refresh_ui() -> Unit {
  set_panel("counter", build_panel_ui())
}

fn build_panel_ui() -> String {
  let direction = if state.count > 0 {
    "Positive"
  } else if state.count < 0 {
    "Negative"
  } else {
    "Zero"
  }
  ui_column([
    ui_heading("Counter", 2),
    ui_separator(),
    ui_large_text(state.count.to_string()),
    ui_badge(direction, if state.count == 0 { Some("Info") } else { Some("Hint") }),
    ui_row([
      ui_button("counter-decrement", "-" + state.step.to_string(), "Secondary"),
      ui_button("counter-reset", "Reset", "Ghost"),
      ui_button("counter-increment", "+" + state.step.to_string(), "Primary"),
    ]),
    ui_row([
      ui_button("counter-step-down", "Step -", "Ghost"),
      ui_text("Step: " + state.step.to_string()),
      ui_button("counter-step-up", "Step +", "Ghost"),
    ]),
    ui_small_text("Command: Counter: Reset"),
  ])
}

fn increment() -> Unit {
  state.count = state.count + state.step
  status("Counter: " + state.count.to_string())
  refresh_ui()
}

fn decrement() -> Unit {
  state.count = state.count - state.step
  status("Counter: " + state.count.to_string())
  refresh_ui()
}

fn reset() -> Unit {
  state.count = 0
  status("Counter reset")
  refresh_ui()
}

fn change_step(delta : Int) -> Unit {
  let next = state.step + delta
  if next >= 1 && next <= 10 {
    state.step = next
  }
  refresh_ui()
}

pub fn activate(_ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  refresh_ui()
  Ok(())
}

pub fn deactivate() -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn handle_event(_event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  Ok(())
}

pub fn execute_command(command_id : String) -> Result[Unit, @types.ExtensionError] {
  if command_id == "counter.reset" || command_id == "vellum.sdk-example.counter.counter.reset" {
    reset()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "counter-increment" => increment()
      "counter-decrement" => decrement()
      "counter-reset" => reset()
      "counter-step-up" => change_step(1)
      "counter-step-down" => change_step(-1)
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


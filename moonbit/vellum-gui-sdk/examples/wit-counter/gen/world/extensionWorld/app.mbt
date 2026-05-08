pub(all) struct WitCounterState {
  mut count : Int
  mut last_action : String
}

let state : WitCounterState = {
  count: 0,
  last_action: "Activated through WIT exports",
}

fn refresh_ui() -> Unit {
  set_panel("wit-counter", ui_column([
    ui_heading("WIT Counter", 2),
    ui_separator(),
    ui_large_text(state.count.to_string()),
    ui_text(state.last_action),
    ui_row([
      ui_button("wit-counter-dec", "Decrement", "Secondary"),
      ui_button("wit-counter-reset", "Reset", "Ghost"),
      ui_button("wit-counter-inc", "Increment", "Primary"),
    ]),
    ui_disclosure("Component Model surface", true, [
      ui_mono_text("activate / deactivate"),
      ui_mono_text("execute-command"),
      ui_mono_text("handle-ui-event"),
      ui_mono_text("set-panel-view import"),
    ]),
  ]))
}

fn set_count(value : Int, action : String) -> Unit {
  state.count = value
  state.last_action = action
  status(action + ": " + state.count.to_string())
  refresh_ui()
}

pub fn activate(ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  state.last_action = "Activated " + ctx.extension_id
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
  if command_id == "wit-counter.reset" || command_id == "vellum.sdk-example.wit-counter.wit-counter.reset" {
    set_count(0, "Command reset")
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "wit-counter-inc" => set_count(state.count + 1, "UI increment")
      "wit-counter-dec" => set_count(state.count - 1, "UI decrement")
      "wit-counter-reset" => set_count(0, "UI reset")
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


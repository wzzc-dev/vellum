fn handle_increment() -> Unit {
  state.counter = state.counter + 1
  ignore(@host.show_status_message("Counter: " + state.counter.to_string()))
  refresh_ui()
}

fn handle_decrement() -> Unit {
  state.counter = state.counter - 1
  ignore(@host.show_status_message("Counter: " + state.counter.to_string()))
  refresh_ui()
}

fn handle_reset() -> Unit {
  state.counter = 0
  ignore(@host.show_status_message("Counter reset"))
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
  match command_id {
    "vellum.moonbit-gui.moonbit-gui.demo" => handle_reset()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "moonbit-gui-increment" => handle_increment()
      "moonbit-gui-decrement" => handle_decrement()
      "moonbit-gui-reset" => handle_reset()
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}

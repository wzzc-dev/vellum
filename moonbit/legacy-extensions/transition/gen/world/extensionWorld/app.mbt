pub(all) struct TransitionState {
  mut step : Int
  mut progress : Double
  mut message : String
}

let state : TransitionState = {
  step: 0,
  progress: 0.0,
  message: "Ready",
}

fn transition_name() -> String {
  match state.step {
    0 => "Fade"
    1 => "Slide"
    2 => "Scale"
    3 => "Modal"
    _ => "Shared Element"
  }
}

fn refresh_ui() -> Unit {
  set_panel("transition", build_panel_ui())
}

fn next_transition() -> Unit {
  state.step = (state.step + 1) % 5
  state.progress = state.progress + 0.25
  if state.progress > 1.0 {
    state.progress = 0.0
  }
  state.message = "Selected " + transition_name()
  status(state.message)
  refresh_ui()
}

fn reset_transition() -> Unit {
  state.step = 0
  state.progress = 0.0
  state.message = "Reset"
  refresh_ui()
}

fn build_panel_ui() -> String {
  ui_column([
    ui_heading("Transitions", 2),
    ui_badge(transition_name(), Some("Info")),
    ui_progress(state.progress, Some((state.progress * 100.0).to_string() + "%")),
    ui_row([
      ui_button("transition-next", "Next", "Primary"),
      ui_button("transition-reset", "Reset", "Ghost"),
    ]),
    ui_separator(),
    ui_list([
      ui_list_item("fade", "Fade", Some("Opacity based scene changes"), None),
      ui_list_item("slide", "Slide", Some("Directional push/pop motion"), None),
      ui_list_item("scale", "Scale", Some("Zoom style emphasis"), None),
      ui_list_item("modal", "Modal", Some("Presented over current content"), None),
      ui_list_item("shared", "Shared Element", Some("Track matching element identities"), None),
    ]),
    ui_small_text(state.message),
  ])
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
    "transition.next" | "vellum.sdk-example.transition.transition.next" => next_transition()
    "transition.reset" | "vellum.sdk-example.transition.transition.reset" => reset_transition()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "transition-next" => next_transition()
      "transition-reset" => reset_transition()
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


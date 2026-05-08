pub(all) struct GestureState {
  mut taps : Int
  mut pans : Int
  mut swipes : Int
  mut pinches : Int
  mut rotations : Int
  mut last : String
}

let state : GestureState = {
  taps: 0,
  pans: 0,
  swipes: 0,
  pinches: 0,
  rotations: 0,
  last: "Use buttons to simulate recognizers",
}

fn refresh_ui() -> Unit {
  set_panel("gesture", build_panel_ui())
}

fn record(kind : String) -> Unit {
  match kind {
    "Tap" => state.taps = state.taps + 1
    "Pan" => state.pans = state.pans + 1
    "Swipe" => state.swipes = state.swipes + 1
    "Pinch" => state.pinches = state.pinches + 1
    "Rotation" => state.rotations = state.rotations + 1
    _ => ()
  }
  state.last = kind
  status("Gesture: " + kind)
  refresh_ui()
}

fn reset_gestures() -> Unit {
  state.taps = 0
  state.pans = 0
  state.swipes = 0
  state.pinches = 0
  state.rotations = 0
  state.last = "Reset"
  refresh_ui()
}

fn build_panel_ui() -> String {
  ui_column([
    ui_heading("Gesture Types", 2),
    ui_badge("Last: " + state.last, Some("Info")),
    ui_row([
      ui_button("gesture-tap", "Tap", "Primary"),
      ui_button("gesture-pan", "Pan", "Secondary"),
      ui_button("gesture-swipe", "Swipe", "Ghost"),
    ]),
    ui_row([
      ui_button("gesture-pinch", "Pinch", "Ghost"),
      ui_button("gesture-rotation", "Rotation", "Ghost"),
      ui_button("gesture-reset", "Reset", "Secondary"),
    ]),
    ui_list([
      ui_list_item("tap", "Tap", Some(state.taps.to_string() + " recognitions"), Some("Info")),
      ui_list_item("pan", "Pan", Some(state.pans.to_string() + " recognitions"), None),
      ui_list_item("swipe", "Swipe", Some(state.swipes.to_string() + " recognitions"), None),
      ui_list_item("pinch", "Pinch", Some(state.pinches.to_string() + " recognitions"), None),
      ui_list_item("rotation", "Rotation", Some(state.rotations.to_string() + " recognitions"), None),
    ]),
    ui_disclosure("SDK gesture helpers represented", true, [
      ui_mono_text("tap_gesture / double_tap_gesture"),
      ui_mono_text("pan_gesture / swipe_right"),
      ui_mono_text("pinch_gesture / rotation_gesture"),
    ]),
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
  if command_id == "gesture.reset" || command_id == "vellum.sdk-example.gesture.gesture.reset" {
    reset_gestures()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "gesture-tap" => record("Tap")
      "gesture-pan" => record("Pan")
      "gesture-swipe" => record("Swipe")
      "gesture-pinch" => record("Pinch")
      "gesture-rotation" => record("Rotation")
      "gesture-reset" => reset_gestures()
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


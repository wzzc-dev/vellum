pub(all) struct ComprehensiveState {
  mut tab : Int
  mut dark : Bool
  mut interactions : Int
}

let state : ComprehensiveState = {
  tab: 0,
  dark: false,
  interactions: 0,
}

fn refresh_ui() -> Unit {
  set_panel("comprehensive", build_panel_ui())
}

fn tab_name() -> String {
  match state.tab {
    0 => "Overview"
    1 => "Widgets"
    2 => "State"
    3 => "Host"
    _ => "Overview"
  }
}

fn set_tab(tab : Int) -> Unit {
  state.tab = tab
  state.interactions = state.interactions + 1
  refresh_ui()
}

fn reset_overview() -> Unit {
  state.tab = 0
  state.dark = false
  state.interactions = 0
  refresh_ui()
}

fn tab_buttons() -> String {
  ui_row([
    ui_button("comp-tab-overview", "Overview", if state.tab == 0 { "Primary" } else { "Ghost" }),
    ui_button("comp-tab-widgets", "Widgets", if state.tab == 1 { "Primary" } else { "Ghost" }),
    ui_button("comp-tab-state", "State", if state.tab == 2 { "Primary" } else { "Ghost" }),
    ui_button("comp-tab-host", "Host", if state.tab == 3 { "Primary" } else { "Ghost" }),
  ])
}

fn tab_content() -> String {
  match state.tab {
    0 => ui_column([
      ui_text("Nine standalone examples build to WASM Components."),
      ui_list([
        ui_list_item("counter", "Counter", Some("Buttons and command handling"), Some("Info")),
        ui_list_item("form", "Form", Some("Inputs, select, checkbox, validation"), None),
        ui_list_item("timer", "Timer", Some("Host timer imports"), None),
      ]),
    ])
    1 => ui_column([
      ui_text("Supported host-rendered UI nodes are JSON encoded."),
      ui_list([
        ui_list_item("layout", "Column and Row", Some("Basic layout"), None),
        ui_list_item("controls", "Button, TextInput, Checkbox, Select", Some("Interactive controls"), None),
        ui_list_item("display", "Badge, Progress, List, Disclosure", Some("Data display"), None),
      ]),
    ])
    2 => ui_column([
      ui_text("State is kept in MoonBit globals with mutable record fields."),
      ui_badge("Interactions: " + state.interactions.to_string(), Some("Hint")),
      ui_checkbox("comp-dark", "Dark mode flag", state.dark),
    ])
    3 => ui_column([
      ui_text("The extension world imports host, ui, editor, and timer interfaces."),
      ui_mono_text("vellum:extension/extension-world@0.1.0"),
      ui_disclosure("Exports implemented", true, [
        ui_mono_text("activate"),
        ui_mono_text("execute-command"),
        ui_mono_text("handle-event"),
        ui_mono_text("handle-ui-event"),
        ui_mono_text("handle-hover"),
      ]),
    ])
    _ => ui_text("Unknown tab")
  }
}

fn build_panel_ui() -> String {
  ui_column([
    ui_heading("MoonBit SDK Overview", 2),
    ui_badge(tab_name(), Some("Info")),
    tab_buttons(),
    ui_separator(),
    tab_content(),
    ui_row([
      ui_button("comp-reset", "Reset", "Secondary"),
      ui_button("comp-status", "Status", "Ghost"),
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
  if command_id == "comprehensive.reset" || command_id == "vellum.sdk-example.comprehensive.comprehensive.reset" {
    reset_overview()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  match event.event_kind {
    "button.clicked" => match event.element_id {
      "comp-tab-overview" => set_tab(0)
      "comp-tab-widgets" => set_tab(1)
      "comp-tab-state" => set_tab(2)
      "comp-tab-host" => set_tab(3)
      "comp-reset" => reset_overview()
      "comp-status" => status("Comprehensive tab: " + tab_name())
      _ => ()
    }
    "checkbox.toggled" => if event.element_id == "comp-dark" {
      state.dark = event.checked.unwrap_or(false)
      state.interactions = state.interactions + 1
      refresh_ui()
    }
    _ => ()
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


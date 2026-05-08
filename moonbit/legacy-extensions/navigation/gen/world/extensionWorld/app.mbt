pub(all) struct NavigationState {
  mut route : String
  mut depth : Int
  mut authenticated : Bool
  mut message : String
}

let state : NavigationState = {
  route: "/",
  depth: 1,
  authenticated: false,
  message: "Home",
}

fn refresh_ui() -> Unit {
  set_panel("navigation", build_panel_ui())
}

fn route_title() -> String {
  match state.route {
    "/" => "Home"
    "/profile" => "Profile"
    "/settings" => "Settings"
    "/detail" => "Detail"
    _ => "Nested"
  }
}

fn navigate(route : String) -> Unit {
  if route == "/settings" && !state.authenticated {
    state.message = "Settings requires login"
    status(state.message)
  } else {
    state.route = route
    state.depth = state.depth + 1
    state.message = "Navigated to " + route_title()
  }
  refresh_ui()
}

fn go_home() -> Unit {
  state.route = "/"
  state.depth = 1
  state.message = "Returned home"
  refresh_ui()
}

fn go_back() -> Unit {
  if state.depth > 1 {
    state.depth = state.depth - 1
    state.route = "/"
    state.message = "Popped one route"
  } else {
    state.message = "Already at root"
  }
  refresh_ui()
}

fn toggle_auth() -> Unit {
  state.authenticated = !state.authenticated
  state.message = if state.authenticated { "Logged in" } else { "Logged out" }
  refresh_ui()
}

fn build_panel_ui() -> String {
  ui_column([
    ui_heading("Navigation", 2),
    ui_badge(route_title(), Some("Info")),
    ui_text("Current route: " + state.route),
    ui_text("Stack depth: " + state.depth.to_string()),
    ui_row([
      ui_button("nav-home", "Home", "Primary"),
      ui_button("nav-back", "Back", "Secondary"),
    ]),
    ui_row([
      ui_button("nav-profile", "Profile", "Ghost"),
      ui_button("nav-detail", "Detail", "Ghost"),
      ui_button("nav-settings", "Settings", "Ghost"),
    ]),
    ui_checkbox("nav-auth", "Authenticated", state.authenticated),
    ui_disclosure("Route guard", true, [
      ui_text("Settings is blocked until Authenticated is checked."),
      ui_small_text(state.message),
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
  match command_id {
    "navigation.home" | "vellum.sdk-example.navigation.navigation.home" => go_home()
    "navigation.back" | "vellum.sdk-example.navigation.navigation.back" => go_back()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  match event.event_kind {
    "button.clicked" => match event.element_id {
      "nav-home" => go_home()
      "nav-back" => go_back()
      "nav-profile" => navigate("/profile")
      "nav-detail" => navigate("/detail")
      "nav-settings" => navigate("/settings")
      _ => ()
    }
    "checkbox.toggled" => if event.element_id == "nav-auth" {
      toggle_auth()
    }
    _ => ()
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


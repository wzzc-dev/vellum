pub(all) struct FormState {
  mut name : String
  mut email : String
  mut age : Int
  mut gender_index : Int
  mut terms : Bool
  mut newsletter : Bool
  mut submitted : Bool
  mut message : String
}

let state : FormState = {
  name: "",
  email: "",
  age: 18,
  gender_index: -1,
  terms: false,
  newsletter: false,
  submitted: false,
  message: "Fill out the form",
}

fn gender_label() -> String {
  match state.gender_index {
    0 => "Designer"
    1 => "Engineer"
    2 => "Writer"
    3 => "Other"
    _ => "Not selected"
  }
}

fn email_is_valid() -> Bool {
  state.email.contains("@") && state.email.contains(".")
}

fn form_is_valid() -> Bool {
  state.name.length() >= 2 && email_is_valid() && state.age >= 18 && state.gender_index >= 0 && state.terms
}

fn validation_message() -> String {
  if form_is_valid() {
    "Ready to submit"
  } else if state.name.length() < 2 {
    "Name must be at least 2 characters"
  } else if !email_is_valid() {
    "Email must include @ and ."
  } else if state.age < 18 {
    "Age must be at least 18"
  } else if state.gender_index < 0 {
    "Choose a role"
  } else {
    "Accept terms to continue"
  }
}

fn refresh_ui() -> Unit {
  set_panel("form", build_panel_ui())
}

fn build_panel_ui() -> String {
  let submit = if form_is_valid() {
    ui_button("form-submit", "Submit", "Primary")
  } else {
    ui_disabled_button("form-submit", "Submit", "Primary")
  }
  ui_column([
    ui_heading("Registration Form", 2),
    ui_separator(),
    ui_text_input("form-name", "Name", state.name),
    ui_text_input("form-email", "Email", state.email),
    ui_row([
      ui_button("form-age-dec", "-", "Ghost"),
      ui_text("Age: " + state.age.to_string()),
      ui_button("form-age-inc", "+", "Ghost"),
    ]),
    ui_select("form-gender", ["Designer", "Engineer", "Writer", "Other"], state.gender_index),
    ui_checkbox("form-terms", "Accept terms", state.terms),
    ui_checkbox("form-newsletter", "Subscribe to newsletter", state.newsletter),
    ui_badge(validation_message(), if form_is_valid() { Some("Info") } else { Some("Warning") }),
    ui_row([
      submit,
      ui_button("form-reset", "Reset", "Ghost"),
    ]),
    ui_disclosure("Preview", true, [
      ui_text("Name: " + (if state.name == "" { "(empty)" } else { state.name })),
      ui_text("Email: " + (if state.email == "" { "(empty)" } else { state.email })),
      ui_text("Role: " + gender_label()),
      ui_text("Newsletter: " + state.newsletter.to_string()),
      ui_small_text(state.message),
    ]),
  ])
}

fn submit_form() -> Unit {
  if form_is_valid() {
    state.submitted = true
    state.message = "Submitted " + state.name + " as " + gender_label()
    status(state.message)
  } else {
    state.message = validation_message()
    status(state.message)
  }
  refresh_ui()
}

fn reset_form() -> Unit {
  state.name = ""
  state.email = ""
  state.age = 18
  state.gender_index = -1
  state.terms = false
  state.newsletter = false
  state.submitted = false
  state.message = "Form reset"
  status(state.message)
  refresh_ui()
}

fn change_age(delta : Int) -> Unit {
  let next = state.age + delta
  if next >= 1 && next <= 100 {
    state.age = next
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
  match command_id {
    "form.submit" | "vellum.sdk-example.form.form.submit" => submit_form()
    "form.reset" | "vellum.sdk-example.form.form.reset" => reset_form()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  match event.event_kind {
    "input.changed" => match event.element_id {
      "form-name" => {
        state.name = event.value.unwrap_or("")
        refresh_ui()
      }
      "form-email" => {
        state.email = event.value.unwrap_or("")
        refresh_ui()
      }
      _ => ()
    }
    "select.changed" => if event.element_id == "form-gender" {
      state.gender_index = match event.index {
        Some(idx) => idx.reinterpret_as_int()
        None => -1
      }
      refresh_ui()
    }
    "checkbox.toggled" => match event.element_id {
      "form-terms" => {
        state.terms = event.checked.unwrap_or(false)
        refresh_ui()
      }
      "form-newsletter" => {
        state.newsletter = event.checked.unwrap_or(false)
        refresh_ui()
      }
      _ => ()
    }
    "button.clicked" => match event.element_id {
      "form-submit" => submit_form()
      "form-reset" => reset_form()
      "form-age-dec" => change_age(-1)
      "form-age-inc" => change_age(1)
      _ => ()
    }
    _ => ()
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}

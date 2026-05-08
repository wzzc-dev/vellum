pub(all) struct TodoState {
  mut input : String
  mut title1 : String
  mut title2 : String
  mut title3 : String
  mut done1 : Bool
  mut done2 : Bool
  mut done3 : Bool
  mut message : String
}

let state : TodoState = {
  input: "",
  title1: "Learn MoonBit",
  title2: "Build a Vellum extension",
  title3: "Validate the component",
  done1: false,
  done2: false,
  done3: false,
  message: "Three editable slots keep this example compact",
}

fn filled_count() -> Int {
  (if state.title1 == "" { 0 } else { 1 }) +
  (if state.title2 == "" { 0 } else { 1 }) +
  (if state.title3 == "" { 0 } else { 1 })
}

fn completed_count() -> Int {
  (if state.title1 != "" && state.done1 { 1 } else { 0 }) +
  (if state.title2 != "" && state.done2 { 1 } else { 0 }) +
  (if state.title3 != "" && state.done3 { 1 } else { 0 })
}

fn add_todo() -> Unit {
  if state.input == "" {
    state.message = "Type a task first"
  } else if state.title1 == "" {
    state.title1 = state.input
    state.done1 = false
    state.input = ""
    state.message = "Added task"
  } else if state.title2 == "" {
    state.title2 = state.input
    state.done2 = false
    state.input = ""
    state.message = "Added task"
  } else if state.title3 == "" {
    state.title3 = state.input
    state.done3 = false
    state.input = ""
    state.message = "Added task"
  } else {
    state.message = "All demo slots are full"
  }
  status(state.message)
  refresh_ui()
}

fn clear_completed() -> Unit {
  if state.done1 {
    state.title1 = ""
    state.done1 = false
  }
  if state.done2 {
    state.title2 = ""
    state.done2 = false
  }
  if state.done3 {
    state.title3 = ""
    state.done3 = false
  }
  state.message = "Completed tasks cleared"
  refresh_ui()
}

fn remove_slot(slot : Int) -> Unit {
  match slot {
    1 => {
      state.title1 = ""
      state.done1 = false
    }
    2 => {
      state.title2 = ""
      state.done2 = false
    }
    3 => {
      state.title3 = ""
      state.done3 = false
    }
    _ => ()
  }
  state.message = "Task removed"
  refresh_ui()
}

fn todo_row(slot : Int, title : String, done : Bool) -> String {
  if title == "" {
    ui_small_text("Empty slot " + slot.to_string())
  } else {
    ui_row([
      ui_checkbox("todo-done-" + slot.to_string(), title, done),
      ui_button("todo-remove-" + slot.to_string(), "Remove", "Ghost"),
    ])
  }
}

fn refresh_ui() -> Unit {
  set_panel("todo-list", build_panel_ui())
}

fn build_panel_ui() -> String {
  let total = filled_count()
  let done = completed_count()
  ui_column([
    ui_heading("Todo List", 2),
    ui_row([
      ui_text_input("todo-input", "New task", state.input),
      ui_button("todo-add", "Add", "Primary"),
    ]),
    ui_separator(),
    todo_row(1, state.title1, state.done1),
    todo_row(2, state.title2, state.done2),
    todo_row(3, state.title3, state.done3),
    ui_row([
      ui_badge((total - done).to_string() + " active", Some("Info")),
      ui_badge(done.to_string() + " completed", Some("Hint")),
    ]),
    ui_button("todo-clear-completed", "Clear Completed", "Secondary"),
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
    "todo-list.add" | "vellum.sdk-example.todo-list.todo-list.add" => add_todo()
    "todo-list.clear-completed" | "vellum.sdk-example.todo-list.todo-list.clear-completed" => clear_completed()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  match event.event_kind {
    "input.changed" => if event.element_id == "todo-input" {
      state.input = event.value.unwrap_or("")
      refresh_ui()
    }
    "checkbox.toggled" => match event.element_id {
      "todo-done-1" => {
        state.done1 = event.checked.unwrap_or(false)
        refresh_ui()
      }
      "todo-done-2" => {
        state.done2 = event.checked.unwrap_or(false)
        refresh_ui()
      }
      "todo-done-3" => {
        state.done3 = event.checked.unwrap_or(false)
        refresh_ui()
      }
      _ => ()
    }
    "button.clicked" => match event.element_id {
      "todo-add" => add_todo()
      "todo-clear-completed" => clear_completed()
      "todo-remove-1" => remove_slot(1)
      "todo-remove-2" => remove_slot(2)
      "todo-remove-3" => remove_slot(3)
      _ => ()
    }
    _ => ()
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}


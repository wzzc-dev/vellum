pub(all) struct DemoState {
  mut clicks : Int
  mut note : String
  mut selected_tab : UInt
}

let state : DemoState = {
  clicks: 0,
  note: "",
  selected_tab: 0U,
}

fn build_tree() -> @types.ViewTree {
  let sidebar = column("demo-sidebar", [
    1U, 2U, 3U, 4U, 5U, 6U,
  ])
  let title = heading("demo-title", "Vellum Framework Demo")
  let intro = muted_text(
    "demo-intro",
    "This shell is described in MoonBit and rendered natively by Rust/GPUI.",
  )
  let count = text_node("demo-count", "Button clicks: " + state.clicks.to_string())
  let toolbar = row("demo-toolbar", [
    7U, 8U,
  ])
  let input_row = column("demo-input-row", [
    9U, 10U,
  ])
  let tabs_node = tabs("demo-tabs", state.selected_tab, [
    @types.TabItem::{ id: "editor", label: "Editor", child: 11U },
    @types.TabItem::{ id: "about", label: "About", child: 12U },
  ])
  let primary_button = button("demo-increment", "Increment", @types.ButtonVariant::PRIMARY)
  let reset_button = button("demo-reset", "Reset", @types.ButtonVariant::GHOST)
  let note_label = muted_text("demo-note-label", "MoonBit controlled input")
  let note_input = input("demo-note-input", "Type in the MoonBit shell", state.note)
  let editor = native_markdown_editor("main-markdown-editor")
  let about = scroll_view("demo-about", [
    13U, 14U,
  ])
  let about_title = heading("demo-about-title", "Typed WIT UI Tree")
  let about_text = muted_text(
    "demo-about-text",
    "Button, input, tabs and this shell are MoonBit nodes. The editor below remains the existing Rust MarkdownEditor native widget.",
  )
  let content = column("demo-content", [
    6U,
  ])
  let root = split_view("demo-root", 0.24, [
    0U, 15U,
  ])

  @types.ViewTree::{
    root: 16U,
    nodes: [
      sidebar,
      title,
      intro,
      count,
      toolbar,
      input_row,
      tabs_node,
      primary_button,
      reset_button,
      note_label,
      note_input,
      editor,
      about,
      about_title,
      about_text,
      content,
      root,
    ],
  }
}

fn handle_ui(event : @types.UiEvent) -> Unit {
  if event.target_id == "demo-increment" && event.event_kind == "button.clicked" {
    state.clicks = state.clicks + 1
    ignore(@host.show_status_message("MoonBit click count: " + state.clicks.to_string()))
  } else if event.target_id == "demo-reset" && event.event_kind == "button.clicked" {
    state.clicks = 0
    state.note = ""
    ignore(@host.show_status_message("MoonBit demo state reset"))
  } else if event.target_id == "demo-note-input" && event.event_kind == "input.changed" {
    match event.value {
      Some(value) => state.note = value
      None => ()
    }
  } else if event.target_id == "demo-tabs" && event.event_kind == "tabs.changed" {
    match event.index {
      Some(index) => state.selected_tab = index
      None => ()
    }
  }
}

pub fn init_(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @host.log(@types.LogLevel::INFO, "initialized " + ctx.app_id)
  Ok(build_tree())
}

pub fn update(event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  match event {
    Ui(ui_event) => handle_ui(ui_event)
    Command(command_event) => {
      if command_event.command_id == "demo.reset" {
        state.clicks = 0
        state.note = ""
      }
    }
    _ => ()
  }
  Ok(build_tree())
}

pub fn shutdown() -> Result[Unit, @types.AppError] {
  Ok(())
}

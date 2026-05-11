pub fn panel_column() -> @types.ViewKind {
  @app.column(Some(8.0), Some(@app.all_insets(12.0)))
}

pub fn panel_title(content : String) -> @types.ViewKind {
  @app.styled_text(content, @app.heading_style())
}

pub fn panel_hint(content : String) -> @types.ViewKind {
  @app.muted_text(content)
}

pub fn show_status(message : String) -> Result[Unit, @types.AppError] {
  @host.show_status_message(message)
}

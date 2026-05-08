fn build_tree() -> @types.ViewTree {
  let editor = native_markdown_editor("main-markdown-editor")

  @types.ViewTree::{
    root: 0U,
    nodes: [
      editor,
    ],
  }
}

pub fn init_(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @host.log(@types.LogLevel::INFO, "initialized " + ctx.app_id)
  Ok(build_tree())
}

pub fn update(_event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  Ok(build_tree())
}

pub fn shutdown() -> Result[Unit, @types.AppError] {
  Ok(())
}

fn build_tree() -> @types.ViewTree {
  let nodes : Array[@types.ViewNode] = []

  let title = @app.add_node(nodes, "counter-title", @plugin.panel_title("Counter Plugin"), [])
  let hint = @app.add_node(
    nodes,
    "counter-hint",
    @plugin.panel_hint("This panel is rendered through vellum:app/app-world."),
    [],
  )
  let button = @app.add_node(nodes, "counter-ping", @app.button("Ping Host"), [])
  let root = @app.add_node(nodes, "counter-root", @plugin.panel_column(), [title, hint, button])

  @app.tree(root, nodes)
}

pub fn app_init(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @host.log(@types.LogLevel::INFO, "initialized plugin " + ctx.app_id)
  Ok(build_tree())
}

pub fn app_update(event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  match event {
    Ui(ui) => {
      if @app.button_clicked(ui, "counter-ping") {
        ignore(@plugin.show_status("Counter plugin received a click"))
      }
    }
    _ => ()
  }
  Ok(build_tree())
}

pub fn app_shutdown() -> Result[Unit, @types.AppError] {
  Ok(())
}

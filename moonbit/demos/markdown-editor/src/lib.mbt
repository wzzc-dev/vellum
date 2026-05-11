fn plugin_state_text(state : @types.PluginState) -> String {
  match state {
    ENABLED => "Enabled"
    DISABLED => "Disabled"
    FAILED => "Failed"
  }
}

fn add_plugin_actions(
  nodes : Array[@types.ViewNode],
  plugin_children : Array[UInt],
  plugin : @types.PluginInfo
) -> Unit {
  let action_children : Array[UInt] = []
  match plugin.state {
    ENABLED => {
      action_children.push(@sdk.add_node(
        nodes,
        "plugin-disable-" + plugin.id,
        @sdk.button_with_variant("Disable", @types.ButtonVariant::SECONDARY, false),
        [],
      ))
      action_children.push(@sdk.add_node(
        nodes,
        "plugin-reload-" + plugin.id,
        @sdk.button_with_variant("Reload", @types.ButtonVariant::GHOST, false),
        [],
      ))
    }
    DISABLED => {
      action_children.push(@sdk.add_node(
        nodes,
        "plugin-enable-" + plugin.id,
        @sdk.button("Enable"),
        [],
      ))
    }
    FAILED => {
      action_children.push(@sdk.add_node(
        nodes,
        "plugin-reload-" + plugin.id,
        @sdk.button_with_variant("Reload", @types.ButtonVariant::PRIMARY, false),
        [],
      ))
    }
  }
  if action_children.length() > 0 {
    plugin_children.push(@sdk.add_node(
      nodes,
      "plugin-actions-" + plugin.id,
      @sdk.row(Some(8.0), None),
      action_children,
    ))
  }
}

fn add_plugin_entry(
  nodes : Array[@types.ViewNode],
  panel_children : Array[UInt],
  plugin : @types.PluginInfo
) -> Unit {
  let plugin_children : Array[UInt] = []
  plugin_children.push(@sdk.add_node(
    nodes,
    "plugin-name-" + plugin.id,
    @sdk.styled_text(@sdk.plugin_summary(plugin), @sdk.heading_style()),
    [],
  ))
  plugin_children.push(@sdk.add_node(
    nodes,
    "plugin-state-" + plugin.id,
    @sdk.muted_text("State: " + plugin_state_text(plugin.state)),
    [],
  ))

  if plugin.description != "" {
    plugin_children.push(@sdk.add_node(
      nodes,
      "plugin-description-" + plugin.id,
      @sdk.muted_text(plugin.description),
      [],
    ))
  }

  match plugin.error {
    Some(err) => plugin_children.push(@sdk.add_node(
      nodes,
      "plugin-error-" + plugin.id,
      @sdk.styled_text("Error: " + err, @sdk.text_style(None, Some("destructive"), None, None, None)),
      [],
    ))
    None => ()
  }

  add_plugin_actions(nodes, plugin_children, plugin)

  panel_children.push(@sdk.add_node(
    nodes,
    "plugin-entry-" + plugin.id,
    @sdk.column(Some(6.0), Some(@sdk.all_insets(8.0))),
    plugin_children,
  ))
}

fn build_tree() -> @types.ViewTree {
  let nodes : Array[@types.ViewNode] = []

  let editor = @sdk.add_node(
    nodes,
    "main-markdown-editor",
    @sdk.markdown_editor(),
    [],
  )

  let panel_children : Array[UInt] = []
  panel_children.push(@sdk.add_node(
    nodes,
    "plugins-title",
    @sdk.styled_text("Plugins", @sdk.heading_style()),
    [],
  ))

  match @host.get_editor_snapshot() {
    Ok(snapshot) => {
      panel_children.push(@sdk.add_node(
        nodes,
        "document-summary",
        @sdk.muted_text(
          "Editing " + snapshot.display_name + " · " + snapshot.view_mode,
        ),
        [],
      ))
    }
    Err(_) => ()
  }

  match @host.plugin_list() {
    Ok(plugins) => {
      if plugins.length() == 0 {
        panel_children.push(@sdk.add_node(
          nodes,
          "plugins-empty",
          @sdk.muted_text("No plugins loaded"),
          [],
        ))
      } else {
        for index = 0; index < plugins.length(); index = index + 1 {
          add_plugin_entry(nodes, panel_children, plugins[index])
        }
        match @sdk.first_plugin_panel_id(plugins) {
          Some(panel_id) => {
            panel_children.push(@sdk.add_node(
              nodes,
              "active-plugin-panel-label",
              @sdk.styled_text("Panel", @sdk.heading_style()),
              [],
            ))
            panel_children.push(@sdk.add_node(
              nodes,
              "active-plugin-panel",
              @sdk.plugin_panel(panel_id),
              [],
            ))
          }
          None => ()
        }
      }
    }
    Err(err) => {
      panel_children.push(@sdk.add_node(
        nodes,
        "plugins-error",
        @sdk.muted_text(err.message),
        [],
      ))
    }
  }

  let plugin_sidebar = @sdk.add_node(
    nodes,
    "plugin-sidebar",
    @sdk.scroll_view(@types.SplitAxis::VERTICAL),
    panel_children,
  )

  let root = @sdk.add_node(
    nodes,
    "markdown-demo-root",
    @sdk.split_view(@types.SplitAxis::HORIZONTAL, 0.74),
    [editor, plugin_sidebar],
  )

  @sdk.tree(root, nodes)
}

fn handle_plugin_action(target_id : String, plugins : Array[@types.PluginInfo]) -> Unit {
  for index = 0; index < plugins.length(); index = index + 1 {
    let plugin = plugins[index]
    if target_id == "plugin-enable-" + plugin.id {
      ignore(@host.plugin_enable(plugin.id))
      ignore(@host.show_status_message("Enabled " + plugin.name))
    }
    if target_id == "plugin-disable-" + plugin.id {
      ignore(@host.plugin_disable(plugin.id))
      ignore(@host.show_status_message("Disabled " + plugin.name))
    }
    if target_id == "plugin-reload-" + plugin.id {
      ignore(@host.plugin_reload(plugin.id))
      ignore(@host.show_status_message("Reloaded " + plugin.name))
    }
  }
}

pub fn app_init(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @host.log(@types.LogLevel::INFO, "initialized " + ctx.app_id)
  Ok(build_tree())
}

pub fn app_update(event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  match event {
    Ui(ui) => {
      match @host.plugin_list() {
        Ok(plugins) => handle_plugin_action(ui.target_id, plugins)
        Err(_) => ()
      }
    }
    _ => ()
  }
  Ok(build_tree())
}

pub fn app_shutdown() -> Result[Unit, @types.AppError] {
  Ok(())
}

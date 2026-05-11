use gpui::{
    AnyElement, AppContext, Context, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme,
    input::{InputEvent, InputState},
    scroll::ScrollableElement,
};
use vellum_renderer_gpui::{FrameworkInput, FrameworkRenderHost, FrameworkRenderScope};
use vellum_runtime::{
    AppEvent, EditorSnapshot as RuntimeEditorSnapshot, LoadedAppComponent, NativeViewProps,
    PluginInfo, Property, UiEvent, VellumAppRuntime, ViewTree,
};

use super::VellumApp;

impl VellumApp {
    pub(super) fn load_framework_app_from_env(
        editor_snapshot: RuntimeEditorSnapshot,
        plugin_infos: Vec<PluginInfo>,
    ) -> (Option<LoadedAppComponent>, Option<ViewTree>) {
        let Some(path) = std::env::var_os("VELLUM_APP") else {
            return (None, None);
        };
        let runtime = match VellumAppRuntime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                eprintln!("failed to initialize Vellum app runtime: {err}");
                return (None, None);
            }
        };
        let mut app = match runtime.load_manifest(std::path::PathBuf::from(path)) {
            Ok(app) => app,
            Err(err) => {
                eprintln!("failed to load Vellum app component: {err}");
                return (None, None);
            }
        };
        app.set_editor_snapshot(editor_snapshot);
        app.set_plugin_infos(plugin_infos);
        let view = match app.init() {
            Ok(view) => Some(view.clone()),
            Err(err) => {
                eprintln!("failed to initialize Vellum app component: {err}");
                None
            }
        };
        (Some(app), view)
    }

    pub(super) fn render_framework_tree(
        &mut self,
        tree: &ViewTree,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        vellum_renderer_gpui::render_framework_tree(self, tree, window, cx)
    }

    pub(super) fn render_plugin_framework_tree(
        &mut self,
        panel_id: &str,
        tree: &ViewTree,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        vellum_renderer_gpui::render_framework_tree_in_scope(
            self,
            tree,
            FrameworkRenderScope::PluginPanel(panel_id.to_string()),
            window,
            cx,
        )
    }
}

impl FrameworkRenderHost for VellumApp {
    fn framework_input_for(
        &mut self,
        scope: &FrameworkRenderScope,
        id: &str,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let input_key = scoped_framework_id(scope, id);
        if let Some(input) = self.framework_inputs.get_mut(&input_key) {
            if input.last_value != value && input.state.read(cx).value().as_ref() != value {
                input.state.update(cx, |state, cx| {
                    state.set_value(value.to_string(), window, cx)
                });
                input.last_value = value.to_string();
            }
            return input.state.clone();
        }

        let state = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder(placeholder.to_string());
            state.set_value(value.to_string(), window, cx);
            state
        });
        self.framework_inputs.insert(
            input_key.clone(),
            FrameworkInput::new(state.clone(), value.to_string()),
        );
        let target_id = id.to_string();
        let scope = scope.clone();
        self.find_input_subscriptions.push(cx.subscribe(
            &state,
            move |this: &mut Self, input: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = input.read(cx).value().to_string();
                    if let Some(tracked) = this
                        .framework_inputs
                        .get_mut(&scoped_framework_id(&scope, &target_id))
                    {
                        tracked.last_value = value.clone();
                    }
                    this.dispatch_framework_event(
                        scope.clone(),
                        AppEvent::Ui(UiEvent {
                            target_id: target_id.clone(),
                            event_kind: "input.changed".into(),
                            value: Some(value),
                            index: None,
                            checked: None,
                        }),
                        None,
                        cx,
                    );
                }
            },
        ));
        state
    }

    fn dispatch_framework_event(
        &mut self,
        scope: FrameworkRenderScope,
        event: AppEvent,
        mut window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        match scope {
            FrameworkRenderScope::App => {
                let Some(app) = self.framework_app.as_mut() else {
                    return;
                };
                match app.update(event) {
                    Ok(view) => {
                        self.framework_view = Some(view.clone());
                        if let Some(status) = app.take_status_message() {
                            self.set_status(status);
                        }
                    }
                    Err(err) => {
                        self.set_status(format!("Framework app event failed: {err}"));
                    }
                }
            }
            FrameworkRenderScope::PluginPanel(panel_id) => {
                if let AppEvent::Ui(event) = event {
                    if let Err(err) = self
                        .plugin_store
                        .dispatch_ui_event_to_panel(&panel_id, event)
                    {
                        self.set_status(format!("Plugin event failed: {err}"));
                    }
                }
            }
        }
        self.drain_framework_outputs(window.as_deref_mut(), cx);
        cx.notify();
    }

    fn render_native_view(
        &mut self,
        node_id: &str,
        props: &NativeViewProps,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if props.kind == "markdown-editor" {
            div()
                .id(ElementId::Name(node_id.to_string().into()))
                .size_full()
                .min_w(px(0.))
                .min_h(px(0.))
                .child(self.active_editor_entity().clone())
                .into_any_element()
        } else if props.kind == "plugin-panel" {
            let panel_id = property_value(&props.props, "panel-id")
                .or_else(|| property_value(&props.props, "panel"))
                .unwrap_or_default();
            if let Some(tree) = self.plugin_store.panel_tree(&panel_id) {
                self.render_plugin_framework_tree(&panel_id, &tree, _window, cx)
            } else {
                div()
                    .id(ElementId::Name(node_id.to_string().into()))
                    .size_full()
                    .min_h(px(0.))
                    .overflow_y_scrollbar()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Plugin panel unavailable")
                    .into_any_element()
            }
        } else {
            div()
                .id(ElementId::Name(node_id.to_string().into()))
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("Unknown native view: {}", props.kind))
                .into_any_element()
        }
    }
}

fn scoped_framework_id(scope: &FrameworkRenderScope, id: &str) -> String {
    match scope {
        FrameworkRenderScope::App => format!("app:{id}"),
        FrameworkRenderScope::PluginPanel(panel_id) => format!("plugin:{panel_id}:{id}"),
    }
}

fn property_value(props: &[Property], name: &str) -> Option<String> {
    props
        .iter()
        .find(|prop| prop.name == name)
        .map(|prop| prop.value.clone())
}

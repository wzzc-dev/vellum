use gpui::{
    AnyElement, AppContext, Context, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Styled, Window, div, px,
};
use gpui_component::{ActiveTheme, input::{InputEvent, InputState}};
use vellum_renderer_gpui::{FrameworkInput, FrameworkRenderHost};
use vellum_runtime::{
    AppEvent, LoadedAppComponent, NativeViewProps, UiEvent, VellumAppRuntime, ViewTree,
};

use super::VellumApp;

impl VellumApp {
    pub(super) fn load_framework_app_from_env() -> (Option<LoadedAppComponent>, Option<ViewTree>) {
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
}

impl FrameworkRenderHost for VellumApp {
    fn framework_input_for(
        &mut self,
        id: &str,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(input) = self.framework_inputs.get_mut(id) {
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
            id.to_string(),
            FrameworkInput::new(state.clone(), value.to_string()),
        );
        let target_id = id.to_string();
        self.find_input_subscriptions.push(cx.subscribe(
            &state,
            move |this: &mut Self, input: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = input.read(cx).value().to_string();
                    if let Some(tracked) = this.framework_inputs.get_mut(&target_id) {
                        tracked.last_value = value.clone();
                    }
                    this.dispatch_framework_event(
                        AppEvent::Ui(UiEvent {
                            target_id: target_id.clone(),
                            event_kind: "input.changed".into(),
                            value: Some(value),
                            index: None,
                            checked: None,
                        }),
                        cx,
                    );
                }
            },
        ));
        state
    }

    fn dispatch_framework_event(&mut self, event: AppEvent, cx: &mut Context<Self>) {
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

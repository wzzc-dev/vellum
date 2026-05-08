use gpui::{
    AnyElement, AppContext, Context, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Selectable,
    button::{Button, ButtonGroup, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::ScrollableElement,
};
use vellum_extension::app_ui::{
    AppEvent, ButtonVariant, SplitAxis, UiEvent, ViewKind, ViewNode, ViewTree,
};

use super::VellumApp;

pub(super) struct FrameworkInput {
    pub state: Entity<InputState>,
    pub last_value: String,
}

impl FrameworkInput {
    pub fn new(state: Entity<InputState>, last_value: String) -> Self {
        Self { state, last_value }
    }
}

impl VellumApp {
    pub(super) fn load_framework_app_from_env() -> (
        Option<vellum_extension::LoadedAppComponent>,
        Option<ViewTree>,
    ) {
        let Some(path) = std::env::var_os("VELLUM_APP") else {
            return (None, None);
        };
        let runtime = match vellum_extension::VellumAppRuntime::new() {
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

    pub(super) fn framework_input_for(
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

    pub(super) fn dispatch_framework_event(&mut self, event: AppEvent, cx: &mut Context<Self>) {
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

    pub(super) fn render_framework_tree(
        &mut self,
        tree: &ViewTree,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match tree.root_node() {
            Some(root) => self.render_framework_node(tree, root, window, cx),
            None => div().into_any_element(),
        }
    }

    fn render_framework_node(
        &mut self,
        tree: &ViewTree,
        node: &ViewNode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &node.kind {
            ViewKind::Empty => div().into_any_element(),
            ViewKind::Column(props) => {
                let mut el = div()
                    .id(ElementId::Name(node.id.clone().into()))
                    .flex()
                    .flex_col()
                    .w_full();
                if let Some(gap) = props.gap {
                    el = el.gap(px(gap));
                }
                if let Some(padding) = props.padding {
                    el = el
                        .pt(px(padding.top))
                        .pr(px(padding.right))
                        .pb(px(padding.bottom))
                        .pl(px(padding.left));
                }
                for child in tree.child_nodes(node) {
                    el = el.child(self.render_framework_node(tree, child, window, cx));
                }
                el.into_any_element()
            }
            ViewKind::Row(props) => {
                let mut el = div()
                    .id(ElementId::Name(node.id.clone().into()))
                    .flex()
                    .flex_row()
                    .w_full();
                if let Some(gap) = props.gap {
                    el = el.gap(px(gap));
                }
                if let Some(padding) = props.padding {
                    el = el
                        .pt(px(padding.top))
                        .pr(px(padding.right))
                        .pb(px(padding.bottom))
                        .pl(px(padding.left));
                }
                for child in tree.child_nodes(node) {
                    el = el.child(self.render_framework_node(tree, child, window, cx));
                }
                el.into_any_element()
            }
            ViewKind::Text(props) => {
                let mut el = div()
                    .id(ElementId::Name(node.id.clone().into()))
                    .text_sm()
                    .child(props.content.clone());
                if props.style.bold.unwrap_or(false) {
                    el = el.font_weight(gpui::FontWeight::BOLD);
                }
                if props.style.italic.unwrap_or(false) {
                    el = el.italic();
                }
                if props.style.monospace.unwrap_or(false) {
                    el = el.font_family("monospace");
                }
                if let Some(size) = props.style.size {
                    el = el.text_size(px(size));
                }
                if props.style.color.as_deref() == Some("muted-foreground") {
                    el = el.text_color(cx.theme().muted_foreground);
                }
                el.into_any_element()
            }
            ViewKind::Button(props) => {
                let mut button =
                    Button::new(ElementId::Name(format!("framework-btn-{}", node.id).into()))
                        .label(props.label.clone());
                button = match props.style {
                    ButtonVariant::Primary => button.primary(),
                    ButtonVariant::Secondary => button.ghost(),
                    ButtonVariant::Ghost => button.ghost(),
                    ButtonVariant::Danger => button.danger(),
                };
                let target_id = node.id.clone();
                let view = cx.entity().downgrade();
                button
                    .disabled(props.disabled)
                    .on_click(move |_, _window, cx| {
                        if let Some(entity) = view.upgrade() {
                            let _ = entity.update(cx, |this, cx| {
                                this.dispatch_framework_event(
                                    AppEvent::Ui(UiEvent {
                                        target_id: target_id.clone(),
                                        event_kind: "button.clicked".into(),
                                        value: None,
                                        index: None,
                                        checked: None,
                                    }),
                                    cx,
                                );
                            });
                        }
                    })
                    .into_any_element()
            }
            ViewKind::Input(props) => {
                let input = self.framework_input_for(
                    &node.id,
                    &props.value,
                    &props.placeholder,
                    window,
                    cx,
                );
                Input::new(&input).w_full().into_any_element()
            }
            ViewKind::Tabs(props) => {
                let selected = props.selected as usize;
                let mut tabs = ButtonGroup::new(ElementId::Name(
                    format!("framework-tabs-{}", node.id).into(),
                ))
                .compact()
                .ghost();
                for (index, tab) in props.tabs.iter().enumerate() {
                    tabs = tabs.child(
                        Button::new(ElementId::Name(
                            format!("framework-tab-{}-{}", node.id, tab.id).into(),
                        ))
                        .label(tab.label.clone())
                        .selected(index == selected),
                    );
                }
                let target_id = node.id.clone();
                let view = cx.entity().downgrade();
                let tabs = tabs.on_click(move |selected: &Vec<usize>, _, cx| {
                    let Some(index) = selected.first().copied() else {
                        return;
                    };
                    if let Some(entity) = view.upgrade() {
                        let _ = entity.update(cx, |this, cx| {
                            this.dispatch_framework_event(
                                AppEvent::Ui(UiEvent {
                                    target_id: target_id.clone(),
                                    event_kind: "tabs.changed".into(),
                                    value: None,
                                    index: Some(index as u32),
                                    checked: None,
                                }),
                                cx,
                            );
                        });
                    }
                });
                let selected_child = props
                    .tabs
                    .get(selected)
                    .and_then(|tab| tree.nodes.get(tab.child as usize));
                let mut el = div()
                    .id(ElementId::Name(node.id.clone().into()))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full();
                el = el.child(tabs);
                if let Some(child) = selected_child {
                    el = el.child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .child(self.render_framework_node(tree, child, window, cx)),
                    );
                }
                for child in tree.child_nodes(node) {
                    el = el.child(self.render_framework_node(tree, child, window, cx));
                }
                el.into_any_element()
            }
            ViewKind::SplitView(props) => {
                let mut layout = match props.axis {
                    SplitAxis::Horizontal => h_resizable(ElementId::Name(node.id.clone().into())),
                    SplitAxis::Vertical => v_resizable(ElementId::Name(node.id.clone().into())),
                };
                let first_size = px((props.ratio.clamp(0.05, 0.95) * 1000.0).round());
                for (index, child) in tree.child_nodes(node).enumerate() {
                    let panel = if index == 0 {
                        resizable_panel().size(first_size)
                    } else {
                        resizable_panel()
                    };
                    layout = layout
                        .child(panel.child(self.render_framework_node(tree, child, window, cx)));
                }
                div().size_full().child(layout).into_any_element()
            }
            ViewKind::ScrollView(props) => {
                let mut el = div()
                    .id(ElementId::Name(node.id.clone().into()))
                    .size_full()
                    .min_h(px(0.));
                for child in tree.child_nodes(node) {
                    el = el.child(self.render_framework_node(tree, child, window, cx));
                }
                match props.axis {
                    SplitAxis::Horizontal => el.overflow_x_scrollbar().into_any_element(),
                    SplitAxis::Vertical => el.overflow_y_scrollbar().into_any_element(),
                }
            }
            ViewKind::NativeView(props) => {
                if props.kind == "markdown-editor" {
                    div()
                        .id(ElementId::Name(node.id.clone().into()))
                        .size_full()
                        .min_w(px(0.))
                        .min_h(px(0.))
                        .child(self.active_editor_entity().clone())
                        .into_any_element()
                } else {
                    div()
                        .id(ElementId::Name(node.id.clone().into()))
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
    }
}

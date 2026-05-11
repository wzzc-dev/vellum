use gpui::{
    AnyElement, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Selectable,
    button::{Button, ButtonGroup, ButtonVariants as _},
    input::{Input, InputState},
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::ScrollableElement,
};
use vellum_runtime::{
    AppEvent, ButtonVariant, NativeViewProps, SplitAxis, UiEvent, ViewKind, ViewNode, ViewTree,
};

pub struct FrameworkInput {
    pub state: Entity<InputState>,
    pub last_value: String,
}

impl FrameworkInput {
    pub fn new(state: Entity<InputState>, last_value: String) -> Self {
        Self { state, last_value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameworkRenderScope {
    App,
    PluginPanel(String),
}

pub trait FrameworkRenderHost: Sized {
    fn framework_input_for(
        &mut self,
        scope: &FrameworkRenderScope,
        id: &str,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState>;

    fn dispatch_framework_event(
        &mut self,
        scope: FrameworkRenderScope,
        event: AppEvent,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    );

    fn render_native_view(
        &mut self,
        node_id: &str,
        props: &NativeViewProps,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement;
}

pub fn render_framework_tree<H: FrameworkRenderHost + 'static>(
    host: &mut H,
    tree: &ViewTree,
    window: &mut Window,
    cx: &mut Context<H>,
) -> AnyElement {
    render_framework_tree_in_scope(host, tree, FrameworkRenderScope::App, window, cx)
}

pub fn render_framework_tree_in_scope<H: FrameworkRenderHost + 'static>(
    host: &mut H,
    tree: &ViewTree,
    scope: FrameworkRenderScope,
    window: &mut Window,
    cx: &mut Context<H>,
) -> AnyElement {
    match tree.root_node() {
        Some(root) => render_framework_node(host, tree, root, &scope, window, cx),
        None => div().into_any_element(),
    }
}

fn render_framework_node<H: FrameworkRenderHost + 'static>(
    host: &mut H,
    tree: &ViewTree,
    node: &ViewNode,
    scope: &FrameworkRenderScope,
    window: &mut Window,
    cx: &mut Context<H>,
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
                el = el.child(render_framework_node(host, tree, child, scope, window, cx));
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
                el = el.child(render_framework_node(host, tree, child, scope, window, cx));
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
            let scope = scope.clone();
            let view = cx.entity().downgrade();
            button
                .disabled(props.disabled)
                .on_click(move |_, _window, cx| {
                    if let Some(entity) = view.upgrade() {
                        let _ = entity.update(cx, |host, cx| {
                            host.dispatch_framework_event(
                                scope.clone(),
                                AppEvent::Ui(UiEvent {
                                    target_id: target_id.clone(),
                                    event_kind: "button.clicked".into(),
                                    value: None,
                                    index: None,
                                    checked: None,
                                }),
                                Some(_window),
                                cx,
                            );
                        });
                    }
                })
                .into_any_element()
        }
        ViewKind::Input(props) => {
            let input = host.framework_input_for(
                scope,
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
            let scope = scope.clone();
            let event_scope = scope.clone();
            let view = cx.entity().downgrade();
            let tabs = tabs.on_click(move |selected: &Vec<usize>, window, cx| {
                let Some(index) = selected.first().copied() else {
                    return;
                };
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |host, cx| {
                        host.dispatch_framework_event(
                            event_scope.clone(),
                            AppEvent::Ui(UiEvent {
                                target_id: target_id.clone(),
                                event_kind: "tabs.changed".into(),
                                value: None,
                                index: Some(index as u32),
                                checked: None,
                            }),
                            Some(window),
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
                        .child(render_framework_node(host, tree, child, &scope, window, cx)),
                );
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
                layout = layout.child(
                    panel.child(render_framework_node(host, tree, child, scope, window, cx)),
                );
            }
            div().size_full().child(layout).into_any_element()
        }
        ViewKind::ScrollView(props) => {
            let mut el = div()
                .id(ElementId::Name(node.id.clone().into()))
                .size_full()
                .min_h(px(0.));
            for child in tree.child_nodes(node) {
                el = el.child(render_framework_node(host, tree, child, scope, window, cx));
            }
            match props.axis {
                SplitAxis::Horizontal => el.overflow_x_scrollbar().into_any_element(),
                SplitAxis::Vertical => el.overflow_y_scrollbar().into_any_element(),
            }
        }
        ViewKind::NativeView(props) => host.render_native_view(&node.id, props, window, cx),
    }
}

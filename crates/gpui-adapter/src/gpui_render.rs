//! GPUI 渲染集成 - 将 Widget 树转换为 GPUI 元素

use crate::types::Color;
use crate::widget::{WidgetId, WidgetManager};
use gpui::{px, prelude::*, Div, Window, Hsla};
use std::sync::Arc;
use parking_lot::RwLock;

/// 渲染 Widget 树为 GPUI 元素
pub fn render_widget_tree(
    widget_manager: &Arc<RwLock<WidgetManager>>,
    root_id: &WidgetId,
    window: &mut Window,
    cx: &mut gpui::Context<()>,
) -> Div {
    let manager = widget_manager.read();
    render_widget_recursive(&manager, root_id, window, cx)
}

/// 递归渲染单个 Widget
fn render_widget_recursive(
    manager: &WidgetManager,
    widget_id: &WidgetId,
    window: &mut Window,
    cx: &mut gpui::Context<()>,
) -> Div {
    let widget = manager.get_widget(widget_id).expect("Widget not found");
    
    // 基础容器
    let mut div = gpui::div()
        .w(px(widget.size.width))
        .h(px(widget.size.height));
    
    // 应用背景
    div = div.bg(convert_color(&widget.background));
    
    // 渲染子组件
    for child_id in &widget.children {
        div = div.child(render_widget_recursive(manager, child_id, window, cx));
    }
    
    div
}

/// 颜色转换：Adapter Color → GPUI Hsla
fn convert_color(_color: &Color) -> Hsla {
    Hsla::white()
}

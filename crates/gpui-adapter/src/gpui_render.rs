//! GPUI 渲染集成 - 将 Widget 树转换为 GPUI 元素

use crate::types::{Color, EdgeInsets, Size, Alignment};
use crate::widget::{Widget, WidgetId, WidgetManager};
use gpui::{px, prelude::*, Div, Window, Context, Hsla};
use std::sync::Arc;
use parking_lot::RwLock;

/// 渲染 Widget 树为 GPUI 元素
pub fn render_widget_tree(
    widget_manager: &Arc<RwLock<WidgetManager>>,
    root_id: &WidgetId,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    let manager = widget_manager.read();
    render_widget_recursive(&manager, root_id, window, cx)
}

/// 递归渲染单个 Widget
fn render_widget_recursive(
    manager: &WidgetManager,
    widget_id: &WidgetId,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    let widget = manager.get_widget(widget_id).expect("Widget not found");
    
    // 基础容器
    let mut div = div()
        .w(px(widget.size.width))
        .h(px(widget.size.height));
    
    // 应用内边距
    div = div
        .pl(px(widget.padding.left))
        .pr(px(widget.padding.right))
        .pt(px(widget.padding.top))
        .pb(px(widget.padding.bottom));
    
    // 应用外边距
    div = div
        .ml(px(widget.margin.left))
        .mr(px(widget.margin.right))
        .mt(px(widget.margin.top))
        .mb(px(widget.margin.bottom));
    
    // 应用背景
    div = div.bg(convert_color(&widget.background));
    
    // 应用前景色
    div = div.text_color(convert_color(&widget.foreground));
    
    // 根据 Widget 类型进行特殊渲染
    match widget.widget_type.as_str() {
        "column" => {
            div = render_column(div, &widget, manager, window, cx);
        }
        "row" => {
            div = render_row(div, &widget, manager, window, cx);
        }
        "text" => {
            div = render_text(div, &widget);
        }
        "button" => {
            div = render_button(div, &widget, widget_id, manager, window, cx);
        }
        _ => {
            div = render_container(div, &widget, manager, window, cx);
        }
    }
    
    div
}

/// 渲染 Column 布局
fn render_column(
    mut div: Div,
    widget: &Widget,
    manager: &WidgetManager,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    div = div.flex_col();
    apply_alignment(&mut div, widget.alignment);
    apply_gap(&mut div, widget.gap);
    
    for child_id in &widget.children {
        div = div.child(render_widget_recursive(manager, child_id, window, cx));
    }
    
    div
}

/// 渲染 Row 布局
fn render_row(
    mut div: Div,
    widget: &Widget,
    manager: &WidgetManager,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    div = div.flex_row();
    apply_alignment(&mut div, widget.alignment);
    apply_gap(&mut div, widget.gap);
    
    for child_id in &widget.children {
        div = div.child(render_widget_recursive(manager, child_id, window, cx));
    }
    
    div
}

/// 渲染 Text 组件
fn render_text(mut div: Div, widget: &Widget) -> Div {
    if let Some(content) = widget.properties.get("content") {
        div = div.child(
            gpui::span()
                .text_color(convert_color(&widget.foreground))
                .size(px(widget.font_size.unwrap_or(14.0)))
                .child(content.clone())
        );
    }
    div
}

/// 渲染 Button 组件
fn render_button(
    mut div: Div,
    widget: &Widget,
    widget_id: &WidgetId,
    manager: &WidgetManager,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    // 按钮样式
    div = div
        .p(px(8.0))
        .border_1()
        .border_color(gpui::gray_500())
        .rounded(px(4.0))
        .cursor_pointer()
        .text_color(convert_color(&widget.foreground))
        .bg(convert_color(&widget.background))
        .hover(|style| {
            style.bg(gpui::blue_500().opacity(0.1))
        });
    
    // 添加按钮内容（通常是 Text）
    if let Some(content) = widget.properties.get("label") {
        div = div.child(
            gpui::span()
                .text_color(convert_color(&widget.foreground))
                .child(content.clone())
        );
    } else {
        // 渲染子组件
        for child_id in &widget.children {
            div = div.child(render_widget_recursive(manager, child_id, window, cx));
        }
    }
    
    // 添加点击事件（简单版本，实际应该调用 bridge）
    div = div.on_click(cx.listener({
        let widget_id = widget_id.clone();
        move |_event, _window, _cx| {
            log::debug!("Clicked button: {}", widget_id);
            // 这里应该调用 event dispatcher
        }
    }));
    
    div
}

/// 渲染通用容器
fn render_container(
    mut div: Div,
    widget: &Widget,
    manager: &WidgetManager,
    window: &mut Window,
    cx: &mut Context,
) -> Div {
    for child_id in &widget.children {
        div = div.child(render_widget_recursive(manager, child_id, window, cx));
    }
    div
}

/// 应用对齐方式
fn apply_alignment(div: &mut Div, alignment: Option<Alignment>) {
    if let Some(alignment) = alignment {
        match alignment {
            Alignment::Start => {
                *div = div.justify_start().items_start();
            }
            Alignment::Center => {
                *div = div.justify_center().items_center();
            }
            Alignment::End => {
                *div = div.justify_end().items_end();
            }
            Alignment::SpaceBetween => {
                *div = div.justify_between();
            }
            Alignment::SpaceAround => {
                *div = div.justify_around();
            }
        }
    }
}

/// 应用间距
fn apply_gap(div: &mut Div, gap: Option<f32>) {
    if let Some(gap) = gap {
        *div = div.gap(px(gap));
    }
}

/// 颜色转换：Adapter Color → GPUI Hsla
fn convert_color(color: &Color) -> Hsla {
    Hsla::new(color.r, color.g, color.b, color.a)
}

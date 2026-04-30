use gpui::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::ActiveTheme;

use crate::core::math_completion::{
    MathCompletionItem, filter_math_completions, math_completion_items,
};

const ITEM_HEIGHT: f32 = 28.;
const PANEL_MAX_HEIGHT: f32 = 240.;
const PANEL_WIDTH: f32 = 260.;

pub struct MathCompletionPanel {
    visible: bool,
    query: String,
    filtered_indices: Vec<usize>,
    selected_index: usize,
    scroll_handle: ScrollHandle,
    replace_start_source: usize,
    replace_end_source: usize,
}

impl MathCompletionPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_handle: ScrollHandle::new(),
            replace_start_source: 0,
            replace_end_source: 0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, replace_start_source: usize, replace_end_source: usize) {
        self.visible = true;
        self.query.clear();
        self.replace_start_source = replace_start_source;
        self.replace_end_source = replace_end_source;
        self.filtered_indices = (0..math_completion_items().len()).collect();
        self.selected_index = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
    }

    pub fn update_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.filtered_indices = filter_math_completions(&self.query);
        self.selected_index = 0;
    }

    pub fn replace_range(&self) -> (usize, usize) {
        (self.replace_start_source, self.replace_end_source)
    }

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
            self.scroll_to_selected();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected_index - 1
            };
            self.scroll_to_selected();
        }
    }

    fn scroll_to_selected(&self) {
        let item_top = self.selected_index as f32 * ITEM_HEIGHT;
        let viewport_top: f32 = (-self.scroll_handle.offset().y).into();
        let viewport = self.scroll_handle.bounds();
        let viewport_bottom = viewport_top + f32::from(viewport.size.height);

        if item_top < viewport_top {
            self.scroll_handle
                .set_offset(gpui::point(self.scroll_handle.offset().x, px(-(item_top))));
        } else if item_top + ITEM_HEIGHT > viewport_bottom {
            self.scroll_handle.set_offset(gpui::point(
                self.scroll_handle.offset().x,
                px(-(item_top + ITEM_HEIGHT - f32::from(viewport.size.height))),
            ));
        }
    }

    pub fn selected_completion(&self) -> Option<&MathCompletionItem> {
        self.filtered_indices
            .get(self.selected_index)
            .map(|&i| &math_completion_items()[i])
    }

    pub fn render_panel(
        &self,
        _window: &mut Window,
        cx: &mut App,
        cursor_y: Option<f32>,
    ) -> Option<gpui::AnyElement> {
        if !self.visible || self.filtered_indices.is_empty() {
            return None;
        }

        let theme = cx.theme();
        let all_items = math_completion_items();
        let list_items: Vec<gpui::AnyElement> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(ui_index, &item_index)| {
                let item = &all_items[item_index];
                let is_selected = ui_index == self.selected_index;

                let bg = if is_selected {
                    div().bg(theme.primary).opacity(0.15)
                } else {
                    div()
                };

                let desc_color = theme.muted_foreground;
                let category_color = theme.muted_foreground;

                bg.id(SharedString::from(std::format!(
                    "math-cmd-{item_index}"
                )))
                .w_full()
                .h(px(ITEM_HEIGHT))
                .px(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.foreground)
                        .child(format!("\\{}", item.command)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(category_color)
                                .child(format!("{:?}", item.category)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(desc_color)
                                .child(item.description),
                        ),
                )
                .into_any_element()
            })
            .collect();

        let top = cursor_y.unwrap_or(0.) + 4.;

        Some(
            div()
                .id("math-completion-panel")
                .absolute()
                .top(px(top))
                .left(px(0.))
                .w(px(PANEL_WIDTH))
                .max_h(px(PANEL_MAX_HEIGHT))
                .bg(theme.background)
                .border(px(1.))
                .border_color(theme.border)
                .rounded(px(8.))
                .shadow(vec![gpui::BoxShadow {
                    color: gpui::hsla(0., 0., 0., 0.2),
                    offset: gpui::point(px(0.), px(4.)),
                    blur_radius: px(12.),
                    spread_radius: px(0.),
                }])
                .p(px(4.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll_handle)
                .children(list_items)
                .into_any_element(),
        )
    }
}

impl Render for MathCompletionPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

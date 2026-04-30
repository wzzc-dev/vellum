use super::*;
use gpui::Focusable;

impl VellumApp {
    pub(super) fn on_open_file(
        &mut self,
        _: &OpenFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_file_dialog(window, cx);
    }

    pub(super) fn on_open_find_panel(
        &mut self,
        _: &OpenFindPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        cx.notify();
    }

    pub(super) fn on_close_find_panel(
        &mut self,
        _: &CloseFindPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.command_palette.is_visible() {
            self.command_palette.hide();
            window.focus(&self.focus_handle);
        } else {
            self.close_find_panel();
        }
        cx.notify();
    }

    pub(super) fn on_find_next_match(
        &mut self,
        _: &FindNextMatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        if let Some(offset) = self.navigate_find_match(false) {
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.select_source_offset(offset, window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn on_find_previous_match(
        &mut self,
        _: &FindPreviousMatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        if let Some(offset) = self.navigate_find_match(true) {
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.select_source_offset(offset, window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn on_open_find_replace_panel(
        &mut self,
        _: &OpenFindReplacePanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_replace_panel();
        cx.notify();
    }

    pub(super) fn on_replace_one(
        &mut self,
        _: &ReplaceOne,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_current_match(window, cx);
        cx.notify();
    }

    pub(super) fn on_replace_all(
        &mut self,
        _: &ReplaceAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_all_matches(window, cx);
        cx.notify();
    }

    pub(super) fn on_open_folder(
        &mut self,
        _: &OpenFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_open_folder(window, cx);
    }

    pub(super) fn on_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_file(window, cx);
    }

    pub(super) fn on_save_now(&mut self, _: &SaveNow, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document(window, cx) {
            self.set_status(format!("Save failed: {err}"));
            cx.notify();
        }
    }

    pub(super) fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document_as(window, cx) {
            self.set_status(format!("Save As failed: {err}"));
            cx.notify();
        }
    }

    pub(super) fn on_toggle_sidebar(
        &mut self,
        _: &ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar_visibility(cx);
    }

    pub(super) fn on_toggle_right_panel(
        &mut self,
        _: &ToggleRightPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_right_panel_visibility(cx);
    }

    pub(super) fn on_toggle_status_bar(
        &mut self,
        _: &ToggleStatusBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_status_bar_pinned(!self.status_bar_pinned, window, cx);
    }

    pub(super) fn on_toggle_focus_mode(
        &mut self,
        _: &ToggleFocusMode,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_mode = !self.focus_mode;
        cx.notify();
    }

    pub(super) fn on_close_tab(
        &mut self,
        _: &CloseTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_active_tab(window, cx);
    }

    pub(super) fn on_previous_tab(
        &mut self,
        _: &PreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.len() > 1 {
            let index = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.switch_to_tab(index, window, cx);
        }
    }

    pub(super) fn on_next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() > 1 {
            let index = (self.active_tab_index + 1) % self.tabs.len();
            self.switch_to_tab(index, window, cx);
        }
    }

    pub(super) fn on_manage_plugins(
        &mut self,
        _: &ManagePlugins,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_right_panel(RightPanelView::Plugins, cx);
    }

    pub(super) fn on_install_dev_extension(
        &mut self,
        _: &InstallDevExtension,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.install_dev_extension(window, cx);
    }

    pub(super) fn on_open_command_palette(
        &mut self,
        _: &OpenCommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_palette.show();
        self.command_palette.input.update(cx, |input, cx| {
            input.set_value(String::new(), window, cx);
        });
        let focus = self.command_palette.input.focus_handle(cx);
        window.focus(&focus);
        cx.notify();
    }

    pub(super) fn execute_palette_command(
        &mut self,
        cmd: crate::app::command_palette::PaletteCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::app::command_palette::PaletteCommand;

        self.command_palette.hide();
        window.focus(&self.focus_handle);

        match cmd {
            PaletteCommand::Bold => {
                window.dispatch_action(Box::new(editor::BoldSelection), cx);
            }
            PaletteCommand::Italic => {
                window.dispatch_action(Box::new(editor::ItalicSelection), cx);
            }
            PaletteCommand::InlineCode => {
                window.dispatch_action(Box::new(editor::ToggleInlineCode), cx);
            }
            PaletteCommand::Strikethrough => {
                window.dispatch_action(Box::new(editor::ToggleStrikethrough), cx);
            }
            PaletteCommand::Link => {
                window.dispatch_action(Box::new(editor::LinkSelection), cx);
            }
            PaletteCommand::Heading1 => {
                window.dispatch_action(Box::new(editor::ToggleHeading1), cx);
            }
            PaletteCommand::Heading2 => {
                window.dispatch_action(Box::new(editor::ToggleHeading2), cx);
            }
            PaletteCommand::Heading3 => {
                window.dispatch_action(Box::new(editor::ToggleHeading3), cx);
            }
            PaletteCommand::Heading4 => {
                window.dispatch_action(Box::new(editor::ToggleHeading4), cx);
            }
            PaletteCommand::Heading5 => {
                window.dispatch_action(Box::new(editor::ToggleHeading5), cx);
            }
            PaletteCommand::Heading6 => {
                window.dispatch_action(Box::new(editor::ToggleHeading6), cx);
            }
            PaletteCommand::Paragraph => {
                window.dispatch_action(Box::new(editor::ToggleParagraph), cx);
            }
            PaletteCommand::Blockquote => {
                window.dispatch_action(Box::new(editor::ToggleBlockquote), cx);
            }
            PaletteCommand::BulletList => {
                window.dispatch_action(Box::new(editor::ToggleBulletList), cx);
            }
            PaletteCommand::OrderedList => {
                window.dispatch_action(Box::new(editor::ToggleOrderedList), cx);
            }
            PaletteCommand::HorizontalRule => {
                window.dispatch_action(Box::new(editor::InsertHorizontalRule), cx);
            }
            PaletteCommand::CodeFence => {
                window.dispatch_action(Box::new(editor::InsertCodeFence), cx);
            }
            PaletteCommand::Table => {
                window.dispatch_action(Box::new(editor::InsertTable), cx);
            }
            PaletteCommand::SourceMode => {
                window.dispatch_action(Box::new(editor::ToggleSourceMode), cx);
            }
            PaletteCommand::Undo => {
                window.dispatch_action(Box::new(editor::UndoEdit), cx);
            }
            PaletteCommand::Redo => {
                window.dispatch_action(Box::new(editor::RedoEdit), cx);
            }
            PaletteCommand::ToggleSidebar => {
                self.toggle_sidebar_visibility(cx);
            }
            PaletteCommand::ToggleStatusBar => {
                self.set_status_bar_pinned(!self.status_bar_pinned, window, cx);
            }
            PaletteCommand::ToggleFocusMode => {
                self.focus_mode = !self.focus_mode;
            }
            PaletteCommand::FindPanel => {
                self.open_find_panel();
            }
            PaletteCommand::FindReplace => {
                self.open_find_replace_panel();
            }
            PaletteCommand::ThemeDefault => {
                editor::set_syntax_theme(editor::SyntaxTheme::Default);
            }
            PaletteCommand::ThemeDracula => {
                editor::set_syntax_theme(editor::SyntaxTheme::Dracula);
            }
            PaletteCommand::ThemeSolarized => {
                editor::set_syntax_theme(editor::SyntaxTheme::Solarized);
            }
            PaletteCommand::ThemeGitHub => {
                editor::set_syntax_theme(editor::SyntaxTheme::GitHub);
            }
            PaletteCommand::MathBlock => {
                window.dispatch_action(Box::new(editor::InsertMathBlock), cx);
            }
        }

        let editor = self.active_editor_entity();
        editor.update(cx, |_, cx| cx.notify());
        cx.notify();
    }

    pub(super) fn on_palette_enter(
        &mut self,
        _: &gpui_component::input::Enter,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(cmd) = self.command_palette.selected_command() {
            self.execute_palette_command(cmd, window, cx);
        }
    }

    pub(super) fn on_palette_move_up(
        &mut self,
        _: &gpui_component::input::MoveUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_palette.select_prev();
        cx.notify();
    }

    pub(super) fn on_palette_move_down(
        &mut self,
        _: &gpui_component::input::MoveDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_palette.select_next();
        cx.notify();
    }
}

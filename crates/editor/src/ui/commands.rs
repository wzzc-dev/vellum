use gpui::{App, Context, KeyBinding, Window};
use gpui_component::input::{DeleteToNextWordEnd, Enter as InputEnter};

use super::{EDITOR_CONTEXT, view::MarkdownEditor};
use crate::{
    BoldSelection, DemoteBlock, ExitBlockEdit, FocusNextBlock, FocusPrevBlock, ItalicSelection,
    LinkSelection, PromoteBlock, RedoEdit, SecondaryEnter, ToggleHeading1, ToggleHeading2,
    ToggleHeading3, ToggleHeading4, ToggleHeading5, ToggleHeading6, ToggleParagraph,
    ToggleSourceMode, UndoEdit,
};

const GPUI_COMPONENT_INPUT_CONTEXT: &str = "Input";

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-b", BoldSelection, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-b", BoldSelection, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-i", ItalicSelection, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-i", ItalicSelection, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-k", LinkSelection, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-k", LinkSelection, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-1", ToggleHeading1, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-1", ToggleHeading1, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-2", ToggleHeading2, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-2", ToggleHeading2, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-3", ToggleHeading3, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-3", ToggleHeading3, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-4", ToggleHeading4, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-4", ToggleHeading4, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-5", ToggleHeading5, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-5", ToggleHeading5, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-6", ToggleHeading6, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-6", ToggleHeading6, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-0", ToggleParagraph, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ToggleParagraph, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-[", PromoteBlock, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-[", PromoteBlock, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-]", DemoteBlock, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-]", DemoteBlock, Some(EDITOR_CONTEXT)),
        KeyBinding::new("escape", ExitBlockEdit, Some(EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-/", ToggleSourceMode, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-/", ToggleSourceMode, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", UndoEdit, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", UndoEdit, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", RedoEdit, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-z", RedoEdit, Some(EDITOR_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", RedoEdit, Some(EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-enter", SecondaryEnter, Some(EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-delete", DeleteToNextWordEnd, Some(EDITOR_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-b", BoldSelection, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-b", BoldSelection, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-i", ItalicSelection, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-i",
            ItalicSelection,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-k", LinkSelection, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-k", LinkSelection, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-1", ToggleHeading1, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-1", ToggleHeading1, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-2", ToggleHeading2, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-2", ToggleHeading2, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-3", ToggleHeading3, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-3", ToggleHeading3, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-4", ToggleHeading4, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-4", ToggleHeading4, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-5", ToggleHeading5, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-5", ToggleHeading5, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-6", ToggleHeading6, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-6", ToggleHeading6, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-0", ToggleParagraph, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ToggleParagraph, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-[", PromoteBlock, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-[", PromoteBlock, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-]", DemoteBlock, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-]", DemoteBlock, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        KeyBinding::new(
            "ctrl-up",
            FocusPrevBlock,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        KeyBinding::new(
            "ctrl-down",
            FocusNextBlock,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-/",
            ToggleSourceMode,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-/",
            ToggleSourceMode,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", UndoEdit, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", UndoEdit, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", RedoEdit, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-z", RedoEdit, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", RedoEdit, Some(GPUI_COMPONENT_INPUT_CONTEXT)),
        KeyBinding::new(
            "ctrl-enter",
            SecondaryEnter,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        KeyBinding::new(
            "ctrl-delete",
            DeleteToNextWordEnd,
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
        KeyBinding::new(
            "shift-enter",
            InputEnter { secondary: true },
            Some(GPUI_COMPONENT_INPUT_CONTEXT),
        ),
    ]);
}

impl MarkdownEditor {
    pub(crate) fn on_bold_selection(
        &mut self,
        _: &BoldSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("**", "**", window, cx);
    }

    pub(crate) fn on_italic_selection(
        &mut self,
        _: &ItalicSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("*", "*", window, cx);
    }

    pub(crate) fn on_link_selection(
        &mut self,
        _: &LinkSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("[", "](https://)", window, cx);
    }

    pub(crate) fn on_promote_block(
        &mut self,
        _: &PromoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(false, window, cx);
    }

    pub(crate) fn on_demote_block(
        &mut self,
        _: &DemoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(true, window, cx);
    }

    pub(crate) fn on_exit_block_edit(
        &mut self,
        _: &ExitBlockEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.exit_edit_mode(window, cx);
    }

    pub(crate) fn on_focus_prev_block(
        &mut self,
        _: &FocusPrevBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(-1, None, window, cx);
    }

    pub(crate) fn on_focus_next_block(
        &mut self,
        _: &FocusNextBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(1, None, window, cx);
    }

    pub(crate) fn on_undo_edit(
        &mut self,
        _: &UndoEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.undo(window, cx);
    }

    pub(crate) fn on_redo_edit(
        &mut self,
        _: &RedoEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.redo(window, cx);
    }

    pub(crate) fn on_secondary_enter(
        &mut self,
        _: &SecondaryEnter,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.secondary_enter(window, cx);
    }

    pub(crate) fn on_toggle_source_mode(
        &mut self,
        _: &ToggleSourceMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_view_mode(window, cx);
    }

    pub(crate) fn on_toggle_heading1(
        &mut self,
        _: &ToggleHeading1,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(1, window, cx);
    }

    pub(crate) fn on_toggle_heading2(
        &mut self,
        _: &ToggleHeading2,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(2, window, cx);
    }

    pub(crate) fn on_toggle_heading3(
        &mut self,
        _: &ToggleHeading3,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(3, window, cx);
    }

    pub(crate) fn on_toggle_heading4(
        &mut self,
        _: &ToggleHeading4,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(4, window, cx);
    }

    pub(crate) fn on_toggle_heading5(
        &mut self,
        _: &ToggleHeading5,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(5, window, cx);
    }

    pub(crate) fn on_toggle_heading6(
        &mut self,
        _: &ToggleHeading6,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(6, window, cx);
    }

    pub(crate) fn on_toggle_paragraph(
        &mut self,
        _: &ToggleParagraph,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_heading(0, window, cx);
    }
}

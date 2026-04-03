use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, Bounds, ClickEvent, Context, Entity, IntoElement, Pixels, Styled, Subscription,
    Window, canvas,
};

use crate::core::controller::{BlockSnapshot, EditorSnapshot};

use super::{
    component_ui::{BlockInput, InputEvent, InputNavigationState},
    layout::byte_offset_for_click_position,
    view::MarkdownEditor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveSelectionView {
    pub(crate) block_id: Option<u64>,
    pub(crate) cursor_offset: Option<usize>,
}

impl ActiveSelectionView {
    pub(crate) fn from_snapshot(snapshot: &EditorSnapshot) -> Self {
        Self {
            block_id: snapshot.active_block_id,
            cursor_offset: snapshot.active_cursor_offset,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveBlockSession {
    pub(crate) block_id: u64,
    pub(crate) input: BlockInput,
}

impl ActiveBlockSession {
    pub(crate) fn new(block_id: u64, input: BlockInput) -> Self {
        Self { block_id, input }
    }
}

pub(crate) struct EditorInteractionState {
    active_session: Option<ActiveBlockSession>,
    input_subscription: Option<Subscription>,
    autosave_generation: u64,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<Pixels>>>>,
}

impl EditorInteractionState {
    pub(crate) fn new() -> Self {
        Self {
            active_session: None,
            input_subscription: None,
            autosave_generation: 0,
            block_bounds: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub(crate) fn reset_after_active_selection_change(
        &mut self,
        previous: ActiveSelectionView,
        next: ActiveSelectionView,
    ) {
        if previous.block_id != next.block_id {
            self.input_subscription = None;
            self.active_session = None;
        }
    }

    pub(crate) fn clear_session(&mut self) {
        self.input_subscription = None;
        self.active_session = None;
    }

    pub(crate) fn active_session(&self) -> Option<&ActiveBlockSession> {
        self.active_session.as_ref()
    }

    pub(crate) fn is_block_active(&self, block_id: u64) -> bool {
        self.active_session
            .as_ref()
            .map(|session| session.block_id == block_id)
            .unwrap_or(false)
    }

    pub(crate) fn sync_active_input(
        &mut self,
        view: Entity<MarkdownEditor>,
        snapshot: &EditorSnapshot,
        active_selection: ActiveSelectionView,
        window: &mut Window,
        cx: &mut Context<MarkdownEditor>,
    ) {
        let Some(block_id) = active_selection.block_id else {
            self.clear_session();
            return;
        };
        let Some(block) = snapshot.block_by_id(block_id).cloned() else {
            self.clear_session();
            return;
        };

        let needs_new_input = self
            .active_session
            .as_ref()
            .map(|session| session.block_id != block_id)
            .unwrap_or(true);

        if needs_new_input {
            self.input_subscription = None;
            let input = BlockInput::new(&block.kind, block.text.clone(), window, cx);
            let subscription = window.subscribe(
                input.entity(),
                cx,
                move |_, event: &InputEvent, window, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.handle_input_event(event, window, cx);
                    });
                },
            );
            self.active_session = Some(ActiveBlockSession::new(block_id, input));
            self.input_subscription = Some(subscription);
        }

        let desired_cursor = active_selection
            .cursor_offset
            .unwrap_or_else(|| block.text.len());
        if let Some(session) = self.active_session.as_ref() {
            session.input.sync(&block.text, desired_cursor, window, cx);
        }
    }

    pub(crate) fn next_autosave_token(&mut self) -> u64 {
        self.autosave_generation = self.autosave_generation.wrapping_add(1);
        self.autosave_generation
    }

    pub(crate) fn autosave_generation(&self) -> u64 {
        self.autosave_generation
    }

    pub(crate) fn navigation_target_for_direction(
        &self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<MarkdownEditor>,
    ) -> Option<(isize, usize)> {
        let session = self.active_session.as_ref()?;
        let state = session.input.navigation_state(window, cx);
        navigation_target_for_state(&state, direction)
    }

    pub(crate) fn cursor_offset_for_click(
        &self,
        block: &BlockSnapshot,
        event: &ClickEvent,
        window: &mut Window,
    ) -> Option<usize> {
        self.block_bounds
            .borrow()
            .get(&block.id)
            .cloned()
            .map(|bounds| {
                byte_offset_for_click_position(
                    &block.kind,
                    &block.text,
                    event.position(),
                    bounds,
                    window,
                )
            })
    }

    pub(crate) fn capture_block_bounds(&self, block_id: u64) -> AnyElement {
        let block_bounds = self.block_bounds.clone();
        canvas(
            move |bounds, _, _| bounds,
            move |_, bounds, _, _| {
                block_bounds.borrow_mut().insert(block_id, bounds);
            },
        )
        .absolute()
        .size_full()
        .into_any_element()
    }

    pub(crate) fn clear_block_bounds(&self) {
        self.block_bounds.borrow_mut().clear();
    }
}

fn navigation_target_for_state(
    state: &InputNavigationState,
    direction: isize,
) -> Option<(isize, usize)> {
    if state.has_selection {
        return None;
    }

    let moving_up = match direction.cmp(&0) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => return None,
    };

    let last_line = state.text.lines().count().max(1).saturating_sub(1);
    let at_boundary = if moving_up {
        state.line == 0
    } else {
        state.line >= last_line
    };

    at_boundary.then_some((direction.signum(), state.column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_target_moves_up_from_first_line() {
        let state = InputNavigationState {
            text: "first\nsecond".to_string(),
            line: 0,
            column: 3,
            has_selection: false,
        };

        assert_eq!(navigation_target_for_state(&state, -1), Some((-1, 3)));
    }

    #[test]
    fn navigation_target_moves_down_from_last_line() {
        let state = InputNavigationState {
            text: "first\nsecond".to_string(),
            line: 1,
            column: 2,
            has_selection: false,
        };

        assert_eq!(navigation_target_for_state(&state, 1), Some((1, 2)));
    }

    #[test]
    fn navigation_target_does_not_move_from_middle_line() {
        let state = InputNavigationState {
            text: "one\ntwo\nthree".to_string(),
            line: 1,
            column: 1,
            has_selection: false,
        };

        assert_eq!(navigation_target_for_state(&state, -1), None);
        assert_eq!(navigation_target_for_state(&state, 1), None);
    }

    #[test]
    fn navigation_target_ignores_selection() {
        let state = InputNavigationState {
            text: "first\nsecond".to_string(),
            line: 0,
            column: 0,
            has_selection: true,
        };

        assert_eq!(navigation_target_for_state(&state, -1), None);
        assert_eq!(navigation_target_for_state(&state, 1), None);
    }
}

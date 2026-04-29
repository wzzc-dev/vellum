use crate::decoration::{Decoration, Tooltip};
use crate::event::ExtensionEvent;
use crate::host;
use crate::manifest::ExtensionManifest;
use crate::ui::{UiEvent, UiNode};

pub type ExtensionResult<T = ()> = Result<T, String>;

pub trait Extension: Sized + Send + 'static {
    fn new() -> Self;

    fn manifest() -> ExtensionManifest;

    fn activate(&mut self, _ctx: &mut ExtensionContext) -> ExtensionResult {
        Ok(())
    }

    fn deactivate(&mut self, _ctx: &mut ExtensionContext) -> ExtensionResult {
        Ok(())
    }

    fn handle_event(
        &mut self,
        _event: ExtensionEvent,
        _ctx: &mut ExtensionContext,
    ) -> ExtensionResult {
        Ok(())
    }

    fn execute_command(
        &mut self,
        _command_id: String,
        _ctx: &mut ExtensionContext,
    ) -> ExtensionResult {
        Ok(())
    }

    fn handle_ui_event(&mut self, _event: UiEvent, _ctx: &mut ExtensionContext) -> ExtensionResult {
        Ok(())
    }

    fn handle_hover(
        &mut self,
        _hover_data: String,
        _ctx: &mut ExtensionContext,
    ) -> ExtensionResult<Option<Tooltip>> {
        Ok(None)
    }
}

pub struct ExtensionContext {
    extension_id: String,
    extension_path: String,
}

impl ExtensionContext {
    pub fn new(extension_id: String, extension_path: String) -> Self {
        Self {
            extension_id,
            extension_path,
        }
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn extension_path(&self) -> &str {
        &self.extension_path
    }

    pub fn set_status_message(&mut self, message: &str) -> ExtensionResult {
        host::show_status_message(message)
    }

    pub fn document_text(&self) -> ExtensionResult<String> {
        host::document_text()
    }

    pub fn document_path(&self) -> ExtensionResult<Option<String>> {
        host::document_path()
    }

    pub fn insert_text(&mut self, position: usize, text: &str) -> ExtensionResult {
        host::insert_text(position, text)
    }

    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) -> ExtensionResult {
        host::replace_range(start, end, text)
    }

    pub fn set_panel_ui(&mut self, panel_id: &str, root: UiNode) -> ExtensionResult {
        host::set_panel_view(panel_id, &root)
    }

    pub fn set_decorations(&mut self, decorations: Vec<Decoration>) -> ExtensionResult {
        host::set_decorations(&decorations)
    }

    pub fn clear_decorations(&mut self) -> ExtensionResult {
        host::clear_decorations()
    }
}

#[macro_export]
macro_rules! register_extension {
    ($extension_type:ty) => {
        struct __VellumExtensionGuest;

        impl __VellumExtensionGuest {
            fn instance() -> &'static std::sync::Mutex<Option<$extension_type>> {
                static INSTANCE: std::sync::OnceLock<
                    std::sync::Mutex<Option<$extension_type>>,
                > = std::sync::OnceLock::new();
                INSTANCE.get_or_init(|| std::sync::Mutex::new(None))
            }

            fn with_instance<R>(
                ctx: $crate::bindings::vellum::extension::types::ActivationContext,
                f: impl FnOnce(&mut $extension_type, &mut $crate::plugin::ExtensionContext) -> Result<R, String>,
            ) -> Result<R, $crate::bindings::vellum::extension::types::ExtensionError> {
                let mutex = Self::instance();
                let mut guard = mutex.lock().map_err(|_| {
                    $crate::bindings::vellum::extension::types::ExtensionError {
                        message: "extension instance mutex poisoned".into(),
                    }
                })?;
                if guard.is_none() {
                    *guard = Some(<$extension_type as $crate::plugin::Extension>::new());
                }
                let instance = guard.as_mut().unwrap();
                let mut ctx = $crate::plugin::ExtensionContext::new(
                    ctx.extension_id,
                    ctx.extension_path,
                );
                f(instance, &mut ctx).map_err(|message| {
                    $crate::bindings::vellum::extension::types::ExtensionError { message }
                })
            }

            fn with_existing<R>(
                f: impl FnOnce(&mut $extension_type, &mut $crate::plugin::ExtensionContext) -> Result<R, String>,
            ) -> Result<R, $crate::bindings::vellum::extension::types::ExtensionError> {
                let mutex = Self::instance();
                let mut guard = mutex.lock().map_err(|_| {
                    $crate::bindings::vellum::extension::types::ExtensionError {
                        message: "extension instance mutex poisoned".into(),
                    }
                })?;
                let instance = guard.as_mut().ok_or_else(|| {
                    $crate::bindings::vellum::extension::types::ExtensionError {
                        message: "extension is not active".into(),
                    }
                })?;
                let mut ctx = $crate::plugin::ExtensionContext::new(String::new(), String::new());
                f(instance, &mut ctx).map_err(|message| {
                    $crate::bindings::vellum::extension::types::ExtensionError { message }
                })
            }
        }

        impl $crate::bindings::Guest for __VellumExtensionGuest {
            fn activate(
                ctx: $crate::bindings::vellum::extension::types::ActivationContext,
            ) -> Result<(), $crate::bindings::vellum::extension::types::ExtensionError> {
                Self::with_instance(ctx, |instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::activate(instance, ctx)
                })
            }

            fn deactivate(
            ) -> Result<(), $crate::bindings::vellum::extension::types::ExtensionError> {
                Self::with_existing(|instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::deactivate(instance, ctx)
                })?;
                let mutex = Self::instance();
                if let Ok(mut guard) = mutex.lock() {
                    *guard = None;
                }
                Ok(())
            }

            fn handle_event(
                event: $crate::bindings::vellum::extension::types::ExtensionEvent,
            ) -> Result<(), $crate::bindings::vellum::extension::types::ExtensionError> {
                Self::with_existing(|instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::handle_event(
                        instance,
                        $crate::event::ExtensionEvent {
                            event_type: event.event_type,
                            document_text: event.document_text,
                            document_path: event.document_path,
                        },
                        ctx,
                    )
                })
            }

            fn execute_command(
                command_id: String,
            ) -> Result<(), $crate::bindings::vellum::extension::types::ExtensionError> {
                Self::with_existing(|instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::execute_command(
                        instance,
                        command_id,
                        ctx,
                    )
                })
            }

            fn handle_ui_event(
                event: $crate::bindings::vellum::extension::types::UiEvent,
            ) -> Result<(), $crate::bindings::vellum::extension::types::ExtensionError> {
                let event = $crate::plugin::ui_event_from_bindings(event);
                Self::with_existing(|instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::handle_ui_event(
                        instance,
                        event,
                        ctx,
                    )
                })
            }

            fn handle_hover(
                hover_data: String,
            ) -> Result<Option<Vec<u8>>, $crate::bindings::vellum::extension::types::ExtensionError> {
                let tooltip = Self::with_existing(|instance, ctx| {
                    <$extension_type as $crate::plugin::Extension>::handle_hover(
                        instance,
                        hover_data,
                        ctx,
                    )
                })?;
                match tooltip {
                    Some(tooltip) => $crate::host::encode_tooltip(tooltip)
                        .map(Some)
                        .map_err(|message| {
                            $crate::bindings::vellum::extension::types::ExtensionError { message }
                        }),
                    None => Ok(None),
                }
            }
        }

        $crate::bindings::export!(__VellumExtensionGuest);
    };
}

#[doc(hidden)]
pub fn ui_event_from_bindings(
    event: crate::bindings::vellum::extension::types::UiEvent,
) -> UiEvent {
    match event.event_kind.as_str() {
        "input.changed" => UiEvent::InputChanged {
            panel_id: event.panel_id,
            element_id: event.element_id,
            value: event.value.unwrap_or_default(),
        },
        "checkbox.toggled" => UiEvent::CheckboxToggled {
            panel_id: event.panel_id,
            element_id: event.element_id,
            checked: event.checked.unwrap_or(false),
        },
        "select.changed" => UiEvent::SelectChanged {
            panel_id: event.panel_id,
            element_id: event.element_id,
            index: event.index.unwrap_or(0) as usize,
        },
        "toggle.changed" => UiEvent::ToggleChanged {
            panel_id: event.panel_id,
            element_id: event.element_id,
            active: event.checked.unwrap_or(false),
        },
        "link.clicked" => UiEvent::LinkClicked {
            panel_id: event.panel_id,
            element_id: event.element_id,
        },
        "list.item.clicked" => UiEvent::ListItemClicked {
            panel_id: event.panel_id,
            element_id: event.element_id,
            item_id: event.value.unwrap_or_default(),
        },
        "disclosure.toggled" => UiEvent::DisclosureToggled {
            panel_id: event.panel_id,
            element_id: event.element_id,
            open: event.checked.unwrap_or(false),
        },
        _ => UiEvent::ButtonClicked {
            panel_id: event.panel_id,
            element_id: event.element_id,
        },
    }
}

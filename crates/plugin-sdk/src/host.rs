unsafe extern "C" {
    fn host_alloc(size: u32) -> u32;
    fn host_dealloc(ptr: u32, size: u32);
    fn host_register_command(
        id_ptr: u32,
        id_len: u32,
        label_ptr: u32,
        label_len: u32,
        key_ptr: u32,
        key_len: u32,
    ) -> u32;
    fn host_register_sidebar_panel(
        id_ptr: u32,
        id_len: u32,
        label_ptr: u32,
        label_len: u32,
        icon: u32,
    ) -> u32;
    fn host_subscribe_event(event_type: u32) -> u32;
    fn host_set_status_message(msg_ptr: u32, msg_len: u32);
    fn host_get_document_text(buf_ptr: u32, buf_len: u32) -> u32;
    fn host_get_document_path(buf_ptr: u32, buf_len: u32) -> u32;
    fn host_set_panel_ui(panel_id: u32, ui_ptr: u32, ui_len: u32);
    fn host_set_decorations(decos_ptr: u32, decos_len: u32);
    fn host_clear_decorations();
    fn host_show_overlay(overlay_ptr: u32, overlay_len: u32);
    fn host_hide_overlay(id_ptr: u32, id_len: u32);
    fn host_show_tooltip(position: u32, content_ptr: u32, content_len: u32);
    fn host_hide_tooltip();
    fn host_insert_text(text_ptr: u32, text_len: u32);
    fn host_replace_range(start: u32, end: u32, text_ptr: u32, text_len: u32);
}

pub fn alloc_and_write(data: &[u8]) -> u32 {
    unsafe {
        let ptr = host_alloc(data.len() as u32);
        if ptr == 0 {
            return 0;
        }
        let slice = core::slice::from_raw_parts_mut(ptr as *mut u8, data.len());
        slice.copy_from_slice(data);
        ptr
    }
}

pub fn dealloc(ptr: u32, size: u32) {
    unsafe {
        host_dealloc(ptr, size);
    }
}

pub fn register_command(id: &str, label: &str, key_binding: Option<&str>) -> u32 {
    unsafe {
        let id_bytes = id.as_bytes();
        let label_bytes = label.as_bytes();
        let id_ptr = alloc_and_write(id_bytes);
        let label_ptr = alloc_and_write(label_bytes);
        let (key_ptr, key_len) = match key_binding {
            Some(key) => {
                let key_bytes = key.as_bytes();
                let ptr = alloc_and_write(key_bytes);
                (ptr, key_bytes.len() as u32)
            }
            None => (0, 0),
        };
        let result = host_register_command(
            id_ptr,
            id_bytes.len() as u32,
            label_ptr,
            label_bytes.len() as u32,
            key_ptr,
            key_len,
        );
        dealloc(id_ptr, id_bytes.len() as u32);
        dealloc(label_ptr, label_bytes.len() as u32);
        if key_binding.is_some() && key_ptr != 0 {
            dealloc(key_ptr, key_len);
        }
        result
    }
}

pub fn register_sidebar_panel(id: &str, label: &str, icon: &str) -> u32 {
    unsafe {
        let id_bytes = id.as_bytes();
        let label_bytes = label.as_bytes();
        let id_ptr = alloc_and_write(id_bytes);
        let label_ptr = alloc_and_write(label_bytes);
        let icon_code = match icon {
            "file-text" => 0,
            "search" => 1,
            "triangle-alert" => 2,
            "settings" => 3,
            "bar-chart" => 4,
            _ => 0,
        };
        let result = host_register_sidebar_panel(
            id_ptr,
            id_bytes.len() as u32,
            label_ptr,
            label_bytes.len() as u32,
            icon_code,
        );
        dealloc(id_ptr, id_bytes.len() as u32);
        dealloc(label_ptr, label_bytes.len() as u32);
        result
    }
}

pub fn subscribe(event_type: u32) -> u32 {
    unsafe { host_subscribe_event(event_type) }
}

pub fn set_status_message(message: &str) {
    unsafe {
        let bytes = message.as_bytes();
        let ptr = alloc_and_write(bytes);
        host_set_status_message(ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn get_document_text() -> String {
    unsafe {
        let mut buf = vec![0u8; 65536];
        let len = host_get_document_text(buf.as_mut_ptr() as u32, buf.len() as u32);
        buf.truncate(len as usize);
        String::from_utf8_lossy(&buf).into_owned()
    }
}

pub fn get_document_path() -> Option<String> {
    unsafe {
        let mut buf = vec![0u8; 4096];
        let len = host_get_document_path(buf.as_mut_ptr() as u32, buf.len() as u32);
        if len == 0 {
            None
        } else {
            buf.truncate(len as usize);
            Some(String::from_utf8_lossy(&buf).into_owned())
        }
    }
}

pub fn set_panel_ui(panel_id: u32, ui: &crate::ui::UiNode) {
    let bytes = postcard::to_allocvec(ui).unwrap_or_default();
    unsafe {
        let ptr = alloc_and_write(&bytes);
        host_set_panel_ui(panel_id, ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn set_decorations(decorations: &[crate::decoration::Decoration]) {
    let bytes = postcard::to_allocvec(decorations).unwrap_or_default();
    unsafe {
        let ptr = alloc_and_write(&bytes);
        host_set_decorations(ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn clear_decorations() {
    unsafe { host_clear_decorations() }
}

pub fn show_overlay(overlay: &crate::decoration::OverlayPanel) {
    let bytes = postcard::to_allocvec(overlay).unwrap_or_default();
    unsafe {
        let ptr = alloc_and_write(&bytes);
        host_show_overlay(ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn hide_overlay(id: &str) {
    unsafe {
        let bytes = id.as_bytes();
        let ptr = alloc_and_write(bytes);
        host_hide_overlay(ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn show_tooltip(position: crate::decoration::TooltipPosition, content: &crate::ui::UiNode) {
    let bytes = postcard::to_allocvec(content).unwrap_or_default();
    let pos_code = match position {
        crate::decoration::TooltipPosition::Above => 0,
        crate::decoration::TooltipPosition::Below => 1,
        crate::decoration::TooltipPosition::Left => 2,
        crate::decoration::TooltipPosition::Right => 3,
    };
    unsafe {
        let ptr = alloc_and_write(&bytes);
        host_show_tooltip(pos_code, ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn hide_tooltip() {
    unsafe { host_hide_tooltip() }
}

pub fn insert_text(text: &str) {
    unsafe {
        let bytes = text.as_bytes();
        let ptr = alloc_and_write(bytes);
        host_insert_text(ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

pub fn replace_range(start: usize, end: usize, text: &str) {
    unsafe {
        let bytes = text.as_bytes();
        let ptr = alloc_and_write(bytes);
        host_replace_range(start as u32, end as u32, ptr, bytes.len() as u32);
        dealloc(ptr, bytes.len() as u32);
    }
}

use std::collections::HashMap;

use gpui::{App, AppContext as _, Entity, Window};
use gpui_component::webview::WebView as GpuiWebView;
use gpui_component::wry::WebViewBuilder;

pub struct WebViewManager {
    webviews: HashMap<String, Entity<GpuiWebView>>,
}

impl WebViewManager {
    pub fn new() -> Self {
        Self {
            webviews: HashMap::new(),
        }
    }

    pub fn get_or_create(
        &mut self,
        id: &str,
        url: &str,
        allow_scripts: bool,
        allow_devtools: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Entity<GpuiWebView>> {
        if let Some(entity) = self.webviews.get(id) {
            return Some(entity.clone());
        }

        let mut builder = WebViewBuilder::new()
            .with_url(url);

        if allow_scripts {
            builder = builder.with_initialization_script(
                "window.__vellum_ready = true;"
            );
        }

        #[cfg(debug_assertions)]
        if allow_devtools {
            builder = builder.with_devtools(true);
        }

        let wry_webview = match builder.build_as_child(window) {
            Ok(wv) => wv,
            Err(e) => {
                eprintln!("failed to create WebView '{}': {}", id, e);
                return None;
            }
        };

        let gpui_webview = GpuiWebView::new(wry_webview, window, cx);
        let entity: Entity<GpuiWebView> = cx.new(|_| gpui_webview);
        self.webviews.insert(id.to_string(), entity.clone());
        Some(entity)
    }

    pub fn navigate(&mut self, id: &str, url: &str, cx: &mut App) {
        if let Some(entity) = self.webviews.get(id) {
            let _ = entity.update(cx, |wv: &mut GpuiWebView, _| wv.load_url(url));
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.webviews.remove(id);
    }

    pub fn remove_all(&mut self) {
        self.webviews.clear();
    }
}

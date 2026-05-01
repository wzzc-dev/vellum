pub mod bridge;
pub mod error;
pub mod event;
pub mod gpui_render;
pub mod paint;
pub mod types;
pub mod widget;
pub mod window;

#[cfg(feature = "wit")]
pub mod wit_host;

pub use bridge::GpuiBridge;
pub use error::{AdapterError, Result};
pub use event::{EventDispatcher, EventType, GpuiEvent, MouseButton, MouseEventKind};
pub use paint::{Canvas, Image};
pub use types::{
    Alignment, AppTheme, Border, BoxShadow, Color, CursorShape, EdgeInsets, FlexDirection,
    FlexParams, FontStyle, FontWeight, Point, Rect, Size, TextAlign, TextDecoration, TextStyle,
    VerticalAlign, Visibility, WidgetDisplay, WidgetLayout, WidgetPosition, Wrap,
};
pub use widget::{Widget, WidgetId, WidgetManager};
pub use window::{Window, WindowId, WindowManager, WindowOptions};

#[cfg(feature = "wit")]
pub use wit_host::{GuiHost, GuiRuntimeState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///|
/// Comprehensive Demo: All New Features
/// 
/// This example demonstrates:
/// - Navigation system (push/pop/tab)
/// - Animation system (tween/spring)
/// - Grid layout
/// - Environment injection

///| Application state
pub struct DemoState {
  mut current_tab : Int
  mut animation_progress : Double
  mut is_dark_mode : Bool
  mut grid_items : @list.List[String]
}

let demo_state : DemoState = {
  current_tab: 0,
  animation_progress: 0.0,
  is_dark_mode: false,
  grid_items: @list.Nil,
}

///| Initialize state
fn init_demo_state() -> Unit {
  let items = @list.Nil
  for i in 0..10 {
    items = @list.cons("Item " + i.to_string(), items)
  }
  demo_state.grid_items = @list.reverse(items)
}

init_demo_state()

///| Navigation example
fn navigation_demo() -> Widget {
  let navigator = @navigation.Navigator::new("/home")
  
  Column::new()
    .add(Text::new("Navigation Demo").with_font_size(24.0))
    .add(Column::new()
      .add(Button::new("Go to Home").on_click(fn() {
        navigator.push("/home")
      }))
      .add(Button::new("Go to Profile").on_click(fn() {
        navigator.push("/profile")
      }))
      .add(Button::new("Go to Settings").on_click(fn() {
        navigator.push("/settings")
      }))
      .add(Button::new("Pop").on_click(fn() {
        navigator.pop()
      }))
      .add(Text::new("Stack depth: " + navigator.get_state().stack_depth.to_string()))
    )
}

///| Animation example
fn animation_demo() -> Widget {
  let controller = @animation.AnimationController::new()
  
  Column::new()
    .add(Text::new("Animation Demo").with_font_size(24.0))
    .add(
      Container::new()
        .add(Text::new("Animated Box"))
        .with_width(100.0)
        .with_height(100.0)
        .with_background(Color::blue())
        .with_border(Border::new(1.0, Color::black()))
    )
    .add(Column::new()
      .add(Button::new("Ease In Out (300ms)").on_click(fn() {
        controller.animate_tween("scale", 1.0, 1.5, 300, @animation.AnimationCurve::EaseInOut)
      }))
      .add(Button::new("Spring Bouncy").on_click(fn() {
        controller.spring_animate("scale", 1.2, @animation.SpringConfig::bouncy())
      }))
      .add(Button::new("Spring Stiff").on_click(fn() {
        controller.spring_animate("scale", 1.3, @animation.SpringConfig::stiff())
      }))
      .add(Button::new("Stop All").on_click(fn() {
        controller.stop_all()
      }))
    )
}

///| Grid layout example
fn grid_demo() -> Widget {
  let config = @grid.GridConfig::builder()
    .columns(3)
    .gaps(8.0)
    .build()
  
  let items = @list.map(demo_state.grid_items, fn(item : String) -> @grid.GridItem {
    @grid.GridItem::new(item).cross_span(1)
  })
  
  Column::new()
    .add(Text::new("Grid Demo").with_font_size(24.0))
    .add(Text::new("3 columns, 10 items"))
    .add(
      GridView::new(config, items)
    )
}

///| Environment injection example
fn environment_demo() -> Widget {
  let provider = @environment.EnvironmentProvider::new()
  
  provider.set_string(@environment.EnvKeys::color_scheme(), if demo_state.is_dark_mode { "dark" } else { "light" })
  provider.set_string(@environment.EnvKeys::locale(), "en-US")
  provider.set_int(@environment.EnvKeys::font_size_base(), 16)
  
  let color_scheme = @environment.EnvConvenience::get_color_scheme(provider)
  let bg_color = match color_scheme {
    @environment.ColorScheme::Dark => Color::black()
    @environment.ColorScheme::Light => Color::white()
    @environment.ColorScheme::System => Color::gray()
  }
  
  Column::new()
    .add(Text::new("Environment Demo").with_font_size(24.0))
    .add(
      Container::new()
        .add(Column::new()
          .add(Text::new("Color Scheme: " + (match color_scheme {
            @environment.ColorScheme::Dark => "Dark"
            @environment.ColorScheme::Light => "Light"
            @environment.ColorScheme::System => "System"
          })))
          .add(Text::new("Locale: en-US"))
          .add(Text::new("Font Size: 16"))
        )
        .with_padding(EdgeInsets::all(16.0))
        .with_background(bg_color)
    )
    .add(Button::new("Toggle Theme").on_click(fn() {
      demo_state.is_dark_mode = not(demo_state.is_dark_mode)
    }))
}

///| Tab bar example
fn tab_bar_demo() -> Widget {
  let tabs = @list.List[
    @navigation.TabBarItem::new("nav", "Navigation"),
    @navigation.TabBarItem::new("anim", "Animation"),
    @navigation.TabBarItem::new("grid", "Grid"),
    @navigation.TabBarItem::new("env", "Environment"),
  ]
  
  let mut tab_bar = @navigation.TabBar::new(tabs)
  tab_bar = tab_bar.with_on_change(fn(index : Int) {
    demo_state.current_tab = index
  })
  
  let content = match demo_state.current_tab {
    0 => navigation_demo()
    1 => animation_demo()
    2 => grid_demo()
    3 => environment_demo()
    _ => Text::new("Unknown tab")
  }
  
  Column::new()
    .add(Container::new()
      .add(
        Row::new()
          .add(Text::new("Tab: " + demo_state.current_tab.to_string()))
      )
      .with_height(50.0)
      .with_background(Color::light_gray())
    )
    .add(Container::new()
      .add(content)
      .with_flex_grow(1.0)
    )
}

///| Main demo widget
pub fn comprehensive_demo() -> Widget {
  Column::new()
    .add(Text::new("MoonBit GUI Framework Demo")
      .with_font_size(32.0)
      .with_bold(true)
      .with_alignment(Alignment::Center))
    .add(Container::new()
      .add(tab_bar_demo())
      .with_flex_grow(1.0)
      .with_padding(EdgeInsets::all(16.0))
    )
}

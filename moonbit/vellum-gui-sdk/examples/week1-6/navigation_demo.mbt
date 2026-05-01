// Navigation Demo - 声明式导航演示
// 展示 NavigationStack、NavigationLink、路由守卫和深层链接

// 导入状态管理模块
use @gui.state.{State, Binding, autorun}
use @gui.types.{Color, color}
use @gui.container.{Column, Row}
use @gui.text.{Text, Heading}
use @gui.button.{Button}
use @gui.navigation.{NavigationStack, NavigationLink, TabView, Route}

// =============================================================================
// 路由定义
// =============================================================================

// 应用路由
enum AppRoute {
  Home
  Profile(userId: String)
  Settings
  Detail(itemId: String)
  Nested(child: String)
}

impl AppRoute {
  fn path(self: AppRoute) -> String {
    match self {
      Home => "/"
      Profile(id) => "/profile/\{id}"
      Settings => "/settings"
      Detail(id) => "/detail/\{id}"
      Nested(child) => "/nested/\{child}"
    }
  }
  
  fn title(self: AppRoute) -> String {
    match self {
      Home => "Home"
      Profile(_) => "Profile"
      Settings => "Settings"
      Detail(_) => "Detail"
      Nested(_) => "Nested"
    }
  }
  
  fn from_path(path: String) -> Option[AppRoute] {
    if path == "/" {
      Some(Home)
    } else if path.starts_with("/profile/") {
      let id = path.replace("/profile/", "")
      Some(Profile(id))
    } else if path == "/settings" {
      Some(Settings)
    } else if path.starts_with("/detail/") {
      let id = path.replace("/detail/", "")
      Some(Detail(id))
    } else if path.starts_with("/nested/") {
      let child = path.replace("/nested/", "")
      Some(Nested(child))
    } else {
      None
    }
  }
}

// =============================================================================
// 导航状态
// =============================================================================

struct NavigationState {
  mut path: List[AppRoute]
  mut presenting: Option[AppRoute]
  mut tab_index: Int
  mut is_authenticated: Bool
}

impl NavigationState {
  fn new() -> NavigationState {
    {
      mut path: [Home],
      mut presenting: None,
      mut tab_index: 0,
      mut is_authenticated: false
    }
  }
  
  fn current_route(self: NavigationState) -> AppRoute {
    match self.path.peek() {
      Some(route) => route,
      None => Home
    }
  }
  
  fn navigate(self: NavigationState, route: AppRoute) -> Unit {
    self.path.push(route)
  }
  
  fn go_back(self: NavigationState) -> Bool {
    if self.path.length() > 1 {
      self.path.pop()
      true
    } else {
      false
    }
  }
  
  fn pop_to_root(self: NavigationState) -> Unit {
    self.path = [Home]
  }
  
  fn present(self: NavigationState, route: AppRoute) -> Unit {
    self.presenting = Some(route)
  }
  
  fn dismiss(self: NavigationState) -> Unit {
    self.presenting = None
  }
  
  fn can_navigate(self: NavigationState, route: AppRoute) -> Bool {
    // 路由守卫：Settings 需要认证
    match route {
      Settings => self.is_authenticated
      _ => true
    }
  }
  
  fn login(self: NavigationState) -> Unit {
    self.is_authenticated = true
  }
  
  fn logout(self: NavigationState) -> Unit {
    self.is_authenticated = false
    // 如果当前在 Settings，返回首页
    if self.current_route() == Settings {
      self.pop_to_root()
    }
  }
}

// =============================================================================
// 视图组件
// =============================================================================

// 首页视图
fn HomeView(nav_state: NavigationState) -> Widget {
  Column::new([
    Heading::new("Home", 2),
    
    Text::new("Welcome to the Navigation Demo!")
      .padding(16.0),
    
    // 导航链接
    Column::new([
      NavigationLink(
        destination: Profile("user123"),
        label: "View Profile"
      ),
      
      NavigationLink(
        destination: Detail("item1"),
        label: "View Item 1"
      ),
      
      NavigationLink(
        destination: Detail("item2"),
        label: "View Item 2"
      ),
      
      NavigationLink(
        destination: Nested("level1"),
        label: "Go to Nested View"
      ),
    ])
    .gap(12.0)
    .padding(16.0),
    
    // 认证状态
    Row::new([
      Text::new(
        if nav_state.is_authenticated {
          "Logged in"
        } else {
          "Not logged in"
        }
      ),
      
      Button::new(
        if nav_state.is_authenticated {
          "Logout"
        } else {
          "Login"
        }
      )
        .on_click(fn() {
          if nav_state.is_authenticated {
            nav_state.logout()
          } else {
            nav_state.login()
          }
        }),
    ])
    .gap(16.0)
    .padding(16.0),
  ])
  .gap(16.0)
  .padding(24.0)
}

// 个人资料视图
fn ProfileView(user_id: String, nav_state: NavigationState) -> Widget {
  Column::new([
    Heading::new("Profile", 2),
    
    Text::new("User ID: \{user_id}")
      .font_size(16.0),
    
    Text::new("This is the profile page for user \{user_id}.")
      .color(color(0.5, 0.5, 0.5, 1.0))
      .padding(16.0),
    
    Row::new([
      Button::new("Go Back")
        .on_click(fn() {
          nav_state.go_back()
        })
        .variant(Ghost),
      
      Button::new("View Settings")
        .on_click(fn() {
          if nav_state.can_navigate(Settings) {
            nav_state.navigate(Settings)
          }
        }),
    ])
    .gap(16.0),
  ])
  .gap(16.0)
  .padding(24.0)
}

// 设置视图
fn SettingsView(nav_state: NavigationState) -> Widget {
  Column::new([
    Heading::new("Settings", 2),
    
    Text::new("This is a protected route. Only authenticated users can see this.")
      .padding(16.0),
    
    Column::new([
      Row::new([
        Text::new("Dark Mode"),
        Switch::new("dark_mode", false),
      ])
        .justify_content(SpaceBetween),
      
      Row::new([
        Text::new("Notifications"),
        Switch::new("notifications", true),
      ])
        .justify_content(SpaceBetween),
      
      Row::new([
        Text::new("Language"),
        Select::new("language", ["English", "Chinese", "Japanese"], "English"),
      ])
        .justify_content(SpaceBetween),
    ])
    .gap(16.0)
    .padding(16.0),
    
    Button::new("Logout")
      .on_click(fn() {
        nav_state.logout()
      })
      .variant(Secondary),
  ])
  .gap(16.0)
  .padding(24.0)
}

// 详情视图
fn DetailView(item_id: String, nav_state: NavigationState) -> Widget {
  Column::new([
    Heading::new("Detail: \{item_id}", 2),
    
    Text::new("This is the detail view for item \{item_id}.")
      .padding(16.0),
    
    // 相关项目链接
    Text::new("Related Items:")
      .font_weight(Medium)
      .padding(16.0),
    
    Column::new([
      NavigationLink(
        destination: Detail("\{item_id}-related-1"),
        label: "Related Item 1"
      ),
      
      NavigationLink(
        destination: Detail("\{item_id}-related-2"),
        label: "Related Item 2"
      ),
    ])
    .gap(8.0),
    
    Button::new("Go Back")
      .on_click(fn() {
        nav_state.go_back()
      })
      .variant(Ghost),
  ])
  .gap(16.0)
  .padding(24.0)
}

// 嵌套视图
fn NestedView(child: String, nav_state: NavigationState) -> Widget {
  Column::new([
    Heading::new("Nested: \{child}", 2),
    
    Text::new("This demonstrates nested navigation.")
      .padding(16.0),
    
    // 深层嵌套链接
    NavigationLink(
      destination: Nested("\{child}-deeper"),
      label: "Go Deeper"
    )
      .padding(16.0),
    
    Row::new([
      Button::new("Go Back")
        .on_click(fn() {
          nav_state.go_back()
        })
        .variant(Ghost),
      
      Button::new("Pop to Root")
        .on_click(fn() {
          nav_state.pop_to_root()
        })
        .variant(Ghost),
    ])
    .gap(16.0),
  ])
  .gap(16.0)
  .padding(24.0)
}

// =============================================================================
// 路由视图
// =============================================================================

// 根据路由渲染视图
fn route_view(route: AppRoute, nav_state: NavigationState) -> Widget {
  match route {
    Home => HomeView(nav_state),
    Profile(id) => ProfileView(id, nav_state),
    Settings => SettingsView(nav_state),
    Detail(id) => DetailView(id, nav_state),
    Nested(child) => NestedView(child, nav_state),
  }
}

// =============================================================================
// 主视图
// =============================================================================

fn NavigationApp(nav_state: NavigationState) -> Widget {
  // 创建响应式状态
  let path_state = State::new(nav_state.path)
  let auth_state = State::new(nav_state.is_authenticated)
  
  // 当前路由
  let current_route = computed(fn() {
    match path_state.get().peek() {
      Some(route) => route,
      None => Home
    }
  })
  
  // 导航栈深度
  let stack_depth = computed(fn() {
    path_state.get().length()
  })
  
  // 主视图
  Column::new([
    // 顶部导航栏
    Row::new([
      // 返回按钮
      if stack_depth.get() > 1 {
        Button::new("← Back")
          .on_click(fn() {
            nav_state.go_back()
          })
          .variant(Ghost)
      } else {
        Text::new("")
      },
      
      // 标题
      Text::new(current_route.get().title())
        .font_size(18.0)
        .font_weight(Medium)
        .flex_grow(1.0)
        .text_align(Center),
      
      // 认证状态
      Text::new(
        if auth_state.get() { "🔐" } else { "" }
      ),
    ])
    .padding(16.0)
    .background(color(0.95, 0.95, 0.95, 1.0)),
    
    // 内容区域
    NavigationStack::new(
      route_view(current_route.get(), nav_state)
    )
    .transition(.slide)
    .flex_grow(1.0),
    
    // 底部标签栏
    TabView::new([
      (Home, "Home", "🏠"),
      (Profile("user123"), "Profile", "👤"),
      (Settings, "Settings", "⚙️"),
    ])
    .current_tab(nav_state.tab_index)
    .on_change(fn(index, route) {
      nav_state.tab_index = index
      if nav_state.can_navigate(route) {
        nav_state.pop_to_root()
        nav_state.navigate(route)
      } else {
        // 显示登录提示
        @gui.dialog.alert("Authentication Required", "Please log in to access Settings.")
      }
    }),
    
    // 调试信息
    Row::new([
      Text::new("Path: \{current_route.get().path()}")
        .font_size(10.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
      
      Text::new("Depth: \{stack_depth.get()}")
        .font_size(10.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
    ])
    .gap(16.0)
    .padding(8.0),
  ])
}

// =============================================================================
// 主函数
// =============================================================================

pub fn main() -> Unit {
  let nav_state = NavigationState::new()
  let view = NavigationApp(nav_state)
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("AppRoute::path returns correct path") {
  assert_eq!(Home.path(), "/")
  assert_eq!(Profile("123").path(), "/profile/123")
  assert_eq!(Settings.path(), "/settings")
  assert_eq!(Detail("abc").path(), "/detail/abc")
}

test("AppRoute::from_path parses correctly") {
  assert_eq!(AppRoute::from_path("/"), Some(Home))
  assert_eq!(AppRoute::from_path("/profile/123"), Some(Profile("123")))
  assert_eq!(AppRoute::from_path("/settings"), Some(Settings))
  assert_eq!(AppRoute::from_path("/invalid"), None)
}

test("NavigationState::navigate adds to path") {
  let state = NavigationState::new()
  
  state.navigate(Profile("123"))
  
  assert_eq!(state.path.length(), 2)
  assert_eq!(state.current_route(), Profile("123"))
}

test("NavigationState::go_back removes from path") {
  let state = NavigationState::new()
  
  state.navigate(Settings)
  state.go_back()
  
  assert_eq!(state.path.length(), 1)
  assert_eq!(state.current_route(), Home)
}

test("NavigationState::pop_to_root clears path") {
  let state = NavigationState::new()
  
  state.navigate(Profile("123"))
  state.navigate(Settings)
  state.navigate(Detail("item"))
  
  state.pop_to_root()
  
  assert_eq!(state.path.length(), 1)
  assert_eq!(state.current_route(), Home)
}

test("NavigationState::can_navigate respects auth guard") {
  let state = NavigationState::new()
  
  assert!(state.can_navigate(Home))
  assert!(state.can_navigate(Profile("123")))
  assert_eq!(state.can_navigate(Settings), false)  // 需要认证
  
  state.login()
  assert!(state.can_navigate(Settings))  // 认证后可以访问
}

test("NavigationState::logout redirects from protected route") {
  let state = NavigationState::new()
  
  state.login()
  state.navigate(Settings)
  
  state.logout()
  
  assert_eq!(state.is_authenticated, false)
  assert_eq!(state.current_route(), Home)  // 重定向到首页
}

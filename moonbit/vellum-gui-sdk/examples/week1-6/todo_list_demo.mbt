// Todo List Demo - 待办事项应用
// 展示列表状态管理、CRUD 操作、过滤和响应式更新

// 导入状态管理模块
use @gui.state.{State, Binding, autorun, computed, watch}
use @gui.types.{Color, color}
use @gui.container.{Column, Row, ListView}
use @gui.text.{Text, Heading}
use @gui.button.{Button, IconButton}
use @gui.input.{TextInput, Checkbox}
use @gui.dialog.{alert, confirm}

// =============================================================================
// Todo 数据模型
// =============================================================================

// 待办事项
struct Todo {
  id: String
  title: String
  completed: Bool
  created_at: UInt64
}

impl Todo {
  fn new(title: String) -> Todo {
    {
      id: generate_id(),
      title,
      completed: false,
      created_at: @time.now_ms()
    }
  }
  
  fn toggle(self: Todo) -> Todo {
    { ..self, completed: !self.completed }
  }
  
  fn set_title(self: Todo, new_title: String) -> Todo {
    { ..self, title: new_title }
  }
}

// 过滤类型
enum Filter {
  All
  Active
  Completed
}

impl Filter {
  fn label(self: Filter) -> String {
    match self {
      All => "All"
      Active => "Active"
      Completed => "Completed"
    }
  }
  
  fn matches(self: Filter, todo: Todo) -> Bool {
    match self {
      All => true
      Active => !todo.completed
      Completed => todo.completed
    }
  }
}

// =============================================================================
// Todo 应用状态
// =============================================================================

struct TodoState {
  mut todos: List[Todo]
  mut filter: Filter
  mut input_text: String
  mut editing_id: Option[String]
}

impl TodoState {
  fn new() -> TodoState {
    {
      mut todos: [],
      mut filter: All,
      mut input_text: "",
      mut editing_id: None
    }
  }
  
  // 添加待办
  fn add_todo(self: TodoState) -> Unit {
    let trimmed = self.input_text.trim()
    if trimmed.length() > 0 {
      let todo = Todo::new(trimmed)
      self.todos.push(todo)
      self.input_text = ""
    }
  }
  
  // 删除待办
  fn remove_todo(self: TodoState, id: String) -> Unit {
    self.todos = self.todos.filter(fn(todo) { todo.id != id })
  }
  
  // 切换完成状态
  fn toggle_todo(self: TodoState, id: String) -> Unit {
    self.todos = self.todos.map(fn(todo) {
      if todo.id == id {
        todo.toggle()
      } else {
        todo
      }
    })
  }
  
  // 更新标题
  fn update_title(self: TodoState, id: String, new_title: String) -> Unit {
    self.todos = self.todos.map(fn(todo) {
      if todo.id == id {
        todo.set_title(new_title)
      } else {
        todo
      }
    })
  }
  
  // 清除已完成的
  fn clear_completed(self: TodoState) -> Unit {
    self.todos = self.todos.filter(fn(todo) { !todo.completed })
  }
  
  // 全部标记为完成/未完成
  fn toggle_all(self: TodoState, completed: Bool) -> Unit {
    self.todos = self.todos.map(fn(todo) {
      { ..todo, completed }
    })
  }
  
  // 获取过滤后的列表
  fn filtered_todos(self: TodoState) -> List[Todo] {
    self.todos.filter(fn(todo) { self.filter.matches(todo) })
  }
  
  // 获取统计信息
  fn stats(self: TodoState) -> (Int, Int, Int) {
    let total = self.todos.length()
    let completed = self.todos.filter(fn(t) { t.completed }).length()
    let active = total - completed
    (total, active, completed)
  }
  
  // 开始编辑
  fn start_editing(self: TodoState, id: String) -> Unit {
    self.editing_id = Some(id)
  }
  
  // 结束编辑
  fn stop_editing(self: TodoState) -> Unit {
    self.editing_id = None
  }
}

// =============================================================================
// Todo 视图组件
// =============================================================================

// 单个 Todo 项视图
fn TodoItemView(todo: Todo, state: TodoState) -> Widget {
  let is_editing = match state.editing_id {
    Some(id) => id == todo.id,
    None => false
  }
  
  Row::new([
    // 完成状态复选框
    Checkbox::new(todo.id, "", todo.completed)
      .on_change(fn(checked) {
        state.toggle_todo(todo.id)
      }),
    
    // 标题（可编辑）
    if is_editing {
      TextInput::new("edit_\{todo.id}", todo.title)
        .on_submit(fn(text) {
          if text.trim().length() > 0 {
            state.update_title(todo.id, text.trim())
          } else {
            state.remove_todo(todo.id)
          }
          state.stop_editing()
        })
        .on_blur(fn() {
          state.stop_editing()
        })
        .auto_focus(true)
    } else {
      Text::new(todo.title)
        .strikethrough(todo.completed)
        .color(
          if todo.completed {
            color(0.5, 0.5, 0.5, 1.0)
          } else {
            color(0.0, 0.0, 0.0, 1.0)
          }
        )
        .on_double_click(fn() {
          state.start_editing(todo.id)
        })
    },
    
    // 删除按钮
    IconButton::new("delete")
      .on_click(fn() {
        state.remove_todo(todo.id)
      })
      .color(color(0.8, 0.0, 0.0, 1.0))
  ])
  .gap(12.0)
  .padding(12.0)
  .background(
    if is_editing {
      color(0.95, 0.95, 1.0, 1.0)
    } else {
      color(1.0, 1.0, 1.0, 1.0)
    }
  )
  .border_radius(8.0)
}

// 过滤器按钮
fn FilterButton(label: String, active: Bool, on_click: () -> Unit) -> Widget {
  Button::new(label)
    .variant(if active { Primary } else { Ghost })
    .on_click(on_click)
    .font_size(14.0)
    .padding(8.0, 16.0)
}

// 统计信息视图
fn StatsView(total: Int, active: Int, completed: Int) -> Widget {
  Row::new([
    Text::new("\{active} items left")
      .font_size(14.0)
      .color(color(0.5, 0.5, 0.5, 1.0)),
    
    Text::new("•")
      .color(color(0.7, 0.7, 0.7, 1.0)),
    
    Text::new("\{completed} completed")
      .font_size(14.0)
      .color(color(0.5, 0.5, 0.5, 1.0)),
    
    Text::new("•")
      .color(color(0.7, 0.7, 0.7, 1.0)),
    
    Text::new("\{total} total")
      .font_size(14.0)
      .color(color(0.5, 0.5, 0.5, 1.0)),
  ])
  .gap(8.0)
}

// =============================================================================
// 主视图
// =============================================================================

fn TodoApp(state: TodoState) -> Widget {
  // 创建响应式状态
  let todos_state = State::new(state.todos)
  let filter_state = State::new(state.filter)
  let input_state = State::new(state.input_text)
  
  // 计算派生值
  let filtered_todos = computed(fn() {
    todos_state.get().filter(fn(todo) {
      filter_state.get().matches(todo)
    })
  })
  
  let stats = computed(fn() {
    let todos = todos_state.get()
    let total = todos.length()
    let completed = todos.filter(fn(t) { t.completed }).length()
    let active = total - completed
    (total, active, completed)
  })
  
  // 主视图
  Column::new([
    // 标题
    Heading::new("Todo List", 1)
      .text_align(Center)
      .padding(24.0),
    
    // 输入区域
    Row::new([
      TextInput::new("new_todo", "What needs to be done?")
        .value(input_state.get())
        .on_change(fn(text) {
          input_state.set(text)
        })
        .on_submit(fn(_) {
          let trimmed = input_state.get().trim()
          if trimmed.length() > 0 {
            let todo = Todo::new(trimmed)
            todos_state.update(fn(list) {
              let mut new_list = list
              new_list.push(todo)
              new_list
            })
            input_state.set("")
          }
        })
        .flex_grow(1.0)
        .padding(12.0),
      
      Button::new("Add")
        .on_click(fn() {
          let trimmed = input_state.get().trim()
          if trimmed.length() > 0 {
            let todo = Todo::new(trimmed)
            todos_state.update(fn(list) {
              let mut new_list = list
              new_list.push(todo)
              new_list
            })
            input_state.set("")
          }
        })
        .variant(Primary)
        .padding(12.0, 24.0),
    ])
    .gap(12.0)
    .padding(16.0),
    
    // 过滤器
    Row::new([
      FilterButton("All", filter_state.get() == All, fn() {
        filter_state.set(All)
      }),
      
      FilterButton("Active", filter_state.get() == Active, fn() {
        filter_state.set(Active)
      }),
      
      FilterButton("Completed", filter_state.get() == Completed, fn() {
        filter_state.set(Completed)
      }),
    ])
    .gap(8.0)
    .padding(8.0),
    
    // Todo 列表
    Column::new(
      filtered_todos.get().map(fn(todo) {
        TodoItemView(todo, state)
      })
    )
    .gap(8.0)
    .padding(16.0),
    
    // 底部操作栏
    Row::new([
      // 全选/取消全选
      Button::new("Toggle All")
        .on_click(fn() {
          let (total, _, completed) = stats.get()
          let all_completed = completed == total
          todos_state.update(fn(list) {
            list.map(fn(todo) {
              { ..todo, completed: !all_completed }
            })
          })
        })
        .variant(Ghost)
        .font_size(14.0),
      
      // 清除已完成
      Button::new("Clear Completed")
        .on_click(fn() {
          todos_state.update(fn(list) {
            list.filter(fn(todo) { !todo.completed })
          })
        })
        .variant(Ghost)
        .font_size(14.0),
    ])
    .gap(16.0)
    .padding(8.0),
    
    // 统计信息
    let (total, active, completed) = stats.get()
    StatsView(total, active, completed),
  ])
  .gap(16.0)
  .padding(24.0)
  .max_width(600.0)
}

// =============================================================================
// 主函数
// =============================================================================

pub fn main() -> Unit {
  // 创建应用状态
  let state = TodoState::new()
  
  // 添加一些示例数据
  state.todos.push(Todo::new("Learn MoonBit"))
  state.todos.push(Todo::new("Build a GUI app"))
  state.todos.push(Todo::new("Write tests"))
  
  // 创建视图
  let view = TodoApp(state)
  
  // 运行应用
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("Todo::new creates todo with correct defaults") {
  let todo = Todo::new("Test")
  
  assert!(todo.id.length() > 0)
  assert_eq!(todo.title, "Test")
  assert_eq!(todo.completed, false)
}

test("Todo::toggle changes completed state") {
  let todo = Todo::new("Test")
  
  let toggled = todo.toggle()
  assert_eq!(toggled.completed, true)
  
  let toggled_again = toggled.toggle()
  assert_eq!(toggled_again.completed, false)
}

test("TodoState::add_todo adds item to list") {
  let state = TodoState::new()
  state.input_text = "New task"
  
  state.add_todo()
  
  assert_eq!(state.todos.length(), 1)
  assert_eq!(state.todos[0].title, "New task")
  assert_eq!(state.input_text, "")
}

test("TodoState::remove_todo removes item") {
  let state = TodoState::new()
  state.todos.push(Todo::new("Task 1"))
  state.todos.push(Todo::new("Task 2"))
  
  let id = state.todos[0].id
  state.remove_todo(id)
  
  assert_eq!(state.todos.length(), 1)
  assert_eq!(state.todos[0].title, "Task 2")
}

test("TodoState::toggle_todo changes completed state") {
  let state = TodoState::new()
  state.todos.push(Todo::new("Task"))
  
  let id = state.todos[0].id
  state.toggle_todo(id)
  
  assert_eq!(state.todos[0].completed, true)
}

test("TodoState::clear_completed removes completed items") {
  let state = TodoState::new()
  state.todos.push(Todo::new("Task 1"))
  state.todos.push(Todo::new("Task 2"))
  state.todos[0].completed = true
  
  state.clear_completed()
  
  assert_eq!(state.todos.length(), 1)
  assert_eq!(state.todos[0].title, "Task 2")
}

test("TodoState::stats returns correct counts") {
  let state = TodoState::new()
  state.todos.push(Todo::new("Task 1"))
  state.todos.push(Todo::new("Task 2"))
  state.todos.push(Todo::new("Task 3"))
  state.todos[0].completed = true
  state.todos[1].completed = true
  
  let (total, active, completed) = state.stats()
  
  assert_eq!(total, 3)
  assert_eq!(active, 1)
  assert_eq!(completed, 2)
}

test("Filter::matches works correctly") {
  let todo_completed = { id: "1", title: "Done", completed: true, created_at: 0 }
  let todo_active = { id: "2", title: "Pending", completed: false, created_at: 0 }
  
  assert!(All.matches(todo_completed))
  assert!(All.matches(todo_active))
  
  assert!(Active.matches(todo_active))
  assert_eq!(Active.matches(todo_completed), false)
  
  assert!(Completed.matches(todo_completed))
  assert_eq!(Completed.matches(todo_active), false)
}

// =============================================================================
// 辅助函数
// =============================================================================

fn generate_id() -> String {
  let ts = @time.now_ms()
  let rand = @random.int(1000000)
  "todo_\{ts}_\{rand}"
}

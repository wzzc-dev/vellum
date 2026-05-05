# MoonBit GUI 转场动画系统

## 概述

MoonBit GUI 框架提供了完整的页面转场和动画系统，支持多种转场效果和动画类型。该系统参考了 Material Motion 和 iOS UIKit 的设计模式。

## 核心概念

### 转场类型

| 类型 | 说明 | 适用场景 |
|------|------|----------|
| **导航转场** | 页面之间的切换动画 | 页面导航 |
| **可见性转场** | 视图的进入/退出动画 | 列表项、弹窗 |
| **共享元素转场** | 元素在页面间的平滑过渡 | Hero 动画 |
| **交互式转场** | 可拖动的转场效果 | 滑动返回 |

---

## 导航转场

### 转场样式

#### 1. Push（推出）

iOS 风格，新页面从右侧滑入。

```moonbit
let config = default_push_config();
navigator.push_with_transition("settings", config);
```

**配置参数：**

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `duration_ms` | 300 | 动画持续时间（毫秒） |
| `interactive` | true | 是否支持交互式 |
| `curve` | EaseInOut | 动画曲线 |

#### 2. Modal（模态）

从底部向上推出，通常用于弹窗。

```moonbit
let config = default_modal_config();
navigator.push_with_transition("dialog", config);
```

**配置参数：**

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `duration_ms` | 350 | 动画持续时间（毫秒） |
| `interactive` | true | 是否支持交互式 |
| `curve` | EaseOut | 动画曲线 |

#### 3. Flip（翻转）

3D 翻转动画效果。

```moonbit
let style = flip_transition(config);
navigator.push_with_style("detail", style);
```

#### 4. Full Screen（全屏）

全屏覆盖效果，无滑动动画。

```moonbit
let style = TransitionStyle::FullScreen;
```

#### 5. Page Curl（翻页）

书本翻页效果。

```moonbit
let style = TransitionStyle::PageCurl;
```

---

## 可见性转场

用于容器中视图的进入和退出动画。

### 转场类型

#### 1. Fade（淡入淡出）

```moonbit
let fade = fade_transition();
```

#### 2. Slide（滑动）

```moonbit
slide_from_left()
slide_from_right()  
slide_from_top()
slide_from_bottom()
```

#### 3. Scale（缩放）

```moonbit
scale_transition()  // 从 0 缩放
```

#### 4. Offset（偏移）

```moonbit
offset_transition()  // 从偏移位置进入
```

### 使用示例

```moonbit
import from state {
    AnimatedContainerConfig,
    fade_transition,
    slide_from_bottom,
    default_animation_spec,
}

pub fn create_animated_container() {
    let config = AnimatedContainerConfig {
        insert_transition: slide_from_bottom(),
        remove_transition: fade_transition(),
        animation: default_animation_spec(),
    };
    
    AnimatedContainer {
        config: config,
        children: my_items,
    }
}
```

---

## 共享元素转场

实现类似 Flutter Hero 的共享元素动画效果。

### 基本用法

```moonbit
// 源页面 - 设置共享 ID
let thumbnail = Image {
    src: "photo.jpg",
    ..
} |> set_shared_id("hero-image-123");

// 目标页面 - 使用相同的共享 ID
let full_size = LargeImage {
    src: "photo.jpg",
    ..
} |> set_shared_id("hero-image-123");
```

### 配置选项

```moonbit
let options = default_shared_element_options();
```

**选项说明：**

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `duration_ms` | 300 | 动画持续时间 |
| `curve` | EaseInOut | 动画曲线 |
| `use_spring` | false | 是否使用弹簧动画 |
| `spring` | None | 弹簧配置 |

---

## 交互式转场

支持用户拖动来控制转场进度。

### 实现滑动返回

```moonbit
import from state {
    left_edge_pan,
    GestureState,
}

pub fn setup_swipe_back() {
    let edge_pan = left_edge_pan();
    
    add_gesture_recognizer(edge_pan, fn(_type, state, result) {
        match state {
            GestureState::Began => {
                // 开始交互式转场
                navigator.begin_interactive_pop();
            }
            GestureState::Changed => {
                // 更新转场进度
                let screen_width = get_screen_width();
                let progress = result.translation.x / screen_width;
                navigator.update_interactive_pop(progress);
            }
            GestureState::Ended => {
                // 完成或取消转场
                let should_complete = result.translation.x > 100.0;
                navigator.end_interactive_pop(should_complete);
            }
            GestureState::Cancelled => {
                navigator.end_interactive_pop(false);
            }
            _ => {}
        }
    });
}
```

---

## 转场协调器

`TransitionCoordinator` 负责管理转场的整个生命周期。

### 核心功能

1. **转场进度管理** - 控制动画进度（0.0 - 1.0）
2. **共享元素跟踪** - 追踪跨页面的共享元素
3. **动画状态同步** - 同步多个元素的动画状态
4. **转场完成/取消** - 处理转场的完成和取消

### API 参考

| 方法 | 说明 |
|------|------|
| `begin_transition(operation)` | 开始转场 |
| `add_element(share_id, config)` | 添加共享元素 |
| `complete_transition()` | 完成转场 |
| `cancel_transition()` | 取消转场 |
| `update_progress(progress)` | 更新转场进度 |

---

## 核心类型定义

### TransitionConfig（转场配置）

```moonbit
pub struct TransitionConfig {
    pub duration_ms: Int,        // 动画持续时间（毫秒）
    pub interactive: Bool,       // 是否支持交互式
    pub curve: AnimationCurve,   // 动画曲线
}
```

### TransitionStyle（转场样式）

```moonbit
pub enum TransitionStyle {
    Push,           // 推入式
    Modal,          // 模态式
    FullScreen,     // 全屏覆盖
    PageCurl,       // 翻页效果
    Flip,           // 翻转动画
    Custom,         // 自定义样式
}
```

### VisibilityTransition（可见性转场）

```moonbit
pub enum VisibilityTransition {
    Fade,           // 淡入淡出
    Slide,          // 滑动
    Scale,          // 缩放
    Offset,         // 偏移
}
```

### AnimatedContainerConfig（动画容器配置）

```moonbit
pub struct AnimatedContainerConfig {
    pub insert_transition: VisibilityTransition,  // 进入动画
    pub remove_transition: VisibilityTransition,  // 退出动画
    pub animation: AnimationSpec,                 // 动画规格
}
```

---

## 完整示例

### 带共享元素的页面导航

```moonbit
// 列表页面
pub fn list_page(albums: List<Album>) {
    Column {
        children: albums.map(fn(album) {
            // 为每个图片设置共享 ID
            let item = Row {
                children: [
                    Image {
                        src: album.thumbnail,
                        width: 80,
                        height: 80,
                    } |> set_shared_id("album-" + album.id),
                    Text { text: album.title },
                ]
            };
            
            // 点击时导航到详情页
            item.on_tap(fn() {
                let config = default_push_config();
                navigator.push_with_transition("detail/" + album.id, config);
            })
        })
    }
}

// 详情页面
pub fn detail_page(album: Album) {
    Column {
        children: [
            // 使用相同的共享 ID
            LargeImage {
                src: album.full_size,
                width: full_width(),
                height: 300,
            } |> set_shared_id("album-" + album.id),
            Text { text: album.title },
            Text { text: album.description },
        ]
    }
}
```

### 模态弹窗动画

```moonbit
pub fn show_modal_dialog() {
    let transition_config = default_modal_config();
    
    let dialog = Dialog {
        title: "确认操作",
        content: "确定要执行此操作吗？",
        actions: [
            Button::text("取消").on_tap(close_dialog),
            Button::new("确认").on_tap(confirm_action),
        ],
        transition: slide_from_bottom(),
    };
    
    navigator.push_with_transition("modal-dialog", transition_config);
}
```

---

## 性能优化建议

### 1. 避免重排

```moonbit
// 在动画期间避免修改布局属性
let animated_view = View {
    transform: translate(x, y),  // 使用 transform 而非 position
    ..
};
```

### 2. 使用快照

共享元素转场时使用 GPU 快照优化性能。

```moonbit
// 框架自动处理快照，无需手动操作
```

### 3. 降级策略

在性能受限的设备上自动降级到简单动画。

```moonbit
let config = if is_low_end_device() {
    TransitionConfig {
        duration_ms: 150,
        ..default_push_config()
    }
} else {
    default_push_config()
};
```

---

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| 转场中断 | 在 `Cancelled` 状态恢复原状态 |
| 共享元素未找到 | 回退到普通转场 |
| 动画冲突 | 使用 `TransitionCoordinator` 协调 |
| 内存警告 | 取消非关键动画 |

---

## API 参考

### 转场配置函数

| 函数 | 返回类型 | 说明 |
|------|----------|------|
| `default_push_config()` | `TransitionConfig` | 默认推入配置 |
| `default_modal_config()` | `TransitionConfig` | 默认模态配置 |

### 转场创建函数

| 函数 | 返回类型 | 说明 |
|------|----------|------|
| `push_transition(config)` | `TransitionStyle` | 创建推入转场 |
| `modal_transition(config)` | `TransitionStyle` | 创建模态转场 |
| `flip_transition(config)` | `TransitionStyle` | 创建翻转动场 |

### 可见性转场函数

| 函数 | 返回类型 | 说明 |
|------|----------|------|
| `fade_transition()` | `VisibilityTransition` | 创建淡入淡出 |
| `slide_transition()` | `VisibilityTransition` | 创建滑动动画 |
| `scale_transition()` | `VisibilityTransition` | 创建缩放动画 |
| `offset_transition()` | `VisibilityTransition` | 创建偏移动画 |
| `slide_from_left()` | `VisibilityTransition` | 从左侧滑入 |
| `slide_from_right()` | `VisibilityTransition` | 从右侧滑入 |
| `slide_from_top()` | `VisibilityTransition` | 从顶部滑入 |
| `slide_from_bottom()` | `VisibilityTransition` | 从底部滑入 |

### 共享元素函数

| 函数 | 返回类型 | 说明 |
|------|----------|------|
| `default_shared_element_options()` | `SharedElementOptions` | 默认共享元素选项 |
| `set_shared_id(view, id)` | `View` | 设置共享元素 ID |

---

更多信息参考 [手势系统文档](./gestures.md)

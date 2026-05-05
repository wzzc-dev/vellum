# MoonBit GUI 手势识别系统

## 概述

MoonBit GUI 框架提供了完整的手势识别系统，支持多种常用手势类型。该系统以 iOS UIKit 的手势识别器为模型，提供一致且强大的手势处理能力。

## 核心概念

### 手势状态生命周期

```
Possible → Began → Changed → Ended
           ↓
         Failed/Cancelled
```

| 状态 | 说明 |
|------|------|
| `Possible` | 等待触发，尚未识别到手势 |
| `Began` | 手势开始识别，首次检测到有效手势 |
| `Changed` | 手势状态更新，持续跟踪手势变化 |
| `Ended` | 手势成功完成 |
| `Failed` | 识别失败，无法识别为有效手势 |
| `Cancelled` | 系统取消，如被其他手势或系统事件中断 |

## 支持的手势类型

### 1. Tap（点击）

支持单指/多指点击，可配置点击次数。

**创建方式：**

```moonbit
// 单指单击
let single_tap = single_tap_gesture();

// 双指单击
let double_tap = double_tap_gesture();

// 自定义点击配置
let triple_tap = tap_gesture(3, 1);  // 3次点击，1个手指
```

**参数说明：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `taps_required` | `U32` | 要求的点击次数 |
| `touches_required` | `U32` | 要求的手指数量 |

---

### 2. Pan（拖拽/平移）

触摸滑动手势，支持配置最小识别距离。

**创建方式：**

```moonbit
// 最小识别距离为 5.0 像素
let pan = pan_gesture(5.0);
```

**参数说明：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `minimum_distance` | `Float` | 触发手势所需的最小移动距离（像素） |
| `maximum_distance` | `Option<Float>` | 可选的最大移动距离限制 |

**手势结果：**

```moonbit
// 在 Changed 状态中获取平移信息
match result.state {
    GestureState::Changed => {
        println!("平移: ({}, {})", result.translation.x, result.translation.y);
        println!("速度: ({}, {})", result.velocity.x, result.velocity.y);
    }
    _ => {}
}
```

---

### 3. Swipe（滑动）

快速滑动手势，支持四个方向。

**创建方式：**

```moonbit
// 从右向左滑动
let swipe_left = swipe_left(20.0);

// 从左向右滑动  
let swipe_right = swipe_right(15.0);

// 自定义方向
let swipe_up = swipe_gesture(SwipeDirection::Up, 20.0);
```

**参数说明：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `direction` | `SwipeDirection` | 滑动方向（Right/Left/Up/Down） |
| `minimum_distance` | `Float` | 触发滑动所需的最小距离 |

---

### 4. Long Press（长按）

长按手势，可配置持续时间和移动容忍度。

**创建方式：**

```moonbit
// 500ms 长按，允许 10 像素移动
let long_press = long_press_gesture(500, 10.0);
```

**参数说明：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `minimum_duration_ms` | `U32` | 触发长按所需的最小持续时间（毫秒） |
| `allow_movement` | `Float` | 长按期间允许的最大移动距离（像素） |

---

### 5. Pinch（捏合缩放）

双指捏合手势，用于缩放操作。

**创建方式：**

```moonbit
let pinch = pinch_gesture();
```

**手势结果：**

```moonbit
match result.state {
    GestureState::Changed => {
        if let Some(scale) = result.scale {
            println!("缩放比例: {}", scale);
        }
    }
    _ => {}
}
```

---

### 6. Rotation（旋转）

双指旋转手势，提供旋转角度。

**创建方式：**

```moonbit
let rotation = rotation_gesture();
```

**手势结果：**

```moonbit
match result.state {
    GestureState::Changed => {
        if let Some(angle) = result.rotation {
            println!("旋转角度: {}°", angle);
        }
    }
    _ => {}
}
```

---

### 7. Screen Edge Pan（边缘拖拽）

屏幕边缘识别，实现 iOS 风格的滑动返回。

**创建方式：**

```moonbit
// 左侧边缘
let left_edge = left_edge_pan();

// 右侧边缘
let right_edge = right_edge_pan();

// 自定义边缘
let top_edge = screen_edge_pan_gesture(Edge::Top);
```

---

## 核心类型定义

### TouchPoint（触摸点）

```moonbit
pub struct TouchPoint {
    pub id: U64,              // 触摸点唯一标识
    pub position: Point,       // 当前位置
    pub start_position: Point, // 起始位置
    pub timestamp_ms: U64,    // 时间戳（毫秒）
    pub force: Float,          // 触摸压力（0.0-1.0）
    pub radius: Float,         // 触摸半径
}
```

### GestureResult（手势结果）

```moonbit
pub struct GestureResult {
    pub gesture_type: GestureType,  // 手势类型
    pub state: GestureState,        // 当前状态
    pub touches: List<TouchPoint>,  // 当前触摸点列表
    pub velocity: Point,            // 当前速度（像素/秒）
    pub translation: Point,         // 累计平移量
    pub scale: Option<Float>,       // 缩放比例（Pinch）
    pub rotation: Option<Float>,    // 旋转角度（Rotation）
}
```

### Point（二维点）

```moonbit
pub struct Point {
    pub x: Float,
    pub y: Float,
}

pub impl Point {
    pub fn new(x: Float, y: Float) -> Self;
    pub fn zero() -> Self;  // 返回 (0, 0)
}
```

---

## 使用示例

### 完整示例：处理多种手势

```moonbit
import from state {
    GestureType,
    GestureState,
    GestureResult,
    GestureHandler,
    single_tap_gesture,
    pan_gesture,
    swipe_left,
    long_press_gesture,
}

// 定义手势处理器
pub fn create_gesture_handler() -> GestureHandler {
    fn handler(type: GestureType, state: GestureState, result: GestureResult) -> Unit {
        match state {
            GestureState::Began => {
                println!("手势开始: {:?}", type);
            }
            GestureState::Changed => {
                // 根据手势类型处理
                match type {
                    GestureType::Tap { .. } => {
                        // 点击手势通常不需要 Changed 处理
                    }
                    GestureType::Pan { .. } => {
                        println!("平移: ({}, {})", 
                            result.translation.x, 
                            result.translation.y);
                    }
                    GestureType::Pinch => {
                        if let Some(scale) = result.scale {
                            println!("缩放: {}", scale);
                        }
                    }
                    GestureType::Rotation => {
                        if let Some(angle) = result.rotation {
                            println!("旋转: {}°", angle);
                        }
                    }
                    _ => {}
                }
            }
            GestureState::Ended => {
                println!("手势完成");
            }
            GestureState::Failed => {
                println!("手势识别失败");
            }
            GestureState::Cancelled => {
                println!("手势被取消");
            }
            GestureState::Possible => {}
        }
    }
    handler
}

// 注册手势识别器
pub fn setup_gestures() {
    let handler = create_gesture_handler();
    
    // 注册点击手势
    add_gesture_recognizer(single_tap_gesture(), handler);
    
    // 注册拖拽手势
    add_gesture_recognizer(pan_gesture(5.0), handler);
    
    // 注册滑动手势
    add_gesture_recognizer(swipe_left(20.0), handler);
    
    // 注册长按手势
    add_gesture_recognizer(long_press_gesture(500, 10.0), handler);
}
```

### 交互式返回示例

```moonbit
import from state {
    left_edge_pan,
    GestureState,
}

pub fn setup_edge_swipe_back() {
    let edge_pan = left_edge_pan();
    
    add_gesture_recognizer(edge_pan, fn(_type, state, result) {
        match state {
            GestureState::Began => {
                navigator.begin_interactive_pop();
            }
            GestureState::Changed => {
                let screen_width = get_screen_width();
                let progress = result.translation.x / screen_width;
                navigator.update_interactive_pop(progress);
            }
            GestureState::Ended => {
                let should_complete = result.translation.x > 100.0;
                navigator.end_interactive_pop(should_complete);
            }
            GestureState::Cancelled | GestureState::Failed => {
                navigator.end_interactive_pop(false);
            }
            _ => {}
        }
    });
}
```

---

## 最佳实践

### 1. 手势优先级

- **Tap vs LongPress**: 长按需要等待，点击会先触发
- **Pan vs Swipe**: Swipe 是快速的短距离移动，Pan 是持续的拖动
- **Pinch vs Rotation**: 双指手势通常同时支持

### 2. 性能优化

```moonbit
// 避免在 Changed 回调中做复杂计算
fn gesture_handler(type, state, result) {
    if state == GestureState::Changed {
        // 只做必要的 UI 更新
        update_position(result.translation);
    }
}
```

### 3. 状态清理

```moonbit
fn gesture_handler(type, state, result) {
    match state {
        GestureState::Ended | GestureState::Failed | GestureState::Cancelled => {
            // 清理临时状态
            reset_animation_state();
        }
        _ => {}
    }
}
```

### 4. 多点触控

```moonbit
// 处理多个触摸点
fn handle_multi_touch(result: GestureResult) {
    for touch in result.touches {
        println!("触摸点 {}: ({}, {})", 
            touch.id, touch.position.x, touch.position.y);
    }
}
```

---

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| 手势识别失败 | 在 `Failed` 状态重置 UI |
| 手势被取消 | 在 `Cancelled` 状态回滚操作 |
| 触摸点丢失 | 检查 `touches` 列表是否为空 |
| 速度计算异常 | 使用 `velocity` 前检查是否合理 |

---

## API 参考

### 手势创建函数

| 函数 | 返回类型 | 说明 |
|------|----------|------|
| `tap_gesture(taps, touches)` | `GestureType` | 创建点击手势 |
| `single_tap_gesture()` | `GestureType` | 创建单指单击手势 |
| `double_tap_gesture()` | `GestureType` | 创建双击手势 |
| `pan_gesture(min_distance)` | `GestureType` | 创建拖拽手势 |
| `long_press_gesture(duration, movement)` | `GestureType` | 创建长按手势 |
| `swipe_gesture(direction, distance)` | `GestureType` | 创建滑动手势 |
| `swipe_right(distance)` | `GestureType` | 创建向右滑动 |
| `swipe_left(distance)` | `GestureType` | 创建向左滑动 |
| `pinch_gesture()` | `GestureType` | 创建捏合手势 |
| `rotation_gesture()` | `GestureType` | 创建旋转手势 |
| `screen_edge_pan_gesture(edge)` | `GestureType` | 创建边缘手势 |
| `left_edge_pan()` | `GestureType` | 创建左边缘手势 |
| `right_edge_pan()` | `GestureType` | 创建右边缘手势 |

### 辅助类型

| 类型 | 说明 |
|------|------|
| `GestureType` | 手势类型枚举 |
| `GestureState` | 手势状态枚举 |
| `GestureResult` | 手势识别结果 |
| `TouchPoint` | 触摸点信息 |
| `Point` | 二维坐标点 |
| `SwipeDirection` | 滑动方向 |
| `Edge` | 屏幕边缘 |
| `GestureHandler` | 手势处理回调类型 |

---

更多信息参考 [转场系统文档](./transitions.md)

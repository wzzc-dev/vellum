# MoonBit GUI 转场动画系统

## 概述

MoonBit GUI 框架提供了完整的页面转场和动画系统，包括：

- 页面导航转场
- 视图可见性动画
- 共享元素动画
- 交互式拖拽转场

## 转场样式

### Push（推出）
iOS 风格，新页面从右侧滑入

```moonbit
let config = default_push_config();
navigator.push_with_transition("settings", config);
```

### Modal（模态）
从底部向上推出

```moonbit
let config = default_modal_config();
navigator.push_with_transition("dialog", config);
```

### Flip（翻转）
3D 翻转动画

```moonbit
let style = flip_transition(config);
```

### Full Screen（全屏）
全屏覆盖效果

```moonbit
let style = TransitionStyle::FullScreen;
```

## 可见性转场

### Fade（淡入淡出）

```moonbit
let config = AnimatedContainerConfig {
    insert_transition: fade_transition(),
    remove_transition: fade_transition(),
    animation: my_anim_spec,
};
```

### Slide（滑动）

```moonbit
slide_from_left()
slide_from_right()
slide_from_top()
slide_from_bottom()
```

### Scale（缩放）

```moonbit
scale_transition(0.5) // 从 50% 缩放
```

### Offset（偏移）

```moonbit
offset_transition(-50.0, 0.0)
```

## 共享元素动画

### 配置共享元素

```moonbit
// 在源页面
let my_image = set_shared_id(
    Image { ... }, 
    "hero-image-123"
);

// 在目标页面
let large_image = set_shared_id(
    Image { ... }, 
    "hero-image-123"
);
```

### 配置选项

```moonbit
let shared_options = SharedElementOptions {
    duration_ms: 400,
    curve: AnimationCurve::Spring,
    use_spring: true,
    spring: Some(SpringConfig { ... }),
};
```

## 交互式转场

### 使用屏幕边缘手势触发转场

```moonbit
// 设置交互式弹出手势
let navigator = extended_navigator();
navigator.begin_interactive_pop();
navigator.update_interactive_pop(0.5); // 50%
navigator.end_interactive_pop(true); // 完成
```

## 使用示例

### MoonBit 扩展中使用

```moonbit
import from state {
    TransitionStyle,
    TransitionConfig,
    VisibilityTransition,
    default_push_config,
    slide_from_right,
    AnimatedContainerConfig,
}

// 页面导航
pub fn open_settings() {
    let config = default_push_config();
    nav_host.push_with_transition("settings", config);
}

// 容器动画
pub fn show_dialog() {
    let container_config = AnimatedContainerConfig {
        insert_transition: slide_from_bottom(),
        remove_transition: slide_from_bottom(),
        animation: my_animation,
    };
}
```

## 转场协调器

```
TransitionCoordinator 负责:
1. 转场进度管理
2. 共享元素跟踪
3. 动画状态同步
4. 转场完成/取消
```

### API 参考

```moonbit
begin_transition(operation)
add_element(share_id, config)
complete_transition()
cancel_transition()
update_progress(progress)
```

## 性能考虑

1. **转场时避免**：在动画期间避免重排
2. **快照使用**：共享元素使用截图优化
3. **GPU 渲染**：使用 GPUI 硬件加速
4. **降级策略**：复杂设备自动降级到简单动画

## 完整示例

### 带共享元素的导航

```moonbit
// 列表页面
let item = Image { ... } 
    |> set_shared_id("album-" + album_id);

// 点击时
let transition_config = default_push_config();
navigator.push_with_transition("detail", transition_config);

// 详情页面
let detail = LargeImage { ... } 
    |> set_shared_id("album-" + album_id);
```

### 交互式返回

```moonbit
// 使用屏幕边缘手势
let edge_pan = left_edge_pan();

add_gesture_recognizer(edge_pan, |result| {
    if result.state == GestureState::Changed {
        let progress = result.translation.x / screen_width;
        navigator.update_interactive_pop(progress);
    }
    
    if result.state == GestureState::Ended {
        let should_complete = result.translation.x > 100;
        navigator.end_interactive_pop(should_complete);
    }
});
```

---

更多信息参考 [手势系统文档](./gestures.md)

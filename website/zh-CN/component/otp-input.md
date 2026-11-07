---
title: OtpInput
description: 带多输入框、自动聚焦和粘贴处理的一次性验证码输入组件。
---

# OtpInput

OtpInput 是为一次性验证码（OTP）设计的输入组件，会以网格方式显示多个输入框，适合短信验证码、验证器 App 动态码以及 PIN 码输入场景。

## 导入

```rust
use gpui_kit::component::{
    Disableable as _, Sizable as _, button::Button,
    input::{OtpEvent, OtpInput, OtpState}, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
```

## 用法

### 基础 OTP 输入

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

OtpInput::new(&otp_state)
```

### 默认值

```rust
let otp_state = cx.new(|cx|
    OtpState::new(6, window, cx)
        .default_value("123456")
);

OtpInput::new(&otp_state)
```

### 掩码输入

```rust
let otp_state = cx.new(|cx|
    OtpState::new(6, window, cx)
        .masked(true)
        .default_value("123456")
);

OtpInput::new(&otp_state)
```

### 不同尺寸

```rust
OtpInput::new(&otp_state).small()
OtpInput::new(&otp_state)
OtpInput::new(&otp_state).large()
OtpInput::new(&otp_state).with_size(px(55.))
```

### 分组布局

```rust
OtpInput::new(&otp_state).groups(1)
OtpInput::new(&otp_state).groups(2)
OtpInput::new(&otp_state).groups(3)
```

### 禁用状态

```rust
OtpInput::new(&otp_state).disabled(true)
```

### 不同长度的验证码

```rust
let pin_state = cx.new(|cx| OtpState::new(4, window, cx));
OtpInput::new(&pin_state).groups(1)

let sms_state = cx.new(|cx| OtpState::new(6, window, cx));
OtpInput::new(&sms_state)

let auth_state = cx.new(|cx| OtpState::new(8, window, cx));
OtpInput::new(&auth_state).groups(2)
```

### 处理 OTP 事件

请把 `cx.subscribe` 返回的 `Subscription` 保存在视图中；一旦丢弃，订阅会立即取消。

```rust
struct OtpForm {
    otp_state: Entity<OtpState>,
    _subscriptions: Vec<Subscription>,
}

impl OtpForm {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

        let _subscriptions = vec![
            cx.subscribe(&otp_state, |this, state, event: &OtpEvent, cx| {
                match event {
                    OtpEvent::Change => {
                        // 每次编辑后触发。
                    }
                    OtpEvent::Complete => {
                        // 输入完最后一位时触发。
                        let code = state.read(cx).value().to_string();
                        this.verify_otp(&code, cx);
                    }
                    OtpEvent::Focus => println!("OTP input focused"),
                    OtpEvent::Blur => println!("OTP input lost focus"),
                }
            }),
        ];

        Self {
            otp_state,
            _subscriptions,
        }
    }

    fn verify_otp(&mut self, code: &str, cx: &mut Context<Self>) {
        println!("Complete OTP: {}", code);
        cx.notify();
    }
}
```

### 程序化控制

```rust
otp_state.update(cx, |state, cx| {
    state.set_value("123456", window, cx);
});

otp_state.update(cx, |state, cx| {
    state.set_masked(true, window, cx);
});

otp_state.update(cx, |state, cx| {
    state.focus(window, cx);
});

// 获取当前值（先克隆，之后才能再次可变借用 cx）
let current_value = otp_state.read(cx).value().to_string();
```

## API 参考

### OtpState

| 方法 | 说明 |
| ------------------------------ | -------------------------------------------- |
| `new(length, window, cx)` | 创建指定长度的 OTP 状态 |
| `default_value(str)` | 设置初始值 |
| `masked(bool)` | 开启掩码显示 |
| `set_value(str, window, cx)` | 以代码方式设置值 |
| `value()` | 获取当前值 |
| `set_masked(bool, window, cx)` | 切换掩码状态 |
| `focus(window, cx)` | 聚焦输入框 |
| `focus_handle(cx)` | 获取焦点句柄 |

### OtpInput

| 方法 | 说明 |
| ---------------- | ---------------------------------------- |
| `new(state)` | 使用状态实体创建 OTP 输入组件 |
| `groups(n)` | 设置可视分组数量，默认值为 2 |
| `disabled(bool)` | 设置禁用状态 |
| `small()` | 小尺寸 |
| `large()` | 大尺寸 |
| `with_size(px)` | 自定义单格尺寸 |

### OtpEvent

| 事件       | 说明                         |
| ---------- | ---------------------------- |
| `Change`   | 每次成功编辑后触发           |
| `Complete` | 输入完最后一位数字时触发     |
| `Focus`    | OTP 输入获得焦点             |
| `Blur`     | OTP 输入失去焦点             |

## 示例

### 短信验证码

```rust
struct SmsVerification {
    otp_state: Entity<OtpState>,
    phone_number: String,
    is_verifying: bool,
    _subscriptions: Vec<Subscription>,
}

impl SmsVerification {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

        let _subscriptions = vec![
            cx.subscribe(&otp_state, |this, state, event: &OtpEvent, cx| {
                if let OtpEvent::Complete = event {
                    let code = state.read(cx).value().to_string();
                    this.verify_sms_code(&code, cx);
                }
            }),
        ];

        Self {
            otp_state,
            phone_number: "+1234567890".to_string(),
            is_verifying: false,
            _subscriptions,
        }
    }

    fn verify_sms_code(&mut self, code: &str, cx: &mut Context<Self>) {
        self.is_verifying = true;
        // API call to verify SMS code
        println!("Verifying SMS code: {}", code);
        cx.notify();
    }
}

impl Render for SmsVerification {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(format!("Enter the 6-digit code sent to {}", self.phone_number))
            .child(OtpInput::new(&self.otp_state))
            .when(self.is_verifying, |this| {
                this.child("Verifying...")
            })
    }
}
```

### 双因素认证

```rust
struct TwoFactorAuth {
    otp_state: Entity<OtpState>,
    is_masked: bool,
}

impl TwoFactorAuth {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

        Self {
            otp_state,
            is_masked: true,
        }
    }

    fn toggle_visibility(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_masked = !self.is_masked;
        let masked = self.is_masked;
        self.otp_state.update(cx, |state, cx| {
            state.set_masked(masked, window, cx);
        });
    }
}

impl Render for TwoFactorAuth {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child("Enter your authenticator code")
            .child(OtpInput::new(&self.otp_state))
            .child(
                Button::new("toggle-visibility")
                    .label(if self.is_masked { "Show" } else { "Hide" })
                    .on_click(cx.listener(Self::toggle_visibility)),
            )
    }
}
```

### PIN 码输入

```rust
struct PinEntry {
    pin_state: Entity<OtpState>,
    attempts: usize,
    max_attempts: usize,
    _subscriptions: Vec<Subscription>,
}

impl PinEntry {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pin_state = cx.new(|cx| OtpState::new(4, window, cx).masked(true));

        let _subscriptions = vec![
            cx.subscribe_in(
                &pin_state,
                window,
                |this, state, event: &OtpEvent, window, cx| {
                    if let OtpEvent::Complete = event {
                        let pin = state.read(cx).value().to_string();
                        this.verify_pin(&pin, window, cx);
                    }
                },
            ),
        ];

        Self {
            pin_state,
            attempts: 0,
            max_attempts: 3,
            _subscriptions,
        }
    }

    fn verify_pin(
        &mut self,
        pin: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.attempts += 1;

        if pin == "1234" {
            println!("PIN verified successfully!");
        } else {
            println!("Incorrect PIN. Attempts: {}/{}", self.attempts, self.max_attempts);

            self.pin_state.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
        }

        cx.notify();
    }
}

impl Render for PinEntry {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_locked = self.attempts >= self.max_attempts;

        v_flex()
            .gap_4()
            .child("Enter your 4-digit PIN")
            .child(
                OtpInput::new(&self.pin_state)
                    .groups(1)
                    .disabled(is_locked)
            )
            .when(is_locked, |this| {
                this.child("Too many attempts. Please try again later.")
            })
            .when(self.attempts > 0 && !is_locked, |this| {
                this.child(format!(
                    "Incorrect PIN. {} attempts remaining.",
                    self.max_attempts - self.attempts
                ))
            })
    }
}
```

## 行为说明

### 输入处理

- **仅数字**：只接受 `0-9`。
- **自动聚焦**：输入数字后自动跳到下一个输入框。
- **退格**：删除当前数字并回到前一个输入框。
- **长度限制**：不会超过设定长度。
- **自动完成**：所有输入框填满后触发 `Complete` 事件。

`OtpState` 会发出 `OtpEvent`：

- 每次成功编辑后触发 `Change`
- 输入完最后一位数字时触发 `Complete`
- OTP 输入获得或失去焦点时触发 `Focus` / `Blur`

### 视觉反馈

- **焦点态**：当前输入框显示高亮边框与闪烁光标。
- **掩码**：启用后显示星号而不是数字。
- **分组**：可将输入框按组分隔，提升可读性。
- **禁用态**：禁用后显示灰化样式。

### 键盘导航

- **Tab**：将焦点移到下一个可聚焦元素；所有 OTP 格子共用一个焦点。
- **Shift+Tab**：将焦点移到上一个可聚焦元素。
- **Backspace**：删除最后一位数字。
- **方向键 / Delete**：`OtpInput` 不处理。

## 常见模式

### 输入完成后自动提交

```rust
// 请把返回的 Subscription 保存在视图中。
cx.subscribe(&otp_state, |this, state, event: &OtpEvent, cx| {
    if let OtpEvent::Complete = event {
        let code = state.read(cx).value().to_string();
        this.submit_verification_code(&code, cx);
    }
});
```

### 聚焦时清空旧值

需要在回调中使用 `window` 时，请改用 `subscribe_in`：

```rust
cx.subscribe_in(&otp_state, window, |this, state, event: &OtpEvent, window, cx| {
    if let OtpEvent::Focus = event {
        // 用户开始输入新验证码时清空旧值
        state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }
});
```

### 重发验证码计时器

```rust
struct OtpWithResend {
    otp_state: Entity<OtpState>,
    resend_timer: Option<Timer>,
    can_resend: bool,
}

// Implementation would include timer logic for resend functionality
```

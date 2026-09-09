---
title: OtpInput
description: 带多输入框、自动聚焦和粘贴处理的一次性验证码输入组件。
---

# OtpInput

OtpInput 是为一次性验证码（OTP）设计的输入组件，会以网格方式显示多个输入框，适合短信验证码、验证器 App 动态码以及 PIN 码输入场景。

## 导入

```rust
use gpui_kit::component::input::{OtpInput, OtpState};
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

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    match event {
        InputEvent::Change => {
            let code = state.read(cx).value();
            if code.len() == 6 {
                println!("Complete OTP: {}", code);
                this.verify_otp(&code, cx);
            }
        }
        InputEvent::Focus => println!("OTP input focused"),
        InputEvent::Blur => println!("OTP input lost focus"),
        _ => {}
    }
});
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

let current_value = otp_state.read(cx).value();
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

### InputEvent

| 事件 | 说明 |
| -------- | ------------------------------------------------- |
| `Change` | 所有数字输入完毕后触发 |
| `Focus` | 输入框获得焦点 |
| `Blur` | 输入框失去焦点 |

## 示例

### 短信验证码

```rust
struct SmsVerification {
    otp_state: Entity<OtpState>,
    phone_number: String,
    is_verifying: bool,
}

impl SmsVerification {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

        cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let code = state.read(cx).value();
                this.verify_sms_code(&code, cx);
            }
        });

        Self {
            otp_state,
            phone_number: "+1234567890".to_string(),
            is_verifying: false,
        }
    }

    fn verify_sms_code(&mut self, code: &str, cx: &mut Context<Self>) {
        self.is_verifying = true;
        println!("Verifying SMS code: {}", code);
        cx.notify();
    }
}

impl Render for SmsVerification {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let otp_state = cx.new(|cx|
            OtpState::new(6, window, cx)
                .masked(true)
        );

        Self {
            otp_state,
            is_masked: true,
        }
    }

    fn toggle_visibility(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_masked = !self.is_masked;
        self.otp_state.update(cx, |state, cx| {
            state.set_masked(self.is_masked, window, cx);
        });
    }
}

impl Render for TwoFactorAuth {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child("Enter your authenticator code")
            .child(OtpInput::new(&self.otp_state))
            .child(
                Button::new("toggle-visibility")
                    .label(if self.is_masked { "Show" } else { "Hide" })
                    .on_click(cx.listener(Self::toggle_visibility))
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
}

impl PinEntry {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let pin_state = cx.new(|cx|
            OtpState::new(4, window, cx)
                .masked(true)
        );

        cx.subscribe(&pin_state, |this, state, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let pin = state.read(cx).value();
                this.verify_pin(&pin, cx);
            }
        });

        Self {
            pin_state,
            attempts: 0,
            max_attempts: 3,
        }
    }

    fn verify_pin(&mut self, pin: &str, cx: &mut Context<Self>) {
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
- **自动完成**：所有输入框填满后触发 `Change` 事件。

### 视觉反馈

- **焦点态**：当前输入框显示高亮边框与闪烁光标。
- **掩码**：启用后显示星号而不是数字。
- **分组**：可将输入框按组分隔，提升可读性。
- **禁用态**：禁用后显示灰化样式。

### 键盘导航

- **方向键**：在输入框之间移动。
- **Tab**：切换到下一个可聚焦元素。
- **Shift+Tab**：切换到上一个可聚焦元素。
- **Backspace**：删除当前数字并向前移动。
- **Delete**：清空当前输入框。

## 常见模式

### 输入完成后自动提交

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if let InputEvent::Change = event {
        let code = state.read(cx).value();
        if code.len() == 6 {
            this.submit_verification_code(&code, cx);
        }
    }
});
```

### 聚焦时清空旧值

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if let InputEvent::Focus = event {
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

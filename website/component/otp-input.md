---
title: OtpInput
description: One-time password input component with multiple fields, auto-focus, and paste handling.
---

# OtpInput

A specialized input component for one-time passwords (OTP) that displays multiple input fields in a grid layout. Perfect for SMS verification codes, authenticator app codes, and other numeric verification scenarios.

## Import

```rust
use gpui_kit::component::input::{OtpInput, OtpState};
```

## Usage

### Basic OTP Input

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

OtpInput::new(&otp_state)
```

### With Default Value

```rust
let otp_state = cx.new(|cx|
    OtpState::new(6, window, cx)
        .default_value("123456")
);

OtpInput::new(&otp_state)
```

### Masked OTP Input

```rust
let otp_state = cx.new(|cx|
    OtpState::new(6, window, cx)
        .masked(true)
        .default_value("123456")
);

OtpInput::new(&otp_state)
```

### Different Sizes

```rust
// Small size
OtpInput::new(&otp_state).small()

// Medium size (default)
OtpInput::new(&otp_state)

// Large size
OtpInput::new(&otp_state).large()

// Custom size
OtpInput::new(&otp_state).with_size(px(55.))
```

### Grouped Layout

```rust
// Single group (all fields together)
OtpInput::new(&otp_state).groups(1)

// Two groups (default) - splits fields in half
OtpInput::new(&otp_state).groups(2)

// Three groups - splits fields into thirds
OtpInput::new(&otp_state).groups(3)
```

### Disabled State

```rust
OtpInput::new(&otp_state).disabled(true)
```

### Different Length Codes

```rust
// 4-digit PIN
let pin_state = cx.new(|cx| OtpState::new(4, window, cx));
OtpInput::new(&pin_state).groups(1)

// 6-digit SMS code (most common)
let sms_state = cx.new(|cx| OtpState::new(6, window, cx));
OtpInput::new(&sms_state)

// 8-digit authenticator code
let auth_state = cx.new(|cx| OtpState::new(8, window, cx));
OtpInput::new(&auth_state).groups(2)
```

### Handle OTP Events

```rust
let otp_state = cx.new(|cx| OtpState::new(6, window, cx));

cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    match event {
        InputEvent::Change => {
            let code = state.read(cx).value();
            if code.len() == 6 {
                println!("Complete OTP: {}", code);
                // Automatically submit when complete
                this.verify_otp(&code, cx);
            }
        }
        InputEvent::Focus => println!("OTP input focused"),
        InputEvent::Blur => println!("OTP input lost focus"),
        _ => {}
    }
});
```

### Programmatic Control

```rust
// Set value programmatically
otp_state.update(cx, |state, cx| {
    state.set_value("123456", window, cx);
});

// Toggle masking
otp_state.update(cx, |state, cx| {
    state.set_masked(true, window, cx);
});

// Focus the input
otp_state.update(cx, |state, cx| {
    state.focus(window, cx);
});

// Get current value
let current_value = otp_state.read(cx).value();
```

## API Reference

### OtpState

| Method                         | Description                                  |
| ------------------------------ | -------------------------------------------- |
| `new(length, window, cx)`      | Create a new OTP state with specified length |
| `default_value(str)`           | Set initial value                            |
| `masked(bool)`                 | Enable masked display (shows asterisks)      |
| `set_value(str, window, cx)`   | Set OTP value programmatically               |
| `value()`                      | Get current OTP value                        |
| `set_masked(bool, window, cx)` | Toggle masked display                        |
| `focus(window, cx)`            | Focus the OTP input                          |
| `focus_handle(cx)`             | Get focus handle                             |

### OtpInput

| Method           | Description                              |
| ---------------- | ---------------------------------------- |
| `new(state)`     | Create OTP input with state entity       |
| `groups(n)`      | Set number of visual groups (default: 2) |
| `disabled(bool)` | Set disabled state                       |
| `small()`        | Small size (6x6 px fields)               |
| `large()`        | Large size (11x11 px fields)             |
| `with_size(px)`  | Custom field size                        |

### InputEvent

| Event    | Description                                       |
| -------- | ------------------------------------------------- |
| `Change` | Emitted when OTP is complete (all digits entered) |
| `Focus`  | Input received focus                              |
| `Blur`   | Input lost focus                                  |

## Examples

### SMS Verification

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
        // API call to verify SMS code
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

### Two-Factor Authentication

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

### PIN Entry

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

        // Simulate PIN verification
        if pin == "1234" {
            println!("PIN verified successfully!");
        } else {
            println!("Incorrect PIN. Attempts: {}/{}", self.attempts, self.max_attempts);

            // Clear PIN on incorrect attempt
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

## Behavior

### Input Handling

- **Numeric Only**: Accepts only digits (0-9)
- **Auto-Focus**: Automatically moves to next field when digit is entered
- **Backspace**: Removes current digit and moves to previous field
- **Length Limit**: Prevents input beyond specified length
- **Auto-Complete**: Emits `Change` event when all fields are filled

### Visual Feedback

- **Focus Indicator**: Blue border and blinking cursor on active field
- **Masking**: Shows asterisk icons instead of numbers when enabled
- **Grouping**: Visual separation of fields into groups for better readability
- **Disabled State**: Grayed out appearance when disabled

### Keyboard Navigation

- **Arrow Keys**: Navigate between fields
- **Tab**: Move to next focusable element
- **Shift+Tab**: Move to previous focusable element
- **Backspace**: Delete current digit and move backward
- **Delete**: Clear current field

## Common Patterns

### Auto-Submit on Complete

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if let InputEvent::Change = event {
        let code = state.read(cx).value();
        if code.len() == 6 {
            // Auto-submit when complete
            this.submit_verification_code(&code, cx);
        }
    }
});
```

### Clear on Focus

```rust
cx.subscribe(&otp_state, |this, state, event: &InputEvent, cx| {
    if let InputEvent::Focus = event {
        // Clear previous value when user starts entering new code
        state.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }
});
```

### Resend Code Timer

```rust
struct OtpWithResend {
    otp_state: Entity<OtpState>,
    resend_timer: Option<Timer>,
    can_resend: bool,
}

// Implementation would include timer logic for resend functionality
```

---
title: Questionnaire
description: 支持单选、多选、自由输入、校验和导航的可组合多步骤问卷。
---

# Questionnaire

`Questionnaire` 引导用户完成一组有序问题。它负责当前 item、答案状态、校验、进度和导航。外层页面、`GroupBox`、`Dialog` 或 `Sheet` 负责关闭、取消、持久化、传输以及应用特有的条件分支。

## 引入

```rust
use gpui_component::questionnaire::{
    Questionnaire, QuestionnaireActions, QuestionnaireChoice,
    QuestionnaireChoices, QuestionnaireDescription, QuestionnaireError,
    QuestionnaireInput, QuestionnaireItem, QuestionnaireNext,
    QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip,
    QuestionnaireState, QuestionnaireSubmit, QuestionnaireTitle,
};
```

## 用法

先创建一次 item 集合，并使用一个 `QuestionnaireState` entity 作为所有部件的状态源。

```rust
use gpui_component::questionnaire::{
    QuestionnaireItemDefinition, QuestionnaireChoiceDefinition,
    QuestionnaireInputDefinition, QuestionnaireState, QuestionnaireAnswer,
    QuestionnaireEvent,
};
use gpui_component::input::InputState;

let direction_input = cx.new(|cx| {
    InputState::new(window, cx).placeholder("Type another answer…")
});

let items = vec![
    QuestionnaireItemDefinition::new("direction", "What should we prototype next?")
        .with_required(true)
        .with_description("Choose a direction or write your own.")
        .with_choices([
            QuestionnaireChoiceDefinition::new("delegation", "Delegation")
                .with_description("Show how work moves to a specialist."),
            QuestionnaireChoiceDefinition::new("questions", "Question prompts"),
            QuestionnaireChoiceDefinition::new("both", "Both together"),
        ])
        .with_input(QuestionnaireInputDefinition::new(
            direction_input,
            "Another answer",
        )),
    QuestionnaireItemDefinition::new("detail", "How much detail should it include?")
        .with_description("You can skip this question if you are not sure yet.")
        .with_choices([
            QuestionnaireChoiceDefinition::new("focused", "Focused"),
            QuestionnaireChoiceDefinition::new("complete", "Complete flow"),
        ]),
];

let state = cx.new(|cx| {
    QuestionnaireState::new(items, cx)
        .expect("valid questionnaire schema")
});
```

将同一个集合映射为组合部件。每个部件都保持足够小，应用可以替换标题、选项内容、进度指示器或操作按钮，同时继续使用 Questionnaire 的状态。

```rust
Questionnaire::new(&state)
    .child(QuestionnaireProgress::new(&state))
    .child(
        QuestionnaireItem::new(&state, "direction")
            .child(QuestionnaireTitle::new(&state, "direction"))
            .child(QuestionnaireDescription::new(&state, "direction"))
            .child(
                QuestionnaireChoices::new(&state, "direction")
                    .child(QuestionnaireChoice::new(&state, "direction", "delegation"))
                    .child(QuestionnaireChoice::new(&state, "direction", "questions"))
                    .child(QuestionnaireChoice::new(&state, "direction", "both"))
                    .child(QuestionnaireInput::new(&state, "direction")),
            )
            .child(QuestionnaireError::new(&state, "direction")),
    )
    .child(
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    )
```

`Questionnaire` 只渲染当前 item；其他 item 仍保留在有序 schema 中，并参与导航和最终校验。

## 组合结构

```text
Questionnaire
├── QuestionnaireProgress
├── QuestionnaireItem
│   ├── QuestionnaireTitle
│   ├── QuestionnaireDescription
│   ├── QuestionnaireChoices
│   │   ├── QuestionnaireChoice
│   │   └── QuestionnaireInput
│   └── QuestionnaireError
└── QuestionnaireActions
    ├── QuestionnairePrevious
    ├── QuestionnaireSkip
    ├── QuestionnaireNext
    └── QuestionnaireSubmit
```

所有部件都接受普通 GPUI 样式，并可以与现有的 `Button`、`Input`、`Radio`、`Checkbox`、`Progress`、`Stepper`、`GroupBox` 和 `Dialog` 组合。自定义部件应读取对应 state 并调用 state 方法处理用户操作，不要复制答案状态。

## 单选

item 默认使用单选模式。激活某个选项后 item 即有答案，`Next` 可以继续。单选 item 也可以提供自由输入；固定选项和自由答案互斥，但用户切换选择时会保留输入草稿。

```rust
let plan_input = cx.new(|cx| InputState::new(window, cx));
let item = QuestionnaireItemDefinition::new("plan", "Which plan fits your team?")
    .with_choices([
        QuestionnaireChoiceDefinition::new("plus", "Plus"),
        QuestionnaireChoiceDefinition::new("pro", "Pro"),
    ])
    .with_input(QuestionnaireInputDefinition::new(plan_input, "Another plan"));
```

## 多选

当一个 item 可以接受多个固定答案时设置 `multiple`。

```rust
let tools_input = cx.new(|cx| InputState::new(window, cx));
let item = QuestionnaireItemDefinition::new("tools", "Which tools do you use?")
    .with_multiple(true)
    .with_choices([
        QuestionnaireChoiceDefinition::new("editor", "Editor"),
        QuestionnaireChoiceDefinition::new("terminal", "Terminal"),
        QuestionnaireChoiceDefinition::new("browser", "Browser"),
    ])
    .with_input(QuestionnaireInputDefinition::new(tools_input, "Something else"));
```

多选 item 可以同时提交多个固定选项和非空自由输入。答案读取器按 schema 顺序返回结果。即使恢复的数据包含 disabled choice，其值也不会进入答案。

## 自由输入

加入 `QuestionnaireInputDefinition`，允许用户输入固定选项之外的答案。请为输入提供可访问名称；placeholder 不能替代 label。

```rust
let feedback_input = cx.new(|cx| {
    InputState::new(window, cx).placeholder("Tell us what would help…")
});
let item = QuestionnaireItemDefinition::new("feedback", "What should we improve?")
    .with_input(QuestionnaireInputDefinition::new(
        feedback_input,
        "Your suggestion",
    ));
```

只有空白的输入视为未回答。选择固定选项时会保留输入草稿，但只有自由输入成为当前答案时才会提交它。

## 显式跳过

可选 item 可以显示 `QuestionnaireSkip`。跳过是一个明确且有效的状态，会清除该 item 的答案并允许 `Next` 继续。必填 item 不允许跳过。重新进入 item 并选择答案后，skipped 状态会被清除。

```rust
let optional = QuestionnaireItemDefinition::new("tone", "What tone should we use?")
    .with_required(false)
    .with_choices([
        QuestionnaireChoiceDefinition::new("direct", "Direct"),
        QuestionnaireChoiceDefinition::new("warm", "Warm"),
    ]);
```

## 导航与状态

`QuestionnaireState` 暴露当前 item、有序 item 状态和导航状态，可用于自定义操作布局。

```rust
let current = state.read(cx).current_item();
let progress = state.read(cx).progress();
let status = state
    .read(cx)
    .item_state("direction")
    .map(|item| item.status());
let navigation = state.read(cx).navigation_state();
let can_confirm = navigation.is_confirmable();
let show_previous = navigation.is_previous_visible();
let show_next = navigation.is_next_visible();
let show_skip = navigation.is_skip_visible();
let show_submit = navigation.is_submit_visible();

state.update(cx, |state, cx| {
    state.go_previous(window, cx);
    state.go_next(window, cx);
});
```

默认操作布局在开头显示 `Previous`，在 item 之间显示 `Next`，当前 item 可选时显示 `Skip`，最后显示 `Submit`。隐藏的操作不会进入键盘导航。disabled item 会从导航和进度总数中排除。

## 校验

必填状态校验已经内置。可以为 item 添加同步 validator，实现领域规则。`Next` 校验当前 item；`Submit` 校验全部 enabled item，并将焦点移到第一个无效 item。

```rust
let item = QuestionnaireItemDefinition::new("handle", "Choose a handle")
    .with_required(true)
    .with_validator(|context| {
        if context
            .answer()
            .freeform()
            .is_some_and(|value| value.as_ref().len() >= 3)
        {
            Ok(())
        } else {
            Err("Use at least three characters.".into())
        }
    });
```

应用可以使用 `set_external_error` 显示外部 schema 或服务器错误，在数据修正后由 owner 清除。Reset 会恢复默认值并清除内部校验状态；owner 管理的 external error 仍由应用控制。

```rust
state.update(cx, |state, cx| {
    state
        .set_external_error("handle", "This handle is already taken.", cx)
        .expect("known questionnaire item");
});
```

## 受控状态、恢复与重置

当页面需要控制当前 item 或恢复已保存草稿时，使用 state reader 和静默 setter。静默 setter 会更新 UI，但不会发出用户交互事件。

```rust
state.update(cx, |state, cx| {
    state
        .set_current_item("detail", window, cx)
        .expect("known enabled questionnaire item");
    state.set_answer(
        "direction",
        QuestionnaireAnswer::new().with_choices(["delegation"]),
        window,
        cx,
    ).expect("known questionnaire item");
    state.reset(window, cx);
});
```

如果前面的答案使某个 item 不适用，可以设置 disabled。disabled item 不计入进度、校验、焦点或提交。

```rust
state.update(cx, |state, cx| {
    state
        .set_item_disabled("advanced", true, window, cx)
        .expect("known questionnaire item");
});
```

## 键盘快捷键

为 state 启用字母或数字快捷键。快捷键只作用于当前 item 的 enabled choices。重复 key event、文本输入、IME 组合以及带修饰键的按键都会保持原有行为。

```rust
use gpui_component::questionnaire::QuestionnaireShortcutMode;

let state = cx.new(|cx| {
    QuestionnaireState::new(items, cx)
        .expect("valid questionnaire schema")
        .with_shortcuts(QuestionnaireShortcutMode::Letters)
});
```

Questionnaire 在单选 radio group 内处理四个方向键，将焦点移到下一个 enabled choice 并选中它。Up/Down 在其他场景下按 schema 顺序在 checkbox、choice 和自由输入控件之间移动；只有焦点不在文本输入或 radio 控件上时，Left/Right 才会在 item 之间移动。Enter 确认已填写的答案，Command/Ctrl+Enter 确认当前 item。空答案不会隐式提交问卷。

## 进度和自定义渲染

`QuestionnaireProgress` 使用 docs 默认的 “Question 2 of 4” 样式。也可以读取 progress state，使用现有 `Progress` 或 `Stepper` 组合自定义指示器。

```rust
QuestionnaireProgress::new(&state)
    .with_size(Size::Small)

let progress = state.read(cx).progress();
let percent = if progress.total() == 0 {
    0.
} else {
    progress.current() as f32 / progress.total() as f32 * 100.
};
Progress::new("questionnaire-progress").value(percent)

Stepper::new("questionnaire-steps")
    .selected_index(progress.current().saturating_sub(1))
```

## 尺寸与主题

Questionnaire 部件实现与其他组件相同的 `Sizable` 契约。默认尺寸为 `Medium`，并遵循 shadcn/ui `base-nova` docs 外观。

```rust
use gpui_component::{Sizable as _, Size};

Questionnaire::new(&state).with_size(Size::Small)
Questionnaire::new(&state).with_size(Size::Medium)
Questionnaire::new(&state).with_size(Size::Large)
```

默认皮肤从当前主题的 semantic tokens 派生 spacing、typography、radius、border、input、primary、muted、destructive 和 focus-ring。局部调整可以使用 `Styled` 方法或 `StyleRefinement`，不需要增加 Questionnaire 专属颜色常量。

## Card 和 Dialog 组合

Questionnaire 负责问题流程；卡片或 dialog 负责容器布局以及关闭、取消行为。

```rust
GroupBox::new()
    .outline()
    .title("Set up your workspace")
    .child(Questionnaire::new(&state))
```

对于 dialog，可以在现有 Dialog 内容中创建 Questionnaire，并由宿主处理关闭。Questionnaire 的 `Submit` event 适合把已校验的 `QuestionnaireSubmission` 交给应用传输层。

## Event 与提交

订阅 `QuestionnaireEvent`，即可监听当前 item 变化、答案变化、完成和成功提交。`Completed` 只在状态首次转为 complete 时发出；每次成功执行显式 submit 都会发出 `Submit`。

```rust
cx.subscribe(&state, |_, _, event, _| match event {
    QuestionnaireEvent::CurrentItemChanged { current, .. } => {
        println!("Current item: {:?}", current);
    }
    QuestionnaireEvent::AnswerChanged(change) => {
        println!("Changed: {:?}", change.item());
    }
    QuestionnaireEvent::Completed(submission)
    | QuestionnaireEvent::Submit(submission) => {
        println!("Answers: {:?}", submission.items());
    }
    _ => {}
});
```

提交结果按 item schema 顺序排列，并且只包含 enabled item。它表示本地已校验的提交请求；远程保存仍由宿主应用负责。

## 可访问性

`Questionnaire` 根部使用 GPUI 的 `Form` role。`QuestionnaireItem` 是带有
item label 和 description 的可访问分组。definition 中的
`accessibility_label` 与 `description` 始终是 item 和 choice 的语义来源；
自定义 child 只替换可见的 fallback 内容，并保留 Questionnaire parts 提供的
状态、role、焦点行为和语义。`QuestionnaireError` 只有 item 无效时才会以
alert 形式播报。Choice 保留 radio 和 checkbox 语义，进度暴露当前值与总数，
导航使用真实按钮。

非当前 item 和隐藏操作不会进入键盘导航。成功切换后，焦点移动到新的当前 item；校验失败时，焦点优先移动到已选或已填写的答案控件，再退回第一个可用控件。

请始终为自由输入在 definition 中提供 `accessibility_label`；可见 label 或
等价的自定义组合可以补充它。GPUI accessibility layer 没有直接对应
`aria-invalid` 的 builder；Questionnaire 仍通过错误 alert、语义分组状态、焦点
行为和 destructive 样式暴露无效状态。

## API 参考

- [Questionnaire]
- [QuestionnaireState]
- [QuestionnaireItemDefinition]
- [QuestionnaireChoiceDefinition]
- [QuestionnaireInputDefinition]
- [QuestionnaireProgress]
- [QuestionnaireItem]
- [QuestionnaireChoice]
- [QuestionnaireInput]
- [QuestionnaireActions]
- [QuestionnaireEvent]
- [QuestionnaireSubmission]
- [Sizable]

[Questionnaire]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.Questionnaire.html
[QuestionnaireState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireState.html
[QuestionnaireItemDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemDefinition.html
[QuestionnaireChoiceDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDefinition.html
[QuestionnaireInputDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInputDefinition.html
[QuestionnaireProgress]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgress.html
[QuestionnaireItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItem.html
[QuestionnaireChoice]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoice.html
[QuestionnaireInput]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInput.html
[QuestionnaireActions]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireActions.html
[QuestionnaireEvent]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireEvent.html
[QuestionnaireSubmission]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmission.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

---
title: Questionnaire
description: 支持单选、多选、自由输入、校验和导航的可组合多步骤问卷。
---

# Questionnaire

`Questionnaire` 引导用户完成一组有序问题。它负责当前 item、答案状态、校验、
进度和导航。外层页面、`GroupBox`、`Dialog` 或 `Sheet` 负责关闭、取消、持久化、
传输以及应用特有的条件分支。

## 引入

```rust
use gpui_component::questionnaire::{
    Questionnaire, QuestionnaireActions, QuestionnaireChoice,
    QuestionnaireChoiceDescription, QuestionnaireChoices, QuestionnaireDescription,
    QuestionnaireError, QuestionnaireInput, QuestionnaireItem, QuestionnaireNext,
    QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip, QuestionnaireState,
    QuestionnaireSubmit, QuestionnaireTitle,
};
```

## 用法

先创建一次 item 集合，并使用一个 `QuestionnaireState` entity 作为所有部件的
状态源。

```rust
use gpui_component::input::InputState;
use gpui_component::questionnaire::{
    QuestionnaireChoiceDefinition, QuestionnaireInputDefinition,
    QuestionnaireItemDefinition, QuestionnaireState,
};

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

将集合中的每个 definition 都映射为组合部件。`QuestionnaireItem` 只渲染当前
item；如果组合中遗漏某个 item，导航到该 item 时界面会为空。

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
        QuestionnaireItem::new(&state, "detail")
            .child(QuestionnaireTitle::new(&state, "detail"))
            .child(QuestionnaireDescription::new(&state, "detail"))
            .child(
                QuestionnaireChoices::new(&state, "detail")
                    .child(QuestionnaireChoice::new(&state, "detail", "focused"))
                    .child(QuestionnaireChoice::new(&state, "detail", "complete")),
            )
            .child(QuestionnaireError::new(&state, "detail")),
    )
    .child(
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    )
```

## 组合结构

```text
Questionnaire
├── QuestionnaireProgress
├── QuestionnaireItem
│   ├── QuestionnaireTitle
│   ├── QuestionnaireDescription
│   ├── QuestionnaireChoices
│   │   ├── QuestionnaireChoice
│   │   │   └── QuestionnaireChoiceDescription (custom child)
│   │   └── QuestionnaireInput
│   └── QuestionnaireError
└── QuestionnaireActions
    ├── QuestionnairePrevious
    ├── QuestionnaireSkip
    ├── QuestionnaireNext
    └── QuestionnaireSubmit
```

所有部件都接受普通 GPUI 样式，并可以与现有的 `Button`、`Input`、`Radio`、
`Checkbox`、`Progress`、`Stepper`、`GroupBox` 和 `Dialog` 组合。将同一个 state
entity 传给每个部件。自定义部件应读取对应 state 并调用 state 方法处理用户操作，
不要创建第二份答案存储。

`QuestionnaireChoice` 默认提供 indicator、content 和 shortcut。加入 child 后，
它会替换 fallback label 与 description，同时保留选项激活、焦点、状态和可访问
行为。使用 `QuestionnaireChoiceDescription::new()` 为自定义 choice body 添加
辅助文字。下面这些 seam 只定制对应区域：

```rust
use gpui::{IntoElement as _, ParentElement as _, StyleRefinement, Styled as _, div};
use gpui_component::{ActiveTheme as _, StyledExt as _};
use gpui_component::questionnaire::{
    QuestionnaireChoice, QuestionnaireChoiceDescription,
};

let _styled_choice = QuestionnaireChoice::new(&state, "direction", "questions")
    .indicator_style(StyleRefinement::default().opacity(0.9))
    .content_style(StyleRefinement::default().opacity(0.95))
    .shortcut_style(StyleRefinement::default().opacity(0.8));

let _rendered_choice = QuestionnaireChoice::new(&state, "direction", "delegation")
    .render_indicator(|choice, _, cx| {
        div()
            .size_4()
            .rounded_full()
            .bg(if choice.is_selected() {
                cx.theme().primary
            } else {
                cx.theme().muted
            })
            .into_any_element()
    })
    .child(
        div()
            .child("Delegation")
            .child(QuestionnaireChoiceDescription::new().child(
                "Show how work moves to a specialist.",
            )),
    );
```

`render_shortcut` 使用相同的 renderer 签名，并接收
`QuestionnaireChoiceState`；需要替换默认 `Kbd` 提示时使用它。renderer 会完整替换
对应区域，因此同一区域的 style seam 不再应用；请直接设置自定义 renderer 的样式。
状态快照提供 `is_selected`、`is_disabled`、`is_invalid` 和 `shortcut`，可用于自定义
渲染。

## 单选

item 默认使用单选模式。激活某个选项后 item 即有答案，`Next` 可以继续。单选
item 也可以提供自由输入；固定选项和自由答案互斥，但用户切换选择时会保留输入
草稿。

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

当一个 item 可以接受多个固定答案时设置 `multiple`。非空自由输入可以和已选
固定选项一起提交。

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

答案 reader 按 schema 顺序保留结果。如果已选 choice 后续被禁用，它会从 effective
answer 中排除。

## 自由输入

加入 `QuestionnaireInputDefinition`，允许用户输入固定选项之外的答案。请为输入
提供可访问名称；placeholder 不能替代 label。

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

只有空白的输入视为未回答。选择固定选项时会保留输入草稿，但只有自由输入成为
当前答案时才会提交它。多选 item 可以同时提交固定选项和非空自由输入。

## 显式跳过

可选 item 可以显示 `QuestionnaireSkip`。跳过是一个明确且有效的状态，会清除该
item 的答案并允许 `Next` 继续。必填 item 不允许跳过。重新进入 item 并选择答案
后，skipped 状态会被清除。跳过最后一个 enabled item 后，会在记录跳过状态后请求
提交。

```rust
let optional = QuestionnaireItemDefinition::new("tone", "What tone should we use?")
    .with_required(false)
    .with_choices([
        QuestionnaireChoiceDefinition::new("direct", "Direct"),
        QuestionnaireChoiceDefinition::new("warm", "Warm"),
    ]);
```

## 默认值与禁用控件

使用 definition builder 设置初始快照。choice 可以初始选中，item 或 choice 可以
初始禁用，input 也可以初始禁用。单选 item 最多只能有一个默认选中的 choice。

```rust
let saved_input = cx.new(|cx| InputState::new(window, cx).default_value("Saved draft"));
let item = QuestionnaireItemDefinition::new("workspace", "Which workspaces?")
    .with_multiple(true)
    .with_choices([
        QuestionnaireChoiceDefinition::new("personal", "Personal")
            .with_default_selected(true),
        QuestionnaireChoiceDefinition::new("team", "Team")
            .with_disabled(true),
    ])
    .with_input(
        QuestionnaireInputDefinition::new(saved_input, "Another workspace")
            .with_disabled(false),
    );
let disabled_item = QuestionnaireItemDefinition::new(
    "advanced",
    "Advanced preferences",
)
    .with_disabled(true);
let disabled_input = cx.new(|cx| InputState::new(window, cx));
let disabled_input_definition = QuestionnaireInputDefinition::new(
    disabled_input,
    "Disabled answer",
)
    .with_disabled(true);
```

`with_default_selected` 属于 `QuestionnaireChoiceDefinition`；item 使用
`with_disabled`，input 使用 `QuestionnaireInputDefinition::with_disabled`。如果要
让 item 初始禁用，使用 `QuestionnaireItemDefinition::with_disabled(true)`。

`QuestionnaireState::new` 会拒绝重复 item name、同一 item 中重复的 choice value，
以及单选 item 的多个默认值。针对未知 item 或 choice 的 setter 会返回
`QuestionnaireStateError`。

## 导航与状态

`QuestionnaireState` 暴露当前 item、有序 item 状态和导航状态，可用于自定义操作
布局。

```rust
let current = state.read(cx).current_item();
let current_ix = state.read(cx).current_ix();
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
```

默认操作布局在开头显示 `Previous`，在 item 之间显示 `Next`，当前 item 可选时
显示 `Skip`，最后显示 `Submit`。隐藏的操作不会渲染，也不会进入键盘导航。
disabled item 会从导航和进度总数中排除。item 有三种状态：`Unanswered`、
`Answered` 和 `Skipped`。

## 校验

必填状态校验已经内置。可以为 item 添加同步 validator，实现领域规则。validator
通过 `QuestionnaireValidationContext` 接收当前 item、当前答案和完整的 enabled
答案快照。`Next` 校验当前 item；`Submit` 校验全部 enabled item，并将焦点移到
第一个无效 item。

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

可选但未回答的 item 在显式跳过前仍然无效；`Skipped` 是明确有效的状态。disabled
item 和 disabled control 不参与校验。提交失败时会选中第一个无效 item，焦点优先
移到其中已填写的 input 或已选 choice，再退回第一个 enabled control。

外部 schema 或服务器响应应使用 external error。外部错误由宿主负责，并会一直
保留到宿主清除它。

```rust
state.update(cx, |state, cx| {
    state
        .set_external_error("handle", "This handle is already taken.", cx)
        .expect("known questionnaire item");
});

// After the owner accepts a corrected answer or a new server response:
state.update(cx, |state, cx| {
    state
        .clear_external_error("handle", cx)
        .expect("known questionnaire item");
});
```

`reset` 会清除内部校验尝试和错误，但保留 owner 管理的 external error。组件支持
Questionnaire 语义校验和同步 validator；原生 HTML constraint validation 不属于此
GPUI 组件。

## 受控状态

当页面需要控制当前 item，或需要在 state 创建后应用已保存答案时，使用静默 setter。
它们会按需更新 UI 和焦点，但不会发出用户交互事件。

```rust
use gpui_component::questionnaire::QuestionnaireAnswer;

state.update(cx, |state, cx| {
    state
        .set_current_item("detail", window, cx)
        .expect("known enabled questionnaire item");
    state
        .set_answer(
            "direction",
            QuestionnaireAnswer::new().with_choices(["delegation"]),
            window,
            cx,
        )
        .expect("known questionnaire item");
    state
        .set_input_value("direction", "A controlled draft", window, cx)
        .expect("item has an input");
});
```

用户意图应使用 `activate_choice`、`confirm_current`、`go_previous`、`go_next`、
`skip_current` 和 `submit`。这些路径会发出相应的 `QuestionnaireEvent`。宿主也
可以使用 `set_item_disabled` 和 `set_choice_disabled`；禁用当前 item 后，焦点会
移动到下一个 enabled item；没有下一个时移动到前一个。

## 恢复

如果希望 `reset` 回到保存的草稿，应在构造 `QuestionnaireState` 之前建立保存的
草稿作为初始快照。使用 `InputState::default_value`、`with_default_selected` 和
`with_current_item`，分别设置 input、choice 和当前 item 的初始基线。

```rust
let saved_input = cx.new(|cx| {
    InputState::new(window, cx).default_value("Saved description")
});
let saved_items = vec![
    QuestionnaireItemDefinition::new("plan", "Which plan?")
        .with_choices([
            QuestionnaireChoiceDefinition::new("plus", "Plus")
                .with_default_selected(true),
            QuestionnaireChoiceDefinition::new("pro", "Pro"),
        ]),
    QuestionnaireItemDefinition::new("detail", "How much detail?")
        .with_input(QuestionnaireInputDefinition::new(saved_input, "More detail")),
];
let state = cx.new(|cx| {
    QuestionnaireState::new(saved_items, cx)
        .expect("valid saved questionnaire")
        .with_current_item("detail")
        .expect("known enabled questionnaire item")
});
```

如果保存值在构造之后才到达，则使用 `set_answer`、`set_input_value` 和
`set_current_item`。这些 setter 只改变当前状态，不会替换 reset 基线。

## 重置

Reset 会恢复初始 choices 和 input 草稿，清除显式 skip、校验尝试和完成状态，回到
初始当前 item，并将焦点移到恢复后的当前 item。

```rust
state.update(cx, |state, cx| {
    state.reset(window, cx);
});
```

External error 在 reset 后仍由 owner 管理。如果 reset 也应该移除服务器错误，请
使用 `clear_external_error` 显式清除。

## 条件 item

Questionnaire 不包含 branching engine。宿主可以根据前一个答案推导 item 的禁用
状态，并通过 `set_item_disabled` 同步。这让条件策略留在页面中，同时由
Questionnaire 继续负责顺序、焦点、进度、校验和提交。

```rust
fn sync_advanced_item(
    state: &Entity<QuestionnaireState>,
    window: &mut Window,
    cx: &mut App,
) {
    let enabled = state.read(cx).answer("direction").is_some_and(|answer| {
        answer
            .choices()
            .iter()
            .any(|choice| choice.as_ref() == "delegation")
    });

    state.update(cx, |state, cx| {
        let _ = state.set_item_disabled("advanced", !enabled, window, cx);
    });
}
```

可以从宿主的 answer-change 处理，或改变前一个答案的 UI action 中调用这个 helper。
被禁用的条件 item 不参与进度、导航、校验、焦点、快捷键和提交。

## 键盘快捷键

为 state 启用字母或数字快捷键。快捷键只作用于当前 item 的 enabled choices。
重复 key event、文本输入、IME 组合以及带修饰键的按键都会保持原有行为。

```rust
use gpui_component::questionnaire::QuestionnaireShortcutMode;

let state = cx.new(|cx| {
    QuestionnaireState::new(items, cx)
        .expect("valid questionnaire schema")
        .with_shortcuts(QuestionnaireShortcutMode::Letters)
});
```

Questionnaire 按原生单选交互处理 radio 的移动。其他场景下，Up/Down 会按 schema
顺序在 enabled choices 和自由输入之间移动；存在 input 时它也会包含在这个顺序中。
非空文本 input 获得焦点时保留正常文本编辑行为。只有焦点不在文本 input 或单选
radio 上时，Left/Right 才会在 item 之间移动；Right 要求当前 item 可确认。

Enter 确认已填写的答案。Command/Ctrl+Enter 确认当前 item。空答案不会隐式提交。
快捷键标签按 enabled choice 顺序分配（`A`–`Z` 或 `1`–`9`），disabled choice
不会分配标签。

## 进度和自定义渲染

`QuestionnaireProgress` 使用默认的 “Question 2 of 4” 样式。也可以读取 progress
state，使用现有 `Progress` 或 `Stepper` 组合自定义指示器。

```rust
QuestionnaireProgress::new(&state)
    .with_size(Size::Small);

let progress = state.read(cx).progress();
let percent = if progress.total() == 0 {
    0.
} else {
    progress.current() as f32 / progress.total() as f32 * 100.
};
Progress::new("questionnaire-progress")
    .value(percent);

Stepper::new("questionnaire-steps")
    .selected_index(progress.current().saturating_sub(1));
```

## 尺寸与主题

Questionnaire 部件实现与其他组件相同的 `Sizable` 契约。默认尺寸为 `Medium`，并
遵循 shadcn/ui `base-nova` docs 外观。支持的命名尺寸为 `XSmall`、`Small`、
`Medium` 和 `Large`；也可以使用 `Size::Size(value)` 自定义比例。

组合部件不会自动继承 root 的 size。需要保持同一比例时，应将相同 size 传给 root、
progress、item、title、description、choices、choice、choice description、input、
error、actions 和 navigation 部件。

```rust
use gpui_component::{Sizable as _, Size};

let size = Size::Small;
Questionnaire::new(&state)
    .with_size(size)
    .child(QuestionnaireProgress::new(&state).with_size(size))
    .child(
        QuestionnaireItem::new(&state, "direction")
            .with_size(size)
            .child(QuestionnaireTitle::new(&state, "direction").with_size(size))
            .child(QuestionnaireDescription::new(&state, "direction").with_size(size))
            .child(
                QuestionnaireChoices::new(&state, "direction")
                    .with_size(size)
                    .child(QuestionnaireChoice::new(&state, "direction", "delegation").with_size(size))
                    .child(QuestionnaireInput::new(&state, "direction").with_size(size)),
            )
            .child(QuestionnaireError::new(&state, "direction").with_size(size)),
    )
    .child(
        QuestionnaireActions::new(&state)
            .with_size(size)
            .child(QuestionnairePrevious::new(&state).with_size(size))
            .child(QuestionnaireSkip::new(&state).with_size(size))
            .child(QuestionnaireNext::new(&state).with_size(size))
            .child(QuestionnaireSubmit::new(&state).with_size(size)),
    );
```

默认皮肤从当前主题的 semantic tokens 派生 spacing、typography、radius、border、
input、primary、muted、destructive 和 focus-ring。局部调整可以使用 `Styled` 方法
或 `StyleRefinement`；局部 style refinement 会在组件默认值之后应用。

## Card 和 Dialog 组合

Questionnaire 负责完整的问题流程；卡片或 dialog 负责容器布局以及关闭、取消行为。
下面两个示例都包含集合中的每个 item，导航到第二个问题时仍会正常显示。

```rust
use gpui::{Entity, IntoElement, ParentElement as _};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{Dialog, DialogClose, DialogFooter, DialogHeader, DialogTitle},
    group_box::{GroupBox, GroupBoxVariants as _},
};

fn questionnaire_content(
    state: &Entity<QuestionnaireState>,
    actions: impl IntoElement,
) -> Questionnaire {
    Questionnaire::new(state)
        .child(QuestionnaireProgress::new(state))
        .child(
            QuestionnaireItem::new(state, "direction")
                .child(QuestionnaireTitle::new(state, "direction"))
                .child(QuestionnaireDescription::new(state, "direction"))
                .child(
                    QuestionnaireChoices::new(state, "direction")
                        .child(QuestionnaireChoice::new(state, "direction", "delegation"))
                        .child(QuestionnaireChoice::new(state, "direction", "questions"))
                        .child(QuestionnaireChoice::new(state, "direction", "both"))
                        .child(QuestionnaireInput::new(state, "direction")),
                )
                .child(QuestionnaireError::new(state, "direction")),
        )
        .child(
            QuestionnaireItem::new(state, "detail")
                .child(QuestionnaireTitle::new(state, "detail"))
                .child(QuestionnaireDescription::new(state, "detail"))
                .child(
                    QuestionnaireChoices::new(state, "detail")
                        .child(QuestionnaireChoice::new(state, "detail", "focused"))
                        .child(QuestionnaireChoice::new(state, "detail", "complete")),
                )
                .child(QuestionnaireError::new(state, "detail")),
        )
        .child(actions)
}

GroupBox::new()
    .outline()
    .title("Set up your workspace")
    .child(questionnaire_content(
        &state,
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    ));
```

对于 dialog，将同一个完整组合放在 dialog content 中，并由宿主处理关闭和取消。

```rust
use gpui_component::{WindowExt as _, questionnaire::QuestionnaireEvent};

let dialog_state = state.clone();
cx.subscribe_in(
    &dialog_state,
    window,
    |_, _, event: &QuestionnaireEvent, window, cx| {
        if matches!(event, QuestionnaireEvent::Submit(_)) {
            window.close_dialog(cx);
        }
    },
)
.detach();

Dialog::new(cx)
    .trigger(
        Button::new("open-questionnaire")
            .outline()
            .label("Open questionnaire"),
    )
    .content(move |content, _, _| {
        content
            .child(DialogHeader::new().child(DialogTitle::new().child("Workspace setup")))
            .child(questionnaire_content(
                &dialog_state,
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel-questionnaire")
                                .outline()
                                .label("Cancel"),
                        ),
                    )
                    .child(
                        QuestionnaireActions::new(&dialog_state)
                            .child(QuestionnairePrevious::new(&dialog_state))
                            .child(QuestionnaireNext::new(&dialog_state))
                            .child(QuestionnaireSubmit::new(&dialog_state)),
                    ),
            ))
    });
```

宿主 subscription 只在成功的 `Submit` 之后关闭 Dialog。同一个 event 也适合将
已校验的 `QuestionnaireSubmission` 交给应用传输层。持久化和网络成功仍由
Questionnaire 外部负责。

## Event 与提交

订阅 `QuestionnaireEvent`，即可监听当前 item 变化、答案变化、完成和成功提交。
`Completed` 只在状态转入 complete 时发出；每次成功执行显式 submit 都会发出
`Submit`。
首次成功提交时，事件顺序为 `Completed`，随后是 `Submit`。
答案或 enabled 条件变化会清除 complete 状态，因此下次成功提交可以再次发出
`Completed`。

```rust
use gpui_component::questionnaire::QuestionnaireEvent;

cx.subscribe(&state, |_, _, event, _| match event {
    QuestionnaireEvent::CurrentItemChanged { current, .. } => {
        println!("Current item: {:?}", current);
    }
    QuestionnaireEvent::AnswerChanged(change) => {
        println!("Changed: {:?} ({:?})", change.item(), change.status());
    }
    QuestionnaireEvent::Completed(submission)
    | QuestionnaireEvent::Submit(submission) => {
        println!("Answers: {:?}", submission.items());
    }
    _ => {}
})
.detach();
```

`detach` 会让 callback 持续有效，直到订阅涉及的 entity 被销毁。如果宿主需要提前
取消监听，请改为保存返回的 `Subscription`。

提交结果按 item schema 顺序排列，并且只包含 enabled item。每个 item 包含 name、
`Unanswered`/`Answered`/`Skipped` 状态和 effective answer。它表示本地已校验的
提交请求；远程保存仍由宿主应用负责。

## 可访问性

`Questionnaire` 根部使用 GPUI 的 `Form` role。`QuestionnaireItem` 是带有 item label
和 description 的可访问分组。definition 中的 `accessibility_label` 与
`description` 始终是 item 和 choice 的语义来源；自定义 child 只替换可见的
fallback 内容，并保留 Questionnaire parts 提供的状态、role、焦点行为和语义。
`QuestionnaireError` 只有 item 无效时才会以 alert 形式播报。Choice 保留 radio 和
checkbox 语义，进度暴露当前值与总数，导航使用真实按钮。

非当前 item 和隐藏操作不会进入键盘导航。成功切换后，焦点移动到新的当前 item；
校验失败时，焦点优先移动到已选或已填写的答案控件，再退回第一个可用控件。

请始终为自由输入在 definition 中提供 `accessibility_label`；可见 label 或等价的
自定义组合可以补充它。GPUI accessibility layer 没有直接对应 `aria-invalid` 的
builder；Questionnaire 仍通过错误 alert、语义分组状态、焦点行为和 destructive
样式暴露无效状态。

## 当前范围

此 GPUI port 覆盖状态、导航、校验、焦点、可访问性、组合渲染和本地提交事件。以下
Web 专属或未来行为暂不提供：SSR/hydration collection diagnostics、`FormData`、
原生 HTML 校验、DOM mutation registration、异步校验、definition 的运行时插入/重排，
以及内置动画或持久化/传输。

## API 参考

### 组合部件

- [Questionnaire]
- [QuestionnaireProgress]
- [QuestionnaireItem]
- [QuestionnaireTitle]
- [QuestionnaireDescription]
- [QuestionnaireChoices]
- [QuestionnaireChoice]
- [QuestionnaireChoiceDescription]
- [QuestionnaireInput]
- [QuestionnaireError]
- [QuestionnaireActions]
- [QuestionnairePrevious]
- [QuestionnaireSkip]
- [QuestionnaireNext]
- [QuestionnaireSubmit]

### 状态、答案与事件

- [QuestionnaireState]
- [QuestionnaireItemDefinition]
- [QuestionnaireChoiceDefinition]
- [QuestionnaireInputDefinition]
- [QuestionnaireAnswer]
- [QuestionnaireAnswers]
- [QuestionnaireItemStatus]
- [QuestionnaireShortcutMode]
- [QuestionnaireProgressState]
- [QuestionnaireItemState]
- [QuestionnaireChoiceState]
- [QuestionnaireNavigationState]
- [QuestionnaireValidationContext]
- [QuestionnaireValidator]
- [QuestionnaireAnswerChange]
- [QuestionnaireSubmission]
- [QuestionnaireSubmissionItem]
- [QuestionnaireEvent]
- [QuestionnaireStateError]
- [Sizable]

[Questionnaire]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.Questionnaire.html
[QuestionnaireProgress]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgress.html
[QuestionnaireItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItem.html
[QuestionnaireTitle]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireTitle.html
[QuestionnaireDescription]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireDescription.html
[QuestionnaireChoices]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoices.html
[QuestionnaireChoice]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoice.html
[QuestionnaireChoiceDescription]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDescription.html
[QuestionnaireInput]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInput.html
[QuestionnaireError]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireError.html
[QuestionnaireActions]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireActions.html
[QuestionnairePrevious]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnairePrevious.html
[QuestionnaireSkip]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSkip.html
[QuestionnaireNext]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireNext.html
[QuestionnaireSubmit]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmit.html
[QuestionnaireState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireState.html
[QuestionnaireItemDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemDefinition.html
[QuestionnaireChoiceDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDefinition.html
[QuestionnaireInputDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInputDefinition.html
[QuestionnaireAnswer]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswer.html
[QuestionnaireAnswers]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswers.html
[QuestionnaireItemStatus]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireItemStatus.html
[QuestionnaireShortcutMode]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireShortcutMode.html
[QuestionnaireProgressState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgressState.html
[QuestionnaireItemState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemState.html
[QuestionnaireChoiceState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceState.html
[QuestionnaireNavigationState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireNavigationState.html
[QuestionnaireValidationContext]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireValidationContext.html
[QuestionnaireValidator]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/type.QuestionnaireValidator.html
[QuestionnaireAnswerChange]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswerChange.html
[QuestionnaireSubmission]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmission.html
[QuestionnaireSubmissionItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmissionItem.html
[QuestionnaireEvent]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireEvent.html
[QuestionnaireStateError]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireStateError.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html

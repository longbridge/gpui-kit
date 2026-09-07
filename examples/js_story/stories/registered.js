// These exports are public constructors from the current component-shell
// inventory. Constructor calls intentionally use `new`, matching the generated
// gpui-component declarations.
import { div } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
import {
  Accordion,
  AccordionItem,
  Alert,
  AlertDialog,
  Attachment,
  ErrorAlert,
  Avatar,
  AreaChart,
  BarChart,
  Badge,
  Breadcrumb,
  Bubble,
  Button,
  Calendar,
  CalendarState,
  Checkbox,
  Clipboard,
  Collapsible,
  ColorPicker,
  ColorPickerState,
  Combobox,
  Command,
  CommandGroup,
  CommandItem,
  CommandState,
  DatePicker,
  DatePickerState,
  DataTable,
  DataTableState,
  DescriptionItem,
  DescriptionList,
  Dialog,
  DropdownButton,
  DropdownMenu,
  Editor,
  EditorState,
  Field,
  Form,
  GroupBox,
  HoverCard,
  Icon,
  Image,
  InfoAlert,
  Input,
  InputState,
  Kbd,
  Label,
  LineChart,
  Link,
  List,
  Menu,
  MenuBar,
  MenuItem,
  MenuSeparator,
  Marker,
  Message,
  MessageScroller,
  MessageScrollerState,
  NumberInput,
  NativeMenuItem,
  NativeMenuSeparator,
  NativeMenuTrigger,
  Notification,
  OtpInput,
  OtpState,
  Pagination,
  PieChart,
  Popover,
  Progress,
  RadarChart,
  Radio,
  RadioGroup,
  Rating,
  Resizable,
  ResizablePanel,
  Scroll,
  Scrollbar,
  ScrollbarHandle,
  Separator,
  VerticalSeparator,
  Select,
  Sheet,
  ShimmerText,
  SettingGroup,
  SettingItem,
  SettingPage,
  Settings,
  Sidebar,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarToggleButton,
  Skeleton,
  Slider,
  SliderState,
  Spinner,
  StatusBar,
  Switch,
  Tag,
  Tab,
  TabBar,
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableFooter,
  TableHead,
  TableHeader,
  TableRow,
  Text,
  Textarea,
  TextareaState,
  Stepper,
  StepperItem,
  SuccessAlert,
  Toggle,
  Tooltip,
  Tree,
  WarningAlert,
  TreeItem,
} from "gpui-component";

/**
 * Registered component elements are runtime Elements. Some generated fluent
 * method names shadow base Element methods, so bridge that structural typing
 * ambiguity only where a typed child is passed to another component.
 * @param {unknown} value
 * @returns {import("gpui-kit").Element}
 */
const asElement = (value) =>
  /** @type {import("gpui-kit").Element} */ (/** @type {unknown} */ (value));

/**
 * Per-case demo state.
 *
 * A registered control is controlled: the script owns `checked`, and the
 * component reports a click through `onChange`. Without somewhere to put the
 * reported value the gallery would render controls that never move, which
 * would say something false about the components.
 *
 * @type {Map<string, unknown>}
 */
const demo = new Map();

/** @param {string} key @param {unknown} fallback */
const state = (key, fallback) => (demo.has(key) ? demo.get(key) : fallback);

/** Test and diagnostic projection of the same controlled state the examples read. */
export const demoValue = state;

/** @param {string} key @param {unknown} value @param {import("gpui-kit").Context} cx */
const setState = (key, value, cx) => {
  demo.set(key, value);
  cx.notify();
};

/**
 * Retained component state must survive Story re-renders. Creating these
 * descriptor objects inside `registeredExamples` would replace the backing
 * GPUI entity after every interaction and make editable controls appear inert.
 *
 * @template T
 * @param {string} key
 * @param {() => T} create
 * @returns {T}
 */
const retained = (key, create) => {
  if (!demo.has(key)) demo.set(key, create());
  return /** @type {T} */ (demo.get(key));
};

/**
 * Create state-backed Story models during the owning View's init phase.
 * GPUI input state cannot be created from render, and every descriptor here
 * needs a stable identity so interaction survives subsequent frames.
 */
export function initializeRegisteredExamples() {
  retained("input-project-name", () => InputState("Enter a project name"));
  retained("input-locked", () => InputState("Managed by your organization"));
  retained("number-input", () => InputState("Quantity", "12"));
  retained("otp-six", () => OtpState(6));
  retained("otp-four", () => OtpState(4));
  retained("textarea-notes", () =>
    TextareaState("Ship the component gallery with verified interactive examples."),
  );
  retained("slider-default", () => SliderState(36));
  retained("slider-reverse", () => SliderState(68));
  retained("slider-vertical", () => SliderState(54));
  retained("slider-disabled", () => SliderState(24));
  retained("color-picker", () => ColorPickerState());
  retained("date-picker", () => DatePickerState());
  retained("calendar-one", () => CalendarState());
  retained("calendar-two", () => CalendarState());
  retained("message-scroller", () => MessageScrollerState(3));
  retained("form-account", () => InputState("Acme Cloud"));
  retained("form-region", () => InputState("us-east-1"));
  retained("form-endpoint", () => InputState("https://api.example.com"));
  retained("form-token", () => InputState("Paste an access token"));
  retained("data-table-default", () => DataTableState(["name", "status"]));
  retained("data-table-striped", () => DataTableState(["name", "status"]));
  retained("command-default", () => CommandState());
  retained("command-filter", () => CommandState());
  retained("scroll-handle", () => ScrollbarHandle());
  retained("scrollbar-handle", () => ScrollbarHandle());
  retained("scrollbar-horizontal-handle", () => ScrollbarHandle());
  retained("editor-rust", () =>
    EditorState("fn main() {\n    println!(\"hello\");\n}", "rust"),
  );
  retained("editor-readonly", () => EditorState("// generated, do not edit", "rust"));
}

/**
 * Whether an accordion section is open.
 *
 * `Accordion.on_toggle` reports the whole open set, and each `AccordionItem`
 * asks about itself.
 *
 * @param {string} key @param {number[]} fallback @param {number} index
 */
const accordionOpen = (key, fallback, index) =>
  /** @type {number[]} */ (state(key, fallback)).includes(index);


/**
 * The cases shown for one registered surface.
 *
 * One to three per surface, chosen to introduce the component rather than to
 * mirror the Rust Story exhaustively: what it looks like by default, the one
 * or two variations a reader most needs to see, and any state — disabled,
 * selected, loading — that changes how it reads.
 *
 * @param {string} surface
 * @param {import("gpui-kit").Context} cx
 * @returns {Array<{ label: string, description?: string, element: unknown }>}
 */
export function registeredExamples(surface, cx) {
  switch (surface) {
    case "Attachment":
      return [
        {
          label: "File metadata",
          description: "A compact file row combines identity, type, size, and a direct action.",
          element: asElement(
            new Attachment("story-attachment")
              .w(520)
              .max_w_full()
              .child(
                h_flex()
                  .w_full()
                  .items_center()
                  .gap(12)
                  .child(asElement(new Icon("icons/file.svg").size("medium")))
                  .child(
                    v_flex()
                      .flex_1()
                      .gap(2)
                      .child(div().text_size(12).font_semibold().child("quarterly-report.pdf"))
                      .child(div().text_size(11).child("PDF · 2.4 MB")),
                  )
                  .child(
                    asElement(
                      new Button("remove-report")
                        .ghost()
                        .size("xsmall")
                        .label("Remove"),
                    ),
                  ),
              ),
          ),
        },
        {
          label: "Upload states",
          description: "Lifecycle styling remains attached to the same file composition.",
          element: asElement(
            new Attachment("story-attachment-failed")
              .w(520)
              .max_w_full()
              .status("failed")
              .child(
                h_flex()
                  .w_full()
                  .items_center()
                  .gap(12)
                  .child(asElement(new Icon("icons/file.svg").size("medium")))
                  .child(
                    v_flex()
                      .flex_1()
                      .gap(2)
                      .child(div().text_size(12).font_semibold().child("screenshot.png"))
                      .child(div().text_size(11).child("Upload failed · 1.8 MB")),
                  )
                  .child(
                    asElement(
                      new Button("retry-screenshot")
                        .outline()
                        .size("xsmall")
                        .label("Retry"),
                    ),
                  ),
              ),
          ),
        },
      ];
    case "Bubble":
      return [
        {
          label: "Alignment",
          description: "Use the same alignment value as the containing Message row.",
          element: v_flex()
            .w(600)
            .max_w_full()
            .gap(12)
            .child(
              asElement(
                new Bubble()
                  .w_full()
                  .alignment("start")
                  .variant("secondary")
                  .child("Can you review the latest draft?"),
              ),
            )
            .child(
              asElement(
                new Bubble()
                  .w_full()
                  .alignment("end")
                  .variant("filled")
                  .child("Yes — I’ll annotate it now."),
              ),
            ),
        },
        {
          label: "Variants",
          description: "Semantic treatments preserve a consistent readable measure.",
          element: v_flex()
            .w(600)
            .max_w_full()
            .gap(12)
            .child(
              asElement(
                new Bubble()
                  .w_full()
                  .alignment("start")
                  .variant("outline")
                  .child("A bordered bubble for rich content."),
              ),
            )
            .child(
              asElement(
                new Bubble()
                  .w_full()
                  .alignment("start")
                  .variant("ghost")
                  .child("Ghost content can use the full conversation width."),
              ),
            ),
        },
      ];
    case "Marker":
      return [
        {
          label: "Streaming status",
          element: asElement(
            new Marker("story-marker")
              .variant("separator")
              .loading(true)
              .loading_style("shimmer")
              .child("Generating response"),
          ),
        },
      ];
    case "Message":
      return [
        {
          label: "Incoming message content",
          element: asElement(
            new Message().alignment("start").child("The build completed successfully."),
          ),
        },
        {
          label: "Outgoing message content",
          element: asElement(
            new Message().alignment("end").child("Ship it."),
          ),
        },
      ];
    case "MessageScroller": {
      /** @type {{ alignment: "start" | "end", variant: "filled" | "secondary", body: string }[]} */
      const messages = [
        {
          alignment: "end",
          variant: "filled",
          body: "Can you review the component gallery before we merge?",
        },
        {
          alignment: "start",
          variant: "secondary",
          body: "Yes. I’m checking the interactive states and making sure longer messages wrap naturally inside the transcript.",
        },
        {
          alignment: "end",
          variant: "filled",
          body: "Perfect — I’ll wait for your notes.",
        },
      ];
      return [
        {
          label: "Conversation",
          description:
            "Virtualized Message rows follow the live edge until the reader scrolls away.",
          element: asElement(
            new MessageScroller(
              "story-message-scroller",
              retained("message-scroller", () => MessageScrollerState(messages.length)),
              (index) => {
                const message = messages[index];
                return div()
                  .w_full()
                  .py(6)
                  .child(
                    asElement(
                      new Message().alignment(message.alignment).child(
                        asElement(
                          new Bubble()
                            .alignment(message.alignment)
                            .variant(message.variant)
                            .child(message.body),
                        ),
                      ),
                    ),
                  );
              },
            )
              .jump_button_label("Jump to latest")
              .h(260),
          ),
        },
      ];
    }
    case "ShimmerText":
      return [
        {
          label: "Looping theme-aware shimmer",
          element: asElement(
            new ShimmerText("Thinking…")
              .id("story-shimmer")
              .duration_ms(1600)
              .spread(0.35),
          ),
        },
        {
          label: "Reverse one-shot shimmer",
          element: asElement(
            new ShimmerText("Loading context")
              .id("story-shimmer-reverse")
              .reverse(true)
              .once(true),
          ),
        },
      ];
    // ---------------------------------------------------------------- actions
    case "Button":
      return [
        {
          label: "Variants",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-primary").primary().label("Primary")))
            .child(asElement(new Button("btn-outline").outline().label("Outline")))
            .child(asElement(new Button("btn-danger").danger().label("Danger")))
            .child(asElement(new Button("btn-ghost").ghost().label("Ghost"))),
        },
        {
          label: "Sizes",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-xs").size("xsmall").label("XSmall")))
            .child(asElement(new Button("btn-sm").size("small").label("Small")))
            .child(asElement(new Button("btn-md").size("medium").label("Medium")))
            .child(asElement(new Button("btn-lg").size("large").label("Large"))),
        },
        {
          label: "States",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Button("btn-loading").primary().label("Saving").loading(true)))
            .child(asElement(new Button("btn-compact").label("Compact").compact()))
            .child(asElement(new Button("btn-link").label("Link").link())),
        },
      ];
    case "DropdownButton":
      return [
        {
          label: "Split button with a menu",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new DropdownButton("dd-actions", "Actions")
                  .variant("primary")
                  .menu_item("Open", (_cx) => {})
                  .menu_item("Duplicate", (_cx) => {}),
              ),
            ),
        },
        {
          label: "Variants and sizes",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new DropdownButton("dd-secondary", "Secondary")
                  .variant("secondary")
                  .size("small")
                  .menu_item("Rename", (_cx) => {}),
              ),
            )
            .child(
              asElement(
                new DropdownButton("dd-danger", "Delete")
                  .variant("danger")
                  .menu_item("Delete forever", (_cx) => {}),
              ),
            )
            .child(
              asElement(
                new DropdownButton("dd-outline", "More")
                  .outline()
                  .menu_anchor("bottom_right")
                  .menu_item("Export", (_cx) => {}),
              ),
            )
            .child(
              asElement(
                new DropdownButton("dd-disabled", "Unavailable")
                  .disabled(true)
                  .menu_item("Nothing", (_cx) => {}),
              ),
            ),
        },
        {
          label: "Menu-only trigger",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new DropdownMenu("dd-more", "More")
                  .item("Rename", (_cx) => {})
                  .item("Archive", (_cx) => {}),
              ),
            ),
        },
      ];
    case "Toggle":
      return [
        {
          label: "Default",
          description: "Text and compact actions show an unmistakable pressed state.",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(
              asElement(
                new Toggle("toggle-off")
                  .label(
                    state("toggle-preview", false)
                      ? "Preview on"
                      : "Preview off",
                  )
                  .checked(/** @type {boolean} */ (state("toggle-preview", false)))
                  .on_change((checked, cx) => setState("toggle-preview", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Toggle("toggle-on")
                  .label(state("toggle-favorite", true) ? "★ Starred" : "☆ Star")
                  .checked(/** @type {boolean} */ (state("toggle-favorite", true)))
                  .on_change((checked, cx) =>
                    setState("toggle-favorite", checked, cx),
                  ),
              ),
            ),
        },
        {
          label: "Variants",
          description: "Ghost and outline treatments suit different surfaces.",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(
              asElement(
                new Toggle("toggle-ghost")
                  .label("Preview")
                  .checked(/** @type {boolean} */ (state("toggle-ghost", false)))
                  .on_change((checked, cx) => setState("toggle-ghost", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Toggle("toggle-outline")
                  .label("Pin toolbar")
                  .outline()
                  .checked(/** @type {boolean} */ (state("toggle-outline", true)))
                  .on_change((checked, cx) =>
                    setState("toggle-outline", checked, cx),
                  ),
              ),
            ),
        },
        {
          label: "Sizes",
          description: "The same action at small, medium, and large density.",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Toggle("toggle-sm").label("Small").size("small")))
            .child(asElement(new Toggle("toggle-md").label("Medium")))
            .child(asElement(new Toggle("toggle-lg").label("Large").size("large"))),
        },
      ];
    case "Link":
      return [
        {
          label: "External link",
          element: asElement(
            new Link("link-docs").href("https://gpui.rs").child("gpui.rs documentation"),
          ),
        },
      ];

    // ----------------------------------------------------------- disclosure
    case "Accordion":
      return [
        {
          label: "Click a header — one section open at a time",
          element: asElement(
            new Accordion("acc-single")
              .bordered(true)
              .multiple(false)
              .on_toggle((indices, cx) => setState("acc-single", indices, cx))
              .child(
                new AccordionItem()
                  .title(new Label("Appearance"))
                  .open(accordionOpen("acc-single", [0], 0))
                  .child("Theme, density and font size."),
              )
              .child(
                new AccordionItem()
                  .title(new Label("Notifications"))
                  .open(accordionOpen("acc-single", [0], 1))
                  .child("Email and desktop notification preferences."),
              ),
          ),
        },
        {
          label: "Several sections open at once",
          element: asElement(
            new Accordion("acc-multiple")
              .multiple(true)
              .on_toggle((indices, cx) => setState("acc-multiple", indices, cx))
              .child(
                new AccordionItem()
                  .title(new Label("Shipping"))
                  .open(accordionOpen("acc-multiple", [0, 1], 0))
                  .child("Ships in 2 days."),
              )
              .child(
                new AccordionItem()
                  .title(new Label("Returns"))
                  .open(accordionOpen("acc-multiple", [0, 1], 1))
                  .child("Free within 30 days."),
              ),
          ),
        },
      ];
    case "Collapsible":
      return [
        {
          label: "Basic",
          description: "A trigger beside the title, with a summary that stays visible.",
          element: asElement(
            v_flex()
              .w(360)
              .border(1)
              .rounded(6)
              .child(
                new Collapsible()
                  .w_full()
                  .open(/** @type {boolean} */ (state("collapsible-order", true)))
                  .motion_id("story-collapsible-order")
                  .child(
                    h_flex()
                      .w_full()
                      .px(12)
                      .py(10)
                      .justify_between()
                      .items_center()
                      .gap(16)
                      .child(div().text_size(13).font_semibold().child("Order #4189"))
                      .child(
                        asElement(
                          new Button("collapsible-order-trigger")
                            .label(state("collapsible-order", true) ? "Hide details" : "Show details")
                            .ghost()
                            .size("xsmall")
                            .on_click((_event, cx) =>
                              setState("collapsible-order", !state("collapsible-order", true), cx),
                            ),
                        ),
                      ),
                  )
                  .child(
                    h_flex()
                      .w_full()
                      .justify_between()
                      .items_center()
                      .px(12)
                      .py(10)
                      .border_t(1)
                      .child(div().text_size(12).child("Status"))
                      .child(asElement(new Tag().variant("success").size("small").child("Shipped"))),
                  )
                  .content(
                    v_flex()
                      .border_t(1)
                      .children(
                        [
                          ["Tracking", "1Z999AA1"],
                          ["Carrier", "UPS Ground"],
                          ["Delivery", "Thursday, September 3"],
                        ].map(([title, value], index) =>
                          h_flex()
                            .w_full()
                            .justify_between()
                            .px(12)
                            .py(9)
                            .when(index > 0, (row) => row.border_t(1))
                            .child(div().text_size(12).child(title))
                            .child(div().text_size(12).font_semibold().child(value)),
                        ),
                      ),
                  ),
              ),
          ),
        },
        {
          label: "Row trigger",
          description: "The whole question row is the trigger, as used by FAQ entries.",
          element: v_flex()
            .w(360)
            .border(1)
            .rounded(6)
            .overflow_hidden()
            .child(
              asElement(
                new Collapsible()
                  .w_full()
                  .open(/** @type {boolean} */ (state("collapsible-faq", false)))
                  .motion_id("story-collapsible-faq")
                  .child(
                    h_flex()
                      .id("collapsible-faq-trigger")
                      .w_full()
                      .justify_between()
                      .items_center()
                      .gap(8)
                      .px(12)
                      .py(10)
                      .on_click((_event, cx) =>
                        setState("collapsible-faq", !state("collapsible-faq", false), cx),
                      )
                      .child(div().text_size(12).child("How do I reset my password?"))
                      .child(
                        asElement(
                          new Icon(
                            state("collapsible-faq", false)
                              ? "icons/chevron-down.svg"
                              : "icons/chevron-right.svg",
                          ).size("xsmall"),
                        ),
                      ),
                  )
                  .content(
                    div()
                      .w_full()
                      .border_t(1)
                      .px(12)
                      .py(10)
                      .text_size(12)
                      .child("Open Settings, choose Security, then select Reset password."),
                  ),
              ),
            ),
        },
      ];

    // --------------------------------------------------------------- inputs
    case "Input":
      return [
        {
          label: "Text fields",
          description: "Click the first field and type; its retained state survives redraws.",
          element: v_flex()
            .w(420)
            .max_w_full()
            .gap(8)
            .child(
              asElement(
                new Input(
                  retained("input-project-name", () => InputState("Enter a project name")),
                ).w_full(),
              ),
            )
            .child(
              asElement(
                new Input(
                  retained("input-locked", () => InputState("Managed by your organization")),
                )
                  .disabled(true)
                  .w_full(),
              ),
            ),
        },
      ];
    case "NumberInput":
      return [
        {
          label: "Stepper buttons on both ends",
          element: asElement(
            new NumberInput(retained("number-input", () => InputState("Quantity", "12")))
              .w(240)
              .max_w_full(),
          ),
        },
      ];
    case "OtpInput":
      return [
        {
          label: "Six digits in two groups",
          element: asElement(
            new OtpInput(retained("otp-six", () => OtpState(6))).groups(2),
          ),
        },
        {
          label: "Four digits, ungrouped",
          element: asElement(new OtpInput(retained("otp-four", () => OtpState(4)))),
        },
      ];
    case "Textarea":
      return [
        {
          label: "Bordered, fixed height",
          element: asElement(
            new Textarea(
              retained("textarea-notes", () =>
                TextareaState("Ship the component gallery with verified interactive examples."),
              ),
            )
              .aria_label("Notes")
              .bordered(true)
              .w(520)
              .max_w_full()
              .h(120),
          ),
        },
      ];
    case "Checkbox":
      return [
        {
          label: "Click any of them — each reports its new state",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(
              asElement(
                new Checkbox("cb-off")
                  .label("Remember me")
                  .checked(/** @type {boolean} */ (state("cb-off", false)))
                  .on_change((checked, cx) => setState("cb-off", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Checkbox("cb-on")
                  .label("Sync devices")
                  .checked(/** @type {boolean} */ (state("cb-on", true)))
                  .on_change((checked, cx) => setState("cb-on", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Checkbox("cb-tip").label("Disabled").tooltip("Not available here"),
              ),
            ),
        },
      ];
    case "Switch":
      return [
        {
          label: "Default",
          description: "Settings remain controlled by the application owner.",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(
              asElement(
                new Switch("sw-off")
                  .label("Notifications")
                  .checked(/** @type {boolean} */ (state("sw-off", false)))
                  .on_change((checked, cx) => setState("sw-off", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Switch("sw-on")
                  .label("Auto-update")
                  .checked(/** @type {boolean} */ (state("sw-on", true)))
                  .on_change((checked, cx) => setState("sw-on", checked, cx)),
              ),
            ),
        },
        {
          label: "Compact",
          description: "A small switch for dense preference rows.",
          element: asElement(
            new Switch("sw-compact")
              .label(
                state("sw-compact", false)
                  ? "Compact controls enabled"
                  : "Compact controls disabled",
              )
              .size("small")
              .checked(/** @type {boolean} */ (state("sw-compact", false)))
              .on_change((checked, cx) => setState("sw-compact", checked, cx)),
          ),
        },
        {
          label: "Disabled",
          description: "Unavailable settings keep their current value visible.",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Switch("sw-disabled-off").label("Managed off").disabled(true)))
            .child(
              asElement(
                new Switch("sw-disabled-on")
                  .label("Managed on")
                  .checked(true)
                  .disabled(true),
              ),
            ),
        },
      ];
    case "Radio":
      return [
        {
          label: "Pick one — the group reports the new index",
          element: asElement(
            new RadioGroup("layout-density")
              .selected_index(/** @type {number} */ (state("radio-group", 1)))
              .on_change((index, cx) => setState("radio-group", index, cx))
              .child(asElement(new Radio("comfortable").label("Comfortable")))
              .child(asElement(new Radio("compact").label("Compact")))
              .child(asElement(new Radio("dense").label("Dense"))),
          ),
        },
        {
          label: "On its own, reporting its own click",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(
              asElement(
                new Radio("radio-standalone")
                  .label("Subscribe")
                  .size("small")
                  .checked(/** @type {boolean} */ (state("radio-standalone", false)))
                  .on_change((checked, cx) => setState("radio-standalone", checked, cx)),
              ),
            )
            .child(
              asElement(
                new Radio("radio-skipped")
                  .label("Skipped by Tab")
                  .accessibility_label("Not a tab stop")
                  .tab_stop(false),
              ),
            ),
        },
      ];
    case "Slider":
      return [
        {
          label: "Horizontal, and reversed",
          element: v_flex()
            .w_full()
            .gap(16)
            .child(asElement(new Slider(retained("slider-default", () => SliderState(36)))))
            .child(
              asElement(new Slider(retained("slider-reverse", () => SliderState(68))).reverse()),
            ),
        },
        {
          label: "Vertical",
          element: asElement(
            new Slider(retained("slider-vertical", () => SliderState(54))).vertical().h(120),
          ),
        },
        {
          label: "Disabled",
          element: asElement(
            new Slider(retained("slider-disabled", () => SliderState(24))).disabled(true),
          ),
        },
      ];
    case "ColorPicker":
      return [
        {
          label: "Labelled trigger",
          element: asElement(
            new ColorPicker(retained("color-picker", () => ColorPickerState()))
              .label("Accent color")
              .accessibility_label("Choose an accent color"),
          ),
        },
      ];
    case "DatePicker":
      return [
        {
          label: "Empty, with a placeholder",
          element: asElement(
            new DatePicker(retained("date-picker", () => DatePickerState())).placeholder(
              "Select a date",
            ),
          ),
        },
      ];
    case "Calendar":
      return [
        {
          label: "One month",
          element: asElement(new Calendar(retained("calendar-one", () => CalendarState()))),
        },
        {
          label: "Two months side by side",
          element: asElement(
            new Calendar(retained("calendar-two", () => CalendarState())).number_of_months(2),
          ),
        },
      ];

    // -------------------------------------------------------------- display
    case "Text":
      return [
        {
          label: "A paragraph of text",
          element: asElement(
            new Text("Text renders a string with the active theme's body style."),
          ),
        },
      ];
    case "Label":
      return [
        {
          label: "Plain, with a secondary value, and masked",
          element: v_flex()
            .gap(8)
            .child(asElement(new Label("Account")))
            .child(asElement(new Label("Account").secondary("Connected")))
            .child(asElement(new Label("API key").masked(true))),
        },
      ];
    case "Icon":
      return [
        {
          label: "Sizes",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Icon("icons/check.svg").size("xsmall")))
            .child(asElement(new Icon("icons/check.svg").size("small")))
            .child(asElement(new Icon("icons/check.svg").size("medium")))
            .child(asElement(new Icon("icons/check.svg").size("large"))),
        },
        {
          label: "Coloured, and rotated a quarter turn",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Icon("icons/check.svg").color("blue-600")))
            .child(asElement(new Icon("icons/check.svg").rotate(Math.PI / 2))),
        },
      ];
    case "Image":
      return [
        {
          label: "An asset from the application directory",
          element: asElement(new Image("assets/pixel.svg").w(64).h(64)),
        },
      ];
    case "Kbd":
      return [
        {
          label: "Default, and outlined",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Kbd("cmd-s")))
            .child(asElement(new Kbd("cmd-shift-p").outline())),
        },
      ];
    case "Separator":
      return [
        {
          label: "Plain, labelled, and dashed",
          element: v_flex()
            .w_full()
            .gap(16)
            .child(asElement(new Separator()))
            .child(asElement(new Separator().label("Account")))
            .child(asElement(new Separator().dashed())),
        },
      ];
    case "Skeleton":
      return [
        {
          label: "A loading placeholder",
          element: v_flex()
            .gap(8)
            .child(asElement(new Skeleton().w(220).h(14)))
            .child(asElement(new Skeleton().secondary().w(160).h(14))),
        },
      ];
    case "Spinner":
      return [
        {
          label: "Sizes",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Spinner().size("small")))
            .child(asElement(new Spinner().size("medium")))
            .child(asElement(new Spinner().size("large"))),
        },
        {
          label: "Alternate icon and easing",
          element: h_flex()
            .gap(16)
            .items_center()
            .child(asElement(new Spinner().icon("loader_circle").color("blue-600")))
            .child(asElement(new Spinner().ease("linear"))),
        },
      ];
    case "Badge":
      return [
        {
          label: "Decorating an icon: a count, a capped count, and a bare dot",
          element: h_flex()
            .gap(24)
            .items_center()
            .child(
              asElement(
                new Badge()
                  .count(3)
                  .child(asElement(new Icon("icons/bell.svg").size("medium"))),
              ),
            )
            .child(
              asElement(
                new Badge()
                  .count(120)
                  .max(99)
                  .child(asElement(new Icon("icons/inbox.svg").size("medium"))),
              ),
            )
            .child(
              asElement(
                new Badge()
                  .dot()
                  .color("red-500")
                  .child(asElement(new Icon("icons/user.svg").size("medium"))),
              ),
            ),
        },
      ];
    case "Tag":
      return [
        {
          label: "Variants",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Tag().variant("primary").child("Primary")))
            .child(asElement(new Tag().variant("success").child("Active")))
            .child(asElement(new Tag().variant("warning").child("Pending")))
            .child(asElement(new Tag().variant("danger").child("Failed"))),
        },
        {
          label: "Outlined, and fully rounded",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(asElement(new Tag().variant("info").outline().child("Draft")))
            .child(asElement(new Tag().variant("secondary").rounded_full().child("Beta"))),
        },
      ];
    case "Avatar":
      return [
        {
          label: "Initials, at three sizes",
          element: h_flex()
            .gap(12)
            .items_center()
            .child(asElement(new Avatar().name("Ada Lovelace").size("small")))
            .child(asElement(new Avatar().name("Ada Lovelace").size("medium")))
            .child(asElement(new Avatar().name("Grace Hopper").size("large"))),
        },
      ];
    case "Alert":
      return [
        {
          label: "Severities",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(asElement(new InfoAlert("alert-info", "A new version is available.").title("Update")))
            .child(asElement(new SuccessAlert("alert-success", "Your changes have been saved.").title("Saved")))
            .child(asElement(new WarningAlert("alert-warning", "Your trial ends in three days.").title("Expiring")))
            .child(asElement(new ErrorAlert("alert-error", "The connection was reset.").title("Upload failed"))),
        },
        {
          label: "Untitled, and as a full-width banner",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(asElement(new Alert("alert-plain", "A neutral message with no title.")))
            .child(
              asElement(
                new WarningAlert("alert-banner", "Scheduled maintenance begins at 02:00 UTC.")
                  .title("Maintenance")
                  .banner(),
              ),
            ),
        },
      ];
    case "Progress":
      return [
        {
          label: "Determinate",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(asElement(new Progress("p-25").value(25).accessibility_label("25 percent")))
            .child(asElement(new Progress("p-64").value(64).accessibility_label("64 percent")))
            .child(asElement(new Progress("p-100").value(100).accessibility_label("Complete"))),
        },
        {
          label: "Indeterminate, while work is in flight",
          element: asElement(
            new Progress("p-loading").loading(true).accessibility_label("Uploading"),
          ),
        },
      ];
    case "Rating":
      return [
        {
          label: "Click a star",
          element: v_flex()
            .gap(12)
            .child(
              asElement(
                new Rating("rating-5")
                  .value(/** @type {number} */ (state("rating-5", 4)))
                  .max(5)
                  .color("amber-500")
                  .on_change((value, cx) => setState("rating-5", value, cx)),
              ),
            )
            .child(
              asElement(
                new Rating("rating-10")
                  .value(/** @type {number} */ (state("rating-10", 7)))
                  .max(10)
                  .on_change((value, cx) => setState("rating-10", value, cx)),
              ),
            ),
        },
      ];
    case "Clipboard":
      return [
        {
          label: "Copies its value, with hover help",
          element: asElement(
            new Clipboard("copy-link").value("https://gpui.rs").tooltip("Copy link"),
          ),
        },
      ];
    case "GroupBox":
      return [
        {
          label: "Variants",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(
              asElement(
                new GroupBox().title("Normal").child(asElement(new Text("Grouped content."))),
              ),
            )
            .child(
              asElement(
                new GroupBox()
                  .title("Outline")
                  .variant("outline")
                  .child(asElement(new Text("Grouped content."))),
              ),
            )
            .child(
              asElement(
                new GroupBox()
                  .title("Fill")
                  .variant("fill")
                  .child(asElement(new Text("Grouped content."))),
              ),
            ),
        },
      ];
    case "StatusBar":
      return [
        {
          label: "Editor status",
          description: "Repository state leads, document state trails, and sync status stays centered.",
          element: asElement(
            new StatusBar()
              .w_full()
              .left_content(asElement(new Button("status-branch").ghost().size("xsmall").label("main")))
              .left_content(asElement(new VerticalSeparator().h(14)))
              .left_content(asElement(new Text("0 errors · 2 warnings")))
              .child(asElement(new Text("All changes saved")))
              .right_content(asElement(new Button("status-position").ghost().size("xsmall").label("Ln 12, Col 34")))
              .right_content(asElement(new VerticalSeparator().h(14)))
              .right_content(asElement(new Button("status-language").ghost().size("xsmall").label("JavaScript"))),
          ),
        },
      ];

    // ------------------------------------------------------------ structure
    case "Breadcrumb":
      return [
        {
          label: "A path of three segments",
          element: asElement(new Breadcrumb(["Home", "Settings", "Profile"])),
        },
      ];
    case "Pagination":
      return [
        {
          label: "Page 2 of 5",
          element: asElement(
            new Pagination("pages")
              .current_page(/** @type {number} */ (state("pages", 2)))
              .on_change((page, cx) => setState("pages", page, cx))
              .total_pages(5)
              .visible_pages(5),
          ),
        },
        {
          label: "Compact, for a narrow toolbar",
          element: asElement(
            new Pagination("pages-compact")
              .current_page(/** @type {number} */ (state("pages-compact", 4)))
              .on_change((page, cx) => setState("pages-compact", page, cx))
              .total_pages(20)
              .compact(),
          ),
        },
      ];
    case "Stepper":
      return [
        {
          label: "Horizontal, on the second step",
          element: asElement(
            new Stepper("onboarding")
              .selected_index(/** @type {number} */ (state("stepper-h", 1)))
              .on_change((index, cx) => setState("stepper-h", index, cx))
              .text_center(true)
              .child(new StepperItem().child("Account"))
              .child(new StepperItem().child("Profile"))
              .child(new StepperItem().child("Finish")),
          ),
        },
        {
          label: "Vertical",
          element: asElement(
            new Stepper("onboarding-vertical")
              .selected_index(/** @type {number} */ (state("stepper-v", 0)))
              .on_change((index, cx) => setState("stepper-v", index, cx))
              .vertical(true)
              .children(
                [
                  ["Account", "Name, email and password."],
                  ["Profile", "Avatar and display name."],
                  ["Review", "Check everything over."],
                  ["Finish", "Nothing left to do."],
                ].map(([title, description], index, all) =>
                  new StepperItem()
                    .pb(index === all.length - 1 ? 0 : 32)
                    .child(
                      v_flex()
                        .gap(2)
                        .child(div().child(title))
                        .child(div().text_size(12).child(description)),
                    ),
                ),
              ),
          ),
        },
      ];
    case "Tab":
      return [];
    case "TabBar":
      return [
        {
          label: "Underline",
          description: "A familiar document-settings navigation pattern.",
          element: asElement(
            new TabBar("profile-tabs")
              .variant("underline")
              .selected_index(/** @type {number} */ (state("tabs-underline", 0)))
              .on_change((index, cx) => setState("tabs-underline", index, cx))
              .child(new Tab().label("Profile"))
              .child(new Tab().label("Security"))
              .child(new Tab().label("Billing")),
          ),
        },
        {
          label: "Segmented",
          description: "A compact view-mode switch for closely related content.",
          element: asElement(
            new TabBar("view-tabs")
              .variant("segmented")
              .selected_index(/** @type {number} */ (state("tabs-segmented", 1)))
              .on_change((index, cx) => setState("tabs-segmented", index, cx))
              .child(new Tab().label("List"))
              .child(new Tab().label("Board")),
          ),
        },
      ];
    case "DescriptionList":
      return [
        {
          label: "Two columns, bordered",
          element: asElement(
            new DescriptionList()
              .columns(2)
              .bordered(true)
              .child(asElement(new DescriptionItem("Owner").value("Ada Lovelace")))
              .child(asElement(new DescriptionItem("Status").value("Active")))
              .child(asElement(new DescriptionItem("Created").value("2026-01-14")))
              .child(asElement(new DescriptionItem("Region").value("eu-west-1"))),
          ),
        },
        {
          label: "Stacked, one field per row",
          element: asElement(
            new DescriptionList()
              .vertical()
              .child(asElement(new DescriptionItem("Owner").value("Ada Lovelace")))
              .child(asElement(new DescriptionItem("Status").value("Active"))),
          ),
        },
      ];
    case "Table":
      return [
        {
          label: "Header, body, footer and caption",
          element: asElement(
            new Table()
              .accessibility_label("Team members")
              .child(new TableCaption().child("Current project members"))
              .child(
                new TableHeader().child(
                  new TableRow()
                    .child(new TableHead().child("Name"))
                    .child(new TableHead().text_right().child("Role")),
                ),
              )
              .child(
                new TableBody()
                  .child(
                    new TableRow()
                      .child(new TableCell().child("Ada Lovelace"))
                      .child(new TableCell().text_right().child("Owner")),
                  )
                  .child(
                    new TableRow()
                      .child(new TableCell().child("Grace Hopper"))
                      .child(new TableCell().text_right().child("Maintainer")),
                  ),
              )
              .child(
                new TableFooter().child(
                  new TableRow().child(new TableCell().col_span(2).child("2 members")),
                ),
              ),
          ),
        },
      ];
    case "Form":
      return [
        {
          label: "Two columns, one field required",
          element: asElement(
            new Form()
              .columns(2)
              .child(
                new Field()
                  .label("Account name")
                  .required(true)
                  .child(
                    asElement(
                      new Input(retained("form-account", () => InputState("Acme Cloud"))).w_full(),
                    ),
                  ),
              )
              .child(
                new Field()
                  .label("Region")
                  .child(
                    asElement(
                      new Input(retained("form-region", () => InputState("us-east-1"))).w_full(),
                    ),
                  ),
              ),
          ),
        },
        {
          label: "One column, a fixed label width, and a small size",
          element: asElement(
            new Form()
              .columns(1)
              .label_width(120)
              .size("small")
              .child(
                new Field()
                  .label("Endpoint")
                  .description("Where requests are sent.")
                  .child(
                    asElement(
                      new Input(
                        retained("form-endpoint", () => InputState("https://api.example.com")),
                      ).w_full(),
                    ),
                  ),
              )
              .child(
                new Field()
                  .label("Token")
                  .required(true)
                  .child(
                    asElement(
                      new Input(
                        retained("form-token", () => InputState("Paste an access token")),
                      ).w_full(),
                    ),
                  ),
              ),
          ),
        },
      ];

    // -------------------------------------------------------------- overlays
    case "Popover":
      return [
        {
          label: "Opens below the trigger",
          element: asElement(
            new Popover("popover-account", "Account details")
              .content(asElement(new Text("Signed in as ada@example.com"))),
          ),
        },
        {
          label: "Open on first render",
          element: asElement(
            new Popover("popover-open", "Already open")
              .default_open(true)
              .content(asElement(new Text("Shown without a click."))),
          ),
        },
        {
          label: "Anchored, kept open by the script, and not closed by the overlay",
          element: v_flex()
            .gap(8)
            .child(
              asElement(
                new Popover("popover-controlled", "Controlled")
                  .card_anchor("top_left")
                  .appearance(true)
                  .overlay_closable(false)
                  .open(/** @type {boolean} */ (state("popover-open", false)))
                  .on_open_change((open, cx) => setState("popover-open", open, cx))
                  .content(asElement(new Text("The script owns this one."))),
              ),
            )
            .child(
              div()
                .text_size(11)
                .child(`open: ${String(state("popover-open", false))}`),
            ),
        },
      ];
    case "HoverCard":
      return [
        {
          label: "Reveals detail after a short hover",
          element: asElement(
            new HoverCard("hover-account")
              .trigger_element(asElement(new Button("hover-trigger").label("Account help")))
              .open_delay(250)
              .content(asElement(new Text("Your account name is visible to collaborators."))),
          ),
        },
        {
          label: "Anchored above, slower to dismiss, and reporting its state",
          element: v_flex()
            .gap(8)
            .child(
              asElement(
                new HoverCard("hover-anchored")
                  .trigger_element(
                    asElement(new Button("hover-anchored-trigger").label("Storage")),
                  )
                  .card_anchor("top_left")
                  .open_delay(120)
                  .close_delay(600)
                  .appearance(true)
                  .on_open_change((open, cx) => setState("hover-open", open, cx))
                  .content(asElement(new Text("42 GB of 100 GB used."))),
              ),
            )
            .child(
              div()
                .text_size(11)
                .child(`open: ${String(state("hover-open", false))}`),
            ),
        },
      ];
    case "Tooltip":
      return [
        {
          label: "Hover help on a control",
          element: asElement(new Tooltip("tooltip-save", "Save", "Writes changes to disk")),
        },
      ];
    case "Dialog":
      return [
        {
          label: "A modal with a title and body",
          element: asElement(
            new Dialog("dialog-project", "Open dialog", (_message, _cx) => {})
              .title("Project details")
              .content(asElement(new Text("Everything about this project."))),
          ),
        },
        {
          label: "Reports which button closed it",
          element: v_flex()
            .gap(8)
            .child(
              asElement(
                new Dialog("dialog-confirm", "Confirm something", (_message, _cx) => {})
                  .title("Confirm")
                  .on_ok((cx) => setState("dialog-outcome", "ok", cx))
                  .on_cancel((cx) => setState("dialog-outcome", "cancel", cx))
                  .on_close((cx) => setState("dialog-outcome", "closed", cx))
                  .content(asElement(new Text("Press a button and watch the line below."))),
              ),
            )
            .child(
              div()
                .text_size(11)
                .child(`last outcome: ${String(state("dialog-outcome", "none"))}`),
            ),
        },
      ];
    case "AlertDialog":
      return [
        {
          label: "Destructive confirmation",
          element: asElement(
            new AlertDialog("alert-discard", "Discard changes", (_message, _cx) => {})
              .title("Discard changes?")
              .description("Unsaved changes will be lost.")
              .show_cancel(true),
          ),
        },
      ];
    case "Sheet":
      return [
        {
          label: "Slides in from the right, and from the bottom",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new Sheet("sheet-right", "Open inspector", (_message, _cx) => {})
                  .title("Inspector")
                  .placement("right")
                  .content(asElement(new Text("Inspector content"))),
              ),
            )
            .child(
              asElement(
                new Sheet("sheet-bottom", "Open drawer", (_message, _cx) => {})
                  .title("Drawer")
                  .placement("bottom")
                  .content(asElement(new Text("Drawer content"))),
              ),
            ),
        },
      ];
    case "Notification":
      return [
        {
          label: "By type",
          element: h_flex()
            .gap(8)
            .items_center()
            .child(
              asElement(
                new Notification("notify-success", "Success", (_message, _cx) => {})
                  .title("Saved")
                  .message("Your changes were saved.")
                  .type("success"),
              ),
            )
            .child(
              asElement(
                new Notification("notify-error", "Error", (_message, _cx) => {})
                  .title("Upload failed")
                  .message("The connection was reset.")
                  .type("error")
                  .autohide(false),
              ),
            ),
        },
      ];

    // ----------------------------------------------------------- collections
    case "List":
      return [
        {
          label: "Rows built from a data callback",
          element: asElement(
            new List(
              "story-list",
              () => [
                { id: "alpha", label: "Alpha" },
                { id: "beta", label: "Beta" },
                { id: "gamma", label: "Gamma" },
              ],
              (row) => asElement(new Text(/** @type {{label: string}} */ (row).label)),
            )
              .w(420)
              .max_w_full()
              .h(180)
              .border(1)
              .border_color(cx.theme().colors.border)
              .rounded(6)
              .overflow_hidden(),
          ),
        },
      ];
    case "Select":
      return [
        {
          label: "Region",
          description: "Choose one deployment region.",
          element: asElement(
            new Select(
              "select-region",
              () => [
                { id: "eu", label: "eu-west-1" },
                { id: "us", label: "us-east-1" },
              ],
              (row) => asElement(new Text(/** @type {{label: string}} */ (row).label)),
              (_value, _cx) => {},
            )
              .placeholder("Choose a region")
              .menu_width(320)
              .w(320)
              .max_w_full(),
          ),
        },
        {
          label: "Disabled",
          description: "A configured value that cannot be changed.",
          element: asElement(
            new Select(
              "select-disabled-static",
              () => [{ id: "managed", label: "Managed by your organization" }],
              (row) => asElement(new Text(/** @type {{label: string}} */ (row).label)),
              (_value, _cx) => {},
            )
              .placeholder("Managed by your organization")
              .menu_width(320)
              .w(320)
              .max_w_full()
              .disabled(true),
          ),
        },
      ];
    case "Combobox":
      return [
        {
          label: "Searchable",
          element: asElement(
            new Combobox(
              "combobox-searchable",
              () => [
                { id: "alpha", label: "Alpha" },
                { id: "beta", label: "Beta" },
                { id: "gamma", label: "Gamma" },
              ],
              (_value, _cx) => {},
              (_value, _cx) => {},
            )
              .placeholder("Choose an option")
              .menu_width(320)
              .w(320)
              .max_w_full()
              .searchable(true)
              .search_placeholder("Filter options"),
          ),
        },
        {
          label: "Without the search field",
          element: asElement(
            new Combobox(
              "combobox-plain",
              () => [{ id: "alpha", label: "Alpha" }],
              (_value, _cx) => {},
              (_value, _cx) => {},
            )
              .placeholder("Choose an option")
              .menu_width(320)
              .w(320)
              .max_w_full()
              .searchable(false),
          ),
        },
      ];
    case "Tree":
      return [
        {
          label: "An expanded folder with two files",
          element: asElement(
            new Tree("project-tree")
              .w(420)
              .max_w_full()
              .h(180)
              .border(1)
              .border_color(cx.theme().colors.border)
              .rounded(6)
              .overflow_hidden()
              .child(
                asElement(
                  new TreeItem("src", "src")
                    .expanded(true)
                    .child(asElement(new TreeItem("main", "main.rs")))
                    .child(asElement(new TreeItem("lib", "lib.rs"))),
                ),
              ),
          ),
        },
      ];
    case "DataTable":
      return [
        {
          label: "Striped rows, sortable and resizable columns",
          element: asElement(
            new DataTable(
              retained("data-table-default", () => DataTableState(["name", "status"])),
              () => [
                { name: "Alpha", status: "Ready" },
                { name: "Beta", status: "Building" },
                { name: "Gamma", status: "Failed" },
              ],
              (row, column) =>
                asElement(
                  new Text(String(/** @type {Record<string, string>} */ (row)[column])),
                ),
            )
              .stripe(true)
              .sortable(true)
              .column_resizable(true)
              .h(180),
          ),
        },
        {
          label: "Bordered, with a row header and selectable rows",
          element: asElement(
            new DataTable(
              retained("data-table-striped", () => DataTableState(["name", "status"])),
              () => [
                { name: "Alpha", status: "Ready" },
                { name: "Beta", status: "Building" },
              ],
              (row, column) =>
                asElement(
                  new Text(String(/** @type {Record<string, string>} */ (row)[column])),
                ),
            )
              .bordered(true)
              .row_header(true)
              .row_selectable(true)
              .column_movable(true)
              .scrollbar_visible(true, false)
              .h(150),
          ),
        },
      ];
    case "Command":
      return [
        {
          label: "A filterable command palette",
          element: asElement(
            new Command(retained("command-default", () => CommandState()))
              .placeholder("Type a command")
              .bordered(true)
              .max_height(200)
              .child(
                asElement(
                  new CommandGroup("Files")
                    .child(asElement(new CommandItem("Open file").keyword("file").action("open")))
                    .child(asElement(new CommandItem("Save file").keyword("save").action("save"))),
                ),
              )
              .child(
                asElement(
                  new CommandGroup("View").child(
                    asElement(new CommandItem("Toggle sidebar").action("sidebar")),
                  ),
                ),
              ),
          ),
        },
        {
          label: "Reports typing, selection and dismissal",
          element: v_flex()
            .w_full()
            .gap(8)
            .child(
              asElement(
                new Command(retained("command-filter", () => CommandState()))
                  .placeholder("Type to filter")
                  .searchable(true)
                  .filterable(true)
                  .max_height(140)
                  .on_query((query, cx) => setState("command-query", query, cx))
                  .on_select((section, row, cx) =>
                    setState("command-at", `${section}:${row}`, cx),
                  )
                  .on_confirm((section, row, cx) =>
                    setState("command-ran", `${section}:${row}`, cx),
                  )
                  .on_cancel((cx) => setState("command-ran", "cancelled", cx))
                  .child(
                    asElement(
                      new CommandGroup("Files")
                        .child(asElement(new CommandItem("Open file").action("open")))
                        .child(asElement(new CommandItem("Save file").action("save"))),
                    ),
                  ),
              ),
            )
            .child(
              div()
                .text_size(11)
                .child(
                  `query ${String(state("command-query", ""))} · highlighted ${String(state("command-at", "none"))} · ran ${String(state("command-ran", "none"))}`,
                ),
            ),
        },
      ];

    // -------------------------------------------------------- layout & panels
    case "Sidebar": {
      const collapsed = /** @type {boolean} */ (state("sidebar-collapsed", false));
      return [
        {
          label: "Application navigation",
          description:
            "A complete workspace sidebar with real destinations, selection, disabled state, account footer, and icon collapse.",
          element: h_flex()
            .w_full()
            .h(340)
            .border(1)
            .rounded(8)
            .overflow_hidden()
            .child(
              asElement(
                new Sidebar("story-sidebar")
                  .side("left")
                  .collapsible("icon")
                  .collapsed(collapsed)
                  .h_full()
                  .header(
                    asElement(
                      new SidebarHeader().child(
                        collapsed
                          ? asElement(new Icon("icons/github.svg").size("small"))
                          : h_flex()
                              .gap(8)
                              .items_center()
                              .child(asElement(new Icon("icons/github.svg").size("small")))
                              .child(
                                v_flex()
                                  .gap(2)
                                  .child(div().font_semibold().child("Acme Studio"))
                                  .child(div().text_size(11).child("Design workspace")),
                              ),
                      ),
                    ),
                  )
                  .footer(
                    asElement(
                      new SidebarFooter().child(
                        collapsed
                          ? asElement(new Icon("icons/user.svg").size("small"))
                          : h_flex()
                              .gap(8)
                              .items_center()
                              .child(asElement(new Icon("icons/user.svg").size("small")))
                              .child(
                                v_flex()
                                  .gap(2)
                                  .child(div().font_semibold().child("Alex Morgan"))
                                  .child(div().text_size(11).child("alex@acme.test")),
                              ),
                      ),
                    ),
                  )
                  .child(
                    asElement(
                      new SidebarMenu()
                        .child(
                          asElement(
                            new SidebarMenuItem("Overview")
                              .icon("home")
                              .selected(true),
                          ),
                        )
                        .child(
                          asElement(
                            new SidebarMenuItem("Components").icon("components"),
                          ),
                        )
                        .child(
                          asElement(
                            new SidebarMenuItem("Settings").icon("settings"),
                          ),
                        )
                        .child(
                          asElement(
                            new SidebarMenuItem("Archive")
                              .icon("archive")
                              .disabled(true),
                          ),
                        ),
                    ),
                  ),
              ),
            )
            .child(
              v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(
                  h_flex()
                    .h(44)
                    .px(12)
                    .gap(10)
                    .items_center()
                    .border_b(1)
                    .child(
                      asElement(
                        new SidebarToggleButton()
                          .collapsed(collapsed)
                          .on_click((_event, cx) =>
                            setState("sidebar-collapsed", !collapsed, cx),
                          ),
                      ),
                    )
                    .child(div().text_size(12).font_semibold().child("Components")),
                )
                .child(
                  v_flex()
                    .flex_1()
                    .p(20)
                    .gap(8)
                    .child(div().text_size(16).font_semibold().child("Component workspace"))
                    .child(
                      div()
                        .text_size(12)
                        .child("Select a destination from the sidebar. Collapse it to keep an icon rail."),
                    ),
                ),
            ),
        },
      ];
    }
    case "Resizable":
      return [
        {
          label: "Two panels with a draggable divider",
          element: v_flex()
            .w_full()
            .border(1)
            .rounded(8)
            .overflow_hidden()
            .child(
              asElement(
                new Resizable("story-split")
                  .axis("horizontal")
                  .cross_size(220)
                  .child(
                    asElement(
                      new ResizablePanel()
                        .size(220)
                        .child(
                          v_flex()
                            .size_full()
                            .p(16)
                            .gap(12)
                            .child(div().text_size(12).font_semibold().child("PROJECT"))
                            .child(div().text_size(12).child("src"))
                            .child(div().pl(16).text_size(12).child("main.rs"))
                            .child(div().pl(16).text_size(12).child("app.rs")),
                        ),
                    ),
                  )
                  .child(
                    asElement(
                      new ResizablePanel().child(
                        v_flex()
                          .size_full()
                          .p(20)
                          .gap(8)
                          .child(div().text_size(15).font_semibold().child("main.rs"))
                          .child(
                            div()
                              .text_size(12)
                              .child("Drag the divider to resize the project tree."),
                          ),
                      ),
                    ),
                  ),
              ),
            ),
        },
      ];
    case "Scroll":
      return [
        {
          label: "A vertical scroll region",
          element: asElement(
            new Scroll(retained("scroll-handle", () => ScrollbarHandle()))
              .scroll_axis("vertical")
              .h(140)
              .child(
                v_flex()
                  .gap(8)
                  .children(
                    Array.from({ length: 12 }, (_unused, index) =>
                      asElement(new Text(`Row ${index + 1}`)),
                    ),
                  ),
              ),
          ),
        },
      ];
    case "Scrollbar": {
      const handle = retained("scrollbar-handle", () => ScrollbarHandle());
      const horizontalHandle = retained("scrollbar-horizontal-handle", () => ScrollbarHandle());
      return [
        {
          label: "An always-visible scrollbar beside its region",
          element: h_flex()
            .w_full()
            .h(160)
            .border(1)
            .rounded(6)
            .overflow_hidden()
            .child(
              asElement(
                new Scroll(handle)
                  .scroll_axis("vertical")
                  .flex_1()
                  .min_w_0()
                  .h_full()
                  .p(12)
                  .child(
                    v_flex()
                      .gap(8)
                      .children(
                        Array.from({ length: 16 }, (_unused, index) =>
                          asElement(new Text(`Activity row ${index + 1}`)),
                        ),
                      ),
                  ),
              ),
            )
            .child(
              asElement(
                new Scrollbar("story-scrollbar", handle)
                  .scroll_axis("vertical")
                  .mode("always"),
              ),
            ),
        },
        {
          label: "An always-visible horizontal scrollbar",
          element: v_flex()
            .w_full()
            .h(110)
            .border(1)
            .rounded(6)
            .overflow_hidden()
            .child(
              asElement(
                new Scroll(horizontalHandle)
                  .scroll_axis("horizontal")
                  .w_full()
                  .flex_1()
                  .min_h(0)
                  .p(12)
                  .child(
                    h_flex()
                      .w(1280)
                      .gap(12)
                      .children(
                        Array.from({ length: 10 }, (_unused, index) =>
                          div()
                            .w(112)
                            .flex_shrink_0()
                            .p(10)
                            .border(1)
                            .rounded(5)
                            .text_size(12)
                            .child(`Column ${index + 1}`),
                        ),
                      ),
                  ),
              ),
            )
            .child(
              asElement(
                new Scrollbar("story-scrollbar-horizontal", horizontalHandle)
                  .scroll_axis("horizontal")
                  .mode("always"),
              ),
            ),
        },
      ];
    }
    case "Settings":
      return [
        {
          label: "A page of grouped settings",
          element: v_flex()
            .w_full()
            .h(420)
            .child(
              asElement(
                new Settings("story-settings")
                  .size("medium")
                  .sidebar_width(220)
                  .default_selected_page(0)
                  .child(
                asElement(
                  new SettingPage("General")
                    .default_open(true)
                    .child(
                      asElement(
                        new SettingGroup()
                          .title("Appearance")
                          .child(
                            asElement(
                              new SettingItem("Theme")
                                .description("Choose the application color scheme.")
                                .content(
                                  asElement(
                                    new Button("settings-theme")
                                      .outline()
                                      .size("small")
                                      .label("System"),
                                  ),
                                ),
                          ),
                          )
                          .child(
                            asElement(
                              new SettingItem("Compact layout")
                                .description("Reduce spacing in navigation and lists.")
                                .content(
                                  asElement(
                                    new Switch("settings-compact")
                                      .checked(false)
                                      .on_change((_checked, _cx) => {}),
                                  ),
                                ),
                            ),
                          ),
                      ),
                    )
                    .child(
                      asElement(
                        new SettingGroup()
                          .title("Updates")
                          .child(
                            asElement(
                              new SettingItem("Automatic updates")
                                .description("Download stable releases in the background.")
                                .content(
                                  asElement(
                                    new Switch("settings-updates")
                                      .checked(true)
                                      .on_change((_checked, _cx) => {}),
                                  ),
                                ),
                            ),
                          ),
                      ),
                    ),
                ),
                  ),
              ),
            ),
        },
      ];
    case "Editor":
      return [
        {
          label: "Editable, and read-only",
          element: v_flex()
            .w_full()
            .gap(12)
            .child(
              asElement(
                new Editor(
                  retained("editor-rust", () =>
                    EditorState("fn main() {\n    println!(\"hello\");\n}", "rust"),
                  ),
                )
                  .aria_label("Source editor")
                  .bordered(true)
                  .w_full()
                  .h(120),
              ),
            )
            .child(
              asElement(
                new Editor(
                  retained("editor-readonly", () =>
                    EditorState("// generated, do not edit", "rust"),
                  ),
                )
                  .aria_label("Generated source")
                  .bordered(true)
                  .readonly(true)
                  .w_full()
                  .h(80),
              ),
            ),
        },
      ];

    // ---------------------------------------------------------------- charts
    case "BarChart":
      return [
        {
          label: "With grid lines and both axes",
          element: asElement(
            new BarChart(() => [
              { label: "Mon", value: 42 },
              { label: "Tue", value: 68 },
              { label: "Wed", value: 31 },
              { label: "Thu", value: 75 },
              { label: "Fri", value: 54 },
            ])
              .grid(true)
              .label_axis(true)
              .value_axis(true)
              .h(200),
          ),
        },
        {
          label: "The other four kinds the catalog registers",
          element: v_flex()
            .w_full()
            .gap(16)
            .child(
              asElement(
                new LineChart(() => [
                  { label: "Mon", value: 42 },
                  { label: "Tue", value: 68 },
                  { label: "Wed", value: 31 },
                  { label: "Thu", value: 75 },
                ])
                  .grid(true)
                  .h(140),
              ),
            )
            .child(
              asElement(
                new AreaChart(() => [
                  { label: "Mon", value: 42 },
                  { label: "Tue", value: 68 },
                  { label: "Wed", value: 31 },
                  { label: "Thu", value: 75 },
                ])
                  .grid(true)
                  .h(140),
              ),
            )
            .child(
              asElement(
                new PieChart(() => [
                  { label: "Rust", value: 62 },
                  { label: "JavaScript", value: 28 },
                  { label: "Other", value: 10 },
                ]).h(160),
              ),
            )
            .child(
              asElement(
                new RadarChart(() => [
                  { label: "Speed", value: 80 },
                  { label: "Memory", value: 55 },
                  { label: "Startup", value: 70 },
                ]).h(160),
              ),
            ),
        },
      ];

    // ------------------------------------------------- platform integration
    case "MenuBar":
      return [
        {
          label: "An application menu installed for this window",
          element: asElement(
            new MenuBar("story-menu-bar").child(
              asElement(
                new Menu("File")
                  .child(asElement(new MenuItem("Open", "open")))
                  .child(asElement(new MenuSeparator()))
                  .child(asElement(new MenuItem("Close", "close").disabled(true))),
              ),
            ),
          ),
        },
      ];
    case "NativeMenuTrigger":
      return [
        {
          label: "Opens the platform's own menu",
          element: asElement(
            new NativeMenuTrigger("native-menu", "Native menu")
              .on_effect_error((_message, _cx) => {})
              .child(asElement(new NativeMenuItem("Open", "open")))
              .child(asElement(new NativeMenuSeparator()))
              .child(asElement(new NativeMenuItem("Close", "close").disabled(true))),
          ),
        },
      ];

    default:
      throw new Error(
        `No JavaScript Story example is defined for registered ${surface}`,
      );
  }
}

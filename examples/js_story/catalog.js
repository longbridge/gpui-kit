// Keep every import explicit. A missing family module is a load error instead
// of a silently incomplete gallery, which makes this the reviewable inventory.
import { stories as foundations } from "./stories/foundations.js";
import { stories as actions } from "./stories/actions.js";
import { stories as inputs } from "./stories/inputs.js";
import { stories as navigation } from "./stories/navigation.js";
import { stories as content } from "./stories/content.js";
import { stories as overlays } from "./stories/overlays.js";
import { stories as collections } from "./stories/collections.js";
import { stories as layouts } from "./stories/layouts.js";
import { coveredBy } from "./stories/coverage.js";

export { coveredBy } from "./stories/coverage.js";

/** The complete JavaScript Story route manifest, in Rust Story display order. */
const byRustStory = [
  ...foundations,
  ...actions,
  ...inputs,
  ...navigation,
  ...content,
  ...overlays,
  ...collections,
  ...layouts,
];

// Preserve the order in `crates/story/src/gallery.rs`, even though the source
// files are grouped by family for maintenance. The explicit list is also a
// second audit: adding a family entry without making it reachable is rejected
// during module load.
const RUST_STORY_ORDER = [
  "WelcomeStory",
  "AccordionStory",
  "AlertStory",
  "AlertDialogStory",
  "AttachmentStory",
  "AvatarStory",
  "BadgeStory",
  "BreadcrumbStory",
  "BubbleStory",
  "ButtonStory",
  "CalendarStory",
  "CarouselStory",
  "ChartStory",
  "CheckboxStory",
  "ClipboardStory",
  "CollapsibleStory",
  "ColorPickerStory",
  "ComboboxStory",
  "CommandStory",
  "DataTableStory",
  "DatePickerStory",
  "DescriptionListStory",
  "DialogStory",
  "DockStory",
  "DropdownButtonStory",
  "EditorStory",
  "FormStory",
  "GroupBoxStory",
  "HoverCardStory",
  "IconStory",
  "ImageStory",
  "InputStory",
  "KbdStory",
  "LabelStory",
  "ListStory",
  "MenuStory",
  "MarkerStory",
  "MessageStory",
  "MessageScrollerStory",
  "NativeMenuStory",
  "NotificationStory",
  "NumberInputStory",
  "OtpInputStory",
  "PaginationStory",
  "PopoverStory",
  "ProgressStory",
  "RadioStory",
  "RatingStory",
  "ResizableStory",
  "ScrollbarStory",
  "SelectStory",
  "SeparatorStory",
  "SettingsStory",
  "SheetStory",
  "ShimmerStory",
  "SidebarStory",
  "SkeletonStory",
  "SliderStory",
  "SpinnerStory",
  "StatusBarStory",
  "StepperStory",
  "SwitchStory",
  "TableStory",
  "TabsStory",
  "TagStory",
  "TextareaStory",
  "ToggleStory",
  "TooltipStory",
  "TreeStory",
  "VirtualListStory",
];

/** The complete JavaScript Story route manifest, in Rust Story display order. */
export const catalog = RUST_STORY_ORDER.map((rustStory) => {
  const story = byRustStory.find(
    (candidate) => candidate.rustStory === rustStory,
  );
  if (!story)
    throw new Error(`JavaScript Story catalog is missing ${rustStory}`);
  return story;
});

/** @type {Map<string, StoryRoute>} */
export const routesById = new Map(catalog.map((story) => [story.id, story]));

if (routesById.size !== catalog.length) {
  throw new Error("JavaScript Story catalog contains duplicate route ids");
}
if (byRustStory.length !== catalog.length) {
  throw new Error(
    "JavaScript Story catalog has a route missing from the Rust Story order",
  );
}
if (coveredBy.some((entry) => !routesById.has(entry.route))) {
  throw new Error("JavaScript Story coverage references an unknown route");
}

/** @param {string} id */
export function route(id) {
  return routesById.get(id) ?? catalog[0];
}

/** @param {string} query */
export function filterCatalog(query) {
  const needle = query.trim().toLowerCase();
  if (needle === "") return catalog;
  return catalog.filter((story) =>
    [story.title, story.group, story.rustStory, story.id].some((value) =>
      value.toLowerCase().includes(needle),
    ),
  );
}

/**
 * @typedef {object} StoryRoute
 * @property {string} id Stable kebab-case route identifier.
 * @property {string} title Rust Story display title.
 * @property {string} group Sidebar family.
 * @property {string} rustStory Source `Story` implementation in crates/story.
 * @property {string} description
 * @property {string[]} states Examples to provide once the binding is available.
 * @property {"registered" | "infrastructure"} availability
 * @property {string} api The expected public gpui-component export.
 * @property {(cx: import("gpui-kit").Context) => import("gpui-kit").Element} render
 */

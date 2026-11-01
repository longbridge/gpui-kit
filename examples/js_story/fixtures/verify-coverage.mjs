#!/usr/bin/env node
// Static coverage audit for the JavaScript scaffold. It reads the canonical
// gpui-shell component inventory and the explicit catalog imports without loading
// gpui, so it remains runnable while the adapter is still being built.
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const fixtureDirectory = dirname(fileURLToPath(import.meta.url));
const repository = resolve(fixtureDirectory, "../../..");
const read = (path) => readFileSync(resolve(repository, path), "utf8");
const fail = (message) => {
  throw new Error(`JavaScript Story coverage: ${message}`);
};

const inventory = JSON.parse(
  read("crates/shell/component-inventory.json"),
);
const catalogSource = read("examples/js_story/catalog.js");
const coverageSource = read("examples/js_story/stories/coverage.js");
const familyFiles = [
  ...catalogSource.matchAll(/from "\.\/stories\/([^"\n]+)"/g),
].map((match) => `examples/js_story/stories/${match[1]}`);

if (familyFiles.length === 0)
  fail("catalog.js does not explicitly import a family module");

const records = familyFiles.flatMap((file) => {
  const source = read(file);
  return [...source.matchAll(/pendingStory\(\{([\s\S]*?)\}\)/g)].map(
    (match) => {
      const field = (name) =>
        match[1].match(new RegExp(`${name}: "([^"]+)"`))?.[1];
      return {
        id: field("id"),
        rustStory: field("rustStory"),
        api: field("api"),
        availability: field("availability"),
        file,
      };
    },
  );
});

const inventoryStories = inventory.items.filter(
  (item) => item.source === "story",
);
// The gallery mirrors the Rust Story app, which shows `gpui-component` alone.
// `gpui-webview` is a separate crate with its own example,
// `examples/js_webview`, so its surfaces have no route to cover them here.
const GALLERY_SOURCES = new Set(["ui", "story"]);
const inventorySurfaces = new Map();
for (const item of inventory.items) {
  if (!item.registration || !GALLERY_SOURCES.has(item.source)) continue;
  const surface =
    item.registration.status === "registered"
      ? (item.registration.api ?? item.registration.descriptor)
      : item.registration.target;
  const current = inventorySurfaces.get(surface);
  const next = {
    status: item.registration.status,
    category: item.registration.category,
  };
  if (
    current &&
    (current.status !== next.status || current.category !== next.category)
  ) {
    fail(`inventory disagrees about ${surface} status`);
  }
  inventorySurfaces.set(surface, next);
}
const renderableRegistrations = [...inventorySurfaces.keys()].sort();

// Rust Stories this gallery deliberately does not mirror, each with the reason.
// An entry here must be `infrastructure` in the inventory, so the list can
// excuse a Story that has no component to show and cannot quietly excuse one
// that does.
const NOT_MIRRORED = new Map([
  [
    "shell",
    "ShellStory embeds a gpui-shell script view inside a Rust story. This " +
      "gallery is already such a view, so a route for it would demonstrate " +
      "the gallery to itself.",
  ],
  [
    "theme",
    "Theme is not a component. Every route already renders through the " +
      "active theme, so a swatch board would restate what the gallery shows.",
  ],
]);
const inventoryNameFor = (rustStory) => {
  const name = rustStory.replace(/Story$/, "");
  if (name === "ThemeColors") return "theme";
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
};

if (new Set(records.map((record) => record.id)).size !== records.length) {
  fail("catalog route ids are not unique");
}
const mirrored = inventoryStories.filter((item) => !NOT_MIRRORED.has(item.name));
for (const name of NOT_MIRRORED.keys()) {
  const item = inventoryStories.find((entry) => entry.name === name);
  if (!item) fail(`${name} is excluded from the gallery but is not a Story`);
  if (item.classification !== "infrastructure") {
    fail(`${name} is excluded from the gallery but is not infrastructure`);
  }
}
if (records.length !== mirrored.length) {
  fail(
    `catalog has ${records.length} routes; inventory has ${mirrored.length} mirrored Story entries`,
  );
}

for (const item of mirrored) {
  const record = records.find(
    (candidate) => inventoryNameFor(candidate.rustStory) === item.name,
  );
  if (!record) fail(`inventory Story ${item.name} has no catalog route`);
  if (item.classification === "infrastructure") {
    if (record.id !== "introduction" && record.availability !== "infrastructure") {
      fail(`${record.id} must declare infrastructure availability`);
    }
  } else {
    const registration =
      item.registration.status === "registered"
        ? (item.registration.api ?? item.registration.descriptor)
        : item.registration.target;
    if (record.api === registration) continue;
    fail(
      `${record.id} expects ${record.api}; inventory tracks ${registration} as ${item.registration.status}`,
    );
  }
}

const order = catalogSource
  .match(/const RUST_STORY_ORDER = \[([\s\S]*?)\];/)?.[1]
  .match(/"([^"]+)"/g)
  ?.map((name) => name.slice(1, -1));
if (!order || order.length !== records.length) {
  fail("catalog order does not enumerate every route");
}
if (
  new Set(order).size !== order.length ||
  order.some((name) => !records.some((record) => record.rustStory === name))
) {
  fail("catalog order and family route records disagree");
}

const coverageBody = coverageSource.match(
  /export const coveredBy = \[([\s\S]*?)\n\];/,
)?.[1];
if (!coverageBody) fail("coverage.js has no explicit coveredBy metadata");
const coverage = [
  ...coverageBody.matchAll(
    /\{ route: "([^"]+)", registrations: \[([^\]]*)\] \}/g,
  ),
].map((match) => ({
  route: match[1],
  registrations: [...match[2].matchAll(/"([^"]+)"/g)].map(
    (registration) => registration[1],
  ),
}));
if (coverage.length !== records.length) {
  fail(
    `coveredBy has ${coverage.length} route entries; catalog has ${records.length}`,
  );
}

const catalogIds = new Set(records.map((record) => record.id));
if (
  new Set(coverage.map((entry) => entry.route)).size !== coverage.length ||
  coverage.some((entry) => !catalogIds.has(entry.route))
) {
  fail("coveredBy routes are not a one-to-one match for catalog routes");
}

for (const record of records) {
  const entry = coverage.find((candidate) => candidate.route === record.id);
  const inventoryItem = inventoryStories.find(
    (item) => item.name === inventoryNameFor(record.rustStory),
  );
  if (
    inventoryItem?.classification !== "infrastructure" &&
    !entry.registrations.includes(record.api)
  ) {
    fail(`${record.id} must explicitly cover its ${record.api} registration`);
  }
}

const coveredRegistrations = [
  ...new Set(coverage.flatMap((entry) => entry.registrations)),
].sort();
const missing = renderableRegistrations.filter(
  (registration) => !coveredRegistrations.includes(registration),
);
const unknown = coveredRegistrations.filter(
  (registration) => !renderableRegistrations.includes(registration),
);
if (missing.length !== 0 || unknown.length !== 0) {
  fail(
    `coveredBy registrations differ from inventory (missing: ${missing.join(", ") || "none"}; unknown: ${unknown.join(", ") || "none"})`,
  );
}

const statusSource = read("examples/js_story/stories/status.js");
const storySource = read("examples/js_story/stories/story.js");
const registeredSource = read("examples/js_story/stories/registered.js");
const appSource = read("examples/js_story/app.js");
const dockSource = read("examples/js_story/stories/dock.js");
const virtualListSource = read("examples/js_story/stories/virtual_list.js");
const allExamplesSource = read("examples/js_story/fixtures/all-examples.js");
const registeredBody = statusSource.match(
  /export const REGISTERED_SURFACES = \[([\s\S]*?)\];/,
)?.[1];
if (registeredBody == null) fail("status projection is missing");
const registered = new Set(
  [...registeredBody.matchAll(/"([^"]+)"/g)].map((match) => match[1]),
);

for (const [surface, expected] of inventorySurfaces) {
  if (expected.status === "registered") {
    if (!registered.has(surface)) {
      fail(
        `${surface} is registered in inventory but not registered in the gallery status projection`,
      );
    }
  }
}
for (const surface of registered) {
  if (inventorySurfaces.get(surface)?.status !== "registered") {
    fail(`${surface} is marked registered outside component-inventory.json`);
  }
}
if (!registeredSource.includes('from "gpui-component"')) {
  fail("registered examples do not import the public gpui-component module");
}
for (const surface of registered) {
  if (!registeredSource.includes(`case "${surface}"`)) {
    fail(`${surface} is registered but has no public constructor example`);
  }
  if (!registeredSource.includes(`new ${surface}(`)) {
    fail(`${surface} registered constructor example does not use new`);
  }
}
if (
  !registeredSource.includes("new Input(") ||
  !registeredSource.includes('InputState("Enter a project name")') ||
  !registeredSource.includes("initializeRegisteredExamples()") ||
  !appSource.includes("initializeRegisteredExamples();")
) {
  fail("Input Story must use the styled component Input with retained state and a placeholder");
}
if (
  !storySource.includes('availability: "registered"') ||
  !storySource.includes("coveredSurfaces(story.id)") ||
  !storySource.includes('availability: "infrastructure"')
) {
  fail("story status rendering does not distinguish registered and infrastructure routes");
}

if (
  storySource.includes("Registered public surface") ||
  !storySource.includes("function storySection(example, cx)") ||
  !storySource.includes("example.description")
) {
  fail("registered routes must use the shared Rust Story-style section presentation");
}

if (
  !registeredSource.includes('label: "Default"') ||
  !registeredSource.includes('label: "Compact"') ||
  !registeredSource.includes('label: "Variants"') ||
  !registeredSource.includes('label: "Sizes"')
) {
  fail("Switch and Toggle must expose the planned interactive Story sections");
}

if (
  !/new Message\(\)\.alignment\(message\.alignment\)/s.test(registeredSource) ||
  !/new Bubble\(\)[\s\S]*?\.alignment\(message\.alignment\)/s.test(registeredSource)
) {
  fail("MessageScroller rows must compose aligned Message and Bubble components");
}

if (
  !registeredSource.includes('new SidebarHeader()') ||
  !registeredSource.includes('new SidebarFooter()') ||
  !registeredSource.includes('label: "Application navigation"') ||
  registeredSource.includes('label: "Collapsed to icons"')
) {
  fail("Sidebar must render one realistic application-navigation Story");
}

if (
  registeredSource.includes('state("collapsible-faq", false) ? "⌃" : "⌄"') ||
  !registeredSource.includes('"icons/chevron-down.svg"') ||
  !registeredSource.includes('"icons/chevron-right.svg"')
) {
  fail("Collapsible row triggers must use real chevron icons");
}

if (registeredSource.includes('label: "Selected, unselected, and disabled"')) {
  fail("Tabs must not retain the rejected third static example");
}

if (
  !appSource.includes("createDockStory(cx)") ||
  !dockSource.includes("DockArea.new") ||
  !dockSource.includes("dock_area(dock)") ||
  !dockSource.includes("dock_content()") ||
  !dockSource.includes("tab_bar((group, cx)") ||
  !dockSource.includes(".drag_tab(group, panel.index)") ||
  !dockSource.includes(".drop_tab(group)") ||
  !dockSource.includes(".drop_indicator((drop, cx)")
) {
  fail("Dock route must mount draggable native panels, dock, tabs, and drop feedback");
}

if (
  !registeredSource.includes('.menu_width(320)') ||
  !registeredSource.includes('EditorState("fn main()') ||
  !registeredSource.includes('", "rust")') ||
  !registeredSource.includes('new Scrollbar("story-scrollbar", handle)') ||
  !registeredSource.includes('new Scrollbar("story-scrollbar-horizontal", horizontalHandle)') ||
  !registeredSource.includes('.scroll_axis("horizontal")') ||
  !registeredSource.includes('.mode("always")')
) {
  fail("Combobox, Editor, and Scrollbar Stories must expose their visible native behavior");
}

if (
  !appSource.includes("renderVirtualListStory(this.virtualListStory, cx)") ||
  !virtualListSource.includes("v_virtual_list(") ||
  !virtualListSource.includes("Scrollbar.vertical(\"project-list\")") ||
  !allExamplesSource.includes("registeredExamples(surface, cx)")
) {
  fail("all Story examples, including VirtualList, must have a real materialization path");
}

if (
  !registeredSource.includes('.left_content(asElement(new Button("status-branch")') ||
  !registeredSource.includes('.right_content(asElement(new Button("status-position")')
) {
  fail("StatusBar Story must exercise its leading and trailing regions");
}

console.log(
  `JavaScript Story coverage: ${records.length} routes track all ${renderableRegistrations.length} tracked catalog surfaces from component-inventory.json`,
);

#!/usr/bin/env bun
/**
 * Publish a snapshot of Zed's GPUI crates to crates.io as `gpui-pre-*`.
 *
 * The `gpui`, `gpui_platform` and `gpui_macros` names on crates.io belong to
 * Zed, and Zed only publishes them occasionally. This script lets GPUI Kit
 * publish its own pre-release builds straight from any Zed commit:
 *
 * 1. Fetch the requested Zed revision into `target/gpui-pre/zed`
 *    (or use an existing checkout passed with `--zed`).
 * 2. Walk the workspace `path` dependencies of the three root crates and
 *    collect every internal crate they need.
 * 3. Rename each crate (`gpui` -> `gpui-pre`, `gpui_platform` ->
 *    `gpui-pre-platform`, `collections` -> `gpui-pre-collections`, ...),
 *    give all of them the same version, and keep the original crate name as
 *    the `[lib]` name so `use gpui::*` keeps working.
 * 4. Drop optional dependencies that come from git without a crates.io
 *    version (crates.io rejects those), together with the features that
 *    enable them. Non-optional ones abort the run.
 * 5. Write a standalone workspace to `target/gpui-pre/workspace`. Every
 *    crate keeps Zed's `license`, copyright notices and `LICENSE-APACHE`,
 *    gets any `NOTICE` Zed ships, and the few files this script rewrites
 *    carry a notice saying so (Apache-2.0 §4).
 * 6. Audit the licenses: a republished crate must be Apache-2.0, and the
 *    dependency graph of the staged workspace must not pull in a copyleft
 *    crate. Zed's own application crates are GPL-3.0-or-later, and one of
 *    them reaching the closure would change the terms for every consumer.
 * 7. Verify with `cargo publish --workspace --dry-run`, build and test
 *    gpui-kit against the staged crates, then publish.
 *
 * crates.io only accepts a handful of brand-new crates per ten minutes. The
 * publish step re-checks crates.io before every attempt, skips versions that
 * already exist, waits out the rate limit, and can be re-run at any time.
 *
 * Usage:
 *     script/bump-gpui.ts [VERSION] [--rev REV] [--zed PATH]
 *                         [--dry-run] [--stage-only] [--no-verify] [--no-wait]
 *
 * Every crate is published at `<VERSION>.<N>`, e.g. `0.3.12`: the
 * `VERSION` constant below (major.minor) plus a patch number that continues
 * from whatever crates.io already has. A positional VERSION overrides that
 * for one run.
 */

import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, normalize, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { parseArgs } from "node:util";

/**
 * The major.minor version of every `gpui-pre-*` crate. Each run publishes
 * `<VERSION>.<N>` with the next unused patch number (see `nextVersion`), so
 * this only moves when the crates should start a new minor series. The Zed
 * commit each build came from is recorded in every crate's description and
 * `[package.metadata.gpui-pre]`.
 */
const VERSION = "0.3";

/**
 * Workspace dependencies swapped for a crate that is published by hand.
 *
 * Zed's reqwest fork (https://github.com/zed-industries/reqwest) is published
 * separately as `gpui-pre-reqwest`; the version here must match that publish.
 */
const DEPENDENCY_OVERRIDES: Record<
  string,
  { package: string; version: string }
> = {
  reqwest: { package: "gpui-pre-reqwest", version: "0.12.15" },
  // Zed pins a git revision; `test-support` compiles against the crates.io
  // release, which has the `attr-macro` feature Zed enables.
  proptest: { package: "proptest", version: "1" },
};

const ZED_GIT_URL = "https://github.com/zed-industries/zed";
const ZED_DEFAULT_REV = "main";
const PUBLISH_PREFIX = "gpui-pre";
const ROOT_CRATES = ["gpui", "gpui_platform", "gpui_macros", "reqwest_client"];
const CRATES_IO_API = "https://crates.io/api/v1/crates";
const USER_AGENT =
  "gpui-kit bump-gpui (https://github.com/longbridge/gpui-kit)";
const RATE_LIMIT_FALLBACK_MS = (10 * 60 + 30) * 1000;
const FACADE_PATH_DEPENDENCY = "proc-macro-crate";

const DEP_TABLES = ["dependencies", "build-dependencies"] as const;
const DEV_DEP_TABLE = "dev-dependencies";

const REPO_ROOT = resolve(import.meta.dir, "..");
const WORK_DIR = join(REPO_ROOT, "target", "gpui-pre");

type Toml = Record<string, any>;

// ---------------------------------------------------------------------------
// Logging (mirrors script/bump-version.sh)
// ---------------------------------------------------------------------------

const USE_COLOR =
  Boolean(process.stdout.isTTY) && process.env.NO_COLOR === undefined;

const paint = (code: string, text: string) =>
  USE_COLOR ? `\x1b[${code}m${text}\x1b[0m` : text;
const bold = (text: string) => paint("1", text);
const dim = (text: string) => paint("2", text);

function logHeader(message: string) {
  const line = "═".repeat(56);
  console.log();
  console.log(paint("1;34", `╔${line}╗`));
  console.log(`${paint("1;34", "║")}  ${paint("1;36", message)}`);
  console.log(paint("1;34", `╚${line}╝`));
  console.log();
}

const logStep = (step: string, message: string) =>
  console.log(`${paint("1;35", `[${step}]`)} ${message}`);
const logSuccess = (message: string) =>
  console.log(`${paint("1;32", "✓")} ${message}`);
const logInfo = (message: string) =>
  console.log(`${paint("36", "ℹ")} ${message}`);
const logWarn = (message: string) =>
  console.log(`${paint("1;33", "!")} ${message}`);
const logError = (message: string) =>
  console.error(`${paint("1;31", "✗")} ${message}`);

/** A user-facing failure; the message is printed without a stack trace. */
class BumpError extends Error {}

// ---------------------------------------------------------------------------
// Shell helpers
// ---------------------------------------------------------------------------

interface RunOptions {
  cwd?: string;
  /** Capture output instead of streaming it to the terminal. */
  capture?: boolean;
}

/** Run a command, echoing it first. Throws BumpError on failure. */
async function run(cmd: string[], options: RunOptions = {}): Promise<string> {
  const shown = options.cwd
    ? `(cd ${options.cwd} && ${cmd.join(" ")})`
    : cmd.join(" ");
  console.log(dim(`$ ${shown}`));
  const { code, output } = await spawn(cmd, options.cwd, !options.capture);
  if (code !== 0) {
    const detail = output.trim() ? `\n${output.trim()}` : "";
    throw new BumpError(`command failed (${code}): ${cmd[0]}${detail}`);
  }
  return output;
}

/** Run a command, streaming its output while also capturing it. */
async function runStreaming(
  cmd: string[],
  cwd: string,
): Promise<{ code: number; output: string }> {
  console.log(dim(`$ (cd ${cwd} && ${cmd.join(" ")})`));
  return spawn(cmd, cwd, true);
}

async function spawn(cmd: string[], cwd: string | undefined, echo: boolean) {
  const process_ = Bun.spawn(cmd, {
    cwd,
    stdin: "inherit",
    stdout: "pipe",
    stderr: "pipe",
  });
  const chunks: string[] = [];
  const decoder = new TextDecoder();
  const pump = async (stream: ReadableStream<Uint8Array>) => {
    for await (const chunk of stream) {
      const text = decoder.decode(chunk, { stream: true });
      chunks.push(text);
      if (echo) process.stdout.write(text);
    }
  };
  await Promise.all([pump(process_.stdout), pump(process_.stderr)]);
  const code = await process_.exited;
  return { code, output: chunks.join("") };
}

// ---------------------------------------------------------------------------
// Zed checkout
// ---------------------------------------------------------------------------

async function prepareZed(
  rev: string,
  existing: string | undefined,
): Promise<string> {
  if (existing !== undefined) {
    const zed = resolve(
      existing.replace(/^~(?=$|\/)/, process.env.HOME ?? "~"),
    );
    if (!existsSync(join(zed, "Cargo.toml"))) {
      throw new BumpError(
        `${zed} does not look like a Zed checkout (no Cargo.toml)`,
      );
    }
    logInfo(`Using existing Zed checkout at ${bold(zed)}`);
    return zed;
  }

  const zed = join(WORK_DIR, "zed");
  if (!existsSync(join(zed, ".git"))) {
    mkdirSync(zed, { recursive: true });
    await run(["git", "init", "-q"], { cwd: zed });
    await run(["git", "remote", "add", "origin", ZED_GIT_URL], { cwd: zed });
  }
  logInfo(`Fetching ${bold(rev)} from ${ZED_GIT_URL}`);
  await run(["git", "fetch", "--depth", "1", "--no-tags", "origin", rev], {
    cwd: zed,
  });
  await run(["git", "checkout", "-q", "--detach", "--force", "FETCH_HEAD"], {
    cwd: zed,
  });
  return zed;
}

async function zedRevision(zed: string): Promise<string> {
  try {
    return (
      await run(["git", "rev-parse", "HEAD"], { cwd: zed, capture: true })
    ).trim();
  } catch {
    throw new BumpError(
      `${zed} is not a git checkout; cannot determine the Zed revision`,
    );
  }
}

// ---------------------------------------------------------------------------
// Workspace model
// ---------------------------------------------------------------------------

/** One Zed workspace member selected for publishing. */
interface Crate {
  relDir: string;
  manifest: Toml;
  name: string;
  version: string;
  publishedName: string;
  prunedDeps: string[];
  prunedFeatures: string[];
}

interface Workspace {
  root: string;
  manifest: Toml;
  /** relDir -> manifest */
  members: Map<string, Toml>;
  /** relDir -> what pruning removed, so a recomputed closure keeps the record */
  pruned: Map<string, { deps: string[]; features: string[] }>;
}

function workspaceDependencies(ws: Workspace): Toml {
  return ws.manifest.workspace.dependencies ?? {};
}

function readToml(path: string): Toml {
  return Bun.TOML.parse(readFileSync(path, "utf8")) as Toml;
}

function loadWorkspace(zed: string): Workspace {
  const manifest = readToml(join(zed, "Cargo.toml"));
  const members = new Map<string, Toml>();
  for (const pattern of manifest.workspace.members as string[]) {
    const matches = [
      ...new Bun.Glob(pattern).scanSync({ cwd: zed, onlyFiles: false }),
    ].sort();
    for (const match of matches) {
      const cargoToml = join(zed, match, "Cargo.toml");
      if (existsSync(cargoToml)) {
        members.set(normalize(match), readToml(cargoToml));
      }
    }
  }
  const ws: Workspace = { root: zed, manifest, members, pruned: new Map() };
  WORKSPACE_PACKAGE_LICENSE = manifest.workspace.package?.license;
  applyDependencyOverrides(ws);
  return ws;
}

/** Point overridden workspace dependencies at their hand-published crates. */
function applyDependencyOverrides(ws: Workspace) {
  const dependencies = workspaceDependencies(ws);
  for (const [name, override] of Object.entries(DEPENDENCY_OVERRIDES)) {
    const spec = dependencies[name];
    if (spec === undefined) {
      logWarn(
        `workspace dependency \`${name}\` is not defined in Zed; override ignored`,
      );
      continue;
    }
    const rest = isPlainObject(spec) ? withoutSource(spec) : {};
    dependencies[name] = {
      package: override.package,
      version: override.version,
      ...rest,
    };
    logInfo(
      `${name}: using ${override.package} ${override.version} instead of ${describeSource(spec)}`,
    );
  }
}

function describeSource(spec: unknown): string {
  if (typeof spec === "string") return `crates.io ${spec}`;
  if (isPlainObject(spec)) {
    if (spec.git !== undefined)
      return `${spec.package ?? "git"} from ${spec.git}`;
    if (spec.path !== undefined) return `path ${spec.path}`;
    if (spec.version !== undefined) return `crates.io ${spec.version}`;
  }
  return "an unknown source";
}

/** `gpui` -> `gpui-pre`, `gpui_macros` -> `gpui-pre-macros`, `collections` -> `gpui-pre-collections`. */
function publishedName(zedName: string): string {
  if (zedName === "gpui") return PUBLISH_PREFIX;
  const suffix = zedName.startsWith("gpui_")
    ? zedName.slice("gpui_".length)
    : zedName;
  return `${PUBLISH_PREFIX}-${suffix.replaceAll("_", "-")}`;
}

/** `util` and `gpui_util` would both become `gpui-pre-util`; refuse rather than publish the wrong one. */
function ensureUniqueNames(crates: Crate[]) {
  const byPublished = new Map<string, string[]>();
  for (const crate of crates) {
    byPublished.set(crate.publishedName, [
      ...(byPublished.get(crate.publishedName) ?? []),
      crate.name,
    ]);
  }
  const clashes = [...byPublished].filter(([, names]) => names.length > 1);
  if (clashes.length > 0) {
    const detail = clashes
      .map(([published, names]) => `${published} <- ${names.join(", ")}`)
      .join("\n  ");
    throw new BumpError(
      `these Zed crates would publish under the same name:\n  ${detail}`,
    );
  }
}

interface DepEntry {
  tablePath: string[];
  name: string;
  spec: any;
}

/** Every dependency entry of a manifest, with the table it lives in. */
function depTables(manifest: Toml, dev = false): DepEntry[] {
  const names = dev ? [DEV_DEP_TABLE] : [...DEP_TABLES];
  const entries: DepEntry[] = [];
  for (const table of names) {
    for (const [name, spec] of Object.entries(manifest[table] ?? {})) {
      entries.push({ tablePath: [table], name, spec });
    }
  }
  for (const [cfg, cfgTables] of Object.entries(manifest.target ?? {})) {
    for (const table of names) {
      for (const [name, spec] of Object.entries(
        (cfgTables as Toml)[table] ?? {},
      )) {
        entries.push({ tablePath: ["target", cfg, table], name, spec });
      }
    }
  }
  return entries;
}

/** Merge a crate's dependency entry with its workspace definition. */
function effectiveSpec(ws: Workspace, name: string, spec: any): Toml {
  if (typeof spec === "string") return { version: spec };
  if (spec.workspace) {
    const base = workspaceDependencies(ws)[name];
    if (base === undefined) {
      throw new BumpError(
        `dependency \`${name}\` inherits from the workspace but is not defined there`,
      );
    }
    const merged: Toml =
      typeof base === "string" ? { version: base } : { ...base };
    merged.optional = Boolean(spec.optional);
    merged.workspace = true;
    return merged;
  }
  return { ...spec };
}

/** The workspace-relative directory of a path dependency, if it is one. */
function pathDepTarget(crateDir: string, spec: Toml): string | undefined {
  if (spec.path === undefined) return undefined;
  return normalize(spec.workspace ? spec.path : join(crateDir, spec.path));
}

/** Every workspace crate the root crates need, in dependency order. */
function collectClosure(ws: Workspace): Crate[] {
  const byName = new Map<string, string>();
  for (const [rel, manifest] of ws.members)
    byName.set(manifest.package.name, rel);
  for (const root of ROOT_CRATES) {
    if (!byName.has(root))
      throw new BumpError(
        `crate \`${root}\` was not found in the Zed workspace`,
      );
  }

  const order: string[] = [];
  const visiting = new Set<string>();
  const done = new Set<string>();

  const visit = (relDir: string) => {
    if (done.has(relDir)) return;
    if (visiting.has(relDir))
      throw new BumpError(`dependency cycle through ${relDir}`);
    visiting.add(relDir);
    const manifest = ws.members.get(relDir);
    if (manifest === undefined)
      throw new BumpError(
        `path dependency \`${relDir}\` is not a workspace member`,
      );
    for (const { name, spec } of depTables(manifest)) {
      const target = pathDepTarget(relDir, effectiveSpec(ws, name, spec));
      if (target !== undefined) visit(target);
    }
    visiting.delete(relDir);
    done.add(relDir);
    order.push(relDir);
  };

  for (const root of ROOT_CRATES) visit(byName.get(root)!);

  return order.map((relDir) => {
    const manifest = ws.members.get(relDir)!;
    const name = manifest.package.name as string;
    return {
      relDir,
      manifest,
      name,
      version: String(manifest.package.version ?? "0.0.0"),
      publishedName: publishedName(name),
      prunedDeps: [],
      prunedFeatures: [],
    };
  });
}

/** crates.io needs a version for every dependency, git or not. */
function isPublishableSource(spec: Toml): boolean {
  if (spec.path !== undefined) return true;
  if (spec.git !== undefined) return spec.version !== undefined;
  return true;
}

// ---------------------------------------------------------------------------
// Pruning unpublishable optional dependencies
// ---------------------------------------------------------------------------

/** Split a feature entry into [dependency-or-feature, sub-feature]. */
function featureTargets(entry: string): [string, string | undefined] {
  if (entry.startsWith("dep:")) return [entry.slice(4), undefined];
  const slash = entry.indexOf("/");
  if (slash !== -1)
    return [entry.slice(0, slash).replace(/\?$/, ""), entry.slice(slash + 1)];
  return [entry, undefined];
}

function removeDependency(manifest: Toml, tablePath: string[], name: string) {
  let table = manifest;
  for (const key of tablePath) table = table[key];
  delete table[name];
}

/**
 * Remove dependencies crates.io would reject, and whatever only existed for them.
 *
 * Manifests are edited in place inside `ws.members`, so a later
 * `collectClosure` sees the pruned dependency graph.
 */
function pruneUnpublishable(ws: Workspace, crates: Crate[]) {
  const errors: string[] = [];
  for (const crate of crates) {
    const features: Record<string, string[]> = (crate.manifest.features ??= {});
    // Optional dependencies named with `dep:` get no implicit feature, so
    // once every feature that enabled them is gone nobody can turn them on.
    const explicitOnly = new Set(
      Object.values(features)
        .flat()
        .filter((entry) => entry.startsWith("dep:"))
        .map((entry) => featureTargets(entry)[0]),
    );

    const removed: string[] = [];
    for (const { tablePath, name, spec } of depTables(crate.manifest)) {
      const merged = effectiveSpec(ws, name, spec);
      if (isPublishableSource(merged)) continue;
      const source = merged.git ?? "?";
      if (!merged.optional) {
        errors.push(
          `${crate.name}: \`${name}\` comes from ${source} without a crates.io version`,
        );
        continue;
      }
      removeDependency(crate.manifest, tablePath, name);
      removed.push(name);
      logWarn(
        `${crate.name}: dropping optional dependency \`${name}\` (${source} has no crates.io version)`,
      );
    }

    const removedSet = new Set(removed);
    const droppedFeatures: string[] = [];
    for (const [feature, entries] of Object.entries(features)) {
      const needsRemoved = entries.filter(
        (e) => e.startsWith("dep:") && removedSet.has(featureTargets(e)[0]),
      );
      if (needsRemoved.length > 0) {
        // The feature exists to turn this dependency on; without it the
        // feature would enable code that cannot compile.
        delete features[feature];
        droppedFeatures.push(feature);
        logWarn(
          `${crate.name}: dropping feature \`${feature}\` (needs \`${featureTargets(needsRemoved[0])[0]}\`)`,
        );
      } else {
        features[feature] = entries.filter(
          (e) => !removedSet.has(featureTargets(e)[0]),
        );
      }
    }

    const referenced = new Set(
      Object.values(features)
        .flat()
        .map((e) => featureTargets(e)[0]),
    );
    for (const { tablePath, name, spec } of depTables(crate.manifest)) {
      const merged = effectiveSpec(ws, name, spec);
      if (merged.optional && explicitOnly.has(name) && !referenced.has(name)) {
        removeDependency(crate.manifest, tablePath, name);
        removed.push(name);
        logWarn(
          `${crate.name}: dropping optional dependency \`${name}\` (no feature enables it any more)`,
        );
      }
    }

    let pruned = ws.pruned.get(crate.relDir);
    if (pruned === undefined)
      ws.pruned.set(crate.relDir, (pruned = { deps: [], features: [] }));
    pruned.deps.push(...removed);
    pruned.features.push(...droppedFeatures);
  }

  // Features that referenced a dropped feature, in this or another crate.
  const droppedByName = new Map(
    crates.map((c) => [
      c.name,
      new Set(ws.pruned.get(c.relDir)?.features ?? []),
    ]),
  );
  for (const crate of crates) {
    const depNames = new Map<string, string>();
    for (const { name, spec } of depTables(crate.manifest)) {
      const target = pathDepTarget(crate.relDir, effectiveSpec(ws, name, spec));
      if (target !== undefined)
        depNames.set(name, ws.members.get(target)!.package.name);
    }
    const features: Record<string, string[]> = crate.manifest.features ?? {};
    for (const [feature, entries] of Object.entries(features)) {
      features[feature] = entries.filter((entry) => {
        const [dep, sub] = featureTargets(entry);
        if (sub === undefined) return !droppedByName.get(crate.name)?.has(dep);
        return !droppedByName.get(depNames.get(dep) ?? "")?.has(sub);
      });
    }
  }

  if (errors.length > 0) {
    throw new BumpError(
      `these dependencies cannot be published to crates.io:\n  ${errors.join("\n  ")}`,
    );
  }
}

/** Compute the publish set, pruning until the dependency graph is stable. */
function selectCrates(ws: Workspace): Crate[] {
  let crates = collectClosure(ws);
  for (;;) {
    pruneUnpublishable(ws, crates);
    const again = collectClosure(ws);
    if (
      again.map((c) => c.relDir).join("\n") ===
      crates.map((c) => c.relDir).join("\n")
    )
      break;
    crates = again;
  }
  for (const crate of crates) {
    const pruned = ws.pruned.get(crate.relDir);
    crate.prunedDeps = pruned?.deps ?? [];
    crate.prunedFeatures = pruned?.features ?? [];
  }
  ensureUniqueNames(crates);
  return crates;
}

// ---------------------------------------------------------------------------
// Manifest generation
// ---------------------------------------------------------------------------

const BARE_KEY = /^[A-Za-z0-9_-]+$/;

function tomlKey(key: string): string {
  if (BARE_KEY.test(key)) return key;
  if (!key.includes("'")) return `'${key}'`;
  return JSON.stringify(key);
}

const isPlainObject = (value: unknown): value is Toml =>
  typeof value === "object" &&
  value !== null &&
  !Array.isArray(value) &&
  !(value instanceof Date);

function tomlScalar(value: unknown): string {
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) {
    const items = value.map(tomlScalar);
    const joined = items.join(", ");
    if (joined.length > 80)
      return `[\n${items.map((item) => `    ${item},\n`).join("")}]`;
    return `[${joined}]`;
  }
  if (isPlainObject(value)) {
    const inner = Object.entries(value)
      .map(([k, v]) => `${tomlKey(k)} = ${tomlScalar(v)}`)
      .join(", ");
    return inner ? `{ ${inner} }` : "{}";
  }
  throw new BumpError(`cannot serialize ${typeof value} to TOML`);
}

function isDepTable(path: string[]): boolean {
  const last = path.at(-1);
  if (
    last !== undefined &&
    [...DEP_TABLES, DEV_DEP_TABLE].includes(last as any)
  )
    return true;
  return (
    path.length === 2 && path[0] === "workspace" && path[1] === "dependencies"
  );
}

function inlineDict(path: string[], value: Toml): boolean {
  if (path.length === 0) return false;
  if (isDepTable(path)) return true;
  if ("workspace" in value) return true;
  // `[lints.clippy] style = { level = "allow", priority = -1 }` and
  // `[profile.dev.package] foo = { opt-level = 3 }` stay one entry per line.
  const last = path.at(-1)!;
  return (
    (path.includes("lints") || path.includes("profile")) &&
    last !== "lints" &&
    last !== "profile"
  );
}

function emitTable(lines: string[], path: string[], table: Toml) {
  const values: [string, unknown][] = [];
  const subtables: [string, Toml][] = [];
  const arraysOfTables: [string, Toml[]][] = [];
  for (const [key, value] of Object.entries(table)) {
    if (isPlainObject(value) && !inlineDict(path, value)) {
      subtables.push([key, value]);
    } else if (
      Array.isArray(value) &&
      value.length > 0 &&
      value.every(isPlainObject)
    ) {
      arraysOfTables.push([key, value]);
    } else {
      values.push([key, value]);
    }
  }

  if (
    path.length > 0 &&
    (values.length > 0 ||
      (subtables.length === 0 && arraysOfTables.length === 0))
  ) {
    if (lines.length > 0) lines.push("");
    lines.push(`[${path.map(tomlKey).join(".")}]`);
  }
  for (const [key, value] of values)
    lines.push(`${tomlKey(key)} = ${tomlScalar(value)}`);
  for (const [key, value] of subtables) emitTable(lines, [...path, key], value);
  for (const [key, items] of arraysOfTables) {
    const header = [...path, key].map(tomlKey).join(".");
    for (const item of items) {
      lines.push("", `[[${header}]]`);
      for (const [k, v] of Object.entries(item))
        lines.push(`${tomlKey(k)} = ${tomlScalar(v)}`);
    }
  }
}

function tomlDump(document: Toml): string {
  const lines: string[] = [];
  emitTable(lines, [], document);
  return `${lines.join("\n")}\n`;
}

const SOURCE_KEYS = [
  "path",
  "package",
  "version",
  "git",
  "rev",
  "branch",
  "tag",
  "registry",
];

const withoutSource = (spec: Toml) =>
  Object.fromEntries(
    Object.entries(spec).filter(([k]) => !SOURCE_KEYS.includes(k)),
  );

function crateManifest(
  crate: Crate,
  cratesByDir: Map<string, Crate>,
  version: string,
  zedSha: string,
): Toml {
  const source = crate.manifest;
  const pkg: Toml = { ...source.package };
  const license = packageLicense(crate);
  if (license !== GPUI_LICENSE) {
    throw new BumpError(
      `${crate.name}: license is ${license === undefined ? "not declared" : `\`${license}\``}, ` +
        `and only ${GPUI_LICENSE} crates are republished as ${PUBLISH_PREFIX}-*; ` +
        "Zed's application crates are GPL-3.0-or-later and must not reach the closure",
    );
  }

  const snapshot = `(${PUBLISH_PREFIX} snapshot of zed@${zedSha.slice(0, 7)})`;
  const description: string =
    pkg.description || `Zed's \`${crate.name}\` crate`;
  pkg.name = crate.publishedName;
  pkg.version = version;
  pkg.publish = true;
  pkg.description = `${description.replace(/\.+$/, "")} ${snapshot}`;
  pkg.repository ??= ZED_GIT_URL;
  pkg.metadata = {
    ...(pkg.metadata ?? {}),
    [PUBLISH_PREFIX]: {
      "zed-crate": crate.name,
      "zed-version": crate.version,
      "zed-rev": zedSha,
    },
  };

  const lib: Toml = { ...(source.lib ?? {}) };
  lib.name ??= crate.name;

  const out: Toml = { package: pkg, lib };
  if (source.features !== undefined) out.features = source.features;

  const rewriteTable = (table: Toml): Toml => {
    const rewritten: Toml = {};
    for (const [name, spec] of Object.entries(table)) {
      let entry = spec;
      if (isPlainObject(spec) && !spec.workspace && spec.path !== undefined) {
        const target = pathDepTarget(crate.relDir, spec);
        const dep = cratesByDir.get(target ?? "");
        if (dep === undefined) {
          throw new BumpError(
            `${crate.name}: path dependency \`${name}\` (${target}) is not being published`,
          );
        }
        entry = {
          path: relative(crate.relDir, dep.relDir),
          package: dep.publishedName,
          version: `=${version}`,
          ...withoutSource(spec),
        };
      }
      rewritten[name] = entry;
    }
    return rewritten;
  };

  for (const table of DEP_TABLES) {
    if (source[table] !== undefined) out[table] = rewriteTable(source[table]);
  }
  if (crate.name === "gpui_macros") {
    const dependencies: Toml = (out.dependencies ??= {});
    const existing = dependencies[FACADE_PATH_DEPENDENCY];
    if (existing !== undefined) {
      throw new BumpError(
        `gpui_macros already depends on \`${FACADE_PATH_DEPENDENCY}\`; ` +
          "update installFacadeAwareMacroPaths in script/bump-gpui.ts",
      );
    }
    dependencies[FACADE_PATH_DEPENDENCY] = "3";
  }
  if (source.target !== undefined) {
    const targets: Toml = {};
    for (const [cfg, cfgTables] of Object.entries(source.target as Toml)) {
      const kept: Toml = {};
      for (const [table, value] of Object.entries(cfgTables as Toml)) {
        if (table !== DEV_DEP_TABLE) kept[table] = rewriteTable(value as Toml);
      }
      if (Object.keys(kept).length > 0) targets[cfg] = kept;
    }
    if (Object.keys(targets).length > 0) out.target = targets;
  }

  for (const [key, value] of Object.entries(source)) {
    if (!(key in out) && ![DEV_DEP_TABLE, "target", "workspace"].includes(key))
      out[key] = value;
  }
  return out;
}

function workspaceManifest(
  ws: Workspace,
  crates: Crate[],
  version: string,
): Toml {
  const zedWs = ws.manifest.workspace;
  const byDir = new Map(crates.map((c) => [c.relDir, c]));
  const dependencies: Toml = {};
  for (const [name, spec] of Object.entries(workspaceDependencies(ws))) {
    if (isPlainObject(spec) && spec.path !== undefined) {
      const crate = byDir.get(normalize(spec.path));
      if (crate === undefined) continue;
      dependencies[name] = {
        path: crate.relDir,
        package: crate.publishedName,
        version: `=${version}`,
        ...withoutSource(spec),
      };
    } else {
      dependencies[name] = spec;
    }
  }

  return {
    workspace: {
      resolver: zedWs.resolver ?? "2",
      members: crates.map((c) => c.relDir),
      package: { ...(zedWs.package ?? {}), publish: true },
      dependencies,
      lints: zedWs.lints ?? {},
    },
  };
}

// ---------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------

function copyCrate(source: string, destination: string) {
  cpSync(source, destination, {
    recursive: true,
    dereference: true,
    force: true,
    filter: (path) => {
      const name = path.split("/").at(-1);
      if (name === "target" || name === ".git") return false;
      try {
        statSync(path); // drops dangling symlinks, which `dereference` cannot copy
        return true;
      } catch {
        return false;
      }
    },
  });
}

function stageWorkspace(
  ws: Workspace,
  crates: Crate[],
  version: string,
  zedSha: string,
): string {
  const staging = join(WORK_DIR, "workspace");
  if (existsSync(staging)) {
    for (const entry of readdirSync(staging)) {
      if (entry === "target") continue; // keep the build cache between runs
      rmSync(join(staging, entry), { recursive: true, force: true });
    }
  }
  mkdirSync(staging, { recursive: true });

  for (const crate of crates)
    copyCrate(join(ws.root, crate.relDir), join(staging, crate.relDir));

  const cratesByDir = new Map(crates.map((c) => [c.relDir, c]));
  for (const crate of crates) {
    const manifest = crateManifest(crate, cratesByDir, version, zedSha);
    writeFileSync(
      join(staging, crate.relDir, "Cargo.toml"),
      tomlDump(manifest),
    );
  }
  installFacadeAwareMacroPaths(staging, crates, zedSha);
  makeDeclarativeMacrosCrateRelative(staging, crates, zedSha);
  vendorGpuiSourcesForApple(staging, crates, zedSha);
  carryLicenseFiles(ws.root, staging, crates);

  writeFileSync(
    join(staging, "Cargo.toml"),
    tomlDump(workspaceManifest(ws, crates, version)),
  );
  const lock = join(ws.root, "Cargo.lock");
  if (existsSync(lock)) cpSync(lock, join(staging, "Cargo.lock"));
  for (const entry of readdirSync(ws.root)) {
    if (
      entry.startsWith("LICENSE") &&
      statSync(join(ws.root, entry)).isFile()
    ) {
      cpSync(join(ws.root, entry), join(staging, entry));
    }
  }

  const summary = {
    version,
    zed: { url: ZED_GIT_URL, rev: zedSha },
    crates: crates.map((c) => ({
      name: c.publishedName,
      zed_name: c.name,
      zed_version: c.version,
      path: c.relDir,
      dropped_dependencies: c.prunedDeps,
      dropped_features: c.prunedFeatures,
    })),
  };
  writeFileSync(
    join(WORK_DIR, "gpui-pre.json"),
    `${JSON.stringify(summary, null, 2)}\n`,
  );
  return staging;
}

const FACADE_PATH_MODULE = "gpui_pre_facade_paths";
const FACADE_PATH_MARKER = `mod ${FACADE_PATH_MODULE};`;
const PROC_MACRO_ATTRIBUTE = /^#\[proc_macro(?:_derive\([^\n]*\)|_attribute)?\]$/;

/**
 * Declarative macros cannot be repaired by the proc-macro output rewriter:
 * names in their expansion must resolve before a derive macro can run. Make
 * the known `actions!` derive crate-relative so it also works when re-exported
 * through gpui-kit (or when gpui-pre itself is renamed by a consumer).
 */
function makeDeclarativeMacrosCrateRelative(
  staging: string,
  crates: Crate[],
  zedSha: string,
) {
  const gpui = crates.find((crate) => crate.name === "gpui");
  if (gpui === undefined)
    throw new BumpError("gpui is missing from the staged crate set");
  const actionPath = join(staging, gpui.relDir, "src/action.rs");
  if (!existsSync(actionPath))
    throw new BumpError("gpui declarative macro source does not exist: src/action.rs");
  const source = readFileSync(actionPath, "utf8");
  writeFileSync(
    actionPath,
    modificationNotice(zedSha, "the `actions!` derive paths are crate-relative") +
      rewriteDeclarativeMacroPaths(source),
  );
  logInfo("gpui: made actions! derive paths crate-relative");
}

function rewriteDeclarativeMacroPaths(source: string): string {
  const needle = "gpui::Action)]";
  // Only the `actions!` macro body expands in the caller's crate; gpui's own
  // modules also derive `gpui::Action` (through `extern crate self as gpui`)
  // and must keep that spelling, since `$crate` is meaningless outside a macro.
  const header = "macro_rules! actions {";
  const start = source.indexOf(header);
  if (start === -1) throw new BumpError("gpui src/action.rs no longer defines `macro_rules! actions`");
  let depth = 0;
  let end = -1;
  for (let index = start + header.length - 1; index < source.length; index++) {
    if (source[index] === "{") depth++;
    else if (source[index] === "}" && --depth === 0) {
      end = index + 1;
      break;
    }
  }
  if (end === -1) throw new BumpError("gpui src/action.rs: unbalanced `macro_rules! actions` block");
  const body = source.slice(start, end);
  if (!body.includes(needle)) {
    throw new BumpError(`gpui actions! no longer derives \`${needle}\`; update rewriteDeclarativeMacroPaths in script/bump-gpui.ts`);
  }
  return source.slice(0, start) + body.replaceAll(needle, "$crate::Action)]") + source.slice(end);
}

/**
 * Wrap every upstream proc-macro entry point at staging time. The wrapper
 * rewrites only generated path heads (`gpui::` and `gpui_platform::`), so
 * ordinary identifiers and the upstream implementations remain untouched.
 *
 * This intentionally transforms the discovered upstream entry points instead
 * of maintaining a hand-written list. Strict signature and count checks make
 * a Zed layout/API change stop the publish rather than silently miss a macro.
 */
function wrapProcMacroEntrypoints(source: string): {
  source: string;
  count: number;
} {
  const lines = source.split("\n");
  const output: string[] = [];
  let count = 0;
  let attributes = 0;

  for (let index = 0; index < lines.length; index++) {
    const attribute = lines[index];
    if (!PROC_MACRO_ATTRIBUTE.test(attribute)) {
      output.push(attribute);
      continue;
    }
    attributes++;

    const between: string[] = [];
    let signatureIndex = index + 1;
    while (signatureIndex < lines.length && lines[signatureIndex].startsWith("#[")) {
      between.push(lines[signatureIndex]);
      signatureIndex++;
    }
    const signature = lines[signatureIndex] ?? "";
    const match = /^pub fn ([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)\) -> TokenStream \{$/.exec(
      signature,
    );
    if (match === null) {
      throw new BumpError(
        `gpui_macros proc-macro entry after \`${attribute}\` has an unsupported signature: \`${signature}\``,
      );
    }
    const [, name, parameters] = match;
    const callArguments = parameters
      .split(",")
      .map((parameter) => parameter.trim())
      .filter(Boolean)
      .map((parameter) => {
        const argument = /^([A-Za-z_][A-Za-z0-9_]*)\s*:/.exec(parameter)?.[1];
        if (argument === undefined)
          throw new BumpError(
            `gpui_macros proc-macro \`${name}\` has an unsupported parameter: \`${parameter}\``,
          );
        return argument;
      });

    // `#[cfg]` gates written before or after the `#[proc_macro*]` attribute
    // must also gate the wrapped body: upstream gates both the entry point and
    // the module it calls (e.g. `derive_inspector_reflection`), so an ungated
    // wrapper would reference a module that was configured out.
    const preceding: string[] = [];
    for (let back = output.length - 1; back >= 0 && output[back].startsWith("#["); back--) {
      preceding.unshift(output[back]);
    }
    const gates = [...preceding, ...between].filter((line) => line.startsWith("#[cfg"));

    output.push(attribute, ...between, signature);
    output.push(
      `    ${FACADE_PATH_MODULE}::rewrite(__gpui_pre_${name}(${callArguments.join(", ")}))`,
      "}",
      "",
      ...gates,
      `fn __gpui_pre_${name}(${parameters}) -> TokenStream {`,
    );
    index = signatureIndex;
    count++;
  }

  if (attributes === 0)
    throw new BumpError(
      "gpui_macros has no #[proc_macro*] entry points; update installFacadeAwareMacroPaths in script/bump-gpui.ts",
    );
  if (count !== attributes)
    throw new BumpError(
      `gpui_macros found ${attributes} proc-macro attributes but wrapped ${count}`,
    );
  return { source: output.join("\n"), count };
}

function facadePathModuleSource(): string {
  return `// Injected by gpui-kit's script/bump-gpui.ts. Keep fixes in that script.
use proc_macro::{Group, Ident, Literal, Punct, Spacing, TokenStream, TokenTree};
use proc_macro_crate::{crate_name, FoundCrate};

enum FacadePath {
    Itself,
    Name(String),
}

pub(crate) fn rewrite(stream: TokenStream) -> TokenStream {
    let facade = match crate_name("gpui-kit") {
        Ok(FoundCrate::Name(name)) => Some(FacadePath::Name(name)),
        Ok(FoundCrate::Itself) => Some(FacadePath::Itself),
        Err(_) => None,
    };
    rewrite_stream(stream, facade.as_ref())
}

fn rewrite_stream(stream: TokenStream, facade: Option<&FacadePath>) -> TokenStream {
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut output = TokenStream::new();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => {
                let mut rewritten = Group::new(group.delimiter(), rewrite_stream(group.stream(), facade));
                rewritten.set_span(group.span());
                output.extend([TokenTree::Group(rewritten)]);
            }
            TokenTree::Ident(ident)
                if is_path_head(&tokens, index)
                    && (ident.to_string() == "gpui" || ident.to_string() == "gpui_platform") =>
            {
                append_path_head(&mut output, ident, facade);
            }
            TokenTree::Literal(literal) => {
                output.extend([TokenTree::Literal(rewrite_literal(literal, facade))]);
            }
            token => output.extend([token.clone()]),
        }
    }
    output
}

fn is_path_head(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index + 1), Some(TokenTree::Punct(first)) if first.as_char() == ':')
        && matches!(tokens.get(index + 2), Some(TokenTree::Punct(second)) if second.as_char() == ':')
}

fn append_path_head(output: &mut TokenStream, original: &Ident, facade: Option<&FacadePath>) {
    let name = match facade {
        Some(FacadePath::Itself) => "crate",
        Some(FacadePath::Name(name)) => name,
        None => {
            output.extend([TokenTree::Ident(original.clone())]);
            return;
        }
    };
    output.extend([TokenTree::Ident(Ident::new(name, original.span()))]);
    if original.to_string() == "gpui_platform" {
        output.extend([
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            TokenTree::Ident(Ident::new("platform", original.span())),
        ]);
    }
}

fn rewrite_literal(literal: &Literal, facade: Option<&FacadePath>) -> Literal {
    let (gpui, platform) = match facade {
        Some(FacadePath::Itself) => ("crate::".into(), "crate::platform::".into()),
        Some(FacadePath::Name(name)) =>
            (format!("::{name}::"), format!("::{name}::platform::")),
        None => return literal.clone(),
    };
    let text = literal.to_string();
    let rewritten = text
        .replace("::gpui_platform::", &platform)
        .replace("::gpui::", &gpui);
    if rewritten == text { return literal.clone() }
    let Ok(parsed) = rewritten.parse::<TokenStream>() else { return literal.clone() };
    let mut tokens = parsed.into_iter();
    match (tokens.next(), tokens.next()) {
        (Some(TokenTree::Literal(literal)), None) => literal,
        _ => literal.clone(),
    }
}
`;
}

function installFacadeAwareMacroPaths(
  staging: string,
  crates: Crate[],
  zedSha: string,
) {
  const macros = crates.find((crate) => crate.name === "gpui_macros");
  if (macros === undefined)
    throw new BumpError("gpui_macros is missing from the staged crate set");
  const crateDir = join(staging, macros.relDir);
  const relativeLib = String(macros.manifest.lib?.path ?? "src/lib.rs");
  const libPath = join(crateDir, relativeLib);
  if (!existsSync(libPath))
    throw new BumpError(`gpui_macros library entry does not exist: ${relativeLib}`);
  const source = readFileSync(libPath, "utf8");
  if (source.includes(FACADE_PATH_MARKER))
    throw new BumpError(`gpui_macros already declares \`${FACADE_PATH_MARKER}\``);
  const wrapped = wrapProcMacroEntrypoints(source);
  writeFileSync(
    libPath,
    modificationNotice(zedSha, "the proc-macro entry points resolve gpui through a facade") +
      `${FACADE_PATH_MARKER}\n${wrapped.source}`,
  );
  writeFileSync(
    join(dirname(libPath), `${FACADE_PATH_MODULE}.rs`),
    modificationNotice(zedSha, "this module is added by the script") +
      facadePathModuleSource(),
  );
  logInfo(`gpui_macros: made ${wrapped.count} proc-macro entry points facade-aware`);
}

function runSelfTest() {
  const declarativeFixture = [
    "macro_rules! actions {",
    "    ($ns:path, [ $($name:ident),* ]) => { $( #[derive(gpui::Action)] pub struct $name; )* };",
    "    ([ $($name:ident),* ]) => { $( #[derive(gpui::Action)] pub struct $name; )* };",
    "}",
    "mod builtin {",
    "    #[derive(gpui::Action)]",
    "    pub struct Unbind;",
    "}",
  ].join("\n");
  const declarativeTransformed = rewriteDeclarativeMacroPaths(declarativeFixture);
  if (
    declarativeTransformed.split("$crate::Action)]").length - 1 !== 2 ||
    // The derive on gpui's own struct, outside the macro, keeps its path.
    declarativeTransformed.split("gpui::Action)]").length - 1 !== 1
  ) {
    throw new BumpError("self-test did not make only the actions! body crate-relative");
  }
  const fixture = `mod implementation;
use proc_macro::TokenStream;
#[proc_macro_derive(Action, attributes(action))]
pub fn derive_action(input: TokenStream) -> TokenStream {
    implementation::derive_action(input)
}
#[proc_macro_attribute]
#[doc(hidden)]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    implementation::test(args, item)
}
#[cfg(any(feature = "inspector", debug_assertions))]
mod gated;
/// Docs stay on the entry point.
#[cfg(any(feature = "inspector", debug_assertions))]
#[proc_macro_attribute]
pub fn gated(args: TokenStream, input: TokenStream) -> TokenStream {
    gated::gated(args, input)
}
fn helper(input: TokenStream) -> TokenStream { input }
`;
  const transformed = wrapProcMacroEntrypoints(fixture);
  if (transformed.count !== 3) throw new BumpError("self-test wrapped the wrong macro count");
  for (const expected of [
    "rewrite(__gpui_pre_derive_action(input))",
    "fn __gpui_pre_test(args: TokenStream, item: TokenStream)",
    // The wrapped body carries the entry point's `#[cfg]` gate, and only that.
    '\n#[cfg(any(feature = "inspector", debug_assertions))]\nfn __gpui_pre_gated(args: TokenStream, input: TokenStream)',
    "\nfn __gpui_pre_derive_action(input: TokenStream)",
    "fn helper(input: TokenStream)",
  ]) {
    if (!transformed.source.includes(expected))
      throw new BumpError(`self-test output is missing \`${expected}\``);
  }
  let driftFailed = false;
  try {
    wrapProcMacroEntrypoints("#[proc_macro]\npub unsafe fn changed() -> TokenStream {");
  } catch (error) {
    driftFailed = error instanceof BumpError;
  }
  if (!driftFailed) throw new BumpError("self-test did not reject an upstream signature change");
  const pathRewriter = facadePathModuleSource();
  for (const expected of [
    'crate_name("gpui-kit")',
    'ident.to_string() == "gpui_platform"',
    'Some(FacadePath::Itself) => "crate"',
    'Some(FacadePath::Itself) => ("crate::".into(), "crate::platform::".into())',
  ]) {
    if (!pathRewriter.includes(expected))
      throw new BumpError(`self-test path rewriter is missing \`${expected}\``);
  }
  logSuccess("Facade-aware gpui_macros transformation self-test passed");
}

const GPUI_APPLE_SIBLING = '.join("../gpui")';
const GPUI_APPLE_VENDORED = '.join("vendor/gpui")';

/**
 * Make `gpui_apple`'s build script work outside the Zed workspace.
 *
 * Its `build.rs` feeds a few `gpui` source files to cbindgen to generate the
 * Metal shader header, and it finds them at `../gpui`. A crate unpacked from
 * crates.io has no such sibling, so copy exactly the files it names into the
 * crate and point the build script at the copy.
 */
function vendorGpuiSourcesForApple(
  staging: string,
  crates: Crate[],
  zedSha: string,
) {
  const apple = crates.find((c) => c.name === "gpui_apple");
  const gpui = crates.find((c) => c.name === "gpui");
  if (apple === undefined || gpui === undefined) return;
  const buildRs = join(staging, apple.relDir, "build.rs");
  const text = readFileSync(buildRs, "utf8");
  if (!text.includes(GPUI_APPLE_SIBLING)) {
    throw new BumpError(
      "crates/gpui_apple/build.rs no longer locates gpui with `../gpui`; " +
        "update vendorGpuiSourcesForApple in script/bump-gpui.ts",
    );
  }
  const sources = [...text.matchAll(/gpui_dir\.join\("([^"]+)"\)/g)].map(
    (m) => m[1],
  );
  if (sources.length === 0) {
    throw new BumpError(
      "crates/gpui_apple/build.rs lists no `gpui_dir.join(...)` sources; update script/bump-gpui.ts",
    );
  }
  const vendor = join(staging, apple.relDir, "vendor", "gpui");
  for (const rel of sources) {
    const source = join(staging, gpui.relDir, rel);
    if (!existsSync(source))
      throw new BumpError(
        `gpui_apple/build.rs needs ${rel}, which gpui does not have`,
      );
    const destination = join(vendor, rel);
    mkdirSync(dirname(destination), { recursive: true });
    cpSync(source, destination);
  }
  writeFileSync(
    buildRs,
    modificationNotice(zedSha, "the gpui sources it reads are vendored under `vendor/gpui`") +
      text.replaceAll(GPUI_APPLE_SIBLING, GPUI_APPLE_VENDORED),
  );
  logInfo(
    `gpui_apple: vendored ${sources.length} gpui source files for its shader bindings`,
  );
}

// ---------------------------------------------------------------------------
// License and attribution
// ---------------------------------------------------------------------------

/** The license GPUI is released under, and the only one a snapshot may carry. */
const GPUI_LICENSE = "Apache-2.0";

/**
 * Licenses a dependency of the staged workspace must not have. Zed's
 * application crates are GPL-3.0-or-later and sit in the same workspace as
 * gpui, so a new internal dependency can pull one in without anyone noticing;
 * a third-party crate can change its terms between snapshots just as quietly.
 */
const COPYLEFT_LICENSE = /\b(?:A?GPL|LGPL|SSPL|EUPL|OSL|CPAL|BUSL|CC-BY-SA)\b/i;

/** Every alternative of an SPDX `OR` expression is copyleft. */
function isCopyleft(license: string): boolean {
  return license
    .split(/\s+OR\s+|\//i)
    .every((alternative) => COPYLEFT_LICENSE.test(alternative));
}

/** The crate's `license`, following `license.workspace = true`. */
function packageLicense(crate: Crate): string | undefined {
  const license = crate.manifest.package?.license;
  if (typeof license === "string") return license;
  if (isPlainObject(license) && license.workspace === true) {
    return WORKSPACE_PACKAGE_LICENSE;
  }
  return undefined;
}

let WORKSPACE_PACKAGE_LICENSE: string | undefined;

/**
 * Apache-2.0 §4: a redistribution keeps the license text and every copyright
 * notice, includes the NOTICE file if the work has one, and marks the files it
 * changed. Copyright notices live in Zed's source files, which are copied
 * untouched; this puts the license and any NOTICE beside every crate, and the
 * functions above stamp the files the script rewrites.
 */
function carryLicenseFiles(zed: string, staging: string, crates: Crate[]) {
  const licenseFile = join(zed, "LICENSE-APACHE");
  if (!existsSync(licenseFile)) {
    throw new BumpError(
      "Zed has no LICENSE-APACHE at its root; check the license before publishing",
    );
  }
  const notices = readdirSync(zed).filter(
    (entry) =>
      entry.startsWith("NOTICE") && statSync(join(zed, entry)).isFile(),
  );
  let copied = 0;
  for (const crate of crates) {
    const dir = join(staging, crate.relDir);
    const destination = join(dir, "LICENSE-APACHE");
    if (!existsSync(destination)) {
      cpSync(licenseFile, destination);
      copied += 1;
    }
    for (const notice of notices) cpSync(join(zed, notice), join(dir, notice));
  }
  logInfo(
    `LICENSE-APACHE travels with every crate (${copied} added)` +
      (notices.length ? `, with ${notices.join(", ")}` : "; Zed ships no NOTICE"),
  );
}

/** A one-line header for a file the script rewrites (Apache-2.0 §4(b)). */
function modificationNotice(zedSha: string, change: string): string {
  return `// Modified for ${PUBLISH_PREFIX} (snapshot of zed@${zedSha.slice(0, 7)}): ${change}.\n`;
}

/**
 * Check the staged workspace's whole dependency graph, not only the crates
 * being published: what reaches a consumer is the closure, and it moves with
 * every snapshot.
 */
async function auditLicenses(staging: string, crates: Crate[]) {
  for (const crate of crates) {
    if (!existsSync(join(staging, crate.relDir, "LICENSE-APACHE")))
      throw new BumpError(`${crate.name}: LICENSE-APACHE is missing from the staged crate`);
  }
  // Not `--locked`: the lock file is Zed's, and the staged workspace is a
  // subset of it with pruned dependencies, so it has to be updated here the
  // way `cargo publish --dry-run` updates it in the next step.
  const cmd = ["cargo", "metadata", "--format-version", "1"];
  console.log(dim(`$ (cd ${staging} && ${cmd.join(" ")})`));
  const process_ = Bun.spawn(cmd, { cwd: staging, stdout: "pipe", stderr: "inherit" });
  const output = await new Response(process_.stdout).text();
  if ((await process_.exited) !== 0)
    throw new BumpError("cargo metadata failed on the staged workspace");
  const metadata = JSON.parse(output);
  const members = new Set<string>(metadata.workspace_members);
  const copyleft: string[] = [];
  const undeclared: string[] = [];
  for (const pkg of metadata.packages as Toml[]) {
    if (members.has(pkg.id)) continue;
    const license: string | null = pkg.license;
    if (license === null || license === undefined || license === "") {
      undeclared.push(`${pkg.name} ${pkg.version}`);
    } else if (isCopyleft(license)) {
      copyleft.push(`${pkg.name} ${pkg.version} (${license})`);
    }
  }
  if (copyleft.length > 0) {
    throw new BumpError(
      "copyleft crates in the dependency graph; a snapshot must not change " +
        `the terms consumers get:\n  ${copyleft.join("\n  ")}`,
    );
  }
  if (undeclared.length > 0) {
    logWarn(
      `${undeclared.length} dependencies declare a license file instead of an ` +
        `SPDX expression; check them by hand: ${undeclared.join(", ")}`,
    );
  }
  logSuccess(
    `${crates.length} crates are ${GPUI_LICENSE}; ` +
      `${metadata.packages.length - members.size} dependencies carry no copyleft license`,
  );
}

// ---------------------------------------------------------------------------
// crates.io
// ---------------------------------------------------------------------------

async function cratesIoGet(path: string): Promise<Toml | undefined> {
  let response: Response;
  try {
    response = await fetch(`${CRATES_IO_API}/${path}`, {
      headers: { "User-Agent": USER_AGENT },
    });
  } catch (error) {
    throw new BumpError(`cannot reach crates.io: ${(error as Error).message}`);
  }
  if (response.status === 404) return undefined;
  if (!response.ok)
    throw new BumpError(`crates.io returned ${response.status} for ${path}`);
  return (await response.json()) as Toml;
}

/**
 * The next `<base>.<N>` to publish, from what crates.io already holds.
 *
 * The patch number continues from the highest one any of the crates has; if
 * a previous run stopped part-way (the rate limit, a crash) some crates lack
 * that number, and the run resumes at it instead of starting a new one.
 * Yanked versions still occupy their number.
 */
async function nextVersion(base: string, crates: Crate[]): Promise<string> {
  const pattern = new RegExp(`^${base.replaceAll(".", "\\.")}\\.(\\d+)$`);
  const numbers = new Map<Crate, Set<number>>();
  let highest = -1;
  for (const crate of crates) {
    const data = await cratesIoGet(`${crate.publishedName}/versions`);
    const published = new Set<number>();
    for (const version of data?.versions ?? []) {
      const match = pattern.exec(String(version.num));
      if (match) published.add(Number(match[1]));
    }
    numbers.set(crate, published);
    highest = Math.max(highest, ...published);
    await Bun.sleep(200); // be polite to the crates.io API
  }
  if (highest < 0) return `${base}.0`;
  const incomplete = crates.filter(
    (crate) => !numbers.get(crate)!.has(highest),
  );
  if (incomplete.length > 0) {
    logInfo(
      `Resuming ${base}.${highest}: ${incomplete.length} crates are still missing it`,
    );
    return `${base}.${highest}`;
  }
  return `${base}.${highest + 1}`;
}

async function versionIsPublished(
  name: string,
  version: string,
): Promise<boolean> {
  const data = await cratesIoGet(`${name}/${version}`);
  return data?.version !== undefined && !data.version.yanked;
}

async function unpublished(crates: Crate[], version: string): Promise<Crate[]> {
  const pending: Crate[] = [];
  for (const crate of crates) {
    if (!(await versionIsPublished(crate.publishedName, version)))
      pending.push(crate);
    await Bun.sleep(200); // be polite to the crates.io API
  }
  return pending;
}

const RATE_LIMIT_MARKERS = [
  "429",
  "Too Many Requests",
  "too many new crates",
  "too many crates",
];

/** When crates.io says to try again, return that instant. */
function rateLimitDeadline(output: string): Date | undefined {
  if (!RATE_LIMIT_MARKERS.some((marker) => output.includes(marker)))
    return undefined;
  const match = /try again after ([^.\n]+?)(?: or |\.|$)/m.exec(output);
  if (match) {
    const parsed = new Date(match[1].trim());
    if (!Number.isNaN(parsed.getTime()))
      return new Date(parsed.getTime() + 15_000);
  }
  return new Date(Date.now() + RATE_LIMIT_FALLBACK_MS);
}

async function waitUntil(deadline: Date) {
  for (;;) {
    const remaining = deadline.getTime() - Date.now();
    if (remaining <= 0) break;
    const total = Math.floor(remaining / 1000);
    const minutes = String(Math.floor(total / 60)).padStart(2, "0");
    const seconds = String(total % 60).padStart(2, "0");
    process.stdout.write(
      `\r${paint("36", "ℹ")} crates.io rate limit; retrying in ${minutes}:${seconds} `,
    );
    await Bun.sleep(Math.min(30_000, remaining));
  }
  console.log();
}

async function publish(
  staging: string,
  crates: Crate[],
  version: string,
  wait: boolean,
) {
  for (;;) {
    const pending = await unpublished(crates, version);
    if (pending.length === 0) {
      logSuccess(
        `All ${crates.length} crates are on crates.io at ${bold(version)}`,
      );
      return;
    }
    const already = crates.filter((c) => !pending.includes(c));
    if (already.length > 0)
      logInfo(
        `Skipping ${already.length} crates already published at ${version}`,
      );
    logInfo(
      `Publishing ${pending.length} crates: ${pending.map((c) => c.publishedName).join(", ")}`,
    );

    const cmd = [
      "cargo",
      "publish",
      "--workspace",
      "--no-verify",
      "--allow-dirty",
    ];
    for (const crate of already) cmd.push("--exclude", crate.publishedName);
    const { code, output } = await runStreaming(cmd, staging);
    if (code === 0) {
      logSuccess(`Published ${pending.length} crates`);
      return;
    }

    const deadline = rateLimitDeadline(output);
    if (deadline === undefined)
      throw new BumpError(
        "cargo publish failed; fix the error above and re-run to resume",
      );
    if (!wait) {
      throw new BumpError(
        "crates.io rate limit reached (new crates are limited per 10 minutes); re-run this command later to resume",
      );
    }
    await waitUntil(deadline);
  }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const USAGE = `Usage: script/bump-gpui.ts [VERSION] [options]

Publish Zed's GPUI crates to crates.io as ${PUBLISH_PREFIX}-*.

Arguments:
  VERSION           publish exactly this version instead of ${VERSION}.<N>,
                    where N continues from what crates.io already has

Options:
  --self-test       test the gpui_macros source transformation and stop
  --rev REV         Zed branch, tag or commit (default: ${ZED_DEFAULT_REV})
  --zed PATH        use this Zed checkout instead of fetching one
  --dry-run         stage and verify, but do not publish
  --stage-only      stage the workspace and stop
  --no-verify       skip \`cargo publish --dry-run\` verification
  --no-wait         abort instead of waiting on the crates.io rate limit
  --skip-kit-check  do not build and test this repository against the staged crates
  -h, --help        show this help
`;

interface Args {
  version?: string;
  rev: string;
  zed?: string;
  dryRun: boolean;
  stageOnly: boolean;
  noVerify: boolean;
  noWait: boolean;
  skipKitCheck: boolean;
  selfTest: boolean;
}

function parseCommandLine(argv: string[]): Args {
  let parsed: ReturnType<typeof parseArgs>;
  try {
    parsed = parseArgs({
      args: argv,
      allowPositionals: true,
      options: {
        rev: { type: "string", default: ZED_DEFAULT_REV },
        zed: { type: "string" },
        "dry-run": { type: "boolean", default: false },
        "stage-only": { type: "boolean", default: false },
        "no-verify": { type: "boolean", default: false },
        "no-wait": { type: "boolean", default: false },
        "skip-kit-check": { type: "boolean", default: false },
        "self-test": { type: "boolean", default: false },
        help: { type: "boolean", short: "h", default: false },
      },
    });
  } catch (error) {
    throw new BumpError(`${(error as Error).message}\n\n${USAGE}`);
  }
  if (parsed.values.help) {
    console.log(USAGE);
    process.exit(0);
  }
  if (parsed.positionals.length > 1)
    throw new BumpError(
      `unexpected argument \`${parsed.positionals[1]}\`\n\n${USAGE}`,
    );
  const version = parsed.positionals[0];
  if (version !== undefined && !SEMVER.test(version))
    throw new BumpError(`\`${version}\` is not a valid semver version`);
  return {
    version,
    rev: parsed.values.rev as string,
    zed: parsed.values.zed as string | undefined,
    dryRun: parsed.values["dry-run"] as boolean,
    stageOnly: parsed.values["stage-only"] as boolean,
    noVerify: parsed.values["no-verify"] as boolean,
    noWait: parsed.values["no-wait"] as boolean,
    skipKitCheck: parsed.values["skip-kit-check"] as boolean,
    selfTest: parsed.values["self-test"] as boolean,
  };
}

/**
 * Build and test this repository against the staged crates before anything is
 * uploaded. Applications depend on `gpui-pre` with a caret requirement, so a
 * snapshot whose API drifted away from `gpui-component` would reach them on
 * their next `cargo update`; this turns that into a failed release instead.
 *
 * The staged crates are injected with `--config patch.crates-io…` so no file
 * in the repository changes. They are patched from a copy outside the
 * repository: a path dependency under the workspace root would be treated as
 * a member of this workspace and lose its own `workspace = true` inheritance.
 * Cargo keeps a locked version when it still satisfies the requirement, so
 * the published crates are moved to the staged version in a scratch copy of
 * `Cargo.lock`, which is restored afterwards.
 */
async function verifyKitAgainstStaging(staging: string, crates: Crate[], version: string) {
  const mirror = join(tmpdir(), `${PUBLISH_PREFIX}-kit-check`);
  rmSync(mirror, { recursive: true, force: true });
  cpSync(staging, mirror, {
    recursive: true,
    // Leave the staged workspace's own build directory behind; the path is
    // judged relative to the staging root, which itself lives under `target/`.
    filter: (path) => relative(staging, path).split("/")[0] !== "target",
  });
  const patches = crates.flatMap((crate) => [
    "--config",
    `patch.crates-io.${crate.publishedName}.path=${JSON.stringify(join(mirror, crate.relDir))}`,
  ]);
  const lockPath = join(REPO_ROOT, "Cargo.lock");
  const lockBackup = existsSync(lockPath) ? readFileSync(lockPath) : undefined;
  const locked = new Set(
    [...(lockBackup?.toString() ?? "").matchAll(/^name = "([^"]+)"$/gm)].map((m) => m[1]),
  );
  try {
    const toUpdate = crates.filter((crate) => locked.has(crate.publishedName));
    if (toUpdate.length > 0) {
      await run(
        ["cargo", "update", ...patches, ...toUpdate.flatMap((crate) => ["-p", crate.publishedName])],
        { cwd: REPO_ROOT },
      );
    }

    const metadata = JSON.parse(
      await run(["cargo", "metadata", "--format-version", "1", ...patches], { cwd: REPO_ROOT, capture: true }),
    ) as { packages: { name: string; version: string; manifest_path: string }[] };
    const foreign = crates
      .map((crate) => metadata.packages.find((pkg) => pkg.name === crate.publishedName))
      .filter((pkg): pkg is NonNullable<typeof pkg> => pkg !== undefined && !pkg.manifest_path.startsWith(mirror));
    if (foreign.length > 0) {
      const detail = foreign.map((pkg) => `${pkg.name} ${pkg.version} from ${pkg.manifest_path}`).join("\n  ");
      throw new BumpError(
        `the workspace did not resolve to the staged ${version}; its Cargo.toml requirement rejects it:\n  ${detail}`,
      );
    }

    // The same commands the repository's CI runs.
    const commands = [
      ["cargo", "check", ...patches, "--workspace", "--all-targets"],
      ["cargo", "clippy", ...patches, "-p", "gpui-component", "-p", "gpui-component-story", "-p", "gpui-kit-assets", "-p", "gpui-kit", "--", "--deny", "warnings"],
      ["cargo", "test", ...patches, "--workspace", "--exclude", "gpui-shell", "--features", "gpui-component-story/test-support"],
    ];
    for (const cmd of commands) {
      const { code } = await runStreaming(cmd, REPO_ROOT);
      if (code !== 0) {
        throw new BumpError(
          `gpui-kit does not build or pass its tests against the staged gpui-pre ${version}; ` +
            "adapt the repository to the Zed changes before publishing",
        );
      }
    }
  } finally {
    if (lockBackup !== undefined) writeFileSync(lockPath, lockBackup);
    else if (existsSync(lockPath)) rmSync(lockPath);
  }
}

async function main(argv: string[]): Promise<number> {
  const args = parseCommandLine(argv);
  if (args.selfTest) {
    runSelfTest();
    return 0;
  }
  if (Bun.which("cargo") === null)
    throw new BumpError("cargo is not installed");

  const totalSteps = args.stageOnly ? 4 : args.dryRun ? 6 : 7;
  logHeader(`Publishing GPUI from Zed as ${PUBLISH_PREFIX}`);
  mkdirSync(WORK_DIR, { recursive: true });

  logStep(`1/${totalSteps}`, "Preparing the Zed checkout");
  const zed = await prepareZed(args.rev, args.zed);
  const zedSha = await zedRevision(zed);
  logSuccess(`Zed at ${bold(zedSha.slice(0, 12))}`);
  console.log();

  logStep(`2/${totalSteps}`, "Collecting the crates that gpui needs");
  const ws = loadWorkspace(zed);
  const crates = selectCrates(ws);
  const version = args.version ?? (await nextVersion(VERSION, crates));
  const width = Math.max(...crates.map((c) => c.name.length));
  for (const crate of crates)
    console.log(`    ${crate.name.padEnd(width)}  ->  ${crate.publishedName}`);
  logSuccess(
    `${crates.length} crates will be published as version ${bold(version)}`,
  );
  console.log();

  logStep(`3/${totalSteps}`, "Staging a standalone workspace");
  const staging = stageWorkspace(ws, crates, version, zedSha);
  logSuccess(`Workspace written to ${bold(staging)}`);
  logInfo(`Summary written to ${join(WORK_DIR, "gpui-pre.json")}`);
  console.log();

  logStep(`4/${totalSteps}`, "Auditing licenses");
  await auditLicenses(staging, crates);
  console.log();
  if (args.stageOnly) return 0;

  if (args.noVerify) {
    logWarn("Skipping verification (--no-verify)");
  } else {
    logStep(`5/${totalSteps}`, "Verifying with `cargo publish --dry-run`");
    const { code } = await runStreaming(
      ["cargo", "publish", "--workspace", "--dry-run", "--allow-dirty"],
      staging,
    );
    if (code !== 0)
      throw new BumpError(
        "verification failed; inspect the staged workspace and fix the issue",
      );
    logSuccess("Every crate packages and builds");
  }
  console.log();

  if (args.skipKitCheck) {
    logWarn("Skipping the gpui-kit compatibility check (--skip-kit-check)");
  } else {
    logStep(`6/${totalSteps}`, "Building and testing gpui-kit against the staged crates");
    await verifyKitAgainstStaging(staging, crates, version);
    logSuccess(`gpui-kit builds and passes its tests against gpui-pre ${version}`);
  }
  console.log();
  if (args.dryRun) {
    logInfo("Dry run complete; nothing was uploaded");
    return 0;
  }

  logStep(`7/${totalSteps}`, "Publishing to crates.io");
  await publish(staging, crates, version, !args.noWait);
  console.log();

  console.log(paint("1;32", `╔${"═".repeat(56)}╗`));
  console.log(
    `${paint("1;32", "║")}  ${bold(`🚀 ${PUBLISH_PREFIX} ${version} is live (zed@${zedSha.slice(0, 7)})`)}`,
  );
  console.log(paint("1;32", `╚${"═".repeat(56)}╝`));
  console.log();
  console.log("Depend on it with:");
  console.log();
  console.log("    [workspace.dependencies]");
  for (const crate of crates) {
    if (ROOT_CRATES.includes(crate.name)) {
      console.log(
        `    ${crate.name} = { package = "${crate.publishedName}", version = "=${version}" }`,
      );
    }
  }
  console.log();
  return 0;
}

try {
  process.exit(await main(process.argv.slice(2)));
} catch (error) {
  if (error instanceof BumpError) {
    logError(error.message);
    process.exit(1);
  }
  throw error;
}

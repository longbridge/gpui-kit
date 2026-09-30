#!/usr/bin/env bun
/**
 * Turn the repository's manifests into the ones gpui-kit publishes.
 *
 * Inside the repository, gpui-kit's crates take GPUI through the selectors in
 * `crates/backend` (`gpui = { package = "gpui-kit-backend-gpui", path = ... }`),
 * which pick gpui-pre or, behind the `gpui-fast` feature, gpui-fast. crates.io
 * accepts neither path dependencies without a version nor git dependencies, so
 * before `cargo publish` this script:
 *
 * 1. points every selector entry in the root `[workspace.dependencies]` back at
 *    the `gpui-pre-*` snapshot it defaults to, copying the pinned `gpui_pre*`
 *    entry (`gpui = { package = "gpui-pre", version = "=0.3.7" }`);
 * 2. drops the `gpui-fast` feature, and the comment above it, from every
 *    published crate: gpui-fast is for git dependencies of gpui-kit only.
 *
 * The published crates then depend on gpui-pre exactly as they did before the
 * selectors existed.
 *
 *     bun script/published-manifest.ts           rewrite the manifests in place
 *     bun script/published-manifest.ts --check   verify the rewrite, change nothing
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const ROOT_MANIFEST = join(REPO_ROOT, "Cargo.toml");

/** Each selector's workspace dependency and the `gpui-pre-*` entry it defaults to. */
const SELECTORS: Record<string, string> = {
  gpui: "gpui_pre",
  gpui_platform: "gpui_pre_platform",
  gpui_web: "gpui_pre_web",
  gpui_macros: "gpui_pre_macros",
  reqwest_client: "gpui_pre_reqwest_client",
  "sum-tree": "gpui_pre_sum_tree",
};
const SELECTOR_PACKAGE_PREFIX = "gpui-kit-backend";
const FEATURE = "gpui-fast";

type Toml = Record<string, any>;

function entryLine(lines: string[], key: string): number {
  const matches = lines.flatMap((line, index) => (/^\S+\s*=/.test(line) && line.split("=")[0].trim() === key ? [index] : []));
  if (matches.length !== 1) throw new Error(`expected one \`${key} = ...\` line in Cargo.toml, found ${matches.length}`);
  return matches[0];
}

function rewriteRoot(text: string): string {
  const lines = text.split("\n");
  for (const [selector, snapshot] of Object.entries(SELECTORS)) {
    const target = entryLine(lines, selector);
    const source = lines[entryLine(lines, snapshot)];
    if (!lines[target].includes(`"${SELECTOR_PACKAGE_PREFIX}`)) {
      throw new Error(`\`${selector}\` does not point at a ${SELECTOR_PACKAGE_PREFIX} selector: ${lines[target]}`);
    }
    lines[target] = `${selector} = ${source.slice(source.indexOf("=") + 1).trim()}`;
  }
  return lines.join("\n");
}

/** Remove the `gpui-fast = [...]` feature and the comment block right above it. */
function dropFeature(text: string): string {
  const lines = text.split("\n");
  const start = lines.findIndex((line) => line.startsWith(`${FEATURE} =`));
  if (start === -1) return text;
  let end = start;
  while (!lines[end].trimEnd().endsWith("]")) end += 1;
  let first = start;
  while (first > 0 && lines[first - 1].startsWith("#")) first -= 1;
  lines.splice(first, end - first + 1);
  return lines.join("\n");
}

function publishedManifests(root: Toml): string[] {
  const members: string[] = root.workspace?.members ?? [];
  return members
    .map((member) => join(REPO_ROOT, member, "Cargo.toml"))
    .filter((path) => {
      const manifest = Bun.TOML.parse(readFileSync(path, "utf8")) as Toml;
      return manifest.package?.publish === true;
    });
}

function problems(rootText: string, manifests: Map<string, string>): string[] {
  const found: string[] = [];
  const root = Bun.TOML.parse(rootText) as Toml;
  const dependencies: Toml = root.workspace?.dependencies ?? {};
  for (const [selector, snapshot] of Object.entries(SELECTORS)) {
    const spec = dependencies[selector];
    if (spec?.package !== dependencies[snapshot]?.package || spec?.version !== dependencies[snapshot]?.version) {
      found.push(`\`${selector}\` does not match \`${snapshot}\` after the rewrite`);
    }
  }
  for (const [path, text] of manifests) {
    const manifest = Bun.TOML.parse(text) as Toml;
    const name = manifest.package?.name ?? path;
    const features: Record<string, string[]> = manifest.features ?? {};
    if (FEATURE in features) found.push(`${name} still has the \`${FEATURE}\` feature`);
    for (const [feature, enables] of Object.entries(features)) {
      if (enables.some((item) => item.endsWith(`/${FEATURE}`))) {
        found.push(`${name}'s \`${feature}\` feature still enables \`${FEATURE}\``);
      }
    }
    if (text.includes(`${SELECTOR_PACKAGE_PREFIX}`)) found.push(`${name} names a ${SELECTOR_PACKAGE_PREFIX} crate`);
  }
  return found;
}

const check = process.argv.includes("--check");
const rootText = readFileSync(ROOT_MANIFEST, "utf8");
const root = Bun.TOML.parse(rootText) as Toml;
const rewrittenRoot = rewriteRoot(rootText);
const published = publishedManifests(root);
const rewritten = new Map(published.map((path) => [path, dropFeature(readFileSync(path, "utf8"))] as const));

const found = problems(rewrittenRoot, rewritten);
if (found.length > 0) {
  for (const problem of found) console.error(`::error::${problem}`);
  process.exit(1);
}

if (check) {
  console.log(`the published manifests depend on gpui-pre and carry no \`${FEATURE}\` feature`);
} else {
  writeFileSync(ROOT_MANIFEST, rewrittenRoot);
  console.log("rewrote Cargo.toml");
  for (const [path, text] of rewritten) {
    if (text === readFileSync(path, "utf8")) continue;
    writeFileSync(path, text);
    console.log(`rewrote ${relative(REPO_ROOT, path)}`);
  }
}

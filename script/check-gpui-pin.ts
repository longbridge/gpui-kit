#!/usr/bin/env bun
/**
 * Verify that the workspace pins every `gpui-pre-*` crate to an exact version.
 *
 * The published gpui-kit crates carry the workspace's requirements to
 * crates.io. A caret requirement lets the weekly gpui-pre release move an
 * application onto a snapshot that gpui-component was never built against,
 * which is how gpui-kit 0.6.4 stopped compiling the day gpui-pre 0.3.6 came
 * out (#3156). Every snapshot crate has to stay on one `=x.y.z` requirement,
 * and all of them on the same version, so a release resolves exactly the
 * snapshot it was tested with. The hand-published `gpui-pre-reqwest` fork
 * (see `DEPENDENCY_OVERRIDES` in bump-gpui.ts) has its own version line and
 * only needs to be exact.
 *
 *     bun script/check-gpui-pin.ts [path/to/Cargo.toml]
 */
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dir, "..");
const MANIFEST = "Cargo.toml";
const SNAPSHOT_PREFIX = "gpui-pre";
const OWN_VERSION_LINE = new Set(["gpui-pre-reqwest"]);
const EXACT_VERSION = /^=(\d+\.\d+\.\d+)$/;

interface Dependency {
  alias: string;
  package: string;
  version: string | undefined;
}

function snapshotDependencies(manifestPath: string): Dependency[] {
  const manifest = Bun.TOML.parse(readFileSync(manifestPath, "utf8")) as {
    workspace?: { dependencies?: Record<string, unknown> };
  };
  const dependencies: Dependency[] = [];
  for (const [alias, spec] of Object.entries(manifest.workspace?.dependencies ?? {})) {
    if (typeof spec !== "object" || spec === null) continue;
    const { package: pkg, version } = spec as { package?: unknown; version?: unknown };
    if (typeof pkg !== "string" || !pkg.startsWith(SNAPSHOT_PREFIX)) continue;
    dependencies.push({ alias, package: pkg, version: typeof version === "string" ? version : undefined });
  }
  return dependencies;
}

function describe(dependency: Dependency): string {
  return `${dependency.alias} = { package = "${dependency.package}", version = ${JSON.stringify(dependency.version ?? "")} }`;
}

function problems(dependencies: Dependency[]): string[] {
  const found: string[] = [];
  if (dependencies.length === 0) found.push(`no \`${SNAPSHOT_PREFIX}-*\` dependency in [workspace.dependencies]`);
  const snapshotVersions = new Map<string, string[]>();
  for (const dependency of dependencies) {
    const exact = dependency.version === undefined ? null : EXACT_VERSION.exec(dependency.version);
    if (exact === null) {
      found.push(`${describe(dependency)} is not an exact \`=x.y.z\` requirement`);
      continue;
    }
    if (OWN_VERSION_LINE.has(dependency.package)) continue;
    const aliases = snapshotVersions.get(exact[1]) ?? [];
    aliases.push(dependency.package);
    snapshotVersions.set(exact[1], aliases);
  }
  if (snapshotVersions.size > 1) {
    const detail = [...snapshotVersions].map(([version, names]) => `${version} (${names.join(", ")})`).join(", ");
    found.push(`the snapshot crates are pinned to more than one version: ${detail}`);
  }
  return found;
}

const manifestPath = process.argv[2] ?? join(REPO_ROOT, MANIFEST);
const found = problems(snapshotDependencies(manifestPath));
if (found.length > 0) {
  for (const problem of found) console.error(`::error file=${MANIFEST}::${problem}`);
  console.error(`\n${MANIFEST} must pin every ${SNAPSHOT_PREFIX}-* crate to the exact snapshot gpui-kit was built against.`);
  process.exit(1);
}
console.log(`${MANIFEST} pins the ${SNAPSHOT_PREFIX}-* crates exactly`);

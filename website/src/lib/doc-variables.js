import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { visit } from 'unist-util-visit';

// Values that change with a release but appear throughout the documentation.
// Pages write `{{gpui_kit_version}}` and `{{gpui_pre_version}}` instead of
// literals, so prose, docs.rs links and both locales follow the documented
// Kit release and its matching GPUI snapshot.
//
// The workspace `Cargo.toml` is the single source. A versioned build copies
// the website out of the checkout and sets `GPUI_PRE_VERSION` from the
// `Cargo.toml` of the revision it documents.
// Versioned builds also set `GPUI_KIT_VERSION` from the selected release tag
// or main's Kit manifest; local development reads the workspace manifest.

const VARIABLE = /\{\{\s*([a-z_]+)\s*\}\}/g;

function readGpuiKitVersion() {
  if (process.env.GPUI_KIT_VERSION) return process.env.GPUI_KIT_VERSION;
  const tag = process.env.PUBLIC_SITE_VERSION?.match(/^v(\d+\.\d+\.\d+)$/);
  if (tag) return tag[1];
  const manifest = resolve(process.cwd(), '..', 'crates', 'kit', 'Cargo.toml');
  if (!existsSync(manifest)) return undefined;
  return readFileSync(manifest, 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1];
}

function readGpuiPreVersion() {
  if (process.env.GPUI_PRE_VERSION) return process.env.GPUI_PRE_VERSION;
  const manifest = resolve(process.cwd(), '..', 'Cargo.toml');
  if (!existsSync(manifest)) return undefined;
  // Match the package, since its workspace alias can be gpui or gpui_upstream.
  const pin = readFileSync(manifest, 'utf8')
    .match(/^[A-Za-z0-9_-]+\s*=\s*\{[^}\n]*package\s*=\s*"gpui-pre"[^}\n]*version\s*=\s*"=?([^"]+)"/m);
  return pin?.[1];
}

const resolvers = {
  gpui_kit_version: readGpuiKitVersion,
  gpui_pre_version: readGpuiPreVersion,
};

const cache = new Map();

/** The value of one documentation variable; throws for an unknown name. */
export function docVariable(name) {
  if (!cache.has(name)) {
    const resolver = resolvers[name];
    if (!resolver) throw new Error(`Unknown documentation variable {{${name}}}`);
    const value = resolver();
    if (!value) throw new Error(`Cannot resolve documentation variable {{${name}}}`);
    cache.set(name, value);
  }
  return cache.get(name);
}

export const DOC_VARIABLE_NAMES = Object.keys(resolvers);

/** Replace every `{{name}}` in a Markdown source or text fragment. */
export function expandDocVariables(text) {
  return text.includes('{{') ? text.replace(VARIABLE, (_, name) => docVariable(name)) : text;
}

/** Expands variables in prose, code, raw HTML and link destinations. */
export function remarkDocVariables() {
  return (tree) => {
    visit(tree, (node) => {
      if (typeof node.value === 'string') node.value = expandDocVariables(node.value);
      if (typeof node.url === 'string' && /\{\{|%7B%7B/i.test(node.url)) {
        node.url = expandDocVariables(node.url.replace(/%7B/gi, '{').replace(/%7D/gi, '}'));
      }
      if (typeof node.title === 'string') node.title = expandDocVariables(node.title);
    });
  };
}

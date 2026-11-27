/**
 * `nsmetry` for JavaScript: browsers, Node, Bun, Deno, React Native, Electron
 * and Workers. Zero dependencies.
 *
 * Everything platform-specific lives here, detected at run time by feature
 * rather than by name, so one build serves every runtime: the queue itself is
 * in ./core.ts and never touches a global.
 */
import { createClient, SDK_VERSION, type KeyValueStorage, type NsMetry, type NsMetryOptions, type Platform } from './core.js';

export type { KeyValueStorage, NsMetry, NsMetryOptions, PropValue, Props, WireEvent } from './core.js';
export { SDK_VERSION, sanitizeProps, batchUrl } from './core.js';

// The globals this file probes, typed loosely on purpose: the package has no
// DOM or Node types as dependencies, and each one may be absent.
interface Globals {
  fetch?: Platform['fetch'];
  navigator?: { sendBeacon?: (url: string, data: string) => boolean; product?: string; userAgent?: string };
  document?: {
    visibilityState?: string;
    addEventListener?: (type: string, fn: () => void) => void;
    removeEventListener?: (type: string, fn: () => void) => void;
  };
  addEventListener?: (type: string, fn: () => void) => void;
  removeEventListener?: (type: string, fn: () => void) => void;
  location?: { pathname?: string };
  screen?: { width?: number };
  localStorage?: KeyValueStorage;
  crypto?: { randomUUID?: () => string; getRandomValues?: (array: Uint8Array) => Uint8Array };
  process?: {
    platform?: string;
    versions?: { node?: string; bun?: string; electron?: string };
    on?: (event: string, fn: () => void) => void;
    off?: (event: string, fn: () => void) => void;
  };
  Deno?: { build?: { os?: string } };
}

const g = globalThis as unknown as Globals;

/** A browser page (including an Electron renderer), as opposed to a server or app runtime. */
function isBrowser(): boolean {
  return typeof g.document?.addEventListener === 'function' && typeof g.location?.pathname === 'string';
}

function isReactNative(): boolean {
  return g.navigator?.product === 'ReactNative';
}

/**
 * The `ua` sent in the body, or undefined to let the User-Agent header speak.
 *
 * A browser's header is already right. Everything else announces itself as
 * `node`, `Bun/1.1`, `Deno/2` or — React Native on Android — `okhttp/4`, which
 * the server's bot filter drops. So outside a browser we describe ourselves,
 * naming the OS in the words the server's parser looks for (src/ua.ts).
 */
function defaultUa(): string | undefined {
  if (isBrowser()) return undefined;
  const base = `NsMetry-JS/${SDK_VERSION}`;
  if (isReactNative()) return `${base} (React Native)`;
  const os = g.process?.platform ?? g.Deno?.build?.os;
  switch (os) {
    case 'darwin':
      return `${base} (macOS)`;
    case 'win32':
    case 'windows':
      return `${base} (Windows NT)`;
    case 'linux':
      return `${base} (Linux)`;
    case 'android':
      return `${base} (Android; Mobile)`;
    default:
      return base;
  }
}

/** localStorage where it exists and works; reading it can throw in sandboxed iframes. */
function defaultStorage(): KeyValueStorage | undefined {
  try {
    const storage = g.localStorage;
    if (storage && typeof storage.getItem === 'function') return storage;
  } catch {
    // Access denied: no storage.
  }
  return undefined;
}

function randomId(): string {
  const c = g.crypto;
  if (c?.randomUUID) return c.randomUUID();
  if (c?.getRandomValues) {
    const bytes = c.getRandomValues(new Uint8Array(16));
    return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  }
  // Last resort for very old runtimes: not cryptographic, but this id only
  // needs to be unlikely to collide, not unguessable.
  return `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}${Math.random().toString(36).slice(2)}`;
}

/**
 * The hook for "the app is going away":
 * - browsers: `visibilitychange` to hidden (the last event reliably delivered
 *   on mobile Safari) and `pagehide`, sent by sendBeacon;
 * - Node and Bun: `beforeExit`, when the event loop has nothing left to do.
 * React Native has no global for it; the README shows the AppState snippet.
 */
function onHide(fn: () => void): () => void {
  if (isBrowser()) {
    const doc = g.document!;
    const onVisibility = (): void => {
      if (doc.visibilityState === 'hidden') fn();
    };
    doc.addEventListener!('visibilitychange', onVisibility);
    g.addEventListener?.('pagehide', fn);
    return () => {
      doc.removeEventListener?.('visibilitychange', onVisibility);
      g.removeEventListener?.('pagehide', fn);
    };
  }
  const proc = g.process;
  if (proc && typeof proc.on === 'function' && !isReactNative()) {
    // Once: `beforeExit` fires again whenever the loop empties, and an
    // offline flush that fails would otherwise retry in a tight loop at exit.
    let fired = false;
    const onExit = (): void => {
      if (fired) return;
      fired = true;
      fn();
      // Bun (1.4) drains no microtasks after `beforeExit` unless a timer is
      // pending, so the flush would never reach fetch. A ref'd zero timer
      // keeps the loop turning until the request itself holds it open.
      setTimeout(() => {}, 0);
    };
    proc.on('beforeExit', onExit);
    return () => proc.off?.('beforeExit', onExit);
  }
  return () => {};
}

function platform(): Platform {
  const browser = isBrowser();
  const beacon = browser ? g.navigator?.sendBeacon : undefined;
  return {
    // Bound: some runtimes throw "Illegal invocation" for a detached fetch.
    fetch: typeof g.fetch === 'function' ? g.fetch.bind(globalThis) : undefined,
    now: () => Date.now(),
    setTimeout: (fn, ms) => {
      const handle = setTimeout(fn, ms) as unknown as { unref?: () => void };
      // Node/Bun: a pending flush must not keep a CLI or server process
      // alive; `beforeExit` flushes what is left instead.
      handle.unref?.();
      return handle;
    },
    clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
    storage: defaultStorage(),
    beacon: beacon ? (url, body) => beacon.call(g.navigator, url, body) : undefined,
    onHide,
    ua: defaultUa,
    currentPath: browser ? () => g.location?.pathname : undefined,
    screenWidth: browser ? () => g.screen?.width : undefined,
    randomId,
  };
}

/**
 * Creates a client. Never throws: invalid options produce a client that does
 * nothing and reports the problem through `onError`.
 *
 * ```ts
 * const nsmetry = createNsMetry({ endpoint: 'https://stats.example.com', site: 'my-app', appVersion: '1.2.0' });
 * nsmetry.screen('/settings');
 * nsmetry.track('export', { format: 'csv' });
 * ```
 */
export function createNsMetry(options: NsMetryOptions): NsMetry {
  return createClient(options, platform());
}

export default createNsMetry;

/**
 * The platform-agnostic NsMetry client: queue, batching, retry, ids.
 *
 * This file is shared verbatim by two packages: the npm package `nsmetry`
 * (sdk/js/src/core.ts) and the NSH nshm `nsmetry` (sdk/nsh/src/core.ts). It
 * touches no platform global — no `window`, `document`, `process`, `nsh` — and
 * gets everything it needs (fetch, timers, storage, the page-hide signal) from a
 * `Platform` the package hands it. That is what lets one queue implementation
 * run in a browser, Node, Bun, Deno, React Native, Electron, a Worker and an NSH
 * app alike. A test in sdk/js checks the two copies stay identical; edit
 * sdk/js/src/core.ts and run `npm run sync:nsh` in sdk/js.
 *
 * Why batching matters: the server lives on Cloudflare's free plan, whose
 * binding limit is 100,000 Worker requests a day for the whole fleet. One
 * request per event would let a single busy app spend that alone. A flush every
 * 5 minutes while events wait, as soon as 50 are queued, and when the app is
 * backgrounded makes a typical session cost one or two requests whatever its
 * event count, which keeps a fleet of apps with thousands of daily users well
 * inside it. (At 30 s, as this once was, a 5-minute session cost ten.) See
 * docs/sdk.md, "Budget".
 */

/** A property value the server keeps: flat, short and typed. */
export type PropValue = string | number | boolean;
export type Props = Record<string, PropValue | null | undefined>;

/** Key-value storage, sync or async (localStorage, NSH `Storage`, AsyncStorage). */
export interface KeyValueStorage {
  getItem(key: string): string | null | Promise<string | null>;
  setItem(key: string, value: string): void | Promise<void>;
}

/** What a host package provides. Every member is optional except `fetch`. */
export interface Platform {
  fetch: ((input: string, init: RequestInit) => Promise<Response>) | undefined;
  /** Milliseconds since the epoch. */
  now(): number;
  setTimeout(fn: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
  storage?: KeyValueStorage | undefined;
  /** Fire-and-forget delivery that survives the page going away (sendBeacon). */
  beacon?: ((url: string, body: string) => boolean) | undefined;
  /** Calls `fn` when the app is hidden or backgrounded; returns an unsubscribe. */
  onHide?: ((fn: () => void) => () => void) | undefined;
  /** The `ua` to send in the body; undefined lets the User-Agent header speak. */
  ua?: (() => string | undefined) | undefined;
  /** The current path, for events tracked without a `screen()` first. */
  currentPath?: (() => string | undefined) | undefined;
  /** Screen width in px; the server only keeps a coarse bucket. */
  screenWidth?: (() => number | undefined) | undefined;
  /** A random id for the anonymous user id. */
  randomId(): string;
}

export interface NsMetryOptions {
  /** The instance's base URL (`https://stats.example.com`), or the full batch URL. */
  endpoint: string;
  /** The site as registered: a domain (`example.com`) or a bare name (`my-app`). */
  site: string;
  /** A stable, anonymous id for this install or user. Hashed per site by the server. */
  userId?: string | undefined;
  /** Generate an anonymous id and keep it in storage across launches. Off by default. */
  persistUserId?: boolean | undefined;
  /** The app's version, e.g. `2.3.1`. */
  appVersion?: string | undefined;
  /** Describes the client to the server's UA parser; see docs/sdk.md. */
  ua?: string | undefined;
  /**
   * Milliseconds from the first queued event to the automatic flush. Default
   * 300 000 (5 minutes): requests are the server's scarcest resource, and the
   * background flush already sends a session's tail the moment it ends.
   */
  flushInterval?: number | undefined;
  /** Events per request; also the queue length that triggers a flush. Max and default 50. */
  maxBatch?: number | undefined;
  /** Events kept while the server is unreachable; the oldest go first. Default 1000. */
  maxQueue?: number | undefined;
  /** Start disabled: every call is a no-op until `setEnabled(true)`. */
  disabled?: boolean | undefined;
  /** Flush when the page is hidden or the app goes to the background. Default true. */
  flushOnHide?: boolean | undefined;
  /** Replaces the platform's fetch (tests, proxies, custom transports). */
  fetch?: ((input: string, init: RequestInit) => Promise<Response>) | undefined;
  /** Replaces the platform's storage for the persisted user id. */
  storage?: KeyValueStorage | undefined;
  /** Hears failures the client swallowed. It must not throw either. */
  onError?: ((error: unknown) => void) | undefined;
}

export interface NsMetry {
  /** Records a custom event, attributed to the current screen. */
  track(name: string, props?: Props): void;
  /** Records a screen (or page) view at `path`, e.g. `/settings`. */
  screen(path: string, props?: Props): void;
  /** Sets the user id for later events; `null` goes back to the anonymous id (or none). */
  identify(userId: string | null | undefined): void;
  /** Sends everything queued now. Resolves when done or failed; never rejects. */
  flush(): Promise<void>;
  /** Flushes, then stops timers and listeners. Later calls are no-ops. */
  shutdown(): Promise<void>;
  /** Turns tracking off (dropping what is queued) or back on. */
  setEnabled(enabled: boolean): void;
  /** Events waiting to be sent. */
  readonly queued: number;
}

/** One event as `POST /api/events/batch` takes it (docs/api.md, "Ingest"). */
export interface WireEvent {
  name: string;
  url: string;
  ts: number;
  user_id?: string;
  app_version?: string;
  ua?: string;
  screen_width?: number;
  props?: Record<string, PropValue>;
}

interface Queued {
  event: WireEvent;
  /** Fill `user_id` with the anonymous id at send time: it may still be loading from storage. */
  anonymous: boolean;
}

export const SDK_VERSION = '0.1.0';
/** See `flushInterval`. */
export const DEFAULT_FLUSH_INTERVAL = 300_000;
export const BATCH_PATH = '/api/events/batch';
export const USER_ID_KEY = 'nsmetry.uid';

// The server's own limits (src/ingest.ts, docs/api.md). Trimming here rather
// than letting the server reject means one oversized property cannot turn a
// whole batch of 50 into a 400.
const SERVER_MAX_BATCH = 50;
const MAX_NAME = 64;
const MAX_PATH = 512;
const MAX_USER_ID = 256;
const MAX_VERSION = 32;
const MAX_PROP_KEYS = 10;
const MAX_PROP_KEY = 40;
const MAX_PROP_VALUE = 100;
// The server silently drops any other key; filtering here keeps such a key
// from using up one of the ten slots a valid key could have had.
const PROP_KEY_PATTERN = /^[A-Za-z0-9_.-]+$/;
// A request body ceiling, kept under sendBeacon's 64 KiB so the page-hide path
// can carry any chunk the normal path would. Fifty typical events are ~10 KB.
const MAX_BODY_BYTES = 32 * 1024;
const BACKOFF_BASE_MS = 2_000;
const BACKOFF_MAX_MS = 5 * 60_000;

function clampText(value: string, max: number): string {
  return value.length > max ? value.slice(0, max) : value;
}

/**
 * Keeps what the server keeps: ≤ 10 keys of `[A-Za-z0-9_.-]` (≤ 40 chars),
 * each a string/number/boolean, strings ≤ 100 chars.
 */
export function sanitizeProps(props: Props | undefined): Record<string, PropValue> | undefined {
  if (!props || typeof props !== 'object') return undefined;
  const out: Record<string, PropValue> = {};
  let count = 0;
  for (const key of Object.keys(props)) {
    if (count >= MAX_PROP_KEYS) break;
    if (key.length > MAX_PROP_KEY || !PROP_KEY_PATTERN.test(key)) continue;
    const value = props[key];
    if (typeof value === 'string') out[key] = clampText(value, MAX_PROP_VALUE);
    else if (typeof value === 'boolean') out[key] = value;
    else if (typeof value === 'number' && Number.isFinite(value)) out[key] = value;
    else continue;
    count += 1;
  }
  return count > 0 ? out : undefined;
}

/** `https://x.com` and `https://x.com/` become `https://x.com/api/events/batch`; a full batch URL stays. */
export function batchUrl(endpoint: string): string {
  const trimmed = endpoint.trim().replace(/\/+$/, '');
  return trimmed.endsWith(BATCH_PATH) ? trimmed : trimmed + BATCH_PATH;
}

/** Splits events into request bodies of ≤ `maxBatch` events and ≤ MAX_BODY_BYTES. */
export function chunk(site: string, events: WireEvent[], maxBatch: number): string[][] {
  // Returns per-chunk JSON strings of the events, so the caller can both
  // measure and reuse them without serializing twice.
  const chunks: string[][] = [];
  let current: string[] = [];
  let bytes = site.length + 32;
  for (const event of events) {
    const json = JSON.stringify(event);
    if (current.length > 0 && (current.length >= maxBatch || bytes + json.length + 1 > MAX_BODY_BYTES)) {
      chunks.push(current);
      current = [];
      bytes = site.length + 32;
    }
    current.push(json);
    bytes += json.length + 1;
  }
  if (current.length > 0) chunks.push(current);
  return chunks;
}

function body(site: string, events: string[]): string {
  return `{"site":${JSON.stringify(site)},"events":[${events.join(',')}]}`;
}

/** What a response status means for the batch that got it. */
function outcome(status: number): 'sent' | 'drop' | 'retry' {
  if (status >= 200 && status < 300) return 'sent';
  // 408 timeout and 429 rate limit pass with time; so do 5xx outages.
  if (status === 408 || status === 429 || status >= 500) return 'retry';
  // 400 malformed, 404 unknown site, 413 too large: sending the same bytes
  // again can only fail again, and would cost a request each time.
  return 'drop';
}

const noop = (): void => {};

/** A client that does nothing: for `disabled` hosts and platforms without fetch. */
function inertClient(): NsMetry {
  return {
    track: noop,
    screen: noop,
    identify: noop,
    flush: () => Promise.resolve(),
    shutdown: () => Promise.resolve(),
    setEnabled: noop,
    queued: 0,
  };
}

export function createClient(options: NsMetryOptions, platform: Platform): NsMetry {
  const report = (error: unknown): void => {
    try {
      options.onError?.(error);
    } catch {
      // The host's error hook failing must not reach the host either.
    }
  };

  try {
    return new Client(options, platform, report);
  } catch (error) {
    report(error);
    return inertClient();
  }
}

class Client implements NsMetry {
  private readonly url: string;
  private readonly site: string;
  private readonly maxBatch: number;
  private readonly maxQueue: number;
  private readonly flushInterval: number;
  private readonly fetchFn: Platform['fetch'];
  private readonly storage: KeyValueStorage | undefined;
  private readonly ua: string | undefined;

  private queue: Queued[] = [];
  private enabled: boolean;
  private stopped = false;
  private explicitUserId: string | undefined;
  private anonymousId: string | undefined;
  private readonly persist: boolean;
  private idReady: Promise<void> = Promise.resolve();
  private lastPath: string | undefined;

  private timer: unknown = undefined;
  private inFlight: Promise<void> | undefined;
  private attempt = 0;
  private retryAt = 0;
  private unsubscribeHide: () => void = noop;

  constructor(
    private readonly options: NsMetryOptions,
    private readonly platform: Platform,
    private readonly report: (error: unknown) => void,
  ) {
    if (!options || typeof options.endpoint !== 'string' || typeof options.site !== 'string' || options.site.trim() === '') {
      throw new Error('nsmetry: `endpoint` and `site` are required');
    }
    this.url = batchUrl(options.endpoint);
    this.site = options.site.trim();
    this.maxBatch = Math.max(1, Math.min(SERVER_MAX_BATCH, Math.floor(options.maxBatch ?? SERVER_MAX_BATCH)));
    this.maxQueue = Math.max(this.maxBatch, Math.floor(options.maxQueue ?? 1000));
    this.flushInterval = Math.max(1000, options.flushInterval ?? DEFAULT_FLUSH_INTERVAL);
    this.fetchFn = options.fetch ?? platform.fetch;
    this.storage = options.storage ?? platform.storage;
    this.ua = options.ua ?? platform.ua?.();
    this.enabled = options.disabled !== true;
    this.explicitUserId = userIdOf(options.userId);
    this.persist = options.persistUserId === true;

    if (this.persist) this.idReady = this.loadAnonymousId();

    if (options.flushOnHide !== false && platform.onHide) {
      this.unsubscribeHide = platform.onHide(() => this.flushOnHide());
    }
  }

  get queued(): number {
    return this.queue.length;
  }

  track(name: string, props?: Props): void {
    this.guard(() => {
      if (typeof name !== 'string' || name.trim() === '') return;
      this.enqueue(clampText(name.trim(), MAX_NAME), this.lastPath ?? this.platform.currentPath?.() ?? '/', props);
    });
  }

  screen(path: string, props?: Props): void {
    this.guard(() => {
      if (typeof path !== 'string') return;
      const clean = clampText(path.trim() === '' ? '/' : path.trim(), MAX_PATH);
      this.lastPath = clean;
      this.enqueue('screen_view', clean, props);
    });
  }

  identify(userId: string | null | undefined): void {
    this.guard(() => {
      this.explicitUserId = userIdOf(userId);
    }, true);
  }

  setEnabled(enabled: boolean): void {
    this.guard(() => {
      this.enabled = enabled;
      if (!enabled) {
        // Opting out means nothing more leaves the device, queued events included.
        this.queue = [];
        this.cancelTimer();
      }
    }, true);
  }

  async flush(): Promise<void> {
    try {
      await this.drain(true);
    } catch (error) {
      this.report(error);
    }
  }

  async shutdown(): Promise<void> {
    if (this.stopped) return;
    try {
      await this.drain(true);
    } catch (error) {
      this.report(error);
    }
    this.stopped = true;
    this.cancelTimer();
    try {
      this.unsubscribeHide();
    } catch (error) {
      this.report(error);
    }
  }

  // --- internals -----------------------------------------------------------

  /** Runs `fn` unless stopped/disabled, swallowing anything it throws. */
  private guard(fn: () => void, evenIfDisabled = false): void {
    if (this.stopped || (!this.enabled && !evenIfDisabled)) return;
    try {
      fn();
    } catch (error) {
      this.report(error);
    }
  }

  private enqueue(name: string, url: string, props: Props | undefined): void {
    const event: WireEvent = { name, url, ts: Math.floor(this.platform.now() / 1000) };
    const appVersion = nonEmpty(this.options.appVersion);
    if (appVersion) event.app_version = clampText(appVersion, MAX_VERSION);
    if (this.ua) event.ua = this.ua;
    const width = this.platform.screenWidth?.();
    if (typeof width === 'number' && Number.isFinite(width) && width > 0) event.screen_width = Math.round(width);
    const clean = sanitizeProps(props);
    if (clean) event.props = clean;

    let anonymous = false;
    if (this.explicitUserId) event.user_id = this.explicitUserId;
    else if (this.persist) anonymous = true;

    this.queue.push({ event, anonymous });
    // A bounded queue: an app offline for a week must not grow without limit.
    // The oldest go first; they are also the least likely to be honoured with
    // their own timestamp (the server keeps `ts` only within the current day).
    if (this.queue.length > this.maxQueue) this.queue.splice(0, this.queue.length - this.maxQueue);

    if (this.queue.length >= this.maxBatch && this.platform.now() >= this.retryAt) {
      void this.drain(false).catch(this.report);
    } else {
      this.schedule(this.flushInterval);
    }
  }

  private async loadAnonymousId(): Promise<void> {
    try {
      const stored = this.storage ? await this.storage.getItem(USER_ID_KEY) : null;
      if (stored) {
        this.anonymousId = clampText(stored, MAX_USER_ID);
        return;
      }
      this.anonymousId = this.platform.randomId();
      if (this.storage) await this.storage.setItem(USER_ID_KEY, this.anonymousId);
    } catch (error) {
      // Storage refused (private mode, quota): an id for this session is still
      // better than none, it simply will not survive a restart.
      this.anonymousId ??= this.platform.randomId();
      this.report(error);
    }
  }

  /** Takes events off the queue with their user ids resolved. */
  private take(count: number): Queued[] {
    const taken = this.queue.splice(0, count);
    for (const item of taken) {
      if (item.anonymous && this.anonymousId) {
        item.event.user_id = this.anonymousId;
        item.anonymous = false;
      }
    }
    return taken;
  }

  /**
   * Sends the queue, one request at a time. `force` ignores the backoff
   * window: an explicit `flush()` is the host asking for an attempt now.
   */
  private async drain(force: boolean): Promise<void> {
    if (this.inFlight) {
      await this.inFlight;
      if (!force) return;
    }
    if (!this.enabled || this.queue.length === 0) return;
    if (!force && this.platform.now() < this.retryAt) return;

    const run = this.sendAll();
    this.inFlight = run;
    try {
      await run;
    } finally {
      this.inFlight = undefined;
    }
  }

  private async sendAll(): Promise<void> {
    this.cancelTimer();
    await this.idReady;
    const fetchFn = this.fetchFn;
    if (!fetchFn) {
      this.report(new Error('nsmetry: no fetch available on this platform'));
      return;
    }

    while (this.enabled && this.queue.length > 0) {
      const items = this.take(this.queue.length);
      const chunks = chunk(this.site, items.map((i) => i.event), this.maxBatch);
      let index = 0;
      let consumed = 0;
      for (; index < chunks.length; index += 1) {
        const events = chunks[index]!;
        let result: 'sent' | 'drop' | 'retry';
        try {
          const response = await fetchFn(this.url, {
            method: 'POST',
            // text/plain is a CORS "simple" content type: a browser sends no
            // OPTIONS preflight, so a flush costs one Worker request, not two.
            // The server parses the body as JSON whatever the header says.
            headers: { 'content-type': 'text/plain;charset=UTF-8' },
            body: body(this.site, events),
          });
          result = outcome(response.status);
          if (result === 'drop') this.report(new Error(`nsmetry: server refused a batch (${response.status})`));
        } catch (error) {
          result = 'retry';
          this.report(error);
        }
        if (result === 'retry') break;
        consumed += events.length;
      }

      if (index < chunks.length) {
        // Disabled mid-send: what failed is dropped with the rest of the queue.
        if (!this.enabled) return;
        // Put what was not delivered back in front, then wait: exponential
        // backoff with jitter so a fleet of apps does not retry in lockstep
        // the moment an instance comes back.
        this.queue = items.slice(consumed).concat(this.queue);
        if (this.queue.length > this.maxQueue) this.queue.splice(0, this.queue.length - this.maxQueue);
        const delay = Math.min(BACKOFF_MAX_MS, BACKOFF_BASE_MS * 2 ** this.attempt) * (0.5 + Math.random() / 2);
        this.attempt = Math.min(this.attempt + 1, 16);
        this.retryAt = this.platform.now() + delay;
        this.schedule(Math.max(delay, 1));
        return;
      }
      this.attempt = 0;
      this.retryAt = 0;
    }
  }

  /** The page is going away or the app is backgrounded: send now, the best way we can. */
  private flushOnHide(): void {
    try {
      if (!this.enabled || this.queue.length === 0) return;
      const beacon = this.platform.beacon;
      if (!beacon || this.inFlight !== undefined || (this.persist && this.anonymousId === undefined)) {
        // No beacon (apps: they keep running briefly in the background), or a
        // send already running: an ordinary flush is the best there is.
        void this.drain(true).catch(this.report);
        return;
      }
      const items = this.take(this.queue.length);
      const chunks = chunk(this.site, items.map((i) => i.event), this.maxBatch);
      let consumed = 0;
      for (const events of chunks) {
        // sendBeacon queues the request with the browser, which delivers it
        // even after the page is gone. `false` means it refused (size, quota).
        if (!beacon(this.url, body(this.site, events))) break;
        consumed += events.length;
      }
      if (consumed < items.length) {
        this.queue = items.slice(consumed).concat(this.queue);
        void this.drain(true).catch(this.report);
      } else {
        this.cancelTimer();
      }
    } catch (error) {
      this.report(error);
    }
  }

  private schedule(ms: number): void {
    if (this.timer !== undefined || this.stopped) return;
    const handle = this.platform.setTimeout(() => {
      this.timer = undefined;
      void this.drain(false).catch(this.report);
    }, ms);
    this.timer = handle;
  }

  private cancelTimer(): void {
    if (this.timer === undefined) return;
    this.platform.clearTimeout(this.timer);
    this.timer = undefined;
  }
}

function nonEmpty(value: string | undefined): string | undefined {
  return typeof value === 'string' && value.trim() !== '' ? value.trim() : undefined;
}

/**
 * A user id the server accepts. It answers 400 for the *whole batch* when one
 * id is longer than 256 characters, so an over-long id is cut here instead:
 * cut consistently, it still names the same user every time.
 */
function userIdOf(value: string | null | undefined): string | undefined {
  const id = nonEmpty(value ?? undefined);
  return id === undefined ? undefined : clampText(id, MAX_USER_ID);
}

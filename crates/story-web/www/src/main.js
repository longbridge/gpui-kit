const embedded = window.parent !== window;

// `gpui_web` reads keyboard and IME input through a 1x1 transparent `<input>`
// it appends to the body, and focuses that element when the window opens and
// again on every `pointerdown`. On a phone, focusing a text field raises the
// on-screen keyboard over the page — so an embedded gallery pops up the iOS
// keyboard while the reader is only scrolling past it, or taps a button.
//
// Touch-only devices therefore get that element marked `readonly` with
// `inputmode="none"`: iOS leaves the keyboard closed for a read-only field,
// and the element stays focusable, so gpui still tracks window activation and
// key events. Typing into a canvas through an off-screen input is not usable
// on a phone regardless. Devices with a real pointer are left alone, so
// desktop keyboards and IME composition behave exactly as before.
const touchOnly =
  window.matchMedia?.('(hover: none) and (pointer: coarse)').matches ?? false;

function keepKeyboardClosed(node) {
  if (node.tagName !== 'INPUT') return;

  if (touchOnly) {
    node.readOnly = true;
    node.setAttribute('inputmode', 'none');
  }

  // The focus on window creation lands before any interaction. Embedded, it
  // also takes focus away from the page hosting this gallery, which moves the
  // reader's caret and Tab order into the iframe. Hand it back; the next
  // `pointerdown` inside the canvas focuses it again.
  if ((touchOnly || embedded) && document.activeElement === node) {
    node.blur();
  }
}

// Watch from before the module boots, so the element is handled as soon as
// gpui appends it rather than after the keyboard has had a chance to appear.
function watchPlatformInput() {
  document.querySelectorAll('body > input').forEach(keepKeyboardClosed);
  new MutationObserver((records) => {
    for (const record of records) {
      record.addedNodes.forEach(keepKeyboardClosed);
    }
  }).observe(document.body, { childList: true });
}

// Read mode, name, and the source JSON before the first frame. The host sets
// these on <html> before paint, so a newly added website theme works even if
// the gallery's embedded fallback themes have not been rebuilt yet.
function hostTheme() {
  if (!embedded) return { dark: undefined, name: undefined, source: undefined };
  try {
    const root = window.parent.document.documentElement;
    return { dark: root.classList.contains('dark'), name: root.dataset.themeName, source: root.dataset.themeSource };
  } catch {
    // Cross-origin embedding: fall back to the viewer's own preference.
    return { dark: window.matchMedia('(prefers-color-scheme: dark)').matches, name: undefined, source: undefined };
  }
}

const themeFiles = new Map();
function loadThemeSource(source) {
  if (!source) return Promise.resolve(undefined);
  if (!themeFiles.has(source)) {
    themeFiles.set(source, fetch(source)
      .then((response) => response.ok ? response.text() : undefined)
      .catch(() => undefined));
  }
  return themeFiles.get(source);
}

// Follow the host page when the reader toggles its theme.
function watchHostTheme(wasm, applied) {
  if (!embedded) return;
  let root;
  try {
    root = window.parent.document.documentElement;
  } catch {
    return;
  }

  let current = applied;
  const sync = () => {
    const next = hostTheme();
    if (next.dark !== current.dark || next.name !== current.name || next.source !== current.source) {
      current = next;
      document.documentElement.classList.toggle('dark', next.dark);
      loadThemeSource(next.source).then((json) => {
        if (current === next) wasm.set_theme(next.dark, next.name, json);
      });
    }
  };
  new MutationObserver(sync).observe(root, { attributes: true, attributeFilter: ['class', 'data-theme-name'] });
  sync();
}

async function init() {
  const loadingEl = document.getElementById('loading');

  watchPlatformInput();

  try {
    // Import the WASM module
    const wasm = await import('./wasm/gpui_component_story_web.js');
    await wasm.default();

    // A documentation page can deep-link to the matching Rust story while the
    // standalone gallery keeps its normal overview.
    const story = new URLSearchParams(window.location.search).get('story');
    const theme = hostTheme();
    const themeJson = await loadThemeSource(theme.source);
    await wasm.run(story || undefined, theme.dark, theme.name, themeJson);
    watchHostTheme(wasm, theme);

    // Hide loading indicator
    loadingEl?.remove();
  } catch (error) {
    console.error('Failed to initialize:', error);

    // Show error message
    if (loadingEl) {
      loadingEl.innerHTML = `
        <div class="error">
          <h2>Failed to load the application</h2>
          <p>${error.message || error}</p>
          <p style="margin-top: 10px; font-size: 14px;">
            Please check the console for more details.
          </p>
        </div>
      `;
    }
  }
}

init();

// VitePress custom containers (`:::tip` … `:::`) rendered nothing in Astro, so
// the markers showed up as literal text in the page. This turns them into
// `<aside class="callout callout--tip">`.
//
// The markers are not their own blocks: `:::tip` sits on the line directly
// above its first paragraph and the closing `:::` directly below the last one,
// with no blank line between, so markdown parses each into the neighbouring
// paragraph. The plugin therefore strips the marker line off the start of the
// opening paragraph and off the end of the closing one, rather than looking for
// paragraphs that consist of a marker alone.

const TYPES = new Set(['tip', 'info', 'warning', 'danger', 'note', 'caution', 'important', 'details']);
const OPEN = /^:::[ \t]*([a-z]+)[ \t]*(.*)$/;

/** Inline code spans are the only markup these titles use. */
function titleToHtml(title) {
  const escaped = title
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  return escaped.replace(/`([^`]+)`/g, '<code>$1</code>');
}

/** Removes a leading marker line from a paragraph, returning the match. */
function takeOpener(paragraph) {
  const first = paragraph.children?.[0];
  if (first?.type !== 'text') return null;
  const newline = first.value.indexOf('\n');
  const line = (newline === -1 ? first.value : first.value.slice(0, newline)).trim();
  const match = OPEN.exec(line);
  if (!match || !TYPES.has(match[1])) return null;

  if (newline === -1) {
    paragraph.children.shift();
  } else {
    first.value = first.value.slice(newline + 1);
  }
  return { type: match[1], title: match[2].trim() };
}

/** Removes a trailing `:::` line from a paragraph. Returns true when found. */
function takeCloser(paragraph) {
  const last = paragraph.children?.[paragraph.children.length - 1];
  if (last?.type !== 'text') return false;
  const trimmed = last.value.replace(/[ \t]+$/, '');
  if (trimmed === ':::') {
    paragraph.children.pop();
    return true;
  }
  if (trimmed.endsWith('\n:::')) {
    last.value = trimmed.slice(0, -4);
    return true;
  }
  return false;
}

const isEmpty = (node) => node.type === 'paragraph' && node.children.length === 0;

export function remarkCallouts() {
  return (tree) => {
    const out = [];
    let open = null;

    for (const node of tree.children) {
      if (!open && node.type === 'paragraph') {
        const opener = takeOpener(node);
        if (opener) {
          open = opener;
          const title = opener.title ? `<p class="callout__title">${titleToHtml(opener.title)}</p>` : '';
          out.push({
            type: 'html',
            value: `<aside class="callout callout--${opener.type}">${title}`,
          });
          // `:::tip` alone on its line leaves the paragraph empty.
          if (isEmpty(node)) continue;
        }
      }

      if (open && node.type === 'paragraph' && takeCloser(node)) {
        if (!isEmpty(node)) out.push(node);
        out.push({ type: 'html', value: '</aside>' });
        open = null;
        continue;
      }

      out.push(node);
    }

    // An unterminated container would otherwise swallow the rest of the page.
    if (open) out.push({ type: 'html', value: '</aside>' });
    tree.children = out;
  };
}

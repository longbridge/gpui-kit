import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// Released onboarding pages predate {{gpui_kit_version}}. Upgrade only their
// version literals, not their API examples, before rendering a snapshot.
const root = process.argv[2];
if (!root) throw new Error('usage: normalize-doc-versions.js <website-root>');

for (const locale of ['', 'zh-CN/']) {
  for (const page of ['installation', 'getting-started']) {
    const file = join(root, locale, 'docs', `${page}.md`);
    if (!existsSync(file)) continue;
    const source = readFileSync(file, 'utf8');
    const normalized = source
      .replace(/^(gpui-kit\s*=\s*(?:\{\s*version\s*=\s*)?)"\d+\.\d+(?:\.\d+)?"/gm, '$1"{{gpui_kit_version}}"')
      .replace(
        'The `0.6` requirement selects a compatible 0.6.x Kit release; this repository currently declares version `0.6.5`.',
        'The `{{gpui_kit_version}}` requirement selects a compatible Kit release.',
      )
      .replace(
        '`0.6` 要求会选择兼容的 0.6.x 版 Kit；本仓库当前声明的是 `0.6.5`。',
        '`{{gpui_kit_version}}` 要求会选择兼容的 Kit 版本。',
      );
    if (normalized !== source) writeFileSync(file, normalized);
  }
}

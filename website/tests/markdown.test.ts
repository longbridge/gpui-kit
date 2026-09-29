import { expect, test } from 'bun:test';
import { createMarkdownProcessor } from '@astrojs/markdown-remark';
import { shikiConfig, defaultHighlightLang } from '../src/lib/markdown.js';

test('diff blocks distinguish additions and deletions in both themes', async () => {
  const processor = await createMarkdownProcessor({ shikiConfig, defaultHighlightLang });
  const { code } = await processor.render('```diff\n unchanged\n-old_api();\n+new_api();\n```');
  const lines = [...code.matchAll(/<span class="line">([\s\S]*?)<\/span>(?=\n|<\/code>)/g)].map((match) => match[1]);
  expect(lines).toHaveLength(3);
  const colors = lines.map((line) => ({
    light: line.match(/color:([^;" ]+)/)?.[1],
    dark: line.match(/--shiki-dark:([^;" ]+)/)?.[1],
  }));
  for (const theme of ['light', 'dark'] as const) {
    expect(colors[1][theme]).toBeDefined();
    expect(colors[2][theme]).toBeDefined();
    expect(colors[1][theme]).not.toBe(colors[2][theme]);
    expect(colors[1][theme]).not.toBe(colors[0][theme]);
    expect(colors[2][theme]).not.toBe(colors[0][theme]);
  }
  expect(lines[1].replace(/<[^>]+>/g, '')).toBe('-old_api();');
  expect(lines[2].replace(/<[^>]+>/g, '')).toBe('+new_api();');
});

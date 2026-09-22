import { readdir, readFile, writeFile } from 'node:fs/promises';
import { basename, join } from 'node:path';

const roots = ['docs', 'component', 'base', 'shell', 'zh-CN/docs', 'zh-CN/component', 'zh-CN/base', 'zh-CN/shell'];
const root = process.argv[2];

if (!root) throw new Error('usage: normalize-website-frontmatter.mjs <website-root>');

async function filesUnder(directory) {
  const entries = await readdir(join(root, directory), { withFileTypes: true }).catch(() => []);
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await filesUnder(path));
    else if (entry.isFile() && entry.name.endsWith('.md')) files.push(path);
  }
  return files;
}

function yamlString(value) {
  return JSON.stringify(value.replace(/\s+/g, ' ').trim());
}

for (const path of (await Promise.all(roots.map(filesUnder))).flat()) {
  const file = join(root, path);
  const source = await readFile(file, 'utf8');
  if (!source.startsWith('---\n')) continue;

  const end = source.indexOf('\n---', 4);
  if (end === -1) continue;
  const frontmatter = source.slice(4, end);
  const body = source.slice(end + 4);
  const additions = [];

  const heading = body.match(/^#\s+(.+)$/m)?.[1]?.trim();
  if (!/^title:\s*\S+/m.test(frontmatter)) {
    additions.push(`title: ${yamlString(heading || basename(path, '.md'))}`);
  }

  if (!/^description:\s*\S+/m.test(frontmatter)) {
    const paragraph = body
      .split(/\n\s*\n/)
      .map((block) => block.replace(/^\s+|\s+$/g, ''))
      .find((block) => block && !block.startsWith('#') && !block.startsWith('```') && !block.startsWith(':::'));
    additions.push(`description: ${yamlString(paragraph || heading || 'GPUI Kit documentation')}`);
  }

  if (additions.length > 0) {
    await writeFile(file, `---\n${additions.join('\n')}\n${frontmatter}\n---${body}`);
  }
}

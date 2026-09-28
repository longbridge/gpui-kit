import { expect, test } from 'bun:test';
import { join } from 'node:path';

for (const [label, variables, expected] of [
  ['release tag', { PUBLIC_SITE_VERSION: 'v0.8.1', GPUI_KIT_VERSION: '' }, '0.8.1'],
  ['main deployment', { PUBLIC_SITE_VERSION: 'main', GPUI_KIT_VERSION: '0.8.2' }, '0.8.2'],
  ['historical deployment', { PUBLIC_SITE_VERSION: 'v0.6.6', GPUI_KIT_VERSION: '0.6.6' }, '0.6.6'],
] as const) {
  test(`documentation variables follow the ${label}`, () => {
    const result = Bun.spawnSync(['bun', '--eval', `
      const { expandDocVariables } = await import('./src/lib/doc-variables.js');
      console.log(expandDocVariables('gpui-kit = "{{gpui_kit_version}}"'));
    `], { cwd: join(import.meta.dir, '..'), env: { ...process.env, ...variables } });
    expect(result.exitCode).toBe(0);
    expect(result.stdout.toString().trim()).toBe(`gpui-kit = "${expected}"`);
  });
}

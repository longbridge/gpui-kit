import { mkdtempSync, copyFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const directory = dirname(fileURLToPath(import.meta.url));
const workspace = resolve(directory, '../../../..');
const temporary = mkdtempSync(join(tmpdir(), 'gpui-shell-types-'));
const shell = process.env.GPUI_COMPONENT_SHELL_BIN
  ? [resolve(process.env.GPUI_COMPONENT_SHELL_BIN)]
  : ['cargo', 'run', '--locked', '-p', 'gpui-component-shell', '--bin', 'gpui-component-shell', '--'];
const compiler = [
  process.execPath, join(directory, 'node_modules/typescript/bin/tsc'),
  // Match the generated editor config: ambient runtime library checking is
  // separate from checking consumers of the fluent API.
  '--skipLibCheck', '--target', 'ES2020', '--lib', 'ES2020',
  '--module', 'ESNext', '--moduleResolution', 'bundler',
  join(temporary, 'gpui-kit.d.ts'),
];

function run([command, ...args], timeout) {
  const result = spawnSync(command, args, { cwd: workspace, stdio: 'inherit', timeout });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} exited with ${result.status}`);
}

try {
  run([...shell, 'types', temporary]);
  copyFileSync(join(directory, 'fluent.ts'), join(temporary, 'fluent.ts'));
  copyFileSync(join(directory, 'rejected.ts'), join(temporary, 'rejected.ts'));
  copyFileSync(join(workspace, 'examples/js_story/stories/registered.js'), join(temporary, 'registered.js'));
  // Execute exactly the positive TypeScript fixture that passed type checking.
  run([...compiler, '--strict', '--outDir', join(temporary, 'emitted'), join(temporary, 'fluent.ts')], 120000);
  copyFileSync(join(temporary, 'emitted/fluent.js'), join(temporary, 'main.js'));
  run([...compiler, '--strict', '--noEmit', join(temporary, 'rejected.ts')], 120000);
  // Shipped JavaScript uses the editor's default (non-strict) checkJs settings.
  run([...compiler, '--noEmit', '--allowJs', '--checkJs', join(temporary, 'registered.js')], 120000);
  run([...shell, 'check', temporary], 30000);

  for (const [expression, diagnostic] of [
    ["new Spinner().transition('opacity', 120)", 'transition'],
    ["new Spinner().p(4).transition('opacity', 120)", 'transition'],
    ["new Spinner().role('status')", 'role'],
    ["new Spinner().p(4).role('status')", 'role'],
    ['new Spinner().p(4).on_click(() => {})', 'on_click'],
    ["new Spinner().when(true, element => element).size('huge')", 'size(size) expects'],
  ]) {
    writeFileSync(join(temporary, 'main.js'), `import { View } from 'gpui-kit'; import { Spinner } from 'gpui-component'; export default class Invalid extends View { render() { return ${expression}; } }`);
    const [command, ...args] = shell;
    const result = spawnSync(command, [...args, 'check', temporary], {
      cwd: workspace, encoding: 'utf8', timeout: 30000,
    });
    if (result.error) throw result.error;
    if (result.status !== 1 || !result.stderr.includes(diagnostic)) {
      throw new Error(`Expected runtime rejection of ${expression}\n${result.stderr}`);
    }
  }
} finally {
  rmSync(temporary, { recursive: true, force: true });
}

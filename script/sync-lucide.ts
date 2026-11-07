#!/usr/bin/env bun
// Sync the complete pinned Lucide SVG set; --check verifies without writing.
// --archive accepts a local upstream tarball for offline operation.
// Existing GPUI-specific icons and retired upstream filenames are preserved.
import { resolve } from "node:path";
import { parseArgs } from "node:util";

async function main() {
  const { values } = parseArgs({
    args: Bun.argv.slice(2),
    options: {
      check: { type: "boolean", default: false },
      archive: { type: "string" },
      help: { type: "boolean", short: "h" },
    },
    strict: true,
    allowPositionals: false,
  });
  if (values.help) {
    console.log("Usage: bun script/sync-lucide.ts [--check] [--archive archive.tar.gz]");
    return;
  }

  const root = resolve(import.meta.dir, "../crates/assets");
  const manifest: {
    version: string;
    archive_url: string;
    sha256: string;
    icon_count: number;
  } = await Bun.file(resolve(root, "lucide.json")).json();
  let data: Uint8Array;
  if (values.archive) {
    data = await Bun.file(values.archive).bytes();
  } else {
    const response = await fetch(manifest.archive_url, { signal: AbortSignal.timeout(60_000) });
    if (!response.ok) throw new Error(`Lucide download failed: HTTP ${response.status}`);
    data = new Uint8Array(await response.arrayBuffer());
  }
  if (new Bun.CryptoHasher("sha256").update(data).digest("hex") !== manifest.sha256) {
    throw new Error("Lucide archive checksum mismatch");
  }

  const files = new Map<string, File>();
  for (const [name, file] of await new Bun.Archive(data).files()) {
    const parts = name.split("/");
    if (parts.length === 3 && parts[1] === "icons" && parts[2].endsWith(".svg")) {
      files.set(`assets/icons/${parts[2]}`, file);
    } else if (parts.length === 2 && parts[1] === "LICENSE") {
      files.set("LICENSE-LUCIDE", file);
    }
  }
  const count = files.size - 1;
  if (!files.has("LICENSE-LUCIDE") || count !== manifest.icon_count) {
    throw new Error("Unexpected Lucide archive layout or icon count");
  }

  let changed = 0;
  for (const [name, file] of [...files].sort(([a], [b]) => a.localeCompare(b))) {
    const target = Bun.file(resolve(root, name));
    const content = Buffer.from(await file.arrayBuffer());
    if (!(await target.exists()) || !content.equals(Buffer.from(await target.arrayBuffer()))) {
      changed++;
      if (!values.check) await Bun.write(target, content);
    }
  }
  if (values.check && changed) {
    throw new Error(`${changed} Lucide files missing or out of date; run bun script/sync-lucide.ts`);
  }
  console.log(
    `Lucide ${manifest.version}: ${count} icons verified; ${changed} files ${values.check ? "differ" : "updated"}`,
  );
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});

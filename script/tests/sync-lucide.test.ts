import { test, expect } from "bun:test";
import { mkdtemp, mkdir, copyFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

test("sync validates the archive, checks without writes, and preserves custom icons", async () => {
  const root = await mkdtemp(join(tmpdir(), "gpui-lucide-sync-"));
  try {
    await mkdir(join(root, "script"));
    await mkdir(join(root, "crates/assets/assets/icons"), { recursive: true });
    const script = join(root, "script/sync-lucide.ts");
    await copyFile(resolve(import.meta.dir, "../sync-lucide.ts"), script);
    const assets = join(root, "crates/assets");
    const archive = new Bun.Archive({
      "lucide/icons/search.svg": "<svg>search</svg>",
      "lucide/icons/check.svg": "<svg>check</svg>",
      "lucide/LICENSE": "Lucide license",
    }, { compress: "gzip" });
    const bytes = await archive.bytes();
    const archivePath = join(root, "lucide.tar.gz");
    await Bun.write(archivePath, bytes);
    const manifest = {
      version: "fixture",
      sha256: new Bun.CryptoHasher("sha256").update(bytes).digest("hex"),
      icon_count: 2,
    };
    const manifestPath = join(assets, "lucide.json");
    await Bun.write(manifestPath, JSON.stringify(manifest));
    const search = Bun.file(join(assets, "assets/icons/search.svg"));
    const custom = Bun.file(join(assets, "assets/icons/custom.svg"));
    await Bun.write(search, "old search");
    await Bun.write(custom, "custom icon");
    const run = (...args: string[]) => Bun.spawnSync({
      cmd: [process.execPath, script, "--archive", archivePath, ...args],
      cwd: root,
    });

    const drift = run("--check");
    expect(drift.exitCode).toBe(1);
    expect(drift.stderr.toString()).toContain("3 Lucide files missing or out of date");
    expect(await search.text()).toBe("old search");
    expect(await Bun.file(join(assets, "LICENSE-LUCIDE")).exists()).toBe(false);

    expect(run().exitCode).toBe(0);
    expect(await search.text()).toBe("<svg>search</svg>");
    expect(await Bun.file(join(assets, "LICENSE-LUCIDE")).text()).toBe("Lucide license");
    expect(await custom.text()).toBe("custom icon");
    expect(run("--check").exitCode).toBe(0);

    await Bun.write(manifestPath, JSON.stringify({ ...manifest, icon_count: 3 }));
    expect(run().stderr.toString()).toContain("Unexpected Lucide archive layout or icon count");
    await Bun.write(manifestPath, JSON.stringify(manifest));
    await Bun.write(archivePath, "corrupt archive");
    const corrupt = run();
    expect(corrupt.exitCode).toBe(1);
    expect(corrupt.stderr.toString()).toContain("checksum mismatch");
    expect(await search.text()).toBe("<svg>search</svg>");
    expect(await custom.text()).toBe("custom icon");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

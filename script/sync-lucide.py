#!/usr/bin/env python3
"""Sync the complete pinned Lucide SVG set; --check verifies without writing.

An optional --archive path permits offline checks against the upstream tarball.
Existing GPUI-specific icons and retired upstream filenames are preserved.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1] / "crates/assets"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--archive", type=Path)
    args = parser.parse_args()
    manifest = json.loads((ROOT / "lucide.json").read_text())
    data = (args.archive.read_bytes() if args.archive else
            urllib.request.urlopen(manifest["archive_url"], timeout=60).read())
    if hashlib.sha256(data).hexdigest() != manifest["sha256"]:
        raise SystemExit("Lucide archive checksum mismatch")
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        files = {}
        for member in archive.getmembers():
            parts = Path(member.name).parts
            if len(parts) == 3 and parts[1] == "icons" and parts[2].endswith(".svg"):
                files["assets/icons/" + parts[2]] = archive.extractfile(member).read()
            elif len(parts) == 2 and parts[1] == "LICENSE":
                files["LICENSE-LUCIDE"] = archive.extractfile(member).read()
    count = len(files) - 1
    if "LICENSE-LUCIDE" not in files or count != manifest["icon_count"]:
        raise SystemExit("Unexpected Lucide archive layout or icon count")
    changed = []
    for name, content in sorted(files.items()):
        target = ROOT / name
        if not target.exists() or target.read_bytes() != content:
            changed.append(name)
            if not args.check:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(content)
    if args.check and changed:
        raise SystemExit(f"{len(changed)} Lucide files missing or out of date; run script/sync-lucide.py")
    print(f"Lucide {manifest['version']}: {count} icons verified; {len(changed)} files {'differ' if args.check else 'updated'}")


if __name__ == "__main__":
    main()

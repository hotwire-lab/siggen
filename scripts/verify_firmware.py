#!/usr/bin/env python3
"""Verify that bundled ESP32 images match the sketch HOTWIRE_FW_VERSION values."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIRMWARE_DIR = ROOT / "firmware"
PUBLIC_FIRMWARE_DIR = ROOT / "public" / "firmware"

VERSION_RE = re.compile(r'#define\s+HOTWIRE_FW_VERSION\s+"([^"]+)"')
BINARY_VERSION_RE = re.compile(rb'"firmware":"([^"]+)"')


def main() -> int:
    sources: dict[str, list[Path]] = {}
    for ino in sorted(FIRMWARE_DIR.rglob("*.ino")):
        text = ino.read_text(encoding="utf-8")
        for version in VERSION_RE.findall(text):
            sources.setdefault(version, []).append(ino)
    if not sources:
        print("ERROR: no HOTWIRE_FW_VERSION found", file=sys.stderr)
        return 1

    binaries = sorted(PUBLIC_FIRMWARE_DIR.glob("*.ino.bin"))
    if not binaries:
        print("ERROR: no public/firmware/*.ino.bin found", file=sys.stderr)
        return 1

    ordered_versions = sorted(sources, key=len, reverse=True)
    errors: list[str] = []
    used_versions: set[str] = set()

    for binary in binaries:
        data = binary.read_bytes()
        embedded = [m.decode("utf-8", "replace") for m in BINARY_VERSION_RE.findall(data)]
        if not embedded:
            errors.append(f"{binary.name}: no embedded firmware version")
            continue
        matched = None
        for candidate in embedded:
            for version in ordered_versions:
                if candidate == version or candidate.startswith(version + "-"):
                    matched = version
                    break
            if matched:
                break
        if matched:
            used_versions.add(matched)
        else:
            errors.append(
                f"{binary.name}: embedded version(s) {embedded} do not match any sketch version {sorted(sources)}"
            )

    for version, paths in sorted(sources.items()):
        if version not in used_versions:
            errors.append(f"sketch version {version} ({paths[0].name}) has no matching bundled binary")

    if errors:
        print("Firmware verification failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(f"Firmware verified: {len(binaries)} binary/binaries match sketch version(s) {sorted(sources)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
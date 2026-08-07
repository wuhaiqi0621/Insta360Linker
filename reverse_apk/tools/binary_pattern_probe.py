#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path


def parse_hex(value: str) -> bytes:
    value = value.replace(" ", "").replace("_", "").replace(":", "")
    return bytes.fromhex(value)


def printable(data: bytes) -> str:
    out = []
    for b in data:
        if 32 <= b <= 126:
            out.append(chr(b))
        elif b == 0:
            out.append(".")
        else:
            out.append(".")
    return "".join(out)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+")
    parser.add_argument("--hex", action="append", default=[])
    parser.add_argument("--ascii", action="append", default=[])
    parser.add_argument("--context", type=int, default=48)
    parser.add_argument("--limit", type=int, default=12)
    args = parser.parse_args()

    needles: list[tuple[str, bytes]] = []
    for item in args.hex:
        name, _, raw = item.partition("=")
        if not raw:
            raw = name
            name = raw
        needles.append((name, parse_hex(raw)))
    for item in args.ascii:
        name, _, raw = item.partition("=")
        if not raw:
            raw = name
            name = raw
        needles.append((name, raw.encode("utf-8")))

    for file_name in args.files:
        path = Path(file_name)
        data = path.read_bytes()
        print(f"## {path} size={len(data)}")
        for name, needle in needles:
            hits = []
            pos = 0
            while len(hits) < args.limit:
                idx = data.find(needle, pos)
                if idx < 0:
                    break
                hits.append(idx)
                pos = idx + 1
            if not hits:
                print(f"- {name}: no hits")
                continue
            print(f"- {name}: {len(hits)} shown")
            for idx in hits:
                lo = max(0, idx - args.context)
                hi = min(len(data), idx + len(needle) + args.context)
                print(f"  @0x{idx:x}: {data[lo:hi].hex(' ')}")
                print(f"          {printable(data[lo:hi])}")


if __name__ == "__main__":
    main()

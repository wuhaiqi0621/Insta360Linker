#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path


def parse_int(value: str) -> int:
    return int(value, 0)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("preimage")
    parser.add_argument("dex")
    parser.add_argument("offsets", nargs="+", type=parse_int)
    parser.add_argument("--length", type=int, default=64)
    parser.add_argument("--limit", type=int, default=8)
    args = parser.parse_args()

    preimage = Path(args.preimage).read_bytes()
    dex = Path(args.dex).read_bytes()
    for off in args.offsets:
        if off < 0 or off + args.length > len(dex):
            print(f"0x{off:x}: out of dex range")
            continue
        pattern = dex[off : off + args.length]
        hits: list[int] = []
        pos = 0
        while len(hits) < args.limit:
            idx = preimage.find(pattern, pos)
            if idx < 0:
                break
            hits.append(idx)
            pos = idx + 1
        if not hits:
            print(f"0x{off:x}: no hits")
            continue
        rendered = ", ".join(f"0x{hit:x} (delta=0x{hit - off:x})" for hit in hits)
        print(f"0x{off:x}: {rendered}")


if __name__ == "__main__":
    main()

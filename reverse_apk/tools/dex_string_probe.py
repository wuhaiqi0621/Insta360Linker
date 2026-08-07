#!/usr/bin/env python3
from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path


def u32(data: bytes, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


def read_uleb(data: bytes, off: int) -> tuple[int, int] | None:
    result = 0
    shift = 0
    pos = off
    for _ in range(5):
        if pos >= len(data):
            return None
        b = data[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            return result, pos
        shift += 7
    return None


def printable(data: bytes) -> str:
    out = []
    for b in data:
        if b == 0:
            out.append(".")
        elif 32 <= b <= 126:
            out.append(chr(b))
        else:
            out.append(".")
    return "".join(out)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("indexes", nargs="+")
    parser.add_argument("--context", type=int, default=96)
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    string_ids_size = u32(data, 0x38)
    string_ids_off = u32(data, 0x3C)
    print(f"string_ids_size=0x{string_ids_size:x} string_ids_off=0x{string_ids_off:x}")

    for raw in args.indexes:
        idx = int(raw, 0)
        print(f"\nstring@{idx:x}")
        entry = string_ids_off + idx * 4
        if idx < 0 or idx >= string_ids_size or entry + 4 > len(data):
            print("  index outside declared string_ids table")
            continue
        off = u32(data, entry)
        print(f"  data_off=0x{off:x}")
        if off >= len(data):
            print("  data_off outside file")
            continue
        u = read_uleb(data, off)
        if u is None:
            print("  bad uleb")
            start = off
        else:
            declared_len, start = u
            print(f"  declared_utf16_len={declared_len} payload_off=0x{start:x}")
        end = data.find(b"\x00", start, min(len(data), start + 4096))
        if end < 0:
            end = min(len(data), start + args.context)
        raw_bytes = data[start:end]
        print(f"  raw_len_until_nul={len(raw_bytes)}")
        print(f"  text={raw_bytes.decode('utf-8', 'replace')!r}")
        lo = max(0, off - args.context)
        hi = min(len(data), off + args.context)
        print(f"  context@0x{lo:x}..0x{hi:x}: {printable(data[lo:hi])}")


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    main()

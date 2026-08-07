#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from dex_code_scan import plausible_code_item, u32


def read_uleb(data: bytes, off: int) -> tuple[int, int]:
    result = 0
    shift = 0
    pos = off
    for _ in range(5):
        if pos >= len(data):
            raise ValueError("uleb out of range")
        b = data[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            return result, pos
        shift += 7
    raise ValueError("uleb too long")


def parse_class_data(data: bytes, off: int, max_count: int) -> list[tuple[int, int, int, str]]:
    if off <= 0 or off >= len(data):
        return []
    start = off
    try:
        static_fields, off = read_uleb(data, off)
        instance_fields, off = read_uleb(data, off)
        direct_methods, off = read_uleb(data, off)
        virtual_methods, off = read_uleb(data, off)
    except Exception:
        return []
    if any(x > max_count for x in (static_fields, instance_fields, direct_methods, virtual_methods)):
        return []
    for _ in range(static_fields + instance_fields):
        try:
            _, off = read_uleb(data, off)
            _, off = read_uleb(data, off)
        except Exception:
            return []
    out: list[tuple[int, int, int, str]] = []
    method_idx = 0
    for kind, count in (("direct", direct_methods), ("virtual", virtual_methods)):
        method_idx = 0
        for _ in range(count):
            try:
                diff, off = read_uleb(data, off)
                access, off = read_uleb(data, off)
                code_off, off = read_uleb(data, off)
            except Exception:
                return []
            method_idx += diff
            if code_off and plausible_code_item(data, code_off) is None:
                return []
            out.append((method_idx, access, code_off, kind))
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--method", action="append", default=[])
    parser.add_argument("--max-count", type=int, default=2048)
    parser.add_argument("--scan-data", action="store_true")
    parser.add_argument("--range-start", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--range-end", type=lambda x: int(x, 0))
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    wanted = {int(x, 0) for x in args.method}
    hits = []

    if args.scan_data:
        end = args.range_end if args.range_end is not None else len(data)
        for off in range(args.range_start, min(end, len(data)), 1):
            methods = parse_class_data(data, off, args.max_count)
            for method_idx, access, code_off, kind in methods:
                if not wanted or method_idx in wanted:
                    hits.append((off, method_idx, access, code_off, kind))
    else:
        class_defs_size = u32(data, 0x60)
        class_defs_off = u32(data, 0x64)
        for i in range(class_defs_size):
            off = class_defs_off + i * 32
            if off + 28 > len(data):
                break
            class_data_off = u32(data, off + 24)
            methods = parse_class_data(data, class_data_off, args.max_count)
            for method_idx, access, code_off, kind in methods:
                if not wanted or method_idx in wanted:
                    hits.append((class_data_off, method_idx, access, code_off, kind))

    for class_data_off, method_idx, access, code_off, kind in hits:
        print(f"class_data=0x{class_data_off:x} method@{method_idx:x} code=0x{code_off:x} {kind} access=0x{access:x}")
    print(f"hits={len(hits)}")


if __name__ == "__main__":
    main()

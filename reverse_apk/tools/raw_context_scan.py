#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import struct
from pathlib import Path


def printable_window(data: bytes, start: int, end: int) -> str:
    chunk = data[start:end]
    return ''.join(chr(c) if 32 <= c < 127 else '.' for c in chunk)


def scan_file(path: Path, terms: list[bytes], context: int, limit: int) -> None:
    data = path.read_bytes()
    print(f"## {path} size={len(data)}")
    for term in terms:
        offsets: list[int] = []
        pos = 0
        while True:
            hit = data.find(term, pos)
            if hit < 0:
                break
            offsets.append(hit)
            pos = hit + 1
            if len(offsets) >= limit:
                break
        print(f"- {term.decode('utf-8', 'replace')}: {len(offsets)} shown")
        for off in offsets:
            lo = max(0, off - context)
            hi = min(len(data), off + len(term) + context)
            text = printable_window(data, lo, hi)
            text = re.sub(r"\.{8,}", "........", text)
            print(f"  @0x{off:x}: {text}")


def u32(data: bytes, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


def string_id_refs(data: bytes, target_off: int) -> list[int]:
    if data[:4] != b"dex\n":
        return []
    size = u32(data, 0x38)
    off = u32(data, 0x3c)
    out = []
    for i in range(size):
        item = off + i * 4
        if item + 4 <= len(data) and u32(data, item) == target_off:
            out.append(i)
    return out


def type_id_refs(data: bytes, string_idx: int) -> list[int]:
    if data[:4] != b"dex\n":
        return []
    size = u32(data, 0x40)
    off = u32(data, 0x44)
    out = []
    for i in range(size):
        item = off + i * 4
        if item + 4 <= len(data) and u32(data, item) == string_idx:
            out.append(i)
    return out


def method_refs_for_type(data: bytes, type_idx: int) -> list[tuple[int, int]]:
    if data[:4] != b"dex\n":
        return []
    size = u32(data, 0x58)
    off = u32(data, 0x5c)
    out = []
    for i in range(size):
        item = off + i * 8
        if item + 8 <= len(data):
            class_idx = struct.unpack_from("<H", data, item)[0]
            name_idx = u32(data, item + 4)
            if class_idx == type_idx:
                out.append((i, name_idx))
    return out


def extract_ascii_tokens(data: bytes, start: int, end: int, pattern: str) -> list[tuple[int, str]]:
    regex = re.compile(pattern.encode("ascii"))
    out: list[tuple[int, str]] = []
    lo = max(0, start)
    hi = min(len(data), end)
    for match in regex.finditer(data[lo:hi]):
        text = match.group(0).decode("ascii", "replace")
        out.append((lo + match.start(), text))
    return out


def plausible_code_items(data: bytes, start: int, end: int) -> list[tuple[int, int, int, int, int, int]]:
    out: list[tuple[int, int, int, int, int, int]] = []
    lo = max(0, start)
    hi = min(len(data) - 16, end)
    for off in range(lo, hi, 4):
        try:
            regs = struct.unpack_from("<H", data, off)[0]
            ins = struct.unpack_from("<H", data, off + 2)[0]
            outs = struct.unpack_from("<H", data, off + 4)[0]
            tries = struct.unpack_from("<H", data, off + 6)[0]
            debug = u32(data, off + 8)
            insns = u32(data, off + 12)
        except Exception:
            continue
        if regs == 0 or regs > 64 or ins > regs or outs > 32 or tries > 16:
            continue
        if insns == 0 or insns > 4000:
            continue
        if off + 16 + insns * 2 > len(data):
            continue
        if debug != 0 and debug > len(data):
            continue
        first_op = data[off + 16]
        if first_op in {0x00, 0xff}:
            continue
        out.append((off, regs, ins, outs, tries, insns))
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+")
    parser.add_argument("--term", action="append", default=[])
    parser.add_argument("--context", type=int, default=160)
    parser.add_argument("--limit", type=int, default=12)
    parser.add_argument("--refs", action="store_true")
    parser.add_argument("--tokens")
    parser.add_argument("--code-items", action="store_true")
    parser.add_argument("--range-start", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--range-end", type=lambda x: int(x, 0))
    args = parser.parse_args()

    terms = [term.encode("utf-8") for term in args.term]
    for item in args.files:
        path = Path(item)
        if args.code_items:
            data = path.read_bytes()
            end = args.range_end if args.range_end is not None else len(data)
            print(f"## {path} plausible code_items range=0x{args.range_start:x}..0x{end:x}")
            for off, regs, ins, outs, tries, insns in plausible_code_items(data, args.range_start, end):
                print(f"0x{off:x} regs={regs} ins={ins} outs={outs} tries={tries} insns={insns}")
            continue
        if args.tokens:
            data = path.read_bytes()
            end = args.range_end if args.range_end is not None else len(data)
            print(f"## {path} tokens /{args.tokens}/ range=0x{args.range_start:x}..0x{end:x}")
            for off, token in extract_ascii_tokens(data, args.range_start, end, args.tokens):
                print(f"0x{off:x} {token}")
            continue
        scan_file(path, terms, args.context, args.limit)
        if args.refs:
            data = path.read_bytes()
            for term in terms:
                pos = 0
                shown = 0
                while shown < args.limit:
                    hit = data.find(term, pos)
                    if hit < 0:
                        break
                    refs = string_id_refs(data, hit - 1)
                    refs += string_id_refs(data, hit)
                    refs = sorted(set(refs))
                    print(f"  refs {term.decode('utf-8', 'replace')} @0x{hit:x}: string_ids={[hex(x) for x in refs]}")
                    for sid in refs:
                        tids = type_id_refs(data, sid)
                        print(f"    string@{sid:x} type_ids={[hex(x) for x in tids[:16]]}")
                        for tid in tids[:8]:
                            mrefs = method_refs_for_type(data, tid)
                            print(f"      type@{tid:x} method_count={len(mrefs)} first_methods={[hex(x[0]) for x in mrefs[:12]]}")
                    pos = hit + 1
                    shown += 1


if __name__ == "__main__":
    main()

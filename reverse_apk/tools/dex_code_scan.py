#!/usr/bin/env python3
from __future__ import annotations

import argparse
import struct
import sys
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def u16(data: bytes, off: int) -> int:
    return struct.unpack_from("<H", data, off)[0]


def u32(data: bytes, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


OP_LEN: dict[int, int] = {
    0x00: 1, 0x01: 1, 0x02: 2, 0x03: 3, 0x04: 1, 0x05: 2, 0x06: 3, 0x07: 1, 0x08: 2, 0x09: 3,
    0x0a: 1, 0x0b: 1, 0x0c: 1, 0x0d: 1, 0x0e: 1, 0x0f: 1, 0x10: 1, 0x11: 1, 0x12: 1,
    0x13: 2, 0x14: 3, 0x15: 2, 0x16: 2, 0x17: 3, 0x18: 5, 0x19: 2, 0x1a: 2, 0x1b: 3,
    0x1c: 2, 0x1d: 1, 0x1e: 1, 0x1f: 2, 0x20: 2, 0x21: 1, 0x22: 2, 0x23: 2, 0x24: 3,
    0x25: 4, 0x26: 3, 0x27: 1, 0x28: 1, 0x29: 2, 0x2a: 3, 0x2b: 3, 0x2c: 3,
}
for op in range(0x2d, 0x32):
    OP_LEN[op] = 2
for op in range(0x32, 0x3e):
    OP_LEN[op] = 2
for op in range(0x44, 0x52):
    OP_LEN[op] = 2
for op in range(0x52, 0x60):
    OP_LEN[op] = 2
for op in range(0x60, 0x6e):
    OP_LEN[op] = 2
for op in range(0x6e, 0x73):
    OP_LEN[op] = 3
for op in range(0x74, 0x79):
    OP_LEN[op] = 3
for op in range(0x7b, 0x90):
    OP_LEN[op] = 1
for op in range(0x90, 0xaf):
    OP_LEN[op] = 2
for op in range(0xb0, 0xcf):
    OP_LEN[op] = 1
for op in range(0xd0, 0xe3):
    OP_LEN[op] = 2


INVOKE_OPS = set(range(0x6e, 0x73)) | set(range(0x74, 0x79))
CONST_OPS = {0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19}
STRING_OPS = {0x1a, 0x1b}
TYPE_OPS = {0x1c, 0x22, 0x23}
FIELD_OPS = set(range(0x52, 0x6e))


def plausible_code_item(data: bytes, off: int) -> tuple[int, int, int, int, int] | None:
    if off < 0 or off + 16 > len(data):
        return None
    try:
        regs = u16(data, off)
        ins = u16(data, off + 2)
        outs = u16(data, off + 4)
        tries = u16(data, off + 6)
        debug = u32(data, off + 8)
        insns = u32(data, off + 12)
    except Exception:
        return None
    if regs == 0 or regs > 96 or ins > regs or outs > 48 or tries > 32:
        return None
    if insns == 0 or insns > 12000:
        return None
    if off + 16 + insns * 2 > len(data):
        return None
    if debug != 0 and debug > len(data):
        return None
    first = data[off + 16]
    if first in {0x00, 0xff}:
        return None
    return regs, ins, outs, tries, insns


def iter_code_items(data: bytes, start: int, end: int):
    hi = min(len(data) - 16, end)
    for off in range(max(0, start), hi, 4):
        item = plausible_code_item(data, off)
        if item is not None:
            yield off, item


def insn_units(data: bytes, start: int, pc: int, insns: int) -> tuple[int, list[int]]:
    unit = u16(data, start + pc * 2)
    op = unit & 0xff
    length = OP_LEN.get(op, 1)
    units = [u16(data, start + (pc + i) * 2) for i in range(length) if pc + i < insns]
    return op, units


def signed16(value: int) -> int:
    return value - 0x10000 if value & 0x8000 else value


def const_value(op: int, units: list[int]) -> int | None:
    if not units:
        return None
    u0 = units[0]
    if op == 0x12:
        lit = (u0 >> 12) & 0x0f
        return lit - 0x10 if lit & 0x8 else lit
    if op == 0x13 and len(units) >= 2:
        return signed16(units[1])
    if op == 0x14 and len(units) >= 3:
        return units[1] | (units[2] << 16)
    if op == 0x15 and len(units) >= 2:
        return signed16(units[1]) << 16
    if op == 0x16 and len(units) >= 2:
        return signed16(units[1])
    if op == 0x17 and len(units) >= 3:
        return units[1] | (units[2] << 16)
    if op == 0x19 and len(units) >= 2:
        return signed16(units[1]) << 48
    return None


def index_for_op(op: int, units: list[int]) -> int | None:
    if op in STRING_OPS:
        if op == 0x1a and len(units) >= 2:
            return units[1]
        if op == 0x1b and len(units) >= 3:
            return units[1] | (units[2] << 16)
    if op in TYPE_OPS and len(units) >= 2:
        return units[1]
    if op in FIELD_OPS and len(units) >= 2:
        return units[1]
    if op in INVOKE_OPS and len(units) >= 2:
        return units[1]
    return None


def summarize_code(data: bytes, code_off: int, item: tuple[int, int, int, int, int]) -> dict[str, object]:
    regs, ins, outs, tries, insns = item
    start = code_off + 16
    invokes: list[int] = []
    strings: list[int] = []
    types: list[int] = []
    fields: list[int] = []
    consts: list[int] = []
    pc = 0
    while pc < insns:
        op, units = insn_units(data, start, pc, insns)
        length = max(OP_LEN.get(op, 1), 1)
        idx = index_for_op(op, units)
        if idx is not None:
            if op in INVOKE_OPS:
                invokes.append(idx)
            elif op in STRING_OPS:
                strings.append(idx)
            elif op in TYPE_OPS:
                types.append(idx)
            elif op in FIELD_OPS:
                fields.append(idx)
        if op in CONST_OPS:
            val = const_value(op, units)
            if val is not None:
                consts.append(val)
        pc += length
    return {
        "code_off": code_off,
        "regs": regs,
        "ins": ins,
        "outs": outs,
        "tries": tries,
        "insns": insns,
        "invokes": invokes,
        "strings": strings,
        "types": types,
        "fields": fields,
        "consts": consts,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--range-start", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--range-end", type=lambda x: int(x, 0))
    parser.add_argument("--target-method", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-string", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-type", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-field", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-const", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--limit", type=int, default=80)
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    end = args.range_end if args.range_end is not None else len(data)
    wanted_methods = set(args.target_method)
    wanted_strings = set(args.target_string)
    wanted_types = set(args.target_type)
    wanted_fields = set(args.target_field)
    wanted_consts = set(args.target_const)
    shown = 0
    print(f"## {args.dex} code scan range=0x{args.range_start:x}..0x{end:x}")
    for off, item in iter_code_items(data, args.range_start, end):
        summary = summarize_code(data, off, item)
        if wanted_methods and not (set(summary["invokes"]) & wanted_methods):
            continue
        if wanted_strings and not (set(summary["strings"]) & wanted_strings):
            continue
        if wanted_types and not (set(summary["types"]) & wanted_types):
            continue
        if wanted_fields and not (set(summary["fields"]) & wanted_fields):
            continue
        if wanted_consts and not (set(summary["consts"]) & wanted_consts):
            continue
        print(
            f"0x{off:x} regs={summary['regs']} ins={summary['ins']} outs={summary['outs']} "
            f"tries={summary['tries']} insns={summary['insns']}"
        )
        print(f"  invokes={[hex(x) for x in summary['invokes'][:48]]}")
        if summary["strings"]:
            print(f"  strings={[hex(x) for x in summary['strings'][:32]]}")
        if summary["types"]:
            print(f"  types={[hex(x) for x in summary['types'][:32]]}")
        if summary["fields"]:
            print(f"  fields={[hex(x) for x in summary['fields'][:32]]}")
        interesting_consts = [x for x in summary["consts"] if -5 <= x <= 300 or x in {0x80000001, 0x40000000}]
        if interesting_consts:
            print(f"  consts={interesting_consts[:64]}")
        shown += 1
        if shown >= args.limit:
            break


if __name__ == "__main__":
    main()

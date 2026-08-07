#!/usr/bin/env python3
from __future__ import annotations

import argparse
import struct
from pathlib import Path

from dex_code_scan import (
    INVOKE_OPS,
    OP_LEN,
    FIELD_OPS,
    STRING_OPS,
    TYPE_OPS,
    CONST_OPS,
    const_value,
    index_for_op,
    iter_code_items,
    insn_units,
    plausible_code_item,
    summarize_code,
)

OP_NAME = {
    0x00: "nop",
    0x01: "move",
    0x02: "move/from16",
    0x03: "move/16",
    0x07: "move-object",
    0x08: "move-object/from16",
    0x0a: "move-result",
    0x0c: "move-result-object",
    0x0e: "return-void",
    0x0f: "return",
    0x11: "return-object",
    0x12: "const/4",
    0x13: "const/16",
    0x14: "const",
    0x1a: "const-string",
    0x1b: "const-string/jumbo",
    0x1c: "const-class",
    0x22: "new-instance",
    0x23: "new-array",
    0x24: "filled-new-array",
    0x26: "fill-array-data",
    0x28: "goto",
    0x29: "goto/16",
    0x2a: "goto/32",
    0x32: "if-eq",
    0x33: "if-ne",
    0x34: "if-lt",
    0x35: "if-ge",
    0x36: "if-gt",
    0x37: "if-le",
    0x38: "if-eqz",
    0x39: "if-nez",
    0x3a: "if-ltz",
    0x3b: "if-gez",
    0x3c: "if-gtz",
    0x3d: "if-lez",
}
for _op in range(0x44, 0x52):
    OP_NAME.setdefault(_op, "array-op")
for _op in range(0x52, 0x60):
    OP_NAME.setdefault(_op, "instance-field-op")
for _op in range(0x60, 0x6e):
    OP_NAME.setdefault(_op, "static-field-op")
for _op in range(0x6e, 0x73):
    OP_NAME.setdefault(_op, "invoke")
for _op in range(0x74, 0x79):
    OP_NAME.setdefault(_op, "invoke/range")
for _op in range(0x90, 0xaf):
    OP_NAME.setdefault(_op, "binop")
for _op in range(0xd0, 0xe3):
    OP_NAME.setdefault(_op, "binop/lit")


def u16(data: bytes, off: int) -> int:
    return struct.unpack_from("<H", data, off)[0]


def invoke_regs(op: int, units: list[int]) -> list[int]:
    if op in range(0x6e, 0x73) and len(units) >= 3:
        a = (units[0] >> 12) & 0x0f
        c = units[2] & 0x0f
        d = (units[2] >> 4) & 0x0f
        e = (units[2] >> 8) & 0x0f
        f = (units[2] >> 12) & 0x0f
        g = (units[0] >> 8) & 0x0f
        return [c, d, e, f, g][:a]
    if op in range(0x74, 0x79) and len(units) >= 3:
        a = (units[0] >> 8) & 0xff
        c = units[2]
        return list(range(c, c + a))
    return []


def format_summary(summary: dict[str, object]) -> str:
    lines = [
        f"0x{summary['code_off']:x} regs={summary['regs']} ins={summary['ins']} "
        f"outs={summary['outs']} tries={summary['tries']} insns={summary['insns']}"
    ]
    for key in ("invokes", "strings", "types", "fields", "consts"):
        values = summary[key]
        if values:
            if key == "consts":
                rendered = ", ".join(str(x) for x in values)
            else:
                rendered = ", ".join(hex(int(x)) for x in values)
            lines.append(f"  {key}=[{rendered}]")
    return "\n".join(lines)


def inspect_invokes(data: bytes, code_off: int, max_units: int | None) -> None:
    item = plausible_code_item(data, code_off)
    if item is None:
        raise SystemExit(f"not a plausible code_item: 0x{code_off:x}")
    regs, ins, outs, tries, insns = item
    start = code_off + 16
    limit = min(insns, max_units if max_units is not None else insns)
    print(f"code_off=0x{code_off:x} regs={regs} ins={ins} outs={outs} tries={tries} insns={insns}")
    pc = 0
    while pc < limit:
        op, units = insn_units(data, start, pc, insns)
        length = max(OP_LEN.get(op, 1), 1)
        idx = index_for_op(op, units)
        if op in INVOKE_OPS and idx is not None:
            regs_text = ", ".join(f"v{x}" for x in invoke_regs(op, units))
            print(f"  pc=0x{pc:04x} method@{idx:x} regs=[{regs_text}] raw={' '.join(f'{u:04x}' for u in units)}")
        pc += length


def raw_disasm(data: bytes, code_off: int, max_units: int | None) -> None:
    item = plausible_code_item(data, code_off)
    if item is None:
        raise SystemExit(f"not a plausible code_item: 0x{code_off:x}")
    regs, ins, outs, tries, insns = item
    start = code_off + 16
    limit = min(insns, max_units if max_units is not None else insns)
    print(f"code_off=0x{code_off:x} regs={regs} ins={ins} outs={outs} tries={tries} insns={insns}")
    pc = 0
    while pc < limit:
        op, units = insn_units(data, start, pc, insns)
        length = max(OP_LEN.get(op, 1), 1)
        idx = index_for_op(op, units)
        parts = [f"{pc:04x}", " ".join(f"{u:04x}" for u in units), OP_NAME.get(op, f"op_{op:02x}")]
        if idx is not None:
            if op in INVOKE_OPS:
                regs_text = ", ".join(f"v{x}" for x in invoke_regs(op, units))
                parts.append(f"method@{idx:x} {{{regs_text}}}")
            elif op in STRING_OPS:
                parts.append(f"string@{idx:x}")
            elif op in TYPE_OPS:
                parts.append(f"type@{idx:x}")
            elif op in FIELD_OPS:
                parts.append(f"field@{idx:x}")
        if op in CONST_OPS:
            val = const_value(op, units)
            if val is not None:
                parts.append(f"const={val}")
        print("  " + "  ".join(parts))
        pc += length


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--target-method", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-field", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-type", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--target-const", action="append", type=lambda x: int(x, 0), default=[])
    parser.add_argument("--range-start", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--range-end", type=lambda x: int(x, 0))
    parser.add_argument("--code-off", type=lambda x: int(x, 0))
    parser.add_argument("--max-units", type=int)
    parser.add_argument("--raw-disasm", action="store_true")
    parser.add_argument("--limit", type=int, default=80)
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    if args.code_off is not None:
        if args.raw_disasm:
            raw_disasm(data, args.code_off, args.max_units)
            return
        inspect_invokes(data, args.code_off, args.max_units)
        return

    wanted_methods = set(args.target_method)
    wanted_fields = set(args.target_field)
    wanted_types = set(args.target_type)
    wanted_consts = set(args.target_const)
    end = args.range_end if args.range_end is not None else len(data)
    shown = 0
    print(f"## {args.dex} slice range=0x{args.range_start:x}..0x{end:x}")
    for off, item in iter_code_items(data, args.range_start, end):
        summary = summarize_code(data, off, item)
        if wanted_methods and not (set(summary["invokes"]) & wanted_methods):
            continue
        if wanted_fields and not (set(summary["fields"]) & wanted_fields):
            continue
        if wanted_types and not (set(summary["types"]) & wanted_types):
            continue
        if wanted_consts and not (set(summary["consts"]) & wanted_consts):
            continue
        print(format_summary(summary))
        shown += 1
        if shown >= args.limit:
            break


if __name__ == "__main__":
    main()

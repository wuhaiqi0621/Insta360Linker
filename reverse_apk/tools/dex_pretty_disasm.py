#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from dex_code_scan import OP_LEN, plausible_code_item, u16, u32


OP = {
    0x01: "move",
    0x02: "move/from16",
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
    0x15: "const/high16",
    0x1a: "const-string",
    0x1f: "check-cast",
    0x21: "array-length",
    0x22: "new-instance",
    0x23: "new-array",
    0x24: "filled-new-array",
    0x27: "throw",
    0x28: "goto",
    0x29: "goto/16",
    0x2b: "packed-switch",
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
    0x44: "aget",
    0x46: "aget-object",
    0x48: "aget-byte",
    0x4b: "aput",
    0x4d: "aput-object",
    0x52: "iget",
    0x53: "iget-wide",
    0x54: "iget-object",
    0x55: "iget-boolean",
    0x56: "iget-byte",
    0x57: "iget-char",
    0x58: "iget-short",
    0x59: "iput",
    0x5a: "iput-wide",
    0x5b: "iput-object",
    0x5c: "iput-boolean",
    0x5d: "iput-byte",
    0x5e: "iput-char",
    0x5f: "iput-short",
    0x60: "sget",
    0x61: "sget-wide",
    0x62: "sget-object",
    0x63: "sget-boolean",
    0x69: "sput-object",
    0x6e: "invoke-virtual",
    0x6f: "invoke-super",
    0x70: "invoke-direct",
    0x71: "invoke-static",
    0x72: "invoke-interface",
    0x74: "invoke-virtual/range",
    0x75: "invoke-super/range",
    0x76: "invoke-direct/range",
    0x77: "invoke-static/range",
    0x78: "invoke-interface/range",
    0x7b: "neg-int",
    0x7c: "not-int",
    0x7d: "neg-long",
    0x7e: "not-long",
    0x7f: "neg-float",
    0x80: "neg-double",
    0x81: "int-to-long",
    0x82: "int-to-float",
    0x83: "int-to-double",
    0x84: "long-to-int",
    0x85: "long-to-float",
    0x86: "long-to-double",
    0x87: "float-to-int",
    0x88: "float-to-long",
    0x89: "float-to-double",
    0x8a: "double-to-int",
    0x8b: "double-to-long",
    0x8c: "double-to-float",
    0x8d: "int-to-byte",
    0x8e: "int-to-char",
    0x8f: "int-to-short",
    0xb0: "add-int/2addr",
    0xb1: "sub-int/2addr",
    0xb2: "mul-int/2addr",
    0xb3: "div-int/2addr",
    0xb4: "rem-int/2addr",
    0xb5: "and-int/2addr",
    0xb6: "or-int/2addr",
    0xb7: "xor-int/2addr",
    0xb8: "shl-int/2addr",
    0xb9: "shr-int/2addr",
    0xba: "ushr-int/2addr",
    0xd0: "add-int/lit16",
    0xd1: "rsub-int",
    0xd2: "mul-int/lit16",
    0xd3: "div-int/lit16",
    0xd4: "rem-int/lit16",
    0xd5: "and-int/lit16",
    0xd6: "or-int/lit16",
    0xd7: "xor-int/lit16",
    0xd8: "add-int/lit8",
    0xd9: "rsub-int/lit8",
    0xda: "mul-int/lit8",
    0xdd: "and-int/lit8",
    0xde: "or-int/lit8",
    0xdf: "xor-int/lit8",
    0xe0: "shl-int/lit8",
    0xe1: "shr-int/lit8",
    0xe2: "ushr-int/lit8",
}

ARRAY_OP = {
    0x4f: "aput-byte",
}

BINOP = {
    0x90: "add-int",
    0x91: "sub-int",
    0x92: "mul-int",
    0x93: "div-int",
    0x94: "rem-int",
    0x95: "and-int",
    0x96: "or-int",
    0x97: "xor-int",
    0x98: "shl-int",
    0x99: "shr-int",
    0x9a: "ushr-int",
    0xa0: "add-long",
    0xa1: "sub-long",
    0xa3: "div-long",
    0xa4: "rem-long",
}


def s4(v: int) -> int:
    return v - 16 if v & 8 else v


def s16(v: int) -> int:
    return v - 0x10000 if v & 0x8000 else v


def s8(v: int) -> int:
    return v - 0x100 if v & 0x80 else v


def invoke_regs(op: int, units: list[int]) -> tuple[int, list[int]] | None:
    if op == 0x24 and len(units) >= 3:
        count = (units[0] >> 12) & 0x0F
        regs = units[2]
        values = [regs & 0x0F, (regs >> 4) & 0x0F, (regs >> 8) & 0x0F, (regs >> 12) & 0x0F, (units[0] >> 8) & 0x0F]
        return units[1], values[:count]
    if op in range(0x6E, 0x73) and len(units) >= 3:
        count = (units[0] >> 12) & 0x0F
        regs = units[2]
        values = [regs & 0x0F, (regs >> 4) & 0x0F, (regs >> 8) & 0x0F, (regs >> 12) & 0x0F, (units[0] >> 8) & 0x0F]
        return units[1], values[:count]
    if op in range(0x74, 0x79) and len(units) >= 3:
        count = (units[0] >> 8) & 0xFF
        first = units[2]
        return units[1], [first + i for i in range(count)]
    return None


def fmt(op: int, units: list[int]) -> str:
    u0 = units[0]
    name = ARRAY_OP.get(op) or BINOP.get(op) or OP.get(op, f"op_{op:02x}")
    if op in (0x01, 0x07, 0x21) or 0x7b <= op <= 0x8f or 0xb0 <= op <= 0xcf:
        a = (u0 >> 8) & 0x0F
        b = (u0 >> 12) & 0x0F
        return f"{name} v{a}, v{b}"
    if op in (0x02, 0x08) and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, v{units[1]}"
    if op == 0x0a or op == 0x0c:
        return f"{name} v{(u0 >> 8) & 0xFF}"
    if op == 0x0e:
        return name
    if op == 0x0f or op == 0x11:
        return f"{name} v{(u0 >> 8) & 0xFF}"
    if op == 0x12:
        return f"{name} v{(u0 >> 8) & 0x0F}, #{s4((u0 >> 12) & 0x0F)}"
    if op == 0x13 and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, #{s16(units[1])}"
    if op == 0x14 and len(units) >= 3:
        return f"{name} v{(u0 >> 8) & 0xFF}, #{units[1] | (units[2] << 16)}"
    if op == 0x15 and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, #0x{units[1] << 16:x}"
    if op == 0x1a and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, string@{units[1]:x}"
    if op == 0x1f and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, type@{units[1]:x}"
    if op == 0x22 and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, type@{units[1]:x}"
    if op == 0x23 and len(units) >= 2:
        a = (u0 >> 8) & 0x0F
        b = (u0 >> 12) & 0x0F
        return f"{name} v{a}, v{b}, type@{units[1]:x}"
    if op == 0x24 and len(units) >= 3:
        inv = invoke_regs(op, units)
        if inv:
            type_idx, regs = inv
            return f"{name} {{{', '.join('v' + str(r) for r in regs)}}}, type@{type_idx:x}"
    if 0x32 <= op <= 0x37 and len(units) >= 2:
        a = (u0 >> 8) & 0x0F
        b = (u0 >> 12) & 0x0F
        return f"{name} v{a}, v{b}, {s16(units[1]):+d}"
    if 0x38 <= op <= 0x3d and len(units) >= 2:
        return f"{name} v{(u0 >> 8) & 0xFF}, {s16(units[1]):+d}"
    if 0x44 <= op <= 0x4f and len(units) >= 2:
        a = (u0 >> 8) & 0xFF
        b = units[1] & 0xFF
        c = (units[1] >> 8) & 0xFF
        return f"{name} v{a}, v{b}, v{c}"
    if 0x52 <= op <= 0x6d and len(units) >= 2:
        a = (u0 >> 8) & 0x0F
        b = (u0 >> 12) & 0x0F
        return f"{name} v{a}, v{b}, field@{units[1]:x}"
    inv = invoke_regs(op, units)
    if inv:
        method, regs = inv
        return f"{name} {{{', '.join('v' + str(r) for r in regs)}}}, method@{method:x}"
    if op in BINOP and len(units) >= 2:
        a = (u0 >> 8) & 0xFF
        b = units[1] & 0xFF
        c = (units[1] >> 8) & 0xFF
        return f"{name} v{a}, v{b}, v{c}"
    if 0xd0 <= op <= 0xd7 and len(units) >= 2:
        a = (u0 >> 8) & 0x0F
        b = (u0 >> 12) & 0x0F
        return f"{name} v{a}, v{b}, #{s16(units[1])}"
    if 0xd8 <= op <= 0xe2 and len(units) >= 2:
        a = (u0 >> 8) & 0xFF
        b = units[1] & 0xFF
        c = (units[1] >> 8) & 0xFF
        return f"{name} v{a}, v{b}, #{s8(c)}"
    if op == 0x28:
        return f"{name} {s8((u0 >> 8) & 0xFF):+d}"
    if op == 0x29 and len(units) >= 2:
        return f"{name} {s16(units[1]):+d}"
    return name


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--code-off", required=True, type=lambda x: int(x, 0))
    parser.add_argument("--start-unit", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--max-units", type=int)
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    item = plausible_code_item(data, args.code_off)
    if item is None:
        raise SystemExit(f"not a plausible code_item: 0x{args.code_off:x}")
    regs, ins, outs, tries, insns = item
    start = args.code_off + 16
    start_unit = max(0, min(args.start_unit, insns))
    limit = min(insns, start_unit + (args.max_units if args.max_units is not None else insns))
    print(f"code_off=0x{args.code_off:x} regs={regs} ins={ins} outs={outs} tries={tries} debug=0x{u32(data, args.code_off + 8):x} insns={insns}")
    pc = start_unit
    while pc < limit:
        op = u16(data, start + pc * 2) & 0xFF
        length = max(OP_LEN.get(op, 1), 1)
        units = [u16(data, start + (pc + i) * 2) for i in range(length) if pc + i < insns]
        print(f"{pc:04x}  {' '.join(f'{u:04x}' for u in units):<16} {fmt(op, units)}")
        pc += length


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from dex_code_scan import OP_LEN, plausible_code_item, u16, u32


def s16(value):
    return value - 0x10000 if value & 0x8000 else value


def decode_invoke_regs(op, units):
    if op in range(0x6e, 0x73) and len(units) >= 3:
        count = (units[0] >> 12) & 0x0F
        regs = units[2]
        values = [regs & 0x0F, (regs >> 4) & 0x0F, (regs >> 8) & 0x0F, (regs >> 12) & 0x0F, (units[0] >> 8) & 0x0F]
        return "{" + ", ".join(f"v{x}" for x in values[:count]) + "}"
    if op in range(0x74, 0x79) and len(units) >= 3:
        count = (units[0] >> 8) & 0xFF
        first = units[2]
        if count <= 0:
            return "{}"
        return "{" + "..".join([f"v{first}", f"v{first + count - 1}"]) + "}"
    return ""


def const_value(op, units):
    if not units:
        return None
    u0 = units[0]
    if op == 0x12:
        lit = (u0 >> 12) & 0x0F
        return lit - 0x10 if lit & 0x8 else lit
    if op == 0x13 and len(units) >= 2:
        return s16(units[1])
    if op == 0x14 and len(units) >= 3:
        return units[1] | (units[2] << 16)
    return None


def insns(data, code_off):
    item = plausible_code_item(data, code_off)
    if item is None:
        raise SystemExit(f"not a plausible code_item: 0x{code_off:x}")
    regs, ins, outs, tries, insns_size = item
    start = code_off + 16
    pc = 0
    out = []
    while pc < insns_size:
        unit = u16(data, start + pc * 2)
        op = unit & 0xFF
        length = OP_LEN.get(op, 1)
        units = [u16(data, start + (pc + i) * 2) for i in range(length) if pc + i < insns_size]
        out.append((pc, op, units))
        pc += max(length, 1)
    return (regs, ins, outs, tries, u32(data, code_off + 8), insns_size), out


def describe(pc, op, units):
    raw = " ".join(f"{u:04x}" for u in units)
    text = f"{pc:04x}  {raw:<18}"
    if op in range(0x6e, 0x73) or op in range(0x74, 0x79):
        text += f" invoke method@{units[1]:x} {decode_invoke_regs(op, units)}"
    elif op in (0x12, 0x13, 0x14):
        text += f" const {const_value(op, units)}"
    elif op == 0x1a and len(units) >= 2:
        text += f" const-string @{units[1]:x}"
    elif op == 0x22 and len(units) >= 2:
        text += f" new-instance type@{units[1]:x}"
    elif op in range(0x52, 0x60) and len(units) >= 2:
        text += f" instance-field field@{units[1]:x}"
    else:
        text += f" op_{op:02x}"
    return text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dex")
    ap.add_argument("--code-off", required=True)
    ap.add_argument("--target-method", action="append", default=[])
    ap.add_argument("--target-const", action="append", default=[])
    ap.add_argument("--context", type=int, default=8)
    args = ap.parse_args()

    data = Path(args.dex).read_bytes()
    code_off = int(args.code_off, 0)
    methods = {int(x, 0) for x in args.target_method}
    consts = {int(x, 0) for x in args.target_const}
    item, rows = insns(data, code_off)
    regs, ins_size, outs, tries, debug, insns_size = item
    print(f"code_off=0x{code_off:x} regs={regs} ins={ins_size} outs={outs} tries={tries} debug=0x{debug:x} insns={insns_size}")
    hit_indexes = []
    for idx, (_, op, units) in enumerate(rows):
        if methods and (op in range(0x6e, 0x73) or op in range(0x74, 0x79)) and len(units) >= 2 and units[1] in methods:
            hit_indexes.append(idx)
        if consts and op in (0x12, 0x13, 0x14) and const_value(op, units) in consts:
            hit_indexes.append(idx)
    seen = set()
    for idx in hit_indexes:
        if idx in seen:
            continue
        seen.add(idx)
        start = max(0, idx - args.context)
        end = min(len(rows), idx + args.context + 1)
        print(f"\n-- hit pc=0x{rows[idx][0]:x} --")
        for pos in range(start, end):
            pc, op, units = rows[pos]
            marker = "=>" if pos == idx else "  "
            print(marker, describe(pc, op, units))


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    main()

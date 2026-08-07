#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from dex_code_scan import OP_LEN, plausible_code_item, u16, u32


def s16(value: int) -> int:
    return value - 0x10000 if value & 0x8000 else value


def const_value(op: int, units: list[int]) -> tuple[int, int] | None:
    if not units:
        return None
    u0 = units[0]
    if op == 0x12:
        reg = (u0 >> 8) & 0x0F
        lit = (u0 >> 12) & 0x0F
        if lit & 0x8:
            lit -= 0x10
        return reg, lit
    if op == 0x13 and len(units) >= 2:
        return (u0 >> 8) & 0xFF, s16(units[1])
    if op == 0x14 and len(units) >= 3:
        return (u0 >> 8) & 0xFF, units[1] | (units[2] << 16)
    if op == 0x15 and len(units) >= 2:
        return (u0 >> 8) & 0xFF, s16(units[1]) << 16
    if op == 0x16 and len(units) >= 2:
        return (u0 >> 8) & 0xFF, s16(units[1])
    return None


def move_regs(op: int, units: list[int]) -> tuple[int, int] | None:
    if not units:
        return None
    u0 = units[0]
    if op in (0x01, 0x04, 0x07):
        return (u0 >> 8) & 0x0F, (u0 >> 12) & 0x0F
    if op in (0x02, 0x05, 0x08) and len(units) >= 2:
        return (u0 >> 8) & 0xFF, units[1]
    if op in (0x03, 0x06, 0x09) and len(units) >= 3:
        return units[1], units[2]
    return None


def invoke_regs(op: int, units: list[int]) -> tuple[int, list[int]] | None:
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


def field_ref(units: list[int]) -> int | None:
    if len(units) < 2:
        return None
    return units[1]


def field_regs(units: list[int]) -> tuple[int, int] | None:
    if not units:
        return None
    u0 = units[0]
    return (u0 >> 8) & 0x0F, (u0 >> 12) & 0x0F


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--code-off", required=True)
    parser.add_argument("--target-method", action="append", default=[])
    parser.add_argument("--all-invokes", action="store_true")
    args = parser.parse_args()

    data = Path(args.dex).read_bytes()
    code_off = int(args.code_off, 0)
    wanted = {int(x, 0) for x in args.target_method}
    item = plausible_code_item(data, code_off)
    if item is None:
        raise SystemExit(f"not a plausible code_item: 0x{code_off:x}")
    regs_size, ins_size, outs_size, tries_size, insns_size = item
    start = code_off + 16
    state: dict[int, str] = {}
    last_result = "result(?)"

    print(f"code_off=0x{code_off:x} regs={regs_size} ins={ins_size} outs={outs_size} tries={tries_size} debug=0x{u32(data, code_off + 8):x} insns={insns_size}")

    pc = 0
    while pc < insns_size:
        unit = u16(data, start + pc * 2)
        op = unit & 0xFF
        length = OP_LEN.get(op, 1)
        units = [u16(data, start + (pc + i) * 2) for i in range(length) if pc + i < insns_size]

        cv = const_value(op, units)
        if cv is not None:
            reg, value = cv
            state[reg] = f"const({value})"
        else:
            mv = move_regs(op, units)
            if mv is not None:
                dst, src = mv
                state[dst] = state.get(src, f"v{src}")
            elif op in (0x0A, 0x0B, 0x0C) and units:
                state[(units[0] >> 8) & 0xFF] = last_result
            elif op == 0x22 and len(units) >= 2:
                state[(units[0] >> 8) & 0xFF] = f"new(type@{units[1]:x})"
            elif op == 0x1A and len(units) >= 2:
                state[(units[0] >> 8) & 0xFF] = f"string@{units[1]:x}"
            elif op in range(0x52, 0x59) and len(units) >= 2:
                regs = field_regs(units)
                if regs is not None:
                    dst, obj = regs
                    state[dst] = f"field@{field_ref(units):x}({state.get(obj, 'v' + str(obj))})"
            elif op in range(0x59, 0x60) and len(units) >= 2:
                # iput* does not change a register, but it is important evidence for constructors.
                pass

        inv = invoke_regs(op, units)
        if inv is not None:
            method, regs = inv
            last_result = f"result(method@{method:x})"
            if args.all_invokes or method in wanted:
                args_rendered = ", ".join(f"v{r}={state.get(r, '?')}" for r in regs)
                print(f"pc=0x{pc:04x} method@{method:x} {{{', '.join('v'+str(r) for r in regs)}}}  {args_rendered}")
        elif op in range(0x59, 0x60) and len(units) >= 2:
            regs = field_regs(units)
            if regs is not None:
                src, obj = regs
                print(
                    f"pc=0x{pc:04x} iput field@{field_ref(units):x} "
                    f"obj=v{obj}({state.get(obj, '?')}) src=v{src}({state.get(src, '?')})"
                )

        pc += max(length, 1)


if __name__ == "__main__":
    main()

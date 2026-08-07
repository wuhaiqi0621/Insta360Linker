#!/usr/bin/env python3
import argparse
import struct
import sys
from dataclasses import dataclass
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def u16(data, off):
    return struct.unpack_from("<H", data, off)[0]


def u32(data, off):
    return struct.unpack_from("<I", data, off)[0]


def read_uleb(data, off):
    result = 0
    shift = 0
    pos = off
    while True:
        if pos >= len(data):
            raise ValueError("uleb out of range")
        b = data[pos]
        pos += 1
        result |= (b & 0x7f) << shift
        if not (b & 0x80):
            return result, pos
        shift += 7


def read_string(data, off):
    if off <= 0 or off >= len(data):
        raise ValueError("string offset out of range")
    _, pos = read_uleb(data, off)
    if pos >= len(data):
        raise ValueError("string data out of range")
    end = data.index(0, pos)
    return data[pos:end].decode("utf-8", "replace")


@dataclass
class Method:
    class_idx: int
    proto_idx: int
    name_idx: int


@dataclass
class EncodedMethod:
    method_idx: int
    access_flags: int
    code_off: int
    kind: str


class Dex:
    def __init__(self, path):
        self.path = Path(path)
        self.data = self.path.read_bytes()
        if self.data[:4] != b"dex\n":
            raise ValueError("not a dex file")
        self.string_ids_size = u32(self.data, 0x38)
        self.string_ids_off = u32(self.data, 0x3c)
        self.type_ids_size = u32(self.data, 0x40)
        self.type_ids_off = u32(self.data, 0x44)
        self.proto_ids_size = u32(self.data, 0x48)
        self.proto_ids_off = u32(self.data, 0x4c)
        self.field_ids_size = u32(self.data, 0x50)
        self.field_ids_off = u32(self.data, 0x54)
        self.method_ids_size = u32(self.data, 0x58)
        self.method_ids_off = u32(self.data, 0x5c)
        self.class_defs_size = u32(self.data, 0x60)
        self.class_defs_off = u32(self.data, 0x64)
        self._strings = None
        self._types = None
        self._methods = None

    @property
    def strings(self):
        if self._strings is None:
            out = []
            for i in range(self.string_ids_size):
                off = u32(self.data, self.string_ids_off + i * 4)
                try:
                    out.append(read_string(self.data, off))
                except Exception:
                    out.append(f"<bad-string@{off:x}>")
            self._strings = out
        return self._strings

    @property
    def types(self):
        if self._types is None:
            self._types = [u32(self.data, self.type_ids_off + i * 4) for i in range(self.type_ids_size)]
        return self._types

    @property
    def methods(self):
        if self._methods is None:
            out = []
            for i in range(self.method_ids_size):
                off = self.method_ids_off + i * 8
                out.append(Method(u16(self.data, off), u16(self.data, off + 2), u32(self.data, off + 4)))
            self._methods = out
        return self._methods

    def type_name(self, type_idx):
        if type_idx < 0 or type_idx >= len(self.types):
            return f"<bad-type@{type_idx:x}>"
        string_idx = self.types[type_idx]
        if string_idx < 0 or string_idx >= len(self.strings):
            return f"<bad-type-string@{string_idx:x}>"
        return self.strings[string_idx]

    def method_name(self, method_idx):
        if method_idx < 0 or method_idx >= len(self.methods):
            return f"<bad-method@{method_idx:x}>"
        m = self.methods[method_idx]
        name = self.strings[m.name_idx] if 0 <= m.name_idx < len(self.strings) else f"<bad-name@{m.name_idx:x}>"
        return f"{self.type_name(m.class_idx)}->{name}"

    def class_defs(self):
        for i in range(self.class_defs_size):
            off = self.class_defs_off + i * 32
            yield {
                "idx": i,
                "class_idx": u32(self.data, off),
                "access_flags": u32(self.data, off + 4),
                "superclass_idx": u32(self.data, off + 8),
                "interfaces_off": u32(self.data, off + 12),
                "source_file_idx": u32(self.data, off + 16),
                "annotations_off": u32(self.data, off + 20),
                "class_data_off": u32(self.data, off + 24),
                "static_values_off": u32(self.data, off + 28),
            }

    def encoded_methods_for_class(self, class_def):
        off = class_def["class_data_off"]
        if off == 0:
            return []
        static_fields_size, off = read_uleb(self.data, off)
        instance_fields_size, off = read_uleb(self.data, off)
        direct_methods_size, off = read_uleb(self.data, off)
        virtual_methods_size, off = read_uleb(self.data, off)
        for _ in range(static_fields_size + instance_fields_size):
            _, off = read_uleb(self.data, off)
            _, off = read_uleb(self.data, off)
        out = []
        method_idx = 0
        for _ in range(direct_methods_size):
            diff, off = read_uleb(self.data, off)
            access, off = read_uleb(self.data, off)
            code_off, off = read_uleb(self.data, off)
            method_idx += diff
            out.append(EncodedMethod(method_idx, access, code_off, "direct"))
        method_idx = 0
        for _ in range(virtual_methods_size):
            diff, off = read_uleb(self.data, off)
            access, off = read_uleb(self.data, off)
            code_off, off = read_uleb(self.data, off)
            method_idx += diff
            out.append(EncodedMethod(method_idx, access, code_off, "virtual"))
        return out


OP_LEN = {
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


OP_NAME = {
    0x01: "move",
    0x02: "move/from16",
    0x03: "move/16",
    0x04: "move-wide",
    0x05: "move-wide/from16",
    0x06: "move-wide/16",
    0x07: "move-object",
    0x08: "move-object/from16",
    0x09: "move-object/16",
    0x0a: "move-result",
    0x0b: "move-result-wide",
    0x0c: "move-result-object",
    0x0d: "move-exception",
    0x0e: "return-void", 0x0f: "return", 0x10: "return-wide", 0x11: "return-object",
    0x12: "const/4", 0x13: "const/16", 0x14: "const", 0x1a: "const-string", 0x1b: "const-string/jumbo",
    0x21: "array-length", 0x22: "new-instance", 0x23: "new-array", 0x24: "filled-new-array",
    0x26: "fill-array-data", 0x28: "goto", 0x29: "goto/16", 0x2a: "goto/32",
    0x32: "if-eq", 0x33: "if-ne", 0x34: "if-lt", 0x35: "if-ge", 0x36: "if-gt", 0x37: "if-le",
    0x38: "if-eqz", 0x39: "if-nez", 0x3a: "if-ltz", 0x3b: "if-gez", 0x3c: "if-gtz", 0x3d: "if-lez",
    0x44: "aget", 0x45: "aget-wide", 0x46: "aget-object", 0x47: "aget-boolean", 0x48: "aget-byte",
    0x49: "aget-char", 0x4a: "aget-short", 0x4b: "aput", 0x4c: "aput-wide", 0x4d: "aput-object",
    0x4e: "aput-boolean", 0x4f: "aput-byte", 0x50: "aput-char", 0x51: "aput-short",
    0x52: "iget", 0x53: "iget-wide", 0x54: "iget-object", 0x55: "iget-boolean", 0x56: "iget-byte",
    0x57: "iget-char", 0x58: "iget-short", 0x59: "iput", 0x5a: "iput-wide", 0x5b: "iput-object",
    0x5c: "iput-boolean", 0x5d: "iput-byte", 0x5e: "iput-char", 0x5f: "iput-short",
    0x60: "sget", 0x61: "sget-wide", 0x62: "sget-object", 0x63: "sget-boolean", 0x64: "sget-byte",
    0x65: "sget-char", 0x66: "sget-short", 0x67: "sput", 0x68: "sput-wide", 0x69: "sput-object",
    0x6a: "sput-boolean", 0x6b: "sput-byte", 0x6c: "sput-char", 0x6d: "sput-short",
    0x6e: "invoke-virtual", 0x6f: "invoke-super", 0x70: "invoke-direct", 0x71: "invoke-static",
    0x72: "invoke-interface", 0x74: "invoke-virtual/range", 0x76: "invoke-direct/range",
    0x77: "invoke-static/range", 0x8d: "int-to-byte", 0x94: "rem-int", 0xb7: "xor-int/2addr",
    0x80: "neg-double", 0x81: "int-to-long", 0x82: "int-to-float", 0x83: "int-to-double",
    0x84: "long-to-int", 0x85: "long-to-float", 0x86: "long-to-double", 0x87: "float-to-int",
    0x88: "float-to-long", 0x89: "float-to-double", 0x8a: "double-to-int", 0x8b: "double-to-long",
    0x8c: "double-to-float", 0x8d: "int-to-byte", 0x8e: "int-to-char", 0x8f: "int-to-short",
    0x90: "add-int", 0x91: "sub-int", 0x92: "mul-int", 0x93: "div-int", 0x94: "rem-int",
    0x95: "and-int", 0x96: "or-int", 0x97: "xor-int", 0x98: "shl-int", 0x99: "shr-int",
    0x9a: "ushr-int", 0x9b: "add-long", 0x9c: "sub-long", 0x9d: "mul-long", 0x9e: "div-long",
    0x9f: "rem-long", 0xa0: "and-long", 0xa1: "or-long", 0xa2: "xor-long", 0xa3: "shl-long",
    0xa4: "shr-long", 0xa5: "ushr-long",
    0xb0: "add-int/2addr", 0xb1: "sub-int/2addr", 0xb2: "mul-int/2addr", 0xb3: "div-int/2addr",
    0xb4: "rem-int/2addr", 0xb5: "and-int/2addr", 0xb6: "or-int/2addr", 0xb7: "xor-int/2addr",
    0xb8: "shl-int/2addr", 0xb9: "shr-int/2addr", 0xba: "ushr-int/2addr",
    0xc0: "add-int/lit16", 0xc1: "rsub-int", 0xc2: "mul-int/lit16", 0xc3: "div-int/lit16",
    0xc4: "rem-int/lit16", 0xc5: "and-int/lit16", 0xc6: "or-int/lit16", 0xc7: "xor-int/lit16",
    0xd0: "add-int/lit16", 0xd1: "rsub-int", 0xd2: "mul-int/lit16", 0xd3: "div-int/lit16",
    0xd4: "rem-int/lit16", 0xd5: "and-int/lit16", 0xd6: "or-int/lit16", 0xd7: "xor-int/lit16",
    0xd8: "add-int/lit8", 0xd9: "rsub-int/lit8", 0xda: "mul-int/lit8", 0xdb: "div-int/lit8",
    0xdc: "rem-int/lit8", 0xdd: "and-int/lit8", 0xde: "or-int/lit8", 0xdf: "xor-int/lit8",
    0xe0: "shl-int/lit8", 0xe1: "shr-int/lit8", 0xe2: "ushr-int/lit8",
}


def nibble_a(unit):
    return (unit >> 8) & 0x0f


def nibble_b(unit):
    return (unit >> 12) & 0x0f


def byte_a(unit):
    return (unit >> 8) & 0xff


def signed16(value):
    return value - 0x10000 if value & 0x8000 else value


def decode_operands(op, units):
    if not units:
        return ""
    u0 = units[0]
    if op in (0x01, 0x04, 0x07):
        return f"v{nibble_a(u0)}, v{nibble_b(u0)}"
    if op in (0x02, 0x05, 0x08) and len(units) >= 2:
        return f"v{byte_a(u0)}, v{units[1]}"
    if op in (0x03, 0x06, 0x09) and len(units) >= 3:
        return f"v{units[1]}, v{units[2]}"
    if op in (0x0a, 0x0b, 0x0c, 0x0d):
        return f"v{byte_a(u0)}"
    if op == 0x12:
        lit = nibble_b(u0)
        if lit & 0x8:
            lit -= 0x10
        return f"v{nibble_a(u0)}, #{lit}"
    if op == 0x13 and len(units) >= 2:
        return f"v{byte_a(u0)}, #{signed16(units[1])}"
    if op == 0x14 and len(units) >= 3:
        value = units[1] | (units[2] << 16)
        return f"v{byte_a(u0)}, #{value:#x}"
    if op == 0x16 and len(units) >= 2:
        return f"v{byte_a(u0)}, #{signed16(units[1])}L"
    if op in (0x1a, 0x1c, 0x22, 0x23, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d) and len(units) >= 2:
        return f"v{byte_a(u0)}, @{units[1]:x}"
    if op in range(0x44, 0x52) and len(units) >= 2:
        a = byte_a(u0)
        b = units[1] & 0xff
        c = (units[1] >> 8) & 0xff
        return f"v{a}, v{b}, v{c}"
    if op in range(0x52, 0x60) and len(units) >= 2:
        a = byte_a(u0)
        b = units[1] & 0xff
        field = (units[1] >> 8) | ((units[2] << 8) if len(units) > 2 else 0)
        return f"v{a}, v{b}, field?{field:x}"
    if op in range(0x6e, 0x73) and len(units) >= 3:
        count = nibble_b(u0)
        method = units[1]
        regs = units[2]
        decoded = [regs & 0x0f, (regs >> 4) & 0x0f, (regs >> 8) & 0x0f, (regs >> 12) & 0x0f, nibble_a(u0)]
        return "{" + ", ".join(f"v{x}" for x in decoded[:count]) + "}" + f" method@{method:x}"
    if op in range(0x74, 0x79) and len(units) >= 3:
        count = byte_a(u0)
        method = units[1]
        first = units[2]
        regs = [f"v{first + i}" for i in range(count)]
        return "{" + "..".join([regs[0], regs[-1]]) + "}" + f" method@{method:x}" if regs else "{}" + f" method@{method:x}"
    if op in range(0x90, 0xaf) and len(units) >= 2:
        a = byte_a(u0)
        b = units[1] & 0xff
        c = (units[1] >> 8) & 0xff
        return f"v{a}, v{b}, v{c}"
    if op in range(0xb0, 0xcf):
        return f"v{nibble_a(u0)}, v{nibble_b(u0)}"
    if op in range(0xd0, 0xd8) and len(units) >= 2:
        a = byte_a(u0)
        b = units[1] & 0xff
        lit = signed16(units[1])
        return f"v{a}, v{b}, #{lit}"
    if op in range(0xd8, 0xe3) and len(units) >= 2:
        a = byte_a(u0)
        b = units[1] & 0xff
        lit = (units[1] >> 8) & 0xff
        if lit & 0x80:
            lit -= 0x100
        return f"v{a}, v{b}, #{lit}"
    return ""


def insn_extra(dex, op, units):
    if op in (0x1a, 0x1b):
        idx = units[1] if op == 0x1a else (units[1] | (units[2] << 16))
        return f" string@{idx:x} {dex.strings[idx]!r}" if idx < len(dex.strings) else f" string@{idx:x}"
    if op in (0x22, 0x23):
        idx = units[1]
        return f" type@{idx:x} {dex.type_name(idx) if idx < len(dex.types) else ''}"
    if op in (0x60, 0x62, 0x67, 0x69):
        return f" field@{units[1]:x}"
    if op in (0x6e, 0x6f, 0x70, 0x71, 0x72, 0x74, 0x76, 0x77):
        idx = units[1]
        return f" method@{idx:x} {dex.method_name(idx) if idx < len(dex.methods) else ''}"
    if op == 0x26 and len(units) >= 3:
        return f" payload_off={units[1] | (units[2] << 16):#x}"
    return ""


def disasm(dex, code_off, max_units=None):
    data = dex.data
    regs = u16(data, code_off)
    ins = u16(data, code_off + 2)
    outs = u16(data, code_off + 4)
    tries = u16(data, code_off + 6)
    debug = u32(data, code_off + 8)
    insns_size = u32(data, code_off + 12)
    if max_units is not None:
        insns_size = min(insns_size, max_units)
    start = code_off + 16
    print(f"code_off=0x{code_off:x} regs={regs} ins={ins} outs={outs} tries={tries} debug=0x{debug:x} insns_size={insns_size}")
    pc = 0
    while pc < insns_size:
        unit = u16(data, start + pc * 2)
        op = unit & 0xff
        length = OP_LEN.get(op, 1)
        units = [u16(data, start + (pc + i) * 2) for i in range(length) if pc + i < insns_size]
        raw = " ".join(f"{x:04x}" for x in units)
        operands = decode_operands(op, units)
        extra = insn_extra(dex, op, units)
        details = f"{operands} {extra}".strip()
        print(f"{pc:04x}  {raw:<24} {OP_NAME.get(op, f'op_{op:02x}'):<20} {details}")
        pc += max(length, 1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dex")
    ap.add_argument("--find-string")
    ap.add_argument("--class-contains")
    ap.add_argument("--method-contains")
    ap.add_argument("--code-off")
    ap.add_argument("--max-units", type=int)
    args = ap.parse_args()
    dex = Dex(args.dex)
    if args.find_string:
        for i, s in enumerate(dex.strings):
            if args.find_string in s:
                print(f"string@{i:x} {s}")
    if args.class_contains:
        for c in dex.class_defs():
            name = dex.type_name(c["class_idx"])
            if args.class_contains in name:
                print(f"class_def@{c['idx']:x} type@{c['class_idx']:x} {name} class_data=0x{c['class_data_off']:x}")
                if c["class_data_off"]:
                    for m in dex.encoded_methods_for_class(c):
                        print(f"  {m.kind:<7} method@{m.method_idx:x} code=0x{m.code_off:x} access=0x{m.access_flags:x} {dex.method_name(m.method_idx)}")
    if args.method_contains:
        for i, m in enumerate(dex.methods):
            full = dex.method_name(i)
            if args.method_contains in full:
                print(f"method@{i:x} {full}")
    if args.code_off:
        disasm(dex, int(args.code_off, 0), args.max_units)


if __name__ == "__main__":
    main()

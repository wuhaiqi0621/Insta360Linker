#!/usr/bin/env python3
from __future__ import annotations

import argparse
import struct
from pathlib import Path


def u16(data: bytes, off: int) -> int:
    return struct.unpack_from("<H", data, off)[0]


def u32(data: bytes, off: int) -> int:
    return struct.unpack_from("<I", data, off)[0]


def read_uleb(data: bytes, off: int) -> tuple[int, int]:
    value = 0
    shift = 0
    for _ in range(5):
        if off >= len(data):
            raise ValueError("uleb out of range")
        byte = data[off]
        off += 1
        value |= (byte & 0x7f) << shift
        if byte & 0x80 == 0:
            return value, off
        shift += 7
    raise ValueError("uleb too long")


class Dex:
    def __init__(self, data: bytes) -> None:
        self.data = data
        self.string_ids_size = u32(data, 0x38)
        self.string_ids_off = u32(data, 0x3c)
        self.type_ids_size = u32(data, 0x40)
        self.type_ids_off = u32(data, 0x44)
        self.proto_ids_size = u32(data, 0x48)
        self.proto_ids_off = u32(data, 0x4c)
        self.method_ids_size = u32(data, 0x58)
        self.method_ids_off = u32(data, 0x5c)

    def string(self, idx: int) -> str:
        if idx < 0 or idx >= self.string_ids_size:
            return f"<string@{idx:x} outside size=0x{self.string_ids_size:x}>"
        item_off = self.string_ids_off + idx * 4
        if item_off + 4 > len(self.data):
            return f"<string@{idx:x} item off out>"
        data_off = u32(self.data, item_off)
        if data_off >= len(self.data):
            return f"<string@{idx:x} data_off=0x{data_off:x} out>"
        try:
            _, pos = read_uleb(self.data, data_off)
            end = self.data.index(0, pos)
            raw = self.data[pos:end]
            return raw.decode("utf-8", errors="replace")
        except Exception as err:
            return f"<string@{idx:x} decode failed: {err}>"

    def type_name(self, idx: int) -> str:
        if idx < 0 or idx >= self.type_ids_size:
            return f"<type@{idx:x} outside size=0x{self.type_ids_size:x}>"
        item_off = self.type_ids_off + idx * 4
        if item_off + 4 > len(self.data):
            return f"<type@{idx:x} item off out>"
        return self.string(u32(self.data, item_off))

    def proto(self, idx: int) -> str:
        if idx < 0 or idx >= self.proto_ids_size:
            return f"<proto@{idx:x} outside size=0x{self.proto_ids_size:x}>"
        off = self.proto_ids_off + idx * 12
        if off + 12 > len(self.data):
            return f"<proto@{idx:x} item off out>"
        shorty_idx = u32(self.data, off)
        return_idx = u32(self.data, off + 4)
        params_off = u32(self.data, off + 8)
        params: list[str] = []
        if params_off:
            try:
                size = u32(self.data, params_off)
                for i in range(size):
                    params.append(self.type_name(u16(self.data, params_off + 4 + i * 2)))
            except Exception as err:
                params.append(f"<params decode failed: {err}>")
        return (
            f"shorty={self.string(shorty_idx)} "
            f"return={self.type_name(return_idx)} "
            f"params=({', '.join(params)})"
        )

    def method(self, idx: int) -> str:
        if idx < 0 or idx >= self.method_ids_size:
            return f"method@{idx:x} outside size=0x{self.method_ids_size:x}"
        off = self.method_ids_off + idx * 8
        if off + 8 > len(self.data):
            return f"method@{idx:x} item off out"
        class_idx = u16(self.data, off)
        proto_idx = u16(self.data, off + 2)
        name_idx = u32(self.data, off + 4)
        return (
            f"method@{idx:x}\n"
            f"  class={self.type_name(class_idx)}\n"
            f"  name={self.string(name_idx)}\n"
            f"  proto@{proto_idx:x}: {self.proto(proto_idx)}"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("dex")
    parser.add_argument("--method", action="append", required=True)
    args = parser.parse_args()

    dex = Dex(Path(args.dex).read_bytes())
    for raw in args.method:
        print(dex.method(int(raw, 0)))


if __name__ == "__main__":
    main()

from __future__ import annotations

import argparse
import struct
from pathlib import Path

from elftools.elf.elffile import ELFFile


DEFAULT_LIB = Path(r"F:\Insta360onWin\reverse_apk\lib\arm64-v8a\libashield.so")


def va_to_file_offset(elf: ELFFile, va: int) -> int:
    for section in elf.iter_sections():
        start = int(section["sh_addr"])
        end = start + int(section["sh_size"])
        if start <= va < end:
            return int(section["sh_offset"]) + va - start
    raise ValueError(f"VA 0x{va:x} is not inside an ELF section")


def classify(word: int) -> str:
    opa = (word >> 10) & 0x3F
    opb = (word >> 21) & 0x3F
    rd = word & 0x1F
    rn = (word >> 5) & 0x1F
    rm = (word >> 16) & 0x1F

    # These names are intentionally conservative. They are based on action
    # blocks observed in libashield.so:0x2a57c, not on public protocol info.
    if opa == 0x03:
        return f"imm/load-like dst=r{rd} base=r{rn} opb=0x{opb:02x}"
    if opa == 0x0B:
        return f"wide/arith-like dst=r{rd} rn=r{rn} opb=0x{opb:02x}"
    if opa == 0x1A:
        return f"pc/const-like dst=r{rd} rn=r{rn} opb=0x{opb:02x}"
    if opa == 0x2A:
        return f"logic/select-like dst=r{rd} rn=r{rn} rm=r{rm} opb=0x{opb:02x}"
    if opa == 0x2F:
        return f"memory/call-like dst=r{rd} rn=r{rn} rm=r{rm} opb=0x{opb:02x}"
    if opa == 0x1D:
        return f"mul/compare-like dst=r{rd} rn=r{rn} rm=r{rm} opb=0x{opb:02x}"
    return f"unknown opa=0x{opa:02x} opb=0x{opb:02x} rd=r{rd} rn=r{rn} rm=r{rm}"


def read_words(blob: bytes, offset: int, count: int) -> list[int]:
    return [struct.unpack_from("<I", blob, offset + i * 4)[0] for i in range(count)]


def main() -> None:
    parser = argparse.ArgumentParser(description="Trace Ashield VM bytecode fields.")
    parser.add_argument("--lib", type=Path, default=DEFAULT_LIB)
    parser.add_argument("--bytecode-va", type=lambda x: int(x, 0), default=0xCF570)
    parser.add_argument("--seed-va", type=lambda x: int(x, 0), default=0xF5350)
    parser.add_argument("--count", type=int, default=160)
    args = parser.parse_args()

    data = args.lib.read_bytes()
    with args.lib.open("rb") as f:
        elf = ELFFile(f)
        bytecode_off = va_to_file_offset(elf, args.bytecode_va)
        seed_off = va_to_file_offset(elf, args.seed_va)

    # 0x2a1e0 builds the VM context at sp+0x30.
    #   context[0] is written by the VM and later returned.
    #   context[1] receives the caller's x0.
    #   context[2..16] are loaded from .data.rel.ro 0xf5350..0xf53c0.
    # The rest is VM scratch/output area.
    seed_qwords = [0, None]
    seed_qwords.extend(
        struct.unpack_from("<Q", data, seed_off + i * 8)[0]
        for i in range(15)
    )
    words = read_words(data, bytecode_off, args.count)

    print(f"lib={args.lib}")
    print(f"bytecode_va=0x{args.bytecode_va:x} seed_va=0x{args.seed_va:x}")
    print()
    print("seed context qwords known before VM entry:")
    for idx, value in enumerate(seed_qwords):
        if value is None:
            print(f"  slot[{idx:02d}] @ +0x{idx * 8:03x} = <caller x0>")
        elif value:
            print(f"  slot[{idx:02d}] @ +0x{idx * 8:03x} = 0x{value:016x}")

    print()
    print("bytecode words:")
    counts: dict[tuple[int, int], int] = {}
    for i, word in enumerate(words):
        va = args.bytecode_va + i * 4
        opa = (word >> 10) & 0x3F
        opb = (word >> 21) & 0x3F
        counts[(opa, opb)] = counts.get((opa, opb), 0) + 1
        rd = word & 0x1F
        rn = (word >> 5) & 0x1F
        rm = (word >> 16) & 0x1F
        print(
            f"  {i:03d} {va:08x}: {word:08x} "
            f"opa={opa:02x} opb={opb:02x} rd={rd:02d} rn={rn:02d} rm={rm:02d} "
            f"{classify(word)}"
        )

    print()
    print("opcode pair counts:")
    for (opa, opb), count in sorted(counts.items(), key=lambda x: (-x[1], x[0])):
        print(f"  opa=0x{opa:02x} opb=0x{opb:02x}: {count}")


if __name__ == "__main__":
    main()

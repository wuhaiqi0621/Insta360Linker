from __future__ import annotations

from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from capstone.arm64 import ARM64_OP_IMM, ARM64_OP_MEM, ARM64_OP_REG
from capstone.arm64 import ARM64_REG_W2, ARM64_REG_W3, ARM64_REG_X0, ARM64_REG_X1, ARM64_REG_X2, ARM64_REG_X3
from elftools.elf.elffile import ELFFile


ROOT = Path(r"F:\Insta360onWin")
LIB = ROOT / "reverse_apk" / "lib" / "arm64-v8a" / "libashield.so"


def signed_adj(value: int) -> int:
    return value - 0x100000000 if value & 0x80000000 else value


class ElfView:
    def __init__(self, path: Path) -> None:
        self.path = path
        with path.open("rb") as f:
            self.data = f.read()
            f.seek(0)
            self.elf = ELFFile(f)
            self.sections = list(self.elf.iter_sections())
            text = self.elf.get_section_by_name(".text")
            self.text_addr = text["sh_addr"]
            self.text_data = text.data()

    def va_to_off(self, va: int) -> int | None:
        for section in self.sections:
            addr = section["sh_addr"]
            size = section["sh_size"]
            if addr <= va < addr + size:
                return section["sh_offset"] + (va - addr)
        return None

    def read_u64(self, va: int) -> int | None:
        off = self.va_to_off(va)
        if off is None or off + 8 > len(self.data):
            return None
        return int.from_bytes(self.data[off : off + 8], "little")

    def read_bytes(self, va: int, size: int) -> bytes | None:
        off = self.va_to_off(va)
        if off is None or off + size > len(self.data):
            return None
        return self.data[off : off + size]


def decoder_1a204(view: ElfView, pseudo_src: int, size: int, key: int) -> bytes | None:
    src = view.read_bytes((pseudo_src + signed_adj(0x9B6EA500)) & 0xFFFFFFFFFFFFFFFF, size)
    if src is None:
        return None
    return bytes(((((byte + 0x94) & 0xFF) ^ index) - key) & 0xFF for index, byte in enumerate(src))


def decoder_e660(view: ElfView, pseudo_src: int, size: int, key: int) -> bytes | None:
    src = view.read_bytes((pseudo_src + signed_adj(0x9FDE43F6)) & 0xFFFFFFFFFFFFFFFF, size)
    if src is None:
        return None
    return bytes(((((byte - 0x69) & 0xFF) ^ 0xAD) - index - key) & 0xFF for index, byte in enumerate(src))


def decoder_8ab44(view: ElfView, pseudo_src: int, size: int, key: int) -> bytes | None:
    src = view.read_bytes((pseudo_src + signed_adj(0xB5264604)) & 0xFFFFFFFFFFFFFFFF, size)
    if src is None:
        return None
    return bytes((((byte ^ 0x14) - index + key) & 0xFF) for index, byte in enumerate(src))


def decoder_94e64(view: ElfView, pseudo_src: int, size: int, key: int) -> bytes | None:
    src = view.read_bytes((pseudo_src + signed_adj(0x9D6CF33E)) & 0xFFFFFFFFFFFFFFFF, size)
    if src is None:
        return None
    return bytes(((((byte + 0x46) & 0xFF) - index) ^ key) & 0xFF for index, byte in enumerate(src))


def printable(raw: bytes | None) -> str:
    if raw is None:
        return "?"
    raw = raw.rstrip(b"\x00")
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw.hex(" ")


def disassemble(view: ElfView):
    md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    md.detail = True
    md.skipdata = True
    return md, list(md.disasm(view.text_data, view.text_addr))


def function_starts(insns) -> list[int]:
    starts: list[int] = []
    for index, insn in enumerate(insns):
        if insn.mnemonic != "stp" or "x29, x30" not in insn.op_str:
            continue
        prev = insns[index - 1] if index else None
        if prev and prev.address == insn.address - 4 and prev.mnemonic == "str" and "x28" in prev.op_str:
            starts.append(prev.address)
        else:
            starts.append(insn.address)
    return sorted(set(starts))


def func_for(starts: list[int], addr: int) -> int | None:
    candidates = [start for start in starts if start <= addr]
    return max(candidates) if candidates else None


def emulate_window(view: ElfView, md: Cs, insns, index: int, window: int = 30) -> dict[int, int]:
    regs: dict[int, int] = {}
    for insn in insns[max(0, index - window) : index]:
        ops = insn.operands
        if insn.mnemonic == "adrp" and len(ops) >= 2 and ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_IMM:
            regs[ops[0].reg] = ops[1].imm
        elif (
            insn.mnemonic == "add"
            and len(ops) >= 3
            and ops[0].type == ARM64_OP_REG
            and ops[1].type == ARM64_OP_REG
            and ops[2].type == ARM64_OP_IMM
            and ops[1].reg in regs
        ):
            regs[ops[0].reg] = (regs[ops[1].reg] + ops[2].imm) & 0xFFFFFFFFFFFFFFFF
        elif insn.mnemonic == "ldr" and len(ops) >= 2 and ops[0].type == ARM64_OP_REG and ops[1].type == ARM64_OP_MEM:
            mem = ops[1].mem
            if mem.base in regs:
                value = view.read_u64((regs[mem.base] + mem.disp) & 0xFFFFFFFFFFFFFFFF)
                if value is not None:
                    regs[ops[0].reg] = value
        elif insn.mnemonic == "mov" and len(ops) >= 2 and ops[0].type == ARM64_OP_REG:
            if ops[1].type == ARM64_OP_IMM:
                regs[ops[0].reg] = ops[1].imm & 0xFFFFFFFFFFFFFFFF
            elif ops[1].type == ARM64_OP_REG and ops[1].reg in regs:
                regs[ops[0].reg] = regs[ops[1].reg]
    return regs


def dump_decoder_calls(view: ElfView, target: int, name: str) -> None:
    md, insns = disassemble(view)
    starts = function_starts(insns)
    print(f"decoded {name} calls:")
    for index, insn in enumerate(insns):
        if insn.mnemonic != "bl" or not insn.operands or insn.operands[0].type != ARM64_OP_IMM:
            continue
        if insn.operands[0].imm != target:
            continue
        regs = emulate_window(view, md, insns, index)
        x0 = regs.get(ARM64_REG_X0)
        x1 = regs.get(ARM64_REG_X1)
        x2 = regs.get(ARM64_REG_W2) or regs.get(ARM64_REG_X2)
        x3 = regs.get(ARM64_REG_W3) or regs.get(ARM64_REG_X3)
        decoded = None
        if None not in (x0, x1, x2, x3):
            if target == 0x1A204:
                decoded = decoder_1a204(view, x1, x2, x3)
            elif target == 0xE660:
                decoded = decoder_e660(view, x1, x2, x3)
            elif target == 0x8AB44:
                decoded = decoder_8ab44(view, x1, x2, x3)
            elif target == 0x94E64:
                decoded = decoder_94e64(view, x1, x2, x3)
        func = func_for(starts, insn.address)
        print(f"{insn.address:#x} func={func:#x} len={x2} key={x3} -> {printable(decoded)}")


def main() -> None:
    view = ElfView(LIB)
    dump_decoder_calls(view, 0x1A204, "0x1a204")
    print()
    dump_decoder_calls(view, 0xE660, "0xe660")
    print()
    dump_decoder_calls(view, 0x8AB44, "0x8ab44")
    print()
    dump_decoder_calls(view, 0x94E64, "0x94e64")


if __name__ == "__main__":
    main()

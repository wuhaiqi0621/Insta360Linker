from pathlib import Path

from capstone import CS_ARCH_ARM64, CS_MODE_ARM, Cs
from elftools.elf.elffile import ELFFile


ROOT = Path(r"F:\Insta360onWin")
LIB = ROOT / "reverse_apk" / "lib" / "arm64-v8a" / "libashield.so"


def plt_imports(elf: ELFFile) -> dict[int, str]:
    rela = elf.get_section_by_name(".rela.plt")
    symtab = elf.get_section(rela["sh_link"])
    plt = elf.get_section_by_name(".plt")
    return {
        plt["sh_addr"] + 0x20 + i * 0x10: symtab.get_symbol(rel["r_info_sym"]).name
        for i, rel in enumerate(rela.iter_relocations())
    }


def main() -> None:
    with LIB.open("rb") as f:
        elf = ELFFile(f)
        text = elf.get_section_by_name(".text")
        code = text.data()
        base = text["sh_addr"]
        imports = plt_imports(elf)

        md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
        md.detail = True
        md.skipdata = True

        wanted = {
            "open",
            "read",
            "lseek64",
            "mmap",
            "inflateInit2_",
            "inflate",
            "inflateEnd",
            "malloc",
            "memcpy",
            "fopen",
            "dlopen",
        }

        print(f"ELF entry={elf['e_entry']:#x} text={base:#x}+{len(code):#x}")
        print("PLT imports:")
        for addr, name in sorted(imports.items()):
            if name in wanted:
                print(f"  {addr:#08x} {name}")

        print("\nInteresting imported calls:")
        for ins in md.disasm(code, base):
            if ins.mnemonic != "bl" or not ins.operands:
                continue
            target = ins.operands[0].imm
            name = imports.get(target)
            if name in wanted:
                print(f"  {ins.address:#08x} -> {name} {target:#08x}")

        print("\nCalls inside unpack cluster 0x4b000..0x50000:")
        for ins in md.disasm(code, base):
            if ins.mnemonic != "bl" or not ins.operands:
                continue
            target = ins.operands[0].imm
            if 0x4B000 <= target <= 0x50000:
                print(f"  {ins.address:#08x} -> {target:#08x}")


if __name__ == "__main__":
    main()

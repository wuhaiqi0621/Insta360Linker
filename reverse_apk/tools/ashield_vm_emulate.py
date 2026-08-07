from __future__ import annotations

import argparse
import struct
from collections import deque
from pathlib import Path

from elftools.elf.elffile import ELFFile
from unicorn import Uc, UcError, UC_ARCH_ARM64, UC_HOOK_CODE, UC_HOOK_MEM_INVALID, UC_HOOK_MEM_WRITE, UC_MODE_ARM
from unicorn.arm64_const import (
    UC_ARM64_REG_LR,
    UC_ARM64_REG_PC,
    UC_ARM64_REG_SP,
    UC_ARM64_REG_X0,
    UC_ARM64_REG_X1,
    UC_ARM64_REG_X2,
    UC_ARM64_REG_X3,
    UC_ARM64_REG_X4,
    UC_ARM64_REG_X8,
    UC_ARM64_REG_X9,
    UC_ARM64_REG_X10,
    UC_ARM64_REG_X11,
    UC_ARM64_REG_X12,
    UC_ARM64_REG_X19,
    UC_ARM64_REG_X29,
)


DEFAULT_LIB = Path(r"F:\Insta360onWin\reverse_apk\lib\arm64-v8a\libashield.so")
DEFAULT_CLASSES = Path(r"F:\Insta360onWin\reverse_apk\classes.dex")

PAGE = 0x1000
STACK_BASE = 0x7000_0000
STACK_SIZE = 0x200_000
CTX_BASE = 0x7200_0000
INPUT_BASE = 0x7300_0000
HEAP_BASE = 0x5000_0000
HEAP_SIZE = 0x1800_0000
STOP_ADDR = 0x7FFF_F000
THREAD_RETURN_ADDR = 0x7FFF_E000


def align_down(value: int, size: int = PAGE) -> int:
    return value & ~(size - 1)


def align_up(value: int, size: int = PAGE) -> int:
    return (value + size - 1) & ~(size - 1)


def map_region(mu: Uc, addr: int, size: int, perms: int = 7) -> None:
    mu.mem_map(align_down(addr), align_up((addr & (PAGE - 1)) + size), perms)


def map_elf_loads(mu: Uc, path: Path) -> bytes:
    blob = path.read_bytes()
    with path.open("rb") as f:
        elf = ELFFile(f)
        for seg in elf.iter_segments():
            if seg["p_type"] != "PT_LOAD":
                continue
            vaddr = int(seg["p_vaddr"])
            memsz = int(seg["p_memsz"])
            filesz = int(seg["p_filesz"])
            off = int(seg["p_offset"])
            start = align_down(vaddr)
            end = align_up(vaddr + memsz)
            mu.mem_map(start, end - start, 7)
            mu.mem_write(vaddr, blob[off : off + filesz])
    return blob


def va_to_file_offset(path: Path, va: int) -> int:
    with path.open("rb") as f:
        elf = ELFFile(f)
        for section in elf.iter_sections():
            start = int(section["sh_addr"])
            end = start + int(section["sh_size"])
            if start <= va < end:
                return int(section["sh_offset"]) + va - start
    raise ValueError(f"VA 0x{va:x} is not inside an ELF section")


def init_context(mu: Uc, lib_blob: bytes, seed_off: int, input_ptr: int) -> None:
    mu.mem_write(CTX_BASE, b"\x00" * 0x1000)
    struct_pack = bytearray(0x250)
    struct.pack_into("<Q", struct_pack, 0x08, input_ptr)
    for i in range(15):
        value = struct.unpack_from("<Q", lib_blob, seed_off + i * 8)[0]
        struct.pack_into("<Q", struct_pack, 0x10 + i * 8, value)
    mu.mem_write(CTX_BASE, bytes(struct_pack))


def init_input(mu: Uc, classes_path: Path) -> None:
    data = classes_path.read_bytes()
    adx = data.find(b"adx0")
    if adx < 0:
        raise RuntimeError("adx0 not found")
    meta_size = struct.unpack_from("<I", data, adx + 8)[0]
    body_off = adx + 12 + meta_size
    body = data[body_off:]
    map_region(mu, INPUT_BASE, align_up(len(body) + 0x1000))
    mu.mem_write(INPUT_BASE, body)


def dump_context(mu: Uc, count: int = 40) -> None:
    blob = mu.mem_read(CTX_BASE, 0x250)
    print("context qwords:")
    for i in range(count):
        value = struct.unpack_from("<Q", blob, i * 8)[0]
        if value:
            print(f"  slot[{i:02d}] +0x{i*8:03x} = 0x{value:016x}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Emulate Ashield VM runner 0x2a328.")
    parser.add_argument("--lib", type=Path, default=DEFAULT_LIB)
    parser.add_argument("--classes", type=Path, default=DEFAULT_CLASSES)
    parser.add_argument("--max-insn", type=int, default=200000)
    parser.add_argument("--trace", action="store_true")
    parser.add_argument("--trace-start", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--trace-end", type=lambda x: int(x, 0), default=0)
    parser.add_argument("--sync-pthreads", action="store_true", help="Run pthread start routines inline.")
    parser.add_argument("--dump-dir", type=Path, default=None, help="Write heap allocations at exit.")
    parser.add_argument("--rc4-native-prefix", type=int, default=0, help="Let native PRGA run this many bytes before fast hook.")
    parser.add_argument("--trace-reader", action="store_true", help="Trace protected chunk reader input/output.")
    parser.add_argument("--trace-threads", action="store_true", help="Trace pthread worker descriptor pointers.")
    parser.add_argument("--trace-parent", action="store_true", help="Trace parent-side pthread descriptor construction.")
    parser.add_argument("--trace-handlers", action="store_true", help="Trace 0x2848c handler/object selection.")
    parser.add_argument("--trace-outctx-writes", action="store_true", help="Trace native writes into the first 0x78-byte output context.")
    parser.add_argument("--trace-worker-array-writes", action="store_true", help="Trace parent writes into the large pthread worker descriptor array.")
    parser.add_argument("--trace-parent-stack-writes", action="store_true", help="Trace parent state-machine writes to the stack slots used by worker descriptor construction.")
    parser.add_argument("--dump-rc4-preimage-dir", type=Path, default=None, help="Write PRGA input buffers that contain target Packet/encryption strings.")
    parser.add_argument("--force-chunk-mode", type=lambda x: int(x, 0), default=None, help="Experimental: overwrite 0x2848c local mode before the chunk branch.")
    parser.add_argument("--normalize-huge-src-len", action="store_true", help="Experimental: if worker src_len is huge, use the next u32 as src_len.")
    parser.add_argument("--normalize-vm-huge-length", action="store_true", help="Experimental: normalize huge VM-read chunk lengths before they update VM registers.")
    parser.add_argument("--skip-invalid-worker", action="store_true", help="Experimental: do not enter worker threads whose source pointer is unmapped.")
    parser.add_argument("--skip-huge-worker-len", type=lambda x: int(x, 0), default=0, help="Experimental: make a worker return 0 when the normalized source length is above this threshold.")
    parser.add_argument("--zero-invalid-u32", action="store_true", help="Experimental: make 0x2905c return zero for unreadable pointers.")
    parser.add_argument("--skip-rc4", action="store_true", help="Skip the PRGA transform and jump to the loop epilogue.")
    parser.add_argument("--native-body-check", action="store_true", help="Run 0x3e1f8 instead of bypassing it.")
    args = parser.parse_args()

    mu = Uc(UC_ARCH_ARM64, UC_MODE_ARM)
    lib_blob = map_elf_loads(mu, args.lib)
    map_region(mu, STACK_BASE, STACK_SIZE)
    map_region(mu, CTX_BASE, 0x1000)
    map_region(mu, STOP_ADDR, PAGE)
    map_region(mu, THREAD_RETURN_ADDR, PAGE)
    map_region(mu, HEAP_BASE, HEAP_SIZE)
    init_input(mu, args.classes)

    seed_off = va_to_file_offset(args.lib, 0xF5350)
    init_context(mu, lib_blob, seed_off, INPUT_BASE)

    sp = STACK_BASE + STACK_SIZE - 0x100
    mu.reg_write(UC_ARM64_REG_SP, sp)
    mu.reg_write(UC_ARM64_REG_LR, STOP_ADDR)
    mu.reg_write(UC_ARM64_REG_X0, 0x250)
    mu.reg_write(UC_ARM64_REG_X1, 0xCF570)
    mu.reg_write(UC_ARM64_REG_X2, CTX_BASE)

    stats = {
        "count": 0,
        "heap": HEAP_BASE,
        "pthread_seq": 1,
        "out_ctx": 0,
        "worker_array": 0,
        "worker_array_size": 0,
        "last_worker_src": 0,
    }
    allocations: list[tuple[int, int, str]] = []
    pending_thread_return: list[tuple[int, int]] = []
    thread_results: dict[int, int] = {}
    rc4_snapshots: dict[int, tuple[bytes, bytes]] = {}
    rc4_preimage_dumps: set[tuple[int, int, int]] = set()
    recent: deque[str] = deque(maxlen=80)

    def heap_alloc(size: int, align: int = 0x10, label: str = "alloc") -> int:
        if size <= 0:
            size = 1
        ptr = align_up(stats["heap"], align)
        stats["heap"] = ptr + align_up(size, align)
        if stats["heap"] >= HEAP_BASE + HEAP_SIZE:
            raise RuntimeError(
                f"emulated heap exhausted: request=0x{size:x} ptr=0x{ptr:x} "
                f"next=0x{stats['heap']:x} limit=0x{HEAP_BASE + HEAP_SIZE:x}"
            )
        mu.mem_write(ptr, b"\x00" * align_up(size, align))
        allocations.append((ptr, size, label))
        if label == "alloc@0xa4570" and size == 0x78 and not stats["out_ctx"]:
            stats["out_ctx"] = ptr
        if size == 0x1101080 and not stats["worker_array"]:
            stats["worker_array"] = ptr
            stats["worker_array_size"] = size
            print(f"trace worker-array alloc base=0x{ptr:x} size=0x{size:x} label={label}")
        return ptr

    def safe_read(addr: int, size: int) -> bytes:
        if size <= 0:
            return b""
        return bytes(mu.mem_read(addr, size))

    def try_read(addr: int, size: int) -> bytes:
        try:
            return safe_read(addr, size)
        except UcError:
            return b""

    def qword_at(addr: int) -> int:
        data = try_read(addr, 8)
        return struct.unpack("<Q", data)[0] if len(data) == 8 else 0

    def u32_at(addr: int) -> int:
        data = try_read(addr, 4)
        return struct.unpack("<I", data)[0] if len(data) == 4 else 0

    def trace_thread_arg(prefix: str, arg: int) -> None:
        arg_head = try_read(arg, 0x40)
        out_ptr = qword_at(arg)
        src_ptr = qword_at(arg + 8)
        src_head = try_read(src_ptr, 0x40)
        body_rel = src_ptr - INPUT_BASE if INPUT_BASE <= src_ptr < INPUT_BASE + 0x08000000 else None
        body_text = f" bodyrel=0x{body_rel:x}" if body_rel is not None else ""
        src_len = u32_at(src_ptr)
        first_word_after_len = u32_at(src_ptr + 4)
        print(
            f"{prefix} arg=0x{arg:x} out=0x{out_ptr:x} src=0x{src_ptr:x}{body_text} "
            f"src_len=0x{src_len:08x} data_u32=0x{first_word_after_len:08x} "
            f"arg_head={arg_head.hex(' ')} src_head={src_head.hex(' ')}"
        )

    def trace_ptr_blob(label: str, addr: int, size: int = 0x40) -> None:
        data = try_read(addr, size)
        print(f"{label}=0x{addr:x} head={data.hex(' ')}")

    def trace_parent_state(label: str) -> None:
        sp_now = mu.reg_read(UC_ARM64_REG_SP)
        obj = qword_at(sp_now + 0x40)
        thread_ptr = qword_at(sp_now + 0x50)
        attr = qword_at(sp_now + 0x60)
        start = qword_at(sp_now + 0x70)
        arg = qword_at(obj + 0x20) if obj else 0
        print(
            f"{label} sp=0x{sp_now:x} obj=0x{obj:x} thread*=0x{thread_ptr:x} "
            f"attr=0x{attr:x} start=0x{start:x} arg_from_obj20=0x{arg:x}"
        )
        trace_ptr_blob("trace parent stack+0x40", sp_now + 0x40, 0x40)
        if obj:
            trace_ptr_blob("trace parent object", obj, 0x50)
        if arg:
            trace_thread_arg("trace parent arg_from_obj20", arg)

    def c_string(addr: int, limit: int = 0x10000) -> bytes:
        out = bytearray()
        for i in range(limit):
            try:
                ch = mu.mem_read(addr + i, 1)[0]
            except UcError:
                break
            if ch == 0:
                break
            out.append(ch)
        return bytes(out)

    def parse_c_int(data: bytes, base: int = 10) -> int:
        text = data.decode("ascii", errors="ignore").strip()
        if not text:
            return 0
        sign = 1
        if text[0] in "+-":
            sign = -1 if text[0] == "-" else 1
            text = text[1:]
        if base == 0:
            if text.lower().startswith("0x"):
                base = 16
                text = text[2:]
            elif text.startswith("0") and len(text) > 1:
                base = 8
            else:
                base = 10
        digits = []
        for ch in text:
            try:
                int(ch, base)
            except ValueError:
                break
            digits.append(ch)
        return sign * int("".join(digits), base) if digits else 0

    def rc4_prga(state_bytes: bytes, data_bytes: bytes, i: int, j: int) -> tuple[bytes, bytes, int, int]:
        state = bytearray(state_bytes)
        data = bytearray(data_bytes)
        for n in range(len(data)):
            i = (i + 1) & 0xFF
            j = (j + state[i]) & 0xFF
            state[i], state[j] = state[j], state[i]
            data[n] ^= state[(state[i] + state[j]) & 0xFF]
        return bytes(state), bytes(data), i, j

    def maybe_dump_rc4_preimage(data_ptr: int, offset: int, length: int, data: bytes) -> None:
        if args.dump_rc4_preimage_dir is None:
            return
        needles = (b"Packet", b"EncryptionManager", b"UCD2-XOR-KEY-001", b"Lcom/arashivision/onedriver/packet/Packet;")
        hits = [(needle, data.find(needle)) for needle in needles]
        hits = [(needle, pos) for needle, pos in hits if pos >= 0]
        if not hits:
            return
        key = (data_ptr, offset, length)
        if key in rc4_preimage_dumps:
            return
        rc4_preimage_dumps.add(key)
        args.dump_rc4_preimage_dir.mkdir(parents=True, exist_ok=True)
        out = args.dump_rc4_preimage_dir / f"rc4_preimage_{data_ptr:08x}_{offset:x}_{length:x}.bin"
        out.write_bytes(data)
        hit_text = ", ".join(f"{needle.decode('ascii', 'ignore')}@+0x{pos:x}" for needle, pos in hits)
        print(f"dump rc4 preimage {out} hits={hit_text}")

    def return_from_hook(value: int) -> None:
        lr = mu.reg_read(UC_ARM64_REG_LR)
        mu.reg_write(UC_ARM64_REG_X0, value & 0xFFFFFFFFFFFFFFFF)
        mu.reg_write(UC_ARM64_REG_PC, lr)

    def hook_code(mu: Uc, addr: int, size: int, user_data: object) -> None:
        stats["count"] += 1
        recent.append(
            f"pc=0x{addr:x} x0=0x{mu.reg_read(UC_ARM64_REG_X0):x} "
            f"x1=0x{mu.reg_read(UC_ARM64_REG_X1):x} x2=0x{mu.reg_read(UC_ARM64_REG_X2):x} "
            f"x3=0x{mu.reg_read(UC_ARM64_REG_X3):x} "
            f"x8=0x{mu.reg_read(UC_ARM64_REG_X8):x} x9=0x{mu.reg_read(UC_ARM64_REG_X9):x} "
            f"lr=0x{mu.reg_read(UC_ARM64_REG_LR):x}"
        )
        if addr == THREAD_RETURN_ADDR:
            thread_value = mu.reg_read(UC_ARM64_REG_X0)
            caller_lr, fake_tid = pending_thread_return.pop() if pending_thread_return else (STOP_ADDR, 0)
            if fake_tid:
                thread_results[fake_tid] = thread_value
            print(f"hook pthread inline return tid=0x{fake_tid:x} value=0x{thread_value:x} -> 0x{caller_lr:x}")
            mu.reg_write(UC_ARM64_REG_X0, 0)
            mu.reg_write(UC_ARM64_REG_PC, caller_lr)
            return
        if args.trace_threads and addr == 0x2A0BC:
            trace_thread_arg(f"trace worker pc=0x{addr:x}", mu.reg_read(UC_ARM64_REG_X0))
        if args.trace_threads and addr == 0x2A118:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            data_ptr = qword_at(sp_now + 0x38)
            out_ctx = qword_at(sp_now + 0x48)
            src_len = mu.reg_read(UC_ARM64_REG_X1) & 0xFFFFFFFF
            data_len = u32_at(data_ptr)
            print(
                f"trace call 0x2848c data=0x{data_ptr:x} src_len=0x{src_len:08x} "
                f"data_u32=0x{data_len:08x} out=0x{out_ctx:x}"
            )
        if args.normalize_huge_src_len and addr == 0x2A118:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            data_ptr = qword_at(sp_now + 0x38)
            src_len = mu.reg_read(UC_ARM64_REG_X1) & 0xFFFFFFFF
            candidate = u32_at(data_ptr)
            if src_len > 0x20000000 and 0 < candidate < 0x20000000:
                mu.reg_write(UC_ARM64_REG_X1, candidate)
                print(
                    f"normalize huge src_len old=0x{src_len:08x} "
                    f"new=0x{candidate:08x} data=0x{data_ptr:x}"
                )
        if args.skip_huge_worker_len and addr == 0x2A118:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            data_ptr = qword_at(sp_now + 0x38)
            src_len = mu.reg_read(UC_ARM64_REG_X1) & 0xFFFFFFFF
            candidate = u32_at(data_ptr)
            effective = candidate if src_len > 0x20000000 and candidate else src_len
            if effective > args.skip_huge_worker_len:
                print(
                    f"skip huge worker len=0x{effective:08x} "
                    f"threshold=0x{args.skip_huge_worker_len:08x} data=0x{data_ptr:x}"
                )
                mu.reg_write(UC_ARM64_REG_X0, 0)
                mu.reg_write(UC_ARM64_REG_PC, THREAD_RETURN_ADDR)
                return
        if args.normalize_vm_huge_length and addr == 0x2C368:
            source_ptr = int(stats.get("last_worker_src", 0))
            current = mu.reg_read(UC_ARM64_REG_X10) & 0xFFFFFFFF
            candidate = u32_at(source_ptr + 4) if source_ptr else 0
            source_head = u32_at(source_ptr) if source_ptr else 0
            if current > 0x20000000 and source_head == current and 0 < candidate < 0x20000000 and try_read(source_ptr, 8):
                mu.reg_write(UC_ARM64_REG_X10, candidate)
                print(
                    f"normalize vm huge length old=0x{current:08x} "
                    f"new=0x{candidate:08x} source=0x{source_ptr:x}"
                )
        if args.trace_parent and addr in (0x2D5AC, 0x2D7DC, 0x2D824):
            trace_parent_state(f"trace parent pc=0x{addr:x}")
        if args.trace_handlers and addr in (0x284EC, 0x28500, 0x285B0, 0x287A0, 0x287B4, 0x287BC, 0x287E0, 0x28994, 0x289B8):
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            if addr == 0x284EC:
                obj = qword_at(sp_now + 0x218)
                trace_ptr_blob("trace handler object", obj, 0x80)
                if obj:
                    trace_ptr_blob("trace handler object+0x50", obj + 0x50, 0x28)
                trace_ptr_blob("trace handler vtbl", qword_at(obj), 0x30)
            elif addr == 0x28500:
                obj = mu.reg_read(UC_ARM64_REG_X0)
                vtbl = qword_at(obj)
                fn = qword_at(vtbl + 0x18)
                print(f"trace handler vcall object=0x{obj:x} vtbl=0x{vtbl:x} fn18=0x{fn:x}")
            elif addr == 0x285B0:
                local = (mu.reg_read(UC_ARM64_REG_X29) - 0x40) & 0xFFFFFFFFFFFFFFFF
                trace_ptr_blob("trace handler vcall result local", local, 0x40)
            elif addr == 0x287A0:
                print(
                    f"trace handler direct-match x0=0x{mu.reg_read(UC_ARM64_REG_X0):x} "
                    f"x3=0x{mu.reg_read(UC_ARM64_REG_X3):x} x4=0x{mu.reg_read(UC_ARM64_REG_X4):x}"
                )
            elif addr == 0x287B4:
                x0 = mu.reg_read(UC_ARM64_REG_X0)
                x1 = mu.reg_read(UC_ARM64_REG_X1)
                x2 = mu.reg_read(UC_ARM64_REG_X2)
                x3 = mu.reg_read(UC_ARM64_REG_X3)
                x4 = mu.reg_read(UC_ARM64_REG_X4)
                print(
                    f"trace handler compare-call x0=0x{x0:x} {c_string(x0, 0x80)!r} "
                    f"x1=0x{x1:x} x2=0x{x2:x} "
                    f"x3=0x{x3:x} {c_string(x3, 0x80)!r} x4=0x{x4:x} {c_string(x4, 0x80)!r}"
                )
            elif addr == 0x287BC:
                print(f"trace handler compare-ret w0=0x{mu.reg_read(UC_ARM64_REG_X0) & 0xFFFFFFFF:08x}")
            elif addr == 0x287E0:
                ok = try_read(sp_now + 0x2AF, 1)
                mode = u32_at(sp_now + 0x200)
                print(f"trace handler match-ok={ok.hex(' ') if ok else ''} mode=0x{mode:08x}")
            elif addr == 0x28994:
                x0 = mu.reg_read(UC_ARM64_REG_X0)
                print(f"trace handler atoi-call x0=0x{x0:x} {c_string(x0, 0x80)!r}")
            elif addr == 0x289B8:
                mode = u32_at(sp_now + 0x200)
                print(f"trace handler final mode before chunk=0x{mode:08x}")
        if addr in (0xA4570, 0xE070):
            requested = mu.reg_read(UC_ARM64_REG_X0)
            ptr = heap_alloc(requested, label=f"alloc@0x{addr:x}")
            print(f"hook alloc pc=0x{addr:x} size=0x{requested:x} -> 0x{ptr:x}")
            return_from_hook(ptr)
            return
        if addr == 0xDCB0:
            count = mu.reg_read(UC_ARM64_REG_X0)
            size_each = mu.reg_read(UC_ARM64_REG_X1)
            total = count * size_each
            ptr = heap_alloc(total, label="calloc")
            print(f"hook calloc count=0x{count:x} size=0x{size_each:x} -> 0x{ptr:x}")
            return_from_hook(ptr)
            return
        if addr == 0xDC20:
            old = mu.reg_read(UC_ARM64_REG_X0)
            new_size = mu.reg_read(UC_ARM64_REG_X1)
            ptr = heap_alloc(new_size, label="realloc")
            if old and new_size:
                try:
                    mu.mem_write(ptr, safe_read(old, min(new_size, 0x100000)))
                except UcError:
                    pass
            print(f"hook realloc old=0x{old:x} size=0x{new_size:x} -> 0x{ptr:x}")
            return_from_hook(ptr)
            return
        if addr in (0xE080, 0xDFB0):
            dst = mu.reg_read(UC_ARM64_REG_X0)
            src = mu.reg_read(UC_ARM64_REG_X1)
            length = mu.reg_read(UC_ARM64_REG_X2)
            if length:
                mu.mem_write(dst, safe_read(src, length))
            print(f"hook {'memcpy' if addr == 0xE080 else 'memmove'} dst=0x{dst:x} src=0x{src:x} len=0x{length:x}")
            return_from_hook(dst)
            return
        if addr == 0xE090:
            dst = mu.reg_read(UC_ARM64_REG_X0)
            value = mu.reg_read(UC_ARM64_REG_X1) & 0xFF
            length = mu.reg_read(UC_ARM64_REG_X2)
            if length:
                mu.mem_write(dst, bytes([value]) * length)
            print(f"hook memset dst=0x{dst:x} value=0x{value:x} len=0x{length:x}")
            return_from_hook(dst)
            return
        if addr == 0x290D4:
            dst = mu.reg_read(UC_ARM64_REG_X0)
            src = mu.reg_read(UC_ARM64_REG_X1)
            length = mu.reg_read(UC_ARM64_REG_X2) & 0xFFFFFFFF
            if length:
                mu.mem_write(dst, safe_read(src, length))
            print(f"hook copy-state-machine 0x290d4 dst=0x{dst:x} src=0x{src:x} len=0x{length:x}")
            return_from_hook(dst)
            return
        if args.zero_invalid_u32 and addr == 0x2905C:
            src = mu.reg_read(UC_ARM64_REG_X0)
            if not try_read(src, 4):
                print(f"zero invalid u32 read src=0x{src:x}")
                return_from_hook(0)
                return
        if addr == 0xE000:
            src = mu.reg_read(UC_ARM64_REG_X0)
            length = len(c_string(src))
            print(f"hook strlen src=0x{src:x} -> 0x{length:x}")
            return_from_hook(length)
            return
        if addr == 0xE100:
            dst = mu.reg_read(UC_ARM64_REG_X0)
            src = mu.reg_read(UC_ARM64_REG_X1)
            data = c_string(src) + b"\x00"
            mu.mem_write(dst, data)
            print(f"hook strcpy dst=0x{dst:x} src=0x{src:x} len=0x{len(data)-1:x}")
            return_from_hook(dst)
            return
        if addr in (0xDBF0, 0xDD20, 0xE0D0):
            src = mu.reg_read(UC_ARM64_REG_X0)
            end_ptr = mu.reg_read(UC_ARM64_REG_X1)
            base = mu.reg_read(UC_ARM64_REG_X2) if addr in (0xDD20, 0xE0D0) else 10
            text = c_string(src, 0x1000)
            value = parse_c_int(text, base)
            if addr in (0xDD20, 0xE0D0) and end_ptr:
                mu.mem_write(end_ptr, struct.pack("<Q", src + len(text)))
            name = {0xDBF0: "atoi", 0xDD20: "strtoul", 0xE0D0: "strtol"}[addr]
            print(f"hook {name} src=0x{src:x} text={text!r} base={base} -> {value}")
            return_from_hook(value)
            return
        if addr in (0xDF40, 0xDCE0, 0xDDF0):
            left = mu.reg_read(UC_ARM64_REG_X0)
            right = mu.reg_read(UC_ARM64_REG_X1)
            limit = mu.reg_read(UC_ARM64_REG_X2) if addr == 0xDCE0 else 0x10000
            a = c_string(left, limit)
            b = c_string(right, limit)
            if addr == 0xDDF0:
                a = a.lower()
                b = b.lower()
            result = 0
            for av, bv in zip(a, b):
                if av != bv:
                    result = av - bv
                    break
            if result == 0 and len(a) != len(b) and addr != 0xDCE0:
                result = len(a) - len(b)
            name = {0xDF40: "strcmp", 0xDCE0: "strncmp", 0xDDF0: "strcasecmp"}[addr]
            print(f"hook {name} left=0x{left:x} {a!r} right=0x{right:x} {b!r} -> {result}")
            return_from_hook(result)
            return
        if addr == 0xDE30:
            left = mu.reg_read(UC_ARM64_REG_X0)
            right = mu.reg_read(UC_ARM64_REG_X1)
            length = mu.reg_read(UC_ARM64_REG_X2)
            a = safe_read(left, length)
            b = safe_read(right, length)
            result = 0
            for av, bv in zip(a, b):
                if av != bv:
                    result = av - bv
                    break
            print(f"hook memcmp left=0x{left:x} right=0x{right:x} len=0x{length:x} -> {result}")
            return_from_hook(result)
            return
        if addr in (0xDBB0, 0xDBE0, 0xDC40, 0xDC60, 0xDD30, 0xDF70, 0xE0C0, 0xE120):
            print(f"hook pthread/libc noop pc=0x{addr:x}")
            return_from_hook(0)
            return
        if addr in (0xE020, 0xE060):
            print(f"hook close/munmap noop pc=0x{addr:x}")
            return_from_hook(0)
            return
        if addr == 0xDF80:
            mmap_addr = mu.reg_read(UC_ARM64_REG_X0)
            length = mu.reg_read(UC_ARM64_REG_X1)
            prot = mu.reg_read(UC_ARM64_REG_X2)
            flags = mu.reg_read(UC_ARM64_REG_X3)
            if 0 < length < 0x20000000:
                ptr = heap_alloc(length, align=PAGE, label="mmap")
                print(f"hook mmap addr=0x{mmap_addr:x} len=0x{length:x} prot=0x{prot:x} flags=0x{flags:x} -> 0x{ptr:x}")
                return_from_hook(ptr)
            else:
                print(f"hook mmap addr=0x{mmap_addr:x} len=0x{length:x} prot=0x{prot:x} flags=0x{flags:x} -> MAP_FAILED")
                return_from_hook(0xFFFFFFFFFFFFFFFF)
            return
        if args.trace_reader and addr == 0x5F15C:
            reader_buf = mu.reg_read(UC_ARM64_REG_X1)
            reader_len = mu.reg_read(UC_ARM64_REG_X2) & 0xFFFFFFFF
            head = safe_read(reader_buf, min(reader_len, 0x30))
            field5 = struct.unpack("<I", head[5:9])[0] if len(head) >= 9 else 0
            field9 = struct.unpack("<I", head[9:13])[0] if len(head) >= 13 else 0
            print(
                f"trace reader enter buf=0x{reader_buf:x} len=0x{reader_len:x} "
                f"head={head.hex(' ')} field5=0x{field5:08x} field9=0x{field9:08x}"
            )
        if args.trace_reader and addr == 0x5F2E0:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            ret_value = struct.unpack("<I", safe_read(sp_now + 0x10, 4))[0]
            print(f"trace reader return value=0x{ret_value:08x}")
        if args.trace_reader and addr == 0x28E28:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            orig_len = struct.unpack("<I", safe_read(sp_now + 0x244, 4))[0]
            consumed = struct.unpack("<I", safe_read(sp_now + 0x1FC, 4))[0]
            reader_ret = struct.unpack("<I", safe_read(sp_now + 0x1C4, 4))[0]
            remaining = struct.unpack("<I", safe_read(sp_now + 0x1C0, 4))[0]
            total = (reader_ret + remaining) & 0xFFFFFFFF
            print(
                f"trace chunk-len orig=0x{orig_len:08x} consumed=0x{consumed:08x} "
                f"reader_ret=0x{reader_ret:08x} remaining=0x{remaining:08x} total=0x{total:08x}"
            )
        if args.trace_reader and addr == 0x289DC:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            mode = struct.unpack("<I", safe_read(sp_now + 0x200, 4))[0]
            consumed = struct.unpack("<I", safe_read(sp_now + 0x1FC, 4))[0]
            data_ptr = struct.unpack("<Q", safe_read(sp_now + 0x248, 8))[0]
            orig_len = struct.unpack("<I", safe_read(sp_now + 0x244, 4))[0]
            print(
                f"trace chunk-mode mode=0x{mode:08x} orig=0x{orig_len:08x} "
                f"consumed=0x{consumed:08x} data_ptr=0x{data_ptr:x}"
            )
        if args.force_chunk_mode is not None and addr == 0x289DC:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            forced = args.force_chunk_mode & 0xFFFFFFFF
            old_mode = u32_at(sp_now + 0x200)
            mu.mem_write(sp_now + 0x200, struct.pack("<I", forced))
            print(f"force chunk mode old=0x{old_mode:08x} new=0x{forced:08x}")
        if args.trace_handlers and addr == 0x49AB8:
            candidate = mu.reg_read(UC_ARM64_REG_X0)
            aux = mu.reg_read(UC_ARM64_REG_X1)
            print(f"trace handler lookup candidate=0x{candidate:x} text={c_string(candidate, 0x200)!r} aux=0x{aux:x}")
        if args.native_body_check and addr == 0x3DCD4:
            src = mu.reg_read(UC_ARM64_REG_X0)
            length = mu.reg_read(UC_ARM64_REG_X1)
            rolling_ptr = mu.reg_read(UC_ARM64_REG_X2)
            acc_ptr = mu.reg_read(UC_ARM64_REG_X3)
            acc = u32_at(acc_ptr)
            rolling = bytearray(try_read(rolling_ptr, 16))
            if len(rolling) < 16:
                rolling.extend(b"\x00" * (16 - len(rolling)))
            data = safe_read(src, length)
            for index, value in enumerate(data):
                idx = index & 0xFF
                mixed = ((value ^ idx) ^ ((value + idx) & 0xFF)) & 0xFF
                rolling[index & 0x0F] = ((acc ^ value) ^ mixed) & 0xFF
                acc = (acc ^ (index ^ mixed)) & 0xFFFFFFFF
            mu.mem_write(rolling_ptr, bytes(rolling))
            mu.mem_write(acc_ptr, struct.pack("<I", acc))
            print(
                f"hook body rolling pc=0x{addr:x} src=0x{src:x} len=0x{length:x} "
                f"rolling_ptr=0x{rolling_ptr:x} acc_ptr=0x{acc_ptr:x} "
                f"rolling={bytes(rolling).hex(' ')} acc=0x{acc:08x}"
            )
            return_from_hook(0)
            return
        if args.native_body_check and addr == 0x3DB5C:
            dst = mu.reg_read(UC_ARM64_REG_X0)
            src = mu.reg_read(UC_ARM64_REG_X1)
            length = mu.reg_read(UC_ARM64_REG_X2)
            print(f"trace body precheck native pc=0x{addr:x} dst=0x{dst:x} src=0x{src:x} len=0x{length:x}")
        if args.native_body_check and addr == 0x3E2A4:
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            offset = struct.unpack("<I", safe_read(sp_now + 0x44C, 4))[0]
            length = struct.unpack("<I", safe_read(sp_now + 0x45C, 4))[0]
            buf = struct.unpack("<Q", safe_read(sp_now + 0x450, 8))[0]
            if offset < length:
                data = bytearray(safe_read(buf + offset, length - offset))
                for idx in range(len(data)):
                    data[idx] ^= 0x52
                mu.mem_write(buf + offset, bytes(data))
                mu.mem_write(sp_now + 0x44C, struct.pack("<I", length))
                print(f"hook body xor52 buf=0x{buf:x} offset=0x{offset:x} len=0x{length:x}")
            mu.reg_write(UC_ARM64_REG_PC, 0x3E2E8)
            return
        if addr in (0x4A99C, 0x4AAE8):
            if addr == 0x4A99C:
                state_ptr = mu.reg_read(UC_ARM64_REG_X0)
                data_ptr = mu.reg_read(UC_ARM64_REG_X1)
                length = mu.reg_read(UC_ARM64_REG_X2)
                offset = 0
                i = 0
                j = 0
            else:
                sp_now = mu.reg_read(UC_ARM64_REG_SP)
                state_ptr = struct.unpack("<Q", safe_read(sp_now + 0x50, 8))[0]
                data_ptr = struct.unpack("<Q", safe_read(sp_now + 0x48, 8))[0]
                length = struct.unpack("<Q", safe_read(sp_now + 0x40, 8))[0]
                offset = struct.unpack("<Q", safe_read(sp_now + 0x28, 8))[0]
                i = struct.unpack("<i", safe_read(sp_now + 0x3C, 4))[0] & 0xFF
                j = struct.unpack("<i", safe_read(sp_now + 0x38, 4))[0] & 0xFF
            if addr != 0x4A99C and args.rc4_native_prefix > 0:
                prefix = min(args.rc4_native_prefix, length)
                if offset == 0 and data_ptr not in rc4_snapshots:
                    rc4_snapshots[data_ptr] = (safe_read(state_ptr, 0x100), safe_read(data_ptr, prefix))
                    print(f"rc4 native-prefix snapshot data=0x{data_ptr:x} prefix=0x{prefix:x}")
                if offset < prefix:
                    return
                if data_ptr in rc4_snapshots:
                    initial_state, initial_data = rc4_snapshots.pop(data_ptr)
                    expected_state, expected_data, expected_i, expected_j = rc4_prga(initial_state, initial_data, 0, 0)
                    actual_state = safe_read(state_ptr, 0x100)
                    actual_data = safe_read(data_ptr, prefix)
                    state_ok = actual_state == expected_state
                    data_ok = actual_data == expected_data
                    ij_ok = i == expected_i and j == expected_j
                    print(
                        f"rc4 native-prefix compare data=0x{data_ptr:x} prefix=0x{prefix:x} "
                        f"state_ok={state_ok} data_ok={data_ok} ij_ok={ij_ok} "
                        f"native_i=0x{i:x} native_j=0x{j:x} expected_i=0x{expected_i:x} expected_j=0x{expected_j:x}"
                    )
                    if not data_ok:
                        for n, (av, bv) in enumerate(zip(actual_data, expected_data)):
                            if av != bv:
                                print(f"rc4 first data mismatch +0x{n:x}: native=0x{av:02x} python=0x{bv:02x}")
                                break
            state = bytearray(safe_read(state_ptr, 0x100))
            remaining = length - offset
            data = bytearray(safe_read(data_ptr + offset, remaining))
            maybe_dump_rc4_preimage(data_ptr, offset, length, bytes(data))
            if args.skip_rc4:
                new_state = bytes(state)
                new_data = bytes(data)
                print("hook rc4-prga skip transform")
            else:
                new_state, new_data, i, j = rc4_prga(bytes(state), bytes(data), i, j)
                mu.mem_write(state_ptr, new_state)
                mu.mem_write(data_ptr + offset, new_data)
            print(
                f"hook rc4-prga pc=0x{addr:x} state=0x{state_ptr:x} "
                f"data=0x{data_ptr:x} offset=0x{offset:x} len=0x{length:x}"
            )
            if addr == 0x4A99C:
                return_from_hook(0)
            else:
                sp_now = mu.reg_read(UC_ARM64_REG_SP)
                mu.mem_write(sp_now + 0x28, struct.pack("<Q", length))
                mu.mem_write(sp_now + 0x3C, struct.pack("<i", i))
                mu.mem_write(sp_now + 0x38, struct.pack("<i", j))
                mu.reg_write(UC_ARM64_REG_PC, 0x4AC54)
            return
        if addr == 0xDC00:
            thread_ptr = mu.reg_read(UC_ARM64_REG_X0)
            start = mu.reg_read(UC_ARM64_REG_X2)
            arg = mu.reg_read(UC_ARM64_REG_X3)
            caller_lr = mu.reg_read(UC_ARM64_REG_LR)
            fake_tid = stats["pthread_seq"]
            stats["pthread_seq"] += 1
            if thread_ptr:
                mu.mem_write(thread_ptr, struct.pack("<Q", fake_tid))
            print(f"hook pthread_create thread*=0x{thread_ptr:x} start=0x{start:x} arg=0x{arg:x} sync={args.sync_pthreads}")
            if args.trace_threads:
                trace_thread_arg("trace pthread arg", arg)
            stats["last_worker_src"] = qword_at(arg + 8)
            if args.skip_invalid_worker:
                src = qword_at(arg + 8)
                if src and not try_read(src, 4):
                    print(f"skip invalid worker arg=0x{arg:x} src=0x{src:x}")
                    return_from_hook(0)
                    return
            if args.sync_pthreads and start:
                pending_thread_return.append((caller_lr, fake_tid))
                mu.reg_write(UC_ARM64_REG_X0, arg)
                mu.reg_write(UC_ARM64_REG_LR, THREAD_RETURN_ADDR)
                mu.reg_write(UC_ARM64_REG_PC, start)
                return
            return_from_hook(0)
            return
        if addr == 0xDC10:
            tid = mu.reg_read(UC_ARM64_REG_X0)
            value_ptr = mu.reg_read(UC_ARM64_REG_X1)
            thread_value = thread_results.get(tid, 0)
            if value_ptr:
                mu.mem_write(value_ptr, struct.pack("<Q", thread_value))
            print(f"hook pthread_join tid=0x{tid:x} value_ptr=0x{value_ptr:x} value=0x{thread_value:x} -> 0")
            return_from_hook(0)
            return
        if addr == 0x3E1F8 and not args.native_body_check:
            x0 = mu.reg_read(UC_ARM64_REG_X0)
            x1 = mu.reg_read(UC_ARM64_REG_X1)
            x2 = mu.reg_read(UC_ARM64_REG_X2)
            print(f"hook body-check pc=0x{addr:x} dst=0x{x0:x} src=0x{x1:x} len=0x{x2:x} -> 1")
            return_from_hook(1)
            return
        if addr in (0xE0F0, 0xA184C, 0xA45D8):
            ptr = mu.reg_read(UC_ARM64_REG_X0)
            print(f"hook free/noop pc=0x{addr:x} ptr=0x{ptr:x}")
            return_from_hook(0)
            return
        in_trace_range = args.trace_start <= addr < args.trace_end if args.trace_end else False
        if args.trace and (in_trace_range or addr in (0x2A328, 0x2A57C, 0x2AFD8, 0x2B098, 0x2C9E4, 0x2CA28, 0x3E1F8) or stats["count"] < 30):
            x0 = mu.reg_read(UC_ARM64_REG_X0)
            x1 = mu.reg_read(UC_ARM64_REG_X1)
            x2 = mu.reg_read(UC_ARM64_REG_X2)
            x8 = mu.reg_read(UC_ARM64_REG_X8)
            x9 = mu.reg_read(UC_ARM64_REG_X9)
            print(f"pc=0x{addr:x} size={size} x0=0x{x0:x} x1=0x{x1:x} x2=0x{x2:x} x8=0x{x8:x} x9=0x{x9:x}")
        if stats["count"] >= args.max_insn:
            print("max instruction count reached")
            mu.emu_stop()

    def hook_invalid(mu: Uc, access: int, address: int, size: int, value: int, user_data: object) -> bool:
        pc = mu.reg_read(UC_ARM64_REG_PC)
        print(f"invalid memory access pc=0x{pc:x} access={access} addr=0x{address:x} size={size} value=0x{value:x}")
        regs = {
            "x0": UC_ARM64_REG_X0,
            "x1": UC_ARM64_REG_X1,
            "x2": UC_ARM64_REG_X2,
            "x3": UC_ARM64_REG_X3,
            "x4": UC_ARM64_REG_X4,
            "x8": UC_ARM64_REG_X8,
            "x9": UC_ARM64_REG_X9,
            "x10": UC_ARM64_REG_X10,
            "x11": UC_ARM64_REG_X11,
            "x12": UC_ARM64_REG_X12,
            "x19": UC_ARM64_REG_X19,
        }
        for name, reg in regs.items():
            print(f"  {name}=0x{mu.reg_read(reg):016x}")
        return False

    def hook_mem_write(mu: Uc, access: int, address: int, size: int, value: int, user_data: object) -> None:
        if args.trace_parent_stack_writes:
            pc = mu.reg_read(UC_ARM64_REG_PC)
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            sp230 = qword_at(sp_now + 0x230)
            if sp230 and sp230 <= address < sp230 + 0x100:
                slot = (address - sp230) // 8
                slot_off = (address - sp230) % 8
                print(
                    f"trace vm-reg write pc=0x{pc:x} slot={slot} off=0x{slot_off:x} "
                    f"addr=0x{address:x} size={size} value=0x{value:x} "
                    f"sp254=0x{u32_at(sp_now + 0x254):08x} "
                    f"x8=0x{mu.reg_read(UC_ARM64_REG_X8):x} x9=0x{mu.reg_read(UC_ARM64_REG_X9):x} "
                    f"x10=0x{mu.reg_read(UC_ARM64_REG_X10):x} x11=0x{mu.reg_read(UC_ARM64_REG_X11):x}"
                )
            if sp_now + 0x560 <= address < sp_now + 0x598:
                sp254 = u32_at(sp_now + 0x254)
                print(
                    f"trace parent-stack write pc=0x{pc:x} off=0x{address - sp_now:x} "
                    f"size={size} value=0x{value:x} "
                    f"sp230=0x{sp230:x} sp254=0x{sp254:08x} "
                    f"x0=0x{mu.reg_read(UC_ARM64_REG_X0):x} x8=0x{mu.reg_read(UC_ARM64_REG_X8):x} "
                    f"x9=0x{mu.reg_read(UC_ARM64_REG_X9):x} x10=0x{mu.reg_read(UC_ARM64_REG_X10):x} "
                    f"x11=0x{mu.reg_read(UC_ARM64_REG_X11):x}"
                )
        out_ctx = int(stats.get("out_ctx", 0))
        if args.trace_outctx_writes and out_ctx and out_ctx <= address < out_ctx + 0x78:
            pc = mu.reg_read(UC_ARM64_REG_PC)
            print(
                f"trace outctx write pc=0x{pc:x} off=0x{address - out_ctx:x} "
                f"size={size} value=0x{value:x}"
            )
        worker_array = int(stats.get("worker_array", 0))
        worker_array_size = int(stats.get("worker_array_size", 0))
        if args.trace_worker_array_writes and worker_array and worker_array <= address < worker_array + min(worker_array_size, 0x200):
            pc = mu.reg_read(UC_ARM64_REG_PC)
            off = address - worker_array
            slot = off // 0x10
            slot_off = off % 0x10
            sp_now = mu.reg_read(UC_ARM64_REG_SP)
            x0 = mu.reg_read(UC_ARM64_REG_X0)
            x10 = mu.reg_read(UC_ARM64_REG_X10)
            x11 = mu.reg_read(UC_ARM64_REG_X11)
            stack_570 = qword_at(sp_now + 0x570)
            stack_580 = qword_at(sp_now + 0x580)
            stack_58c = u32_at(sp_now + 0x58C)
            print(
                f"trace worker-array write pc=0x{pc:x} slot={slot} off=0x{slot_off:x} "
                f"addr=0x{address:x} size={size} value=0x{value:x} "
                f"x0=0x{x0:x} x10=0x{x10:x} x11=0x{x11:x} "
                f"sp570=0x{stack_570:x} sp580=0x{stack_580:x} sp58c=0x{stack_58c:x}"
            )

    mu.hook_add(UC_HOOK_CODE, hook_code)
    mu.hook_add(UC_HOOK_MEM_INVALID, hook_invalid)
    mu.hook_add(UC_HOOK_MEM_WRITE, hook_mem_write)

    try:
        mu.emu_start(0x2A328, STOP_ADDR, count=args.max_insn)
    except (UcError, RuntimeError) as exc:
        print(f"emulation stopped with {exc}")
        print("recent instructions:")
        for line in recent:
            print("  " + line)

    print(f"instructions={stats['count']}")
    print(f"pc=0x{mu.reg_read(UC_ARM64_REG_PC):x}")
    print(f"x0=0x{mu.reg_read(UC_ARM64_REG_X0):x}")
    dump_context(mu)
    print("heap allocations:")
    for ptr, size, label in allocations:
        print(f"  0x{ptr:08x} size=0x{size:x} {label}")
        try:
            sample_size = min(size, 0x2000000)
            blob = safe_read(ptr, sample_size)
        except UcError:
            continue
        for needle in (b"dex\n", b"Packet", b"EncryptionManager", b"UCD2-XOR-KEY-001"):
            pos = blob.find(needle)
            if pos >= 0:
                print(f"    contains {needle!r} at +0x{pos:x}")
        if args.dump_dir is not None:
            args.dump_dir.mkdir(parents=True, exist_ok=True)
            dump_path = args.dump_dir / f"heap_{ptr:08x}_{size:x}_{label.replace('@', '_').replace(':', '_')}.bin"
            dump_path.write_bytes(safe_read(ptr, size))
            print(f"    dumped {dump_path}")


if __name__ == "__main__":
    main()

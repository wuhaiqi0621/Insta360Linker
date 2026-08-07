#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass


POLY = 0x04C11DB7


def crc_table() -> list[int]:
    table: list[int] = []
    for i in range(256):
        crc = i << 24
        for _ in range(8):
            if crc & 0x80000000:
                crc = ((crc << 1) ^ POLY) & 0xFFFFFFFF
            else:
                crc = (crc << 1) & 0xFFFFFFFF
        table.append(crc)
    return table


CRC_TABLE = crc_table()

UCD2_XOR_KEY = b"UCD2-XOR-KEY-001"
UCD2_NEGOTIATION_SYNC = b"syNceNdinS"
UCD2_NEGOTIATION_ACK_8 = bytes.fromhex("08 00 00 00 b0 00 00 01")
UCD2_NEGOTIATION_ACK_7 = bytes.fromhex("07 00 00 00 05 00 00")
OBSERVED_DEVICE_INFO_TIME_BYTES = bytes.fromhex("01 00 00 80")
STOP_CAPTURE_BUILDER_DESC = "Linsta360/messages/StopCapture$Builder;"
STOP_CAPTURE_ADAPTER_DESC = "Linsta360/messages/StopCapture$Companion$ADAPTER$1;"
STOP_CAPTURE_DESC = "Linsta360/messages/StopCapture;"
STOP_CAPTURE_TYPE_URL = "type.googleapis.com/insta360.messages.StopCapture"
WRAPPER_DESCRIPTOR_3865 = "\x10\x18\x1a\x020\x012\n\x10\r\x1a\x06\x12\x02\x08\x030\x0eH\x16"


def ucd2_crc(data: bytes) -> int:
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(4):
            crc = ((crc << 8) ^ CRC_TABLE[(crc >> 24) & 0xFF]) & 0xFFFFFFFF
    return crc


def hex_to_bytes(text: str) -> bytes:
    cleaned = "".join(ch for ch in text if ch in "0123456789abcdefABCDEF")
    if len(cleaned) % 2:
        raise ValueError("hex string has odd number of digits")
    return bytes.fromhex(cleaned)


def hx(data: bytes) -> str:
    return data.hex(" ")


def u16_be(value: int) -> bytes:
    return value.to_bytes(2, "big", signed=False)


def u32_le(value: int) -> bytes:
    return value.to_bytes(4, "little", signed=False)


def u32_be(value: int) -> bytes:
    return value.to_bytes(4, "big", signed=False)


def append_u16(out: bytearray, value: int) -> None:
    # APK evidence: type@34e8.method@9c71 writes two bytes, high byte first.
    out.extend(value.to_bytes(2, "big", signed=False))


def append_u32(out: bytearray, value: int) -> None:
    # APK evidence: type@34e8.method@9c6f writes four bytes, high byte first.
    out.extend(value.to_bytes(4, "big", signed=False))


def append_u64(out: bytearray, value: int) -> None:
    # APK evidence: type@34e8.method@9c70 writes eight bytes, high word first.
    out.extend(value.to_bytes(8, "big", signed=False))


def append_u8(out: bytearray, value: int) -> None:
    # APK evidence: type@34e8.method@9c6d appends one byte.
    out.append(value & 0xFF)


def modified_utf8_bytes(text: str) -> bytes:
    # APK evidence: type@34e8.method@9c72 uses Java-style modified UTF-8:
    # NUL is encoded as c0 80; non-ASCII chars are emitted as 2/3-byte UTF.
    out = bytearray()
    for ch in text:
        code = ord(ch)
        if 1 <= code <= 0x7F:
            out.append(code)
        elif code <= 0x7FF:
            out.append(0xC0 | ((code >> 6) & 0x1F))
            out.append(0x80 | (code & 0x3F))
        else:
            out.append(0xE0 | ((code >> 12) & 0x0F))
            out.append(0x80 | ((code >> 6) & 0x3F))
            out.append(0x80 | (code & 0x3F))
    return bytes(out)


def append_modified_utf8(out: bytearray, text: str) -> None:
    encoded = modified_utf8_bytes(text)
    if len(encoded) > 0xFFFF:
        raise ValueError("modified UTF-8 string is too long for type@34e8.9c72")
    append_u16(out, len(encoded))
    out.extend(encoded)


def append_marker_u16(out: bytearray, marker: int, value: int) -> None:
    # APK evidence: type@34e8.method@9c6b writes marker + u16_be(value).
    append_u8(out, marker)
    append_u16(out, value)


def append_marker_u16_u16(out: bytearray, marker: int, first: int, second: int) -> None:
    # APK evidence: type@34e8.method@9c6c writes marker + two u16_be ids.
    append_u8(out, marker)
    append_u16(out, first)
    append_u16(out, second)


class ApkRegistry:
    """Minimal model of seg08 type@3503 registry string interning."""

    def __init__(self) -> None:
        # APK evidence: 0x475288 initializes field@466e to 1.
        self._next_id = 1
        self._ids: dict[str, int] = {}
        self._records = bytearray()

    @property
    def count(self) -> int:
        return self._next_id

    @property
    def records(self) -> bytes:
        return bytes(self._records)

    def intern_string(self, value: str) -> int:
        # APK evidence: true 9df4(string) writes marker 1 + 9c72(string).
        existing = self._ids.get(value)
        if existing is not None:
            return existing
        item_id = self._next_id
        self._next_id += 1
        self._ids[value] = item_id
        append_u8(self._records, 1)
        append_modified_utf8(self._records, value)
        return item_id

    def envelope(self) -> bytes:
        # APK evidence: 0x4758e0 / 9e13 writes u16 count then raw records.
        out = bytearray()
        append_u16(out, self.count)
        out.extend(self.records)
        return bytes(out)


def root9cd6_minimal(
    registry: ApkRegistry,
    action_nodes: list[bytes],
    *,
    version: int = 53,
    flags: int = 0,
    field43fb: int = 0,
    field43f9: int = 0,
    field43e8: list[int] | None = None,
) -> bytes:
    # APK evidence from seg08 0x46c838. This covers the root envelope and
    # action-chain area, leaving optional root sections empty.
    version_low = version & 0xFFFF
    masked_flags = flags & ~(0x1000 if version_low < 49 else 0)
    out = bytearray()
    append_u32(out, 0xCAFEBABE)
    append_u32(out, version)
    out.extend(registry.envelope())
    append_u16(out, masked_flags)
    append_u16(out, field43fb)
    append_u16(out, field43f9)
    values = field43e8 or []
    append_u16(out, len(values))
    for value in values:
        append_u16(out, value)
    append_u16(out, 0)
    append_u16(out, len(action_nodes))
    for node in action_nodes:
        out.extend(node)
    append_u16(out, 0)
    return bytes(out)


def root9cd6_with_action_blob(
    registry: ApkRegistry,
    action_count: int,
    action_blob: bytes,
    *,
    version: int = 53,
    flags: int = 0,
    field43fb: int = 0,
    field43f9: int = 0,
    field43e8: list[int] | None = None,
) -> bytes:
    version_low = version & 0xFFFF
    masked_flags = flags & ~(0x1000 if version_low < 49 else 0)
    out = bytearray()
    append_u32(out, 0xCAFEBABE)
    append_u32(out, version)
    out.extend(registry.envelope())
    append_u16(out, masked_flags)
    append_u16(out, field43fb)
    append_u16(out, field43f9)
    values = field43e8 or []
    append_u16(out, len(values))
    for value in values:
        append_u16(out, value)
    append_u16(out, 0)
    append_u16(out, action_count)
    out.extend(action_blob)
    append_u16(out, 0)
    return bytes(out)


def root_structural_wrapped_stop_capture_candidate(action_sequence: bytes) -> bytes:
    # Structural root.9cd6 candidate for the wrapper/action section. The
    # damaged seg08 string_ids prevent these names from being final bytes, but
    # the order follows the already recovered wrapper sequence.
    registry = ApkRegistry()
    for value in (
        "external_arg",
        WRAPPER_DESCRIPTOR_3865,
        "string@259e",
        STOP_CAPTURE_DESC,
        STOP_CAPTURE_TYPE_URL,
        STOP_CAPTURE_BUILDER_DESC,
        STOP_CAPTURE_ADAPTER_DESC,
        "string@224e",
        "string@8a51",
        "string@8b21",
    ):
        registry.intern_string(value)
    if len(action_sequence) < 2:
        raise ValueError("action sequence must start with a u16 action count")
    count = int.from_bytes(action_sequence[:2], "big")
    # apk_wrapped_stop_capture_sequence_candidate returns count + concatenated
    # 9d7f nodes, which maps directly into root.9cd6's action-chain area.
    nodes_blob = action_sequence[2:]
    return root9cd6_with_action_blob(registry, count, nodes_blob, version=53, flags=0)


def build_ee91_internal_packet(
    command_id: int,
    method_id: int,
    body: bytes,
    time_bytes: bytes = OBSERVED_DEVICE_INFO_TIME_BYTES,
) -> bytes:
    # APK evidence from seg12 0x565824:
    # internal[0..1] = f4c9() little-endian.
    # internal[2] = ee7b().
    # internal[3..6] = time/nonce-derived bytes.
    # internal[7..8] = 00 00.
    # internal[9..] = high-level body.
    if len(time_bytes) != 4:
        raise ValueError("ee91 time_bytes must be exactly 4 bytes")
    return (
        command_id.to_bytes(2, "little")
        + bytes([method_id & 0xFF])
        + time_bytes
        + b"\x00\x00"
        + body
    )


def build_internal_packet(command_id: int, method_id: int, body: bytes) -> bytes:
    return build_ee91_internal_packet(command_id, method_id, body)


def build_ucd2_from_encrypt_result(
    message_type: int,
    seq: int,
    ciphertext: bytes,
    tag_a: bytes,
    tag_b: bytes,
    scheme_byte: int,
    long_header: bool = True,
) -> bytes:
    # APK evidence from seg12 0x565824 encrypted/config branch:
    # long header: 40 1d <scheme> + 12 bytes field@7091 + 12 bytes field@7092
    # short header: 40 01 <scheme>
    if long_header:
        if len(tag_a) != 12 or len(tag_b) != 12:
            raise ValueError("long encrypted UCD2 header needs two 12-byte tag segments")
        extra = bytes([0x40, 0x1D, scheme_byte & 0xFF]) + tag_a + tag_b + b"\x00" * 4
    else:
        extra = bytes([0x40, 0x01, scheme_byte & 0xFF])
    return build_ucd2(message_type, seq, ciphertext, extra)


def build_ucd2(message_type: int, seq: int, payload: bytes, extra: bytes = b"") -> bytes:
    header_len = 12 + len(extra)
    out = bytearray()
    out.extend(b"UCD2")
    out.append(1)
    out.append(header_len)
    out.append(message_type & 0xFF)
    out.append(seq & 0xFF)
    out.extend(u32_le(len(payload)))
    out.extend(extra)
    out.extend(payload)
    crc = ucd2_crc(bytes(out))
    out.extend(u32_le(crc))
    return bytes(out)


def observed_device_info_payload() -> bytes:
    return bytes.fromhex("08 30 08 0f 08 0b")


def observed_device_info_internal() -> bytes:
    return build_internal_packet(0x0008, 0x02, observed_device_info_payload())


def stop_capture_command199_empty_body() -> bytes:
    # Candidate from APK evidence:
    # seg08 StopCapture writer uses command selector 199 (0x00c7), while
    # seg07 raw message descriptors show StopCapture{} is an empty protobuf.
    # This tests 199 as the ee91 two-byte command id, instead of using the
    # device-info command id 0x0008 with 199 inside a higher action buffer.
    return b""


def stop_capture_command199_selector_body() -> bytes:
    # Conservative variant that keeps the APK-visible 9d5e selector byte in
    # the body while also using 0x00c7 as the ee91 command id.
    return stop_capture_inner_candidate(bytes.fromhex("59 b3 00 03"), triple_id=3)


def stop_capture_inner_candidate(a03f_suffix: bytes = b"", triple_id: int = 3) -> bytes:
    # Candidate only. Proven pieces from seg08:
    # 9d57(178, first field/string triple) -> 9c6b writes b2 00 03
    # in a fresh registry. 9cb8 allocated ids 1 and 2 before this triple.
    # 9d5a(89) -> 59.
    # 0x472944/9d5e maps command 199 through the compact-id state machine,
    # but this branch writes the original byte c7 before calling
    # type@34f7.9d43. 0x470194 records a 0x20000000 four-byte length patch.
    # 0x46fd20 later fills it with current_len - c7_offset, not
    # current_len - placeholder_offset.
    # 9d5a(87) -> 57, then a03f/a0d1 may append extra state bytes.
    # a03f writes 9d5a(89) and then 9d57(179, same triple) -> 59 b3 00 03.
    # 9d5f(empty) patches length = current_len - c7_offset.
    # 9d5a(176) -> b0.
    prefix = bytes([0xB2]) + u16_be(triple_id) + bytes.fromhex("59 c7 ff ff ff ff 57") + a03f_suffix
    c7_offset = 4
    placeholder_offset = 5
    patched_len = len(prefix) - c7_offset
    return prefix[:placeholder_offset] + u32_be(patched_len) + prefix[placeholder_offset + 4 :] + bytes([0xB0])

def stop_capture_full_node_candidate(
    body: bytes,
    extension: bytes = b"",
    first_arg_id: int = 1,
    second_arg_id: int = 2,
    body_record_id: int = 4,
    zero_meta_id_1: int = 5,
    zero_meta_id_2: int = 6,
) -> bytes:
    # Candidate for the outer builder serializer at seg08 0x471b8c.
    # Proven default constructor state from 0x471344 for 9cb8(4106, 6fc, 9be, 0, 0):
    # field44fd=0x100a, field4523=1, field4505=2, field44ff=0.
    # Registry ids before serialization: 6fc=1, 9be=2, first 9d57 triple=3.
    # 0x471b8c then allocates string@224e as id 4 for the main buffer record.
    # 0x467744/9c5c appends string@8b21 and string@259e as zero-length metadata.
    out = bytearray()
    has_extension = bool(extension)
    field_count = 3
    append_u16(out, 0x1000)
    append_u16(out, first_arg_id)
    append_u16(out, second_arg_id)
    append_u16(out, field_count)
    append_u16(out, body_record_id)
    main_record_len = len(body) + 10 + (len(extension) + 8 if has_extension else 0)
    append_u32(out, main_record_len)
    append_u16(out, 0x0002)
    append_u16(out, 0x0000)
    append_u32(out, len(body))
    out.extend(body)
    append_u16(out, 0x0001 if has_extension else 0x0000)
    next_id = zero_meta_id_1
    if has_extension:
        append_u16(out, next_id)
        next_id += 1
        append_u32(out, len(extension) + 2)
        append_u16(out, 0x0001)
        out.extend(extension)
    append_u16(out, next_id)
    append_u32(out, 0)
    append_u16(out, zero_meta_id_2 if not has_extension else next_id + 1)
    append_u32(out, 0)
    return bytes(out)



def encode_9d58_metadata_group(base_node_id: int, encoded_second_arg: int) -> bytes:
    # APK evidence from seg08 0x46e8e4 + 0x470d0c + 0x4721a8 + 0x4726a8:
    # helper.9d0e(action_node) stages an int array as:
    #   [base, first_count, second_count, encoded...]
    # For Luna's mandatory 9d58(-1, 0, [], 1, [string@98e9]) this becomes
    #   [base, 0, 1, encoded(string@98e9)].
    # The first staged group lives in field@4527 and is serialized later by
    # 9d7f/9d7d, so represent it as big-endian u32 slots for the candidate
    # full-node metadata payload rather than the old guessed fb xx bytes.
    out = bytearray()
    append_u32(out, base_node_id)
    append_u32(out, 0)
    append_u32(out, 1)
    append_u32(out, encoded_second_arg)
    return bytes(out)


def encoded_plain_string_ref(registry_id: int) -> int:
    # APK evidence from seg08 0x46e4c0 / 9d13:
    # object/plain string-like descriptors go through registry.9dfa(value)
    # and OR the returned id with 0x00800000.
    return 0x00800000 | (registry_id & 0xFFFFF)
def apk_entry_marker_node(
    command_id: int,
    first_arg_id: int,
    second_arg_id: int,
    zero_meta_id: int,
) -> bytes:
    # Candidate for the root/action marker observed in APK methods:
    # a03d: 9cb6(4234, string@6fb, string@98e9, 0, 0)
    # a068: 9cb6(4121, string@6fb, string@98e9, 0, 0)
    #
    # This mirrors the minimal 0x471b8c/9d7f path for a type@34fa node with
    # no main buffer.  The fresh registry id order is assumed to be:
    #   string@6fb -> 1, string@98e9 -> 2, string@259e -> 3.
    out = bytearray()
    append_u16(out, command_id)
    append_u16(out, first_arg_id)
    append_u16(out, second_arg_id)
    append_u16(out, 0x0001)
    append_u16(out, zero_meta_id)
    append_u32(out, 0)
    return bytes(out)


def apk_stop_capture_sequence_candidate(
    marker_command_id: int,
    builder_body: bytes,
    extension: bytes = b"",
) -> bytes:
    # Candidate for the action-node order found in method@a03c/a066:
    # marker node first, then the StopCapture builder node.
    #
    # This is not the full root.9cd6() output. 0x46c838/9cd6 writes
    # CA FE BA BE, root header fields, and the registry/string table before
    # this action-node count + node sequence.
    marker = apk_entry_marker_node(marker_command_id, 1, 2, 3)
    builder = stop_capture_full_node_candidate(
        builder_body,
        extension,
        first_arg_id=4,
        second_arg_id=5,
        body_record_id=7,
        zero_meta_id_1=8,
        zero_meta_id_2=3,
    )
    out = bytearray()
    append_u16(out, 0x0002)
    out.extend(marker)
    out.extend(builder)
    return bytes(out)


def apk_wrapped_stop_capture_sequence_candidate(
    marker_command_id: int,
    builder_body: bytes,
    extension: bytes = b"",
    wrapper_first_arg_id: int = 1,
) -> bytes:
    # 480fcc -> a0e0/a0e1 -> a0e4 writes a wrapper marker before the
    # action chain. This still models only the action-node section, not
    # the complete root.9cd6() output:
    #   9cb6(4233, external_arg, string@3865, 0, 0)
    #
    # Registry id order under the null/external-arg-is-first assumption:
    #   1 external arg, 2 string@3865, 3 string@259e,
    #   4 string@6fb, 5 string@98e9,
    #   6 string@6fc, 7 string@9be,
    #   8 first StopCapture triple, 9 string@224e,
    #   10 string@8b21.
    wrapper = apk_entry_marker_node(0x1089, wrapper_first_arg_id, 2, 3)
    marker = apk_entry_marker_node(marker_command_id, 4, 5, 3)
    builder = stop_capture_full_node_candidate(
        builder_body,
        extension,
        first_arg_id=6,
        second_arg_id=7,
        body_record_id=9,
        zero_meta_id_1=10,
        zero_meta_id_2=3,
    )
    out = bytearray()
    append_u16(out, 0x0003)
    out.extend(wrapper)
    out.extend(marker)
    out.extend(builder)
    return bytes(out)


@dataclass(frozen=True)
class Candidate:
    name: str
    body: bytes
    command_id: int = 0x0008
    method_id: int = 0x02
    message_type: int = 0x04
    seq: int = 0x10

    def internal(self) -> bytes:
        return build_internal_packet(self.command_id, self.method_id, self.body)

    def frame(self) -> bytes:
        return build_ucd2(self.message_type, self.seq, self.internal())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--body-hex", help="override high-level body/payload bytes")
    parser.add_argument("--a03f-suffix-hex", help="bytes appended by the unresolved a03f/a0d1 helper")
    parser.add_argument("--seq", type=lambda x: int(x, 0), default=0x10)
    parser.add_argument("--message-type", type=lambda x: int(x, 0), default=0x04)
    args = parser.parse_args()

    if args.body_hex:
        body = hex_to_bytes(args.body_hex)
    else:
        suffix = hex_to_bytes(args.a03f_suffix_hex) if args.a03f_suffix_hex else b""
        body = stop_capture_inner_candidate(suffix)
    cand = Candidate("stop_capture_candidate", body, seq=args.seq, message_type=args.message_type)
    a03f_cand = Candidate(
        "stop_capture_a03f_candidate",
        stop_capture_inner_candidate(bytes.fromhex("59 b3 00 03"), triple_id=3),
        seq=args.seq,
        message_type=args.message_type,
    )
    full_cand = Candidate(
        "stop_capture_full_node_candidate",
        stop_capture_full_node_candidate(cand.body),
        seq=args.seq,
        message_type=args.message_type,
    )
    full_a03f_cand = Candidate(
        "stop_capture_full_node_a03f_candidate",
        stop_capture_full_node_candidate(a03f_cand.body),
        seq=args.seq,
        message_type=args.message_type,
    )
    full_ext_cand = Candidate(
        "stop_capture_full_node_9d58_candidate",
        stop_capture_full_node_candidate(cand.body, bytes.fromhex("fb 0a")),
        seq=args.seq,
        message_type=args.message_type,
    )
    full_a03f_ext_cand = Candidate(
        "stop_capture_full_node_a03f_9d58_candidate",
        stop_capture_full_node_candidate(a03f_cand.body, bytes.fromhex("fb 0e")),
        seq=args.seq,
        message_type=args.message_type,
    )
    seq_base_body = stop_capture_inner_candidate(triple_id=6)
    seq_a03f_body = stop_capture_inner_candidate(bytes.fromhex("59 b3 00 06"), triple_id=6)
    wrapped_a03f_body = stop_capture_inner_candidate(bytes.fromhex("59 b3 00 08"), triple_id=8)
    seq_base_cand = Candidate(
        "stop_capture_apk_sequence_base_candidate",
        apk_stop_capture_sequence_candidate(0x108A, seq_base_body),
        seq=args.seq,
        message_type=args.message_type,
    )
    seq_a03f_cand = Candidate(
        "stop_capture_apk_sequence_a03f_candidate",
        apk_stop_capture_sequence_candidate(0x108A, seq_a03f_body),
        seq=args.seq,
        message_type=args.message_type,
    )
    seq_4121_cand = Candidate(
        "stop_capture_apk_sequence_4121_a03f_9d58_candidate",
        apk_stop_capture_sequence_candidate(0x1019, seq_a03f_body, bytes.fromhex("fb 0e")),
        seq=args.seq,
        message_type=args.message_type,
    )
    wrapped_seq_a03f_cand = Candidate(
        "stop_capture_apk_wrapped_sequence_a03f_candidate",
        apk_wrapped_stop_capture_sequence_candidate(0x108A, wrapped_a03f_body),
        seq=args.seq,
        message_type=args.message_type,
    )
    # APK-derived 9d58 metadata staging candidate. The registry id used here
    # follows the wrapped-sequence comment order above, where string@98e9 is
    # the fifth registered object/string. The base slot is 0 because the mode-3
    # constructor path installs a fresh type@34f7 whose constructor does not
    # set field@44e0, and the later 9d11(199) path does not replace field@44c4.
    metadata_9d58 = encode_9d58_metadata_group(
        base_node_id=0,
        encoded_second_arg=encoded_plain_string_ref(5),
    )
    wrapped_seq_a03f_metadata_cand = Candidate(
        "stop_capture_apk_wrapped_sequence_a03f_9d58_metadata_candidate",
        apk_wrapped_stop_capture_sequence_candidate(0x108A, wrapped_a03f_body, metadata_9d58),
        seq=args.seq,
        message_type=args.message_type,
    )
    root_structural_body = root_structural_wrapped_stop_capture_candidate(wrapped_seq_a03f_metadata_cand.body)
    root_structural_cand = Candidate(
        "stop_capture_root9cd6_structural_wrapped_a03f_9d58_metadata_candidate",
        root_structural_body,
        seq=args.seq,
        message_type=args.message_type,
    )
    command199_empty_cand = Candidate(
        "stop_capture_command199_empty_candidate",
        stop_capture_command199_empty_body(),
        command_id=0x00C7,
        seq=args.seq,
        message_type=args.message_type,
    )
    command199_selector_cand = Candidate(
        "stop_capture_command199_selector_candidate",
        stop_capture_command199_selector_body(),
        command_id=0x00C7,
        seq=args.seq,
        message_type=args.message_type,
    )

    known = build_ucd2(0x04, 0x10, observed_device_info_internal())
    print("ucd2_xor_key              =", hx(UCD2_XOR_KEY))
    print("ucd2_negotiation_sync     =", hx(UCD2_NEGOTIATION_SYNC))
    print("ucd2_negotiation_ack_8    =", hx(UCD2_NEGOTIATION_ACK_8))
    print("ucd2_negotiation_ack_7    =", hx(UCD2_NEGOTIATION_ACK_7))
    print("known_device_info_internal =", hx(observed_device_info_internal()))
    print("known_device_info_frame    =", hx(known))
    print("candidate_body             =", hx(cand.body))
    print("candidate_internal         =", hx(cand.internal()))
    print("candidate_ucd2             =", hx(cand.frame()))
    if not args.body_hex and not args.a03f_suffix_hex:
        print("a03f_candidate_body        =", hx(a03f_cand.body))
        print("a03f_candidate_internal    =", hx(a03f_cand.internal()))
        print("a03f_candidate_ucd2        =", hx(a03f_cand.frame()))
        print("full_node_body             =", hx(full_cand.body))
        print("full_node_internal         =", hx(full_cand.internal()))
        print("full_node_ucd2             =", hx(full_cand.frame()))
        print("full_node_a03f_body        =", hx(full_a03f_cand.body))
        print("full_node_a03f_internal    =", hx(full_a03f_cand.internal()))
        print("full_node_a03f_ucd2        =", hx(full_a03f_cand.frame()))
        print("full_node_9d58_body        =", hx(full_ext_cand.body))
        print("full_node_9d58_internal    =", hx(full_ext_cand.internal()))
        print("full_node_9d58_ucd2        =", hx(full_ext_cand.frame()))
        print("full_node_a03f_9d58_body   =", hx(full_a03f_ext_cand.body))
        print("full_node_a03f_9d58_internal=", hx(full_a03f_ext_cand.internal()))
        print("full_node_a03f_9d58_ucd2   =", hx(full_a03f_ext_cand.frame()))
        print("apk_sequence_base_body      =", hx(seq_base_cand.body))
        print("apk_sequence_base_internal  =", hx(seq_base_cand.internal()))
        print("apk_sequence_base_ucd2      =", hx(seq_base_cand.frame()))
        print("apk_sequence_a03f_body      =", hx(seq_a03f_cand.body))
        print("apk_sequence_a03f_internal  =", hx(seq_a03f_cand.internal()))
        print("apk_sequence_a03f_ucd2      =", hx(seq_a03f_cand.frame()))
        print("apk_sequence_4121_body      =", hx(seq_4121_cand.body))
        print("apk_sequence_4121_internal  =", hx(seq_4121_cand.internal()))
        print("apk_sequence_4121_ucd2      =", hx(seq_4121_cand.frame()))
        print("apk_wrapped_sequence_a03f_body     =", hx(wrapped_seq_a03f_cand.body))
        print("apk_wrapped_sequence_a03f_internal =", hx(wrapped_seq_a03f_cand.internal()))
        print("apk_wrapped_sequence_a03f_ucd2     =", hx(wrapped_seq_a03f_cand.frame()))
        print("metadata_9d58_staged_bytes         =", hx(metadata_9d58))
        print("apk_wrapped_sequence_a03f_9d58_metadata_body     =", hx(wrapped_seq_a03f_metadata_cand.body))
        print("apk_wrapped_sequence_a03f_9d58_metadata_internal =", hx(wrapped_seq_a03f_metadata_cand.internal()))
        print("apk_wrapped_sequence_a03f_9d58_metadata_ucd2     =", hx(wrapped_seq_a03f_metadata_cand.frame()))
        print("root9cd6_structural_body           =", hx(root_structural_cand.body))
        print("root9cd6_structural_internal       =", hx(root_structural_cand.internal()))
        print("root9cd6_structural_ucd2           =", hx(root_structural_cand.frame()))
        print("command199_empty_body       =", hx(command199_empty_cand.body))
        print("command199_empty_internal   =", hx(command199_empty_cand.internal()))
        print("command199_empty_ucd2       =", hx(command199_empty_cand.frame()))
        print("command199_selector_body    =", hx(command199_selector_cand.body))
        print("command199_selector_internal=", hx(command199_selector_cand.internal()))
        print("command199_selector_ucd2    =", hx(command199_selector_cand.frame()))


if __name__ == "__main__":
    main()

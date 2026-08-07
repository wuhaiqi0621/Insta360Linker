#!/usr/bin/env python3
"""Stream a PCAPdroid raw-IP capture and summarize Luna camera traffic."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import socket
import struct
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO


PCAP_MAGIC_LE = b"\xd4\xc3\xb2\xa1"
PCAP_MAGIC_BE = b"\xa1\xb2\xc3\xd4"
UCD2_MAGIC = b"UCD2"


def iso_time(seconds: int, micros: int) -> str:
    value = dt.datetime.fromtimestamp(seconds + micros / 1_000_000, dt.timezone.utc)
    return value.astimezone().isoformat(timespec="milliseconds")


def ip_text(raw: bytes) -> str:
    return socket.inet_ntoa(raw)


def printable_strings(data: bytes, minimum: int = 4) -> list[str]:
    result: list[str] = []
    current = bytearray()
    for value in data:
        if 0x20 <= value <= 0x7E:
            current.append(value)
        else:
            if len(current) >= minimum:
                result.append(current.decode("ascii", errors="replace"))
            current.clear()
    if len(current) >= minimum:
        result.append(current.decode("ascii", errors="replace"))
    return result


@dataclass
class TcpDirection:
    expected_seq: int | None = None
    buffer: bytearray = field(default_factory=bytearray)
    buffer_time: str = ""
    frames: list[dict] = field(default_factory=list)
    gaps: int = 0
    retransmitted_bytes: int = 0
    discarded_prefix_bytes: int = 0
    preview_hevc: bytearray = field(default_factory=bytearray)

    def feed(self, seq: int, payload: bytes, timestamp: str, direction: str, flow_id: str) -> None:
        if not payload:
            return
        if self.expected_seq is None:
            self.expected_seq = seq

        if seq < self.expected_seq:
            overlap = self.expected_seq - seq
            self.retransmitted_bytes += min(overlap, len(payload))
            if overlap >= len(payload):
                return
            payload = payload[overlap:]
            seq += overlap
        elif seq > self.expected_seq:
            self.gaps += 1
            self.buffer.clear()
            self.buffer_time = ""
            self.expected_seq = seq

        if not self.buffer:
            self.buffer_time = timestamp
        self.buffer.extend(payload)
        self.expected_seq = seq + len(payload)
        self._extract(timestamp, direction, flow_id)

    def _extract(self, latest_time: str, direction: str, flow_id: str) -> None:
        while True:
            magic_offset = self.buffer.find(UCD2_MAGIC)
            if magic_offset < 0:
                keep = min(3, len(self.buffer))
                discard = len(self.buffer) - keep
                if discard > 0:
                    self.discarded_prefix_bytes += discard
                    del self.buffer[:discard]
                    self.buffer_time = latest_time
                return
            if magic_offset > 0:
                self.discarded_prefix_bytes += magic_offset
                del self.buffer[:magic_offset]
                self.buffer_time = latest_time
            if len(self.buffer) < 12:
                return

            header_len = self.buffer[5]
            if header_len < 12 or header_len > 96:
                self.discarded_prefix_bytes += 1
                del self.buffer[0]
                self.buffer_time = latest_time
                continue
            payload_len = int.from_bytes(self.buffer[8:12], "little")
            total_len = header_len + payload_len + 4
            if total_len > 64 * 1024 * 1024:
                self.discarded_prefix_bytes += 1
                del self.buffer[0]
                self.buffer_time = latest_time
                continue
            if len(self.buffer) < total_len:
                return

            frame = bytes(self.buffer[:total_len])
            payload = frame[header_len : header_len + payload_len]
            if direction == "camera_to_phone" and frame[6] == 0x01 and len(payload) > 9 and payload[0] == 0x20:
                self.preview_hevc.extend(payload[9:])
            self.frames.append(
                {
                    "time": self.buffer_time or latest_time,
                    "flow": flow_id,
                    "direction": direction,
                    "version": frame[4],
                    "header_len": header_len,
                    "frame_type": f"{frame[6]:02x}",
                    "sequence": frame[7],
                    "message_type": f"{frame[6]:02x} {frame[7]:02x}",
                    "payload_len": payload_len,
                    "frame_len": total_len,
                    "payload_prefix_hex": payload[:96].hex(" "),
                    "control_payload_hex": payload.hex(" ") if frame[6] == 0x04 else "",
                    "payload_strings": printable_strings(payload[:4096]),
                    "frame_prefix_hex": frame[:128].hex(" "),
                }
            )
            del self.buffer[:total_len]
            self.buffer_time = latest_time


@dataclass
class HttpRequestState:
    buffer: bytearray = field(default_factory=bytearray)
    requests: list[dict] = field(default_factory=list)

    def feed(self, payload: bytes, timestamp: str, flow_id: str) -> None:
        if not payload:
            return
        self.buffer.extend(payload)
        if len(self.buffer) > 1024 * 1024:
            del self.buffer[:-65536]
        while True:
            end = self.buffer.find(b"\r\n\r\n")
            if end < 0:
                return
            block = bytes(self.buffer[: end + 4])
            del self.buffer[: end + 4]
            first = block.split(b"\r\n", 1)[0]
            if first.startswith((b"GET ", b"POST ", b"HEAD ", b"PUT ", b"DELETE ", b"OPTIONS ")):
                self.requests.append(
                    {
                        "time": timestamp,
                        "flow": flow_id,
                        "request_line": first.decode("latin-1", errors="replace"),
                    }
                )


def iter_pcap(handle: BinaryIO):
    header = handle.read(24)
    if len(header) != 24:
        raise ValueError("PCAP global header is truncated")
    magic = header[:4]
    if magic == PCAP_MAGIC_LE:
        endian = "<"
    elif magic == PCAP_MAGIC_BE:
        endian = ">"
    else:
        raise ValueError(f"Unsupported PCAP magic: {magic.hex()}")
    _, major, minor, _, _, snaplen, linktype = struct.unpack(endian + "IHHIIII", header)
    yield {"global": {"major": major, "minor": minor, "snaplen": snaplen, "linktype": linktype}}
    record_struct = struct.Struct(endian + "IIII")
    while True:
        raw = handle.read(record_struct.size)
        if not raw:
            return
        if len(raw) != record_struct.size:
            raise ValueError("PCAP record header is truncated")
        ts_sec, ts_usec, captured_len, original_len = record_struct.unpack(raw)
        packet = handle.read(captured_len)
        if len(packet) != captured_len:
            raise ValueError("PCAP packet data is truncated")
        yield {
            "ts_sec": ts_sec,
            "ts_usec": ts_usec,
            "captured_len": captured_len,
            "original_len": original_len,
            "packet": packet,
        }


def parse_ipv4(packet: bytes) -> dict | None:
    if len(packet) < 20 or packet[0] >> 4 != 4:
        return None
    ihl = (packet[0] & 0x0F) * 4
    if ihl < 20 or len(packet) < ihl:
        return None
    total_len = int.from_bytes(packet[2:4], "big")
    if total_len <= 0 or total_len > len(packet):
        total_len = len(packet)
    return {
        "protocol": packet[9],
        "src": ip_text(packet[12:16]),
        "dst": ip_text(packet[16:20]),
        "payload": packet[ihl:total_len],
    }


def parse_tcp(payload: bytes) -> dict | None:
    if len(payload) < 20:
        return None
    src_port, dst_port, seq, ack = struct.unpack("!HHII", payload[:12])
    header_len = ((payload[12] >> 4) & 0x0F) * 4
    if header_len < 20 or len(payload) < header_len:
        return None
    return {
        "src_port": src_port,
        "dst_port": dst_port,
        "seq": seq,
        "ack": ack,
        "flags": payload[13],
        "payload": payload[header_len:],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("pcap", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--camera", default="192.168.42.1")
    parser.add_argument("--preview-hevc", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    directions: dict[tuple[str, int, str], TcpDirection] = defaultdict(TcpDirection)
    http_states: dict[tuple[str, int], HttpRequestState] = defaultdict(HttpRequestState)
    flow_stats: Counter = Counter()
    packet_count = 0
    first_time = ""
    last_time = ""
    global_info: dict = {}

    with args.pcap.open("rb") as handle:
        iterator = iter_pcap(handle)
        global_info = next(iterator)["global"]
        if global_info["linktype"] != 101:
            raise ValueError(f"Expected raw IP linktype 101, got {global_info['linktype']}")
        for record in iterator:
            packet_count += 1
            timestamp = iso_time(record["ts_sec"], record["ts_usec"])
            first_time = first_time or timestamp
            last_time = timestamp
            ip = parse_ipv4(record["packet"])
            if not ip or ip["protocol"] != 6:
                continue
            tcp = parse_tcp(ip["payload"])
            if not tcp:
                continue

            src, dst = ip["src"], ip["dst"]
            src_port, dst_port = tcp["src_port"], tcp["dst_port"]
            camera_related = src == args.camera or dst == args.camera
            if not camera_related:
                continue

            if dst == args.camera:
                client_ip, client_port = src, src_port
                server_port = dst_port
                direction = "phone_to_camera"
            else:
                client_ip, client_port = dst, dst_port
                server_port = src_port
                direction = "camera_to_phone"
            flow_id = f"{client_ip}:{client_port}<->{args.camera}:{server_port}"
            payload = tcp["payload"]
            flow_stats[(flow_id, direction, "packets")] += 1
            flow_stats[(flow_id, direction, "payload_bytes")] += len(payload)

            if server_port == 6666:
                key = (flow_id, server_port, direction)
                directions[key].feed(tcp["seq"], payload, timestamp, direction, flow_id)
            elif server_port == 80 and direction == "phone_to_camera":
                http_states[(flow_id, server_port)].feed(payload, timestamp, flow_id)

            if packet_count % 500000 == 0:
                print(f"processed {packet_count:,} packets", flush=True)

    frames = [frame for state in directions.values() for frame in state.frames]
    frames.sort(key=lambda item: item["time"])
    requests = [request for state in http_states.values() for request in state.requests]
    requests.sort(key=lambda item: item["time"])

    type_counts = Counter((item["direction"], item["frame_type"]) for item in frames)
    flow_rows: list[dict] = []
    flow_ids = sorted({key[0] for key in flow_stats})
    for flow_id in flow_ids:
        for direction in ("phone_to_camera", "camera_to_phone"):
            flow_rows.append(
                {
                    "flow": flow_id,
                    "direction": direction,
                    "packets": flow_stats[(flow_id, direction, "packets")],
                    "payload_bytes": flow_stats[(flow_id, direction, "payload_bytes")],
                }
            )

    parser_diagnostics = [
        {
            "flow": key[0],
            "direction": key[2],
            "frames": len(value.frames),
            "gaps": value.gaps,
            "retransmitted_bytes": value.retransmitted_bytes,
            "discarded_prefix_bytes": value.discarded_prefix_bytes,
            "remaining_buffer_bytes": len(value.buffer),
        }
        for key, value in directions.items()
    ]

    summary = {
        "source": str(args.pcap),
        "camera": args.camera,
        "pcap": global_info,
        "packet_count": packet_count,
        "first_time": first_time,
        "last_time": last_time,
        "ucd2_frame_count": len(frames),
        "http_request_count": len(requests),
        "ucd2_frame_type_counts": [
            {"direction": direction, "frame_type": frame_type, "count": count}
            for (direction, frame_type), count in sorted(type_counts.items())
        ],
        "flow_stats": flow_rows,
        "parser_diagnostics": parser_diagnostics,
    }

    (args.output / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8", newline="\r\n"
    )
    (args.output / "ucd2_frames.json").write_text(
        json.dumps(frames, ensure_ascii=False, indent=2), encoding="utf-8", newline="\r\n"
    )
    (args.output / "http_requests.json").write_text(
        json.dumps(requests, ensure_ascii=False, indent=2), encoding="utf-8", newline="\r\n"
    )
    if args.preview_hevc:
        args.preview_hevc.parent.mkdir(parents=True, exist_ok=True)
        with args.preview_hevc.open("wb") as handle:
            for state in directions.values():
                handle.write(state.preview_hevc)

    with (args.output / "ucd2_timeline.csv").open("w", encoding="utf-8", newline="") as handle:
        fields = [
            "time",
            "direction",
            "flow",
            "frame_type",
            "sequence",
            "message_type",
            "payload_len",
            "frame_len",
            "payload_strings",
            "payload_prefix_hex",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\r\n")
        writer.writeheader()
        for frame in frames:
            row = {field: frame.get(field, "") for field in fields}
            row["payload_strings"] = " | ".join(frame["payload_strings"])
            writer.writerow(row)

    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()

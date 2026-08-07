# Luna Ultra UCD2 Control Findings

## Capture

- Source (read only): `C:/Users/H!Mooo/Downloads/PCAPdroid_22_7月_19_28_38.pcap`
- Capture range: `2026-07-22 19:28:39.141` to `19:32:49.356` (UTC+08:00)
- PCAP format: classic little-endian PCAP, raw IPv4 link type 101
- Camera: `192.168.42.1`
- UCD2 TCP flow: `10.215.173.1:51572 <-> 192.168.42.1:6666`
- Reassembled frames: 612 phone-to-camera and 6061 camera-to-phone
- TCP reassembly gaps: 0 in both directions
- HTTP requests recovered: 38

## Corrected UCD2 Header Interpretation

The 12-byte header is:

```text
55 43 44 32 | 01 | 0c | TT | SS | LL LL LL LL
U  C  D  2    ver  hlen type seq   payload length (little endian)
```

- `TT` is the frame type. This capture contains `01`, `04`, and `05`.
- `SS` is a dynamic sequence byte. For example, `04 09` means type `04`, sequence `09`; it is not a fixed two-byte command type.
- The final four bytes are the recovered UCD2 CRC/checksum.
- Type `04` carries an internal request/response packet.
- Type `05` has no payload in the observed session and behaves as session-control/heartbeat traffic. Its exact role is not yet named.
- Type `01` carries high-volume camera data. Its exact stream role is not yet named.

The observed outbound type-04 payload begins with this 9-byte internal header:

```text
CC CC | 02 | RR RR RR RR | 00 00 | protobuf-like body
command  method  request id          flags
```

- Command is little endian.
- Request ID is little endian and increases during the official-app session (`0x80000034`, `0x8000003a`, `0x80000054`, etc.).
- Responses use internal status/command `0x00c8` and copy the request ID, making deterministic request/response pairing possible.
- A production sender must generate a fresh request ID and UCD2 sequence/checksum instead of replaying an entire captured frame unchanged.

## Confirmed Daily Controls

### Start Video Recording

- Time: `19:30:10.578`
- Internal command: `0x0004`
- Method: `0x02`
- Request ID in this capture: `0x80000034`
- Body: `08 01`
- Full captured request:

```text
55 43 44 32 01 0c 04 74 0b 00 00 00 04 00 02 34 00 00 80 00 00 08 01 91 49 44 a1
```

- Paired response at `19:30:11.481`:

```text
55 43 44 32 01 0c 04 c3 09 00 00 00 c8 00 02 34 00 00 80 00 00 29 86 e1 18
```

- Independent evidence: the resulting file is named `VID_20260722_193010_205.mp4`, matching the command second exactly.

### Stop Video Recording

- Time: `19:30:19.782`
- Internal command: `0x0005`
- Method: `0x02`
- Request ID in this capture: `0x8000003a`
- Body: `10 01`
- Full captured request:

```text
55 43 44 32 01 0c 04 81 0b 00 00 00 05 00 02 3a 00 00 80 00 00 10 01 61 d9 33 28
```

- Paired response at `19:30:21.348` contains the finished media path:

```text
c8 00 02 3a 00 00 80 00 00 0a 2c 0a 2a 2f 44 43 49 4d 2f 43 61 6d 65 72 61 30 31 2f 56 49 44 5f 32 30 32 36 30 37 32 32 5f 31 39 33 30 31 30 5f 32 30 35 2e 6d 70 34
```

- Decoded path: `/DCIM/Camera01/VID_20260722_193010_205.mp4`
- This response provides direct protocol proof that `0x0005` is the stop-recording command.

### Take Photo

- Time: `19:30:28.531`
- Internal command: `0x0003`
- Method: `0x02`
- Request ID in this capture: `0x80000054`
- Body: `30 03`
- Full captured request:

```text
55 43 44 32 01 0c 04 a0 0b 00 00 00 03 00 02 54 00 00 80 00 00 30 03 c8 bf 2d f1
```

- Paired empty-success response at `19:30:29.564`:

```text
55 43 44 32 01 0c 04 02 09 00 00 00 c8 00 02 54 00 00 80 00 00 1c d6 f5 dc
```

- Independent evidence: `IMG_20260722_193028_206.jpg` was requested over HTTP at `19:30:33.972`, matching the command second exactly.

## Media Refresh and Retrieval

- Command `0x00c9`, empty body, received an empty successful response at `19:30:24.129` and `19:30:33.879`.
- Those calls occur immediately before the official app retrieves the newly recorded video/LRV and newly captured JPEG. This is a strong media-refresh candidate, but its precise API name remains unconfirmed.
- Command `0x000b` was sent with a media path at `19:31:24.651` and `19:31:24.653`; the responses contain large image-like data. It is a media thumbnail/preview-data candidate, not a delete command.
- The official app retrieved full/ranged media over HTTP port 80 after receiving paths over UCD2.

## Confirmed Live Preview

- Start-live command at `19:29:54.447`: internal command `0x0001`, method `0x02`, request ID `0x8000002c`.
- Start-live body: `10 01 30 28 38 2c 40 01 48 28 50 22`.
- The matching `0x00c8` response arrived at `19:29:54.495`.
- UCD2 type-01 stream frames began at `19:29:55.117`, about 622 ms after the successful response.
- Stop-live command at `19:31:24.420`: internal command `0x0002`, method `0x02`, request ID `0x800001ba`, empty body.
- The matching `0x00c8` response arrived at `19:31:24.458`.
- The last type-01 stream frame arrived at `19:31:24.417`, immediately before the stop command.

Captured start-live request:

```text
55 43 44 32 01 0c 04 62 15 00 00 00 01 00 02 2c 00 00 80 00 00 10 01 30 28 38 2c 40 01 48 28 50 22 22 8e a7 ba
```

Captured stop-live request:

```text
55 43 44 32 01 0c 04 2c 09 00 00 00 02 00 02 ba 01 00 80 00 00 e1 c3 0d 10
```

Type-01 payload multiplexing observed in the capture:

- Subtype `0x20`: 2224 video access units. Bytes 1..8 are a little-endian millisecond timestamp; bytes 9.. are Annex-B HEVC/H.265.
- Subtype `0x30`: 852 metadata/telemetry blocks.
- Subtype `0x40`: 2073 short timing/auxiliary blocks.
- Subtype `0x85`: 18 additional auxiliary blocks.
- HEVC VPS/SPS data repeats at keyframe boundaries, generally about every two seconds.
- The extracted 24,800,242-byte elementary stream decoded successfully to a real 1280x720 camera image using the bundled FFmpeg runtime.

## Confirmed Device Information

The connection responses expose:

- Model/name: `Insta360 Luna Ultra`
- Serial: `BTLA3ABESWPJTD`
- Firmware: `v1.0.38`
- Service/broadcast name: `Luna Ultra SWPJTD.OSC`

The large initial UCD2 response also carries calibration/intrinsics metadata, device capabilities, and media-list metadata.

## Other Observed Commands

The capture also includes commands `0x0001`, `0x0002`, `0x0007`, `0x0008`, `0x0009`, `0x000a`, `0x000d`, `0x000f`, `0x0011`, `0x0026`, `0x0027`, `0x0053`, `0x0057`, `0x0076`, `0x0097`, `0x0098`, `0x00ac`, `0x00bf`, `0x00c6`, `0x00c9`, `0x00e2`, and `0x00ee`.

Current evidence supports these partial interpretations:

- `0x0008`: batched device/status property query.
- `0x0009`: property update, including fields `0x28`, `0x35`, and `0x55` in this capture.
- `0x000a`: settings/property query with mode/context values `0x06`, `0x07`, or `0x43`.
- `0x0097`: diagnostic/event-track export; response contains `/DCIM/etk.tar`, followed by HTTP retrieval of `ins_event_track_export_...tar`.
- `0x0053`: path/config query; responses include `/CONF` and `/RICONF`.
- `0x00e2`: high-frequency interactive control updates. Exact feature mapping requires the user's action order/timestamps.

These names are deliberately not promoted to production controls until their action semantics are paired with a known user action.

## Safe Implementation Rule

Only `0x0003`, `0x0004`, and `0x0005` are confirmed strongly enough in this capture for daily capture controls. Production code should:

1. Keep one persistent TCP connection to port 6666.
2. Complete the normal UCD2 session setup before control requests.
3. Allocate a fresh internal request ID for each command.
4. Allocate a fresh UCD2 sequence byte and calculate the UCD2 checksum.
5. Wait for a type-04 response with internal status `0x00c8` and the same request ID.
6. Treat timeout, non-`0x00c8` status, or connection closure as failure.
7. Refresh the media list only after capture completion succeeds.

## Generated Analysis Files

- `summary.json`: capture and flow summary
- `ucd2_frames.json`: reassembled UCD2 frames
- `http_requests.json`: recovered HTTP request lines
- `ucd2_timeline.csv`: frame timeline with separate `frame_type` and `sequence` columns
- `CONTROL_FINDINGS.md`: this handoff report

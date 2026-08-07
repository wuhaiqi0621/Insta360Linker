use anyhow::{Context, anyhow};

use regex::Regex;

use reqwest::blocking::Client;

use reqwest::header::{ACCEPT_ENCODING, RANGE, USER_AGENT};

use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

use std::fs;

use std::io::{Read, Write};

use std::net::TcpStream;

use std::path::Path;

use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError};

use std::thread::{self, JoinHandle};

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_MEDIA_PATH: &str = "/storage_internal/DCIM/Camera01/";

pub const INTERNAL_MEDIA_ROOT: &str = "/storage_internal/DCIM/";

pub const SDCARD_MEDIA_ROOT: &str = "/DCIM/";

#[derive(Clone, Copy)]
struct MediaStorage {
    id: &'static str,
    label: &'static str,
    selector: u8,
}

const INTERNAL_STORAGE: MediaStorage = MediaStorage {
    id: "storage_internal",
    label: "内部存储",
    selector: 2,
};

const SDCARD_STORAGE: MediaStorage = MediaStorage {
    id: "sdcard",
    label: "SD 卡",
    selector: 3,
};

const AUTH_PAYLOADS: [&[u8]; 2] = [
    &[
        0x55, 0x43, 0x44, 0x32, 0x01, 0x0C, 0x05, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x37, 0x05, 0x47,
        0x7C,
    ],
    &[
        0x55, 0x43, 0x44, 0x32, 0x01, 0x0C, 0x04, 0x10, 0x0F, 0x00, 0x00, 0x00, 0x08, 0x00, 0x02,
        0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0x08, 0x30, 0x08, 0x0F, 0x08, 0x0B, 0x7C, 0x00, 0x8E,
        0x7C,
    ],
];

const CAMERA_SESSION_INIT_COMMAND: u16 = 0x000f;
const CAMERA_FULL_STATUS_QUERY_BODY: &[u8] = &[
    0x08, 0x01, 0x08, 0x03, 0x08, 0x02, 0x08, 0x4c, 0x08, 0x06, 0x08, 0x4e, 0x08, 0x4f, 0x08, 0x0b,
    0x08, 0x55, 0x08, 0x0c, 0x08, 0x0d, 0x08, 0xaf, 0x01, 0x08, 0x0e, 0x08, 0x0f, 0x08, 0x13, 0x08,
    0x37, 0x08, 0x11, 0x08, 0x14, 0x08, 0x1e, 0x08, 0x24, 0x08, 0x6e, 0x08, 0x72, 0x08, 0x75, 0x08,
    0x59, 0x08, 0x74, 0x08, 0x73, 0x08, 0x25, 0x08, 0x26, 0x08, 0x2a, 0x08, 0x28, 0x08, 0x29, 0x08,
    0x30, 0x08, 0x31, 0x08, 0x32, 0x08, 0x42, 0x08, 0x84, 0x01, 0x08, 0x3a, 0x08, 0x3b, 0x08, 0x3c,
    0x08, 0x43, 0x08, 0x44, 0x08, 0x5d, 0x08, 0x53, 0x08, 0x52, 0x08, 0x46, 0x08, 0x58, 0x08, 0x67,
    0x08, 0x10, 0x08, 0x61, 0x08, 0x85, 0x01, 0x08, 0x86, 0x01, 0x08, 0x77, 0x08, 0x7a, 0x08, 0x7b,
    0x08, 0x7c, 0x08, 0x80, 0x01, 0x08, 0x81, 0x01, 0x08, 0x87, 0x01, 0x08, 0x96, 0x01, 0x08, 0x95,
    0x01, 0x08, 0x93, 0x01, 0x08, 0x9b, 0x01, 0x08, 0x9d, 0x01, 0x08, 0x9e, 0x01, 0x08, 0xa0, 0x01,
    0x08, 0xb3, 0x01, 0x08, 0xa1, 0x01, 0x08, 0x16, 0x08, 0x50, 0x08, 0x51, 0x08, 0xa7, 0x01, 0x08,
    0xa9, 0x01, 0x08, 0xad, 0x01, 0x08, 0xb4, 0x01, 0x08, 0xb0, 0x01, 0x08, 0xb1, 0x01, 0x08, 0x78,
    0x08, 0x6f, 0x08, 0x79, 0x08, 0xac, 0x01,
];
const CAMERA_CLIENT_REGISTER_COMMAND: u16 = 0x0027;
const CAMERA_CLIENT_ID: &str = "ffffffff-d389-4e35-ffff-ffffef05ac4a";
const CAMERA_TIME_SYNC_COMMAND: u16 = 0x0007;
const CAMERA_STATUS_QUERY_COMMAND: u16 = 0x0008;
const CAMERA_STATUS_QUERY_BODY: &[u8] = &[0x08, 0x0b, 0x08, 0x55, 0x08, 0xb4, 0x01];
const CAMERA_CAPTURE_READY_QUERY_BODY: &[u8] = &[0x08, 0x14, 0x08, 0xb0, 0x01, 0x08, 0xb1, 0x01];
const CAMERA_CAPTURE_CONTEXT_COMMAND: u16 = 0x000a;
const CAMERA_CAPTURE_SETTING_IDS: &[u64] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 39, 14, 15, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 40, 33, 32, 34, 58, 59, 43, 55, 56, 35, 100, 36, 37, 38, 41, 99, 42, 44, 45,
    46, 51, 52, 53, 54, 61, 62, 63, 70, 72, 73, 83, 74, 75, 76, 84, 78, 79, 80, 77, 71, 81, 82, 86,
    87, 85, 88, 89, 90, 91, 98, 93, 94, 107,
];
const CAMERA_FILE_LIST_COMMAND: u16 = 0x000d;
const CAMERA_FILE_LIST_PAGE_SIZE: u32 = 100;
const CAMERA_FILE_LIST_MAX_PAGES: u32 = 100;
const CAMERA_TAKE_PHOTO_COMMAND: u16 = 0x0003;
const CAMERA_TAKE_PHOTO_BODY: &[u8] = &[0x30, 0x03];
const CAMERA_START_RECORD_COMMAND: u16 = 0x0004;
const CAMERA_START_RECORD_BODY: &[u8] = &[0x08, 0x01];
const CAMERA_STOP_RECORD_COMMAND: u16 = 0x0005;
const CAMERA_STOP_RECORD_BODY: &[u8] = &[0x10, 0x01];
const CAMERA_SET_OPTION_COMMAND: u16 = 0x0009;
const CAMERA_EVENT_SUBSCRIBE_COMMAND: u16 = 0x0011;
const CAMERA_EVENT_SUBSCRIBE_BODY: &[u8] = &[0x08, 0x01];
const CAMERA_PREVIEW_PREPARE_COMMAND: u16 = 0x00bf;
const CAMERA_PREVIEW_PREPARE_BODY: &[u8] = &[0x58, 0x0a];
const CAMERA_PREVIEW_ACTIVATE_COMMAND: u16 = 0x00c6;

const DEVICE_INFO_PAYLOAD: &[u8] = &[
    0x08, 0x00, 0x02, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0x08, 0x30, 0x08, 0x0f, 0x08, 0x0b,
];

const UCD2_NEGOTIATION_SYNC: &[u8] = b"syNceNdinS";

const STOP_CAPTURE_BASE_BODY: &[u8] = &[
    0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x05, 0x57, 0xb0,
];

const STOP_CAPTURE_A03F_BODY: &[u8] = &[
    0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57, 0x59, 0xb3, 0x00, 0x03, 0xb0,
];

const STOP_CAPTURE_FULL_BASE_BODY: &[u8] = &[
    0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x15, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x05, 0x57,
    0xb0, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00,
];

const STOP_CAPTURE_FULL_A03F_BODY: &[u8] = &[
    0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x19, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57,
    0x59, 0xb3, 0x00, 0x03, 0xb0, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00,
    0x00, 0x00, 0x00,
];

const STOP_CAPTURE_FULL_BASE_9D58_BODY: &[u8] = &[
    0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x05, 0x57,
    0xb0, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0xfb, 0x0a, 0x00, 0x06, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
];

const STOP_CAPTURE_FULL_A03F_9D58_BODY: &[u8] = &[
    0x10, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x00, 0x23, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57,
    0x59, 0xb3, 0x00, 0x03, 0xb0, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0xfb,
    0x0e, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
];

const STOP_CAPTURE_APK_SEQUENCE_BASE_BODY: &[u8] = &[
    0x00, 0x02, 0x10, 0x8a, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x04, 0x00, 0x05, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x15, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0b, 0xb2, 0x00, 0x06, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x05, 0x57,
    0xb0, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
];

const STOP_CAPTURE_APK_SEQUENCE_A03F_BODY: &[u8] = &[
    0x00, 0x02, 0x10, 0x8a, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x04, 0x00, 0x05, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x19, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xb2, 0x00, 0x06, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57,
    0x59, 0xb3, 0x00, 0x06, 0xb0, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00,
    0x00, 0x00, 0x00,
];

const STOP_CAPTURE_APK_SEQUENCE_4121_BODY: &[u8] = &[
    0x00, 0x02, 0x10, 0x19, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x04, 0x00, 0x05, 0x00, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x23, 0x00, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xb2, 0x00, 0x06, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57,
    0x59, 0xb3, 0x00, 0x06, 0xb0, 0x00, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0xfb,
    0x0e, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00,
];

const STOP_CAPTURE_APK_WRAPPED_SEQUENCE_A03F_BODY: &[u8] = &[
    0x00, 0x03, 0x10, 0x89, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x8a, 0x00, 0x04, 0x00, 0x05, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
    0x00, 0x06, 0x00, 0x07, 0x00, 0x03, 0x00, 0x09, 0x00, 0x00, 0x00, 0x19, 0x00, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x0f, 0xb2, 0x00, 0x08, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57, 0x59, 0xb3,
    0x00, 0x08, 0xb0, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x00,
];

const STOP_CAPTURE_COMMAND199_EMPTY_BODY: &[u8] = &[];

const STOP_CAPTURE_COMMAND199_SELECTOR_BODY: &[u8] = &[
    0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57, 0x59, 0xb3, 0x00, 0x03, 0xb0,
];

#[derive(Debug, Clone, Serialize)]

pub struct Ucd2ProbeResult {
    pub host: String,

    pub sent_packets: Vec<String>,

    pub received_packets: Vec<String>,

    pub sent_frames: Vec<Ucd2Frame>,

    pub received_frames: Vec<Ucd2Frame>,

    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]

pub struct Ucd2Frame {
    pub offset: usize,

    pub frame_hex: String,

    pub version: u8,

    pub header_len: u8,

    pub message_type: String,

    pub message_hint: String,

    pub payload_len: u32,

    pub payload_hex: String,

    pub payload_ascii: String,

    pub payload_strings: Vec<String>,

    pub tail_hex: String,
}

#[derive(Debug, Clone, Serialize)]

pub struct LunaStatus {
    pub host: String,

    pub http_ok: bool,

    pub control_ok: bool,

    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct LunaFile {
    pub name: String,

    pub url: String,

    pub date: String,

    pub time: String,

    pub size_text: String,

    pub bytes: Option<u64>,

    pub kind: String,

    pub storage_id: String,

    pub storage_label: String,
}

pub struct LunaAuthSession {
    host: String,

    port: u16,

    command_tx: Option<Sender<LunaAuthWorkerCommand>>,

    worker: Option<JoinHandle<()>>,
}

pub struct Ucd2RawSession {
    host: String,

    stream: TcpStream,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraControlResponse {
    pub command_id: u16,

    pub request_id: u32,

    pub body_hex: String,

    pub media_path: Option<String>,

    #[serde(skip_serializing)]
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LivePreviewChunk {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraCaptureMode {
    Photo,
    Video,
}

impl CameraCaptureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraVideoProfile {
    format_id: &'static str,
    resolution: &'static str,
    aspect_ratio: &'static str,
    width: u16,
    height: u16,
    fps: u16,
    protocol_value: u16,
}

impl CameraVideoProfile {
    pub fn format_id(self) -> &'static str {
        self.format_id
    }

    pub fn resolution(self) -> &'static str {
        self.resolution
    }

    pub fn aspect_ratio(self) -> &'static str {
        self.aspect_ratio
    }

    pub fn width(self) -> u16 {
        self.width
    }

    pub fn height(self) -> u16 {
        self.height
    }

    pub fn fps(self) -> u16 {
        self.fps
    }

    pub fn display_label(self) -> String {
        format!(
            "{} {}（{}×{}）· {} fps",
            self.resolution, self.aspect_ratio, self.width, self.height, self.fps
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct CameraVideoFormat {
    id: &'static str,
    resolution: &'static str,
    aspect_ratio: &'static str,
    width: u16,
    height: u16,
    fps_values: &'static [(u16, u16)],
}

const CAMERA_VIDEO_FORMATS: &[CameraVideoFormat] = &[
    CameraVideoFormat {
        id: "8k_16_9",
        resolution: "8K",
        aspect_ratio: "16:9",
        width: 7680,
        height: 4320,
        fps_values: &[(30, 154), (25, 211), (24, 210)],
    },
    CameraVideoFormat {
        id: "8k_2_35_1",
        resolution: "8K",
        aspect_ratio: "2.35:1",
        width: 7680,
        height: 3264,
        // APK 内部枚举名为 CAPTURE_7680_3268_*，应用界面沿用相机显示的 3264。
        fps_values: &[(30, 213), (25, 212), (24, 219)],
    },
    CameraVideoFormat {
        id: "4k_16_9",
        resolution: "4K",
        aspect_ratio: "16:9",
        width: 3840,
        height: 2160,
        fps_values: &[
            (120, 214),
            (100, 220),
            (60, 23),
            (50, 92),
            (48, 258),
            (30, 24),
            (25, 48),
            (24, 49),
        ],
    },
    CameraVideoFormat {
        id: "4k_2_35_1",
        resolution: "4K",
        aspect_ratio: "2.35:1",
        width: 3840,
        height: 1632,
        fps_values: &[
            (120, 433),
            (100, 434),
            (60, 435),
            (50, 436),
            (48, 437),
            (30, 438),
            (25, 439),
            (24, 440),
        ],
    },
    CameraVideoFormat {
        id: "3k_1_1",
        resolution: "3K",
        aspect_ratio: "1:1",
        width: 3072,
        height: 3072,
        fps_values: &[
            (60, 446),
            (50, 447),
            (48, 448),
            (30, 121),
            (25, 120),
            (24, 119),
        ],
    },
    CameraVideoFormat {
        id: "3k_9_16",
        resolution: "3K",
        aspect_ratio: "9:16",
        width: 1728,
        height: 3072,
        fps_values: &[
            (60, 450),
            (50, 451),
            (48, 452),
            (30, 453),
            (25, 454),
            (24, 455),
        ],
    },
    CameraVideoFormat {
        id: "2_7k_16_9",
        resolution: "2.7K",
        aspect_ratio: "16:9",
        width: 2688,
        height: 1520,
        fps_values: &[
            (120, 242),
            (100, 243),
            (60, 244),
            (50, 245),
            (48, 331),
            (30, 246),
            (25, 247),
            (24, 248),
        ],
    },
    CameraVideoFormat {
        id: "2_7k_9_16",
        resolution: "2.7K",
        aspect_ratio: "9:16",
        width: 1520,
        height: 2688,
        fps_values: &[
            (60, 441),
            (50, 442),
            (48, 468),
            (30, 443),
            (25, 444),
            (24, 445),
        ],
    },
    CameraVideoFormat {
        id: "1080p_16_9",
        resolution: "1080p",
        aspect_ratio: "16:9",
        width: 1920,
        height: 1080,
        fps_values: &[
            (240, 27),
            (200, 26),
            (120, 28),
            (100, 150),
            (60, 40),
            (50, 81),
            (48, 260),
            (30, 29),
            (25, 52),
            (24, 53),
        ],
    },
    CameraVideoFormat {
        id: "1080p_9_16",
        resolution: "1080p",
        aspect_ratio: "9:16",
        width: 1080,
        height: 1920,
        fps_values: &[(60, 68), (50, 82), (48, 298), (30, 64), (25, 71), (24, 85)],
    },
];

pub fn resolve_camera_video_profile(format_id: &str, fps: u16) -> Option<CameraVideoProfile> {
    let format = CAMERA_VIDEO_FORMATS
        .iter()
        .find(|format| format.id == format_id)?;
    let protocol_value = format
        .fps_values
        .iter()
        .find_map(|(candidate_fps, value)| (*candidate_fps == fps).then_some(*value))?;
    Some(CameraVideoProfile {
        format_id: format.id,
        resolution: format.resolution,
        aspect_ratio: format.aspect_ratio,
        width: format.width,
        height: format.height,
        fps,
        protocol_value,
    })
}

pub struct CameraControlSession {
    host: String,

    command_tx: Sender<CameraWorkerCommand>,

    worker: Option<JoinHandle<()>>,

    capture_mode: Option<CameraCaptureMode>,

    zoom: Option<f64>,

    recording: bool,
}

enum CameraWorkerCommand {
    Execute {
        command_id: u16,
        body: Vec<u8>,
        timeout: Duration,
        reply: Sender<Result<CameraControlResponse, String>>,
    },
    WaitForRecordingState {
        recording: bool,
        timeout: Duration,
        reply: Sender<Result<(), String>>,
    },
    WaitForGimbalSpeed {
        level: u8,
        timeout: Duration,
        reply: Sender<Result<(), String>>,
    },
    WaitForCaptureMode {
        mode: CameraCaptureMode,
        timeout: Duration,
        reply: Sender<Result<(), String>>,
    },
    Shutdown,
}

struct PendingControl {
    command_id: u16,
    deadline: Instant,
    reply: Sender<Result<CameraControlResponse, String>>,
}

struct PendingRecordingState {
    recording: bool,
    deadline: Instant,
    reply: Sender<Result<(), String>>,
}

struct PendingGimbalSpeedState {
    level: u8,
    deadline: Instant,
    reply: Sender<Result<(), String>>,
}

struct PendingCaptureModeState {
    mode: CameraCaptureMode,
    deadline: Instant,
    reply: Sender<Result<(), String>>,
}

enum LunaAuthWorkerCommand {
    Execute {
        command_id: u16,
        body: Vec<u8>,
        timeout: Duration,
        reply: Sender<Result<CameraControlResponse, String>>,
    },
    Shutdown,
}

impl LunaAuthSession {
    pub fn open(host: &str) -> anyhow::Result<Self> {
        let mut session = Self {
            host: host.to_string(),

            port: 6666,

            command_tx: None,

            worker: None,
        };

        session.refresh()?;

        Ok(session)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn is_active(&self) -> bool {
        self.worker
            .as_ref()
            .map(|worker| !worker.is_finished())
            .unwrap_or(false)
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        if self.is_active() {
            return Ok(());
        }

        self.close();
        self.start_worker()?;
        Ok(())
    }

    fn start_worker(&mut self) -> anyhow::Result<()> {
        let mut last_error = None;
        let mut stream = None;
        for attempt in 0..3 {
            match TcpStream::connect((&*self.host, self.port)) {
                Ok(mut connected) => {
                    connected.set_read_timeout(Some(Duration::from_millis(120)))?;
                    connected.set_write_timeout(Some(Duration::from_secs(3)))?;
                    let auth_result = (|| -> Result<(), String> {
                        for payload in AUTH_PAYLOADS {
                            connected
                                .write_all(payload)
                                .and_then(|_| connected.flush())
                                .map_err(|error| format!("发送 Luna 认证包失败：{error}"))?;
                            std::thread::sleep(Duration::from_millis(35));
                        }
                        wait_for_camera_session_ready(&mut connected)
                    })();
                    if let Err(error) = auth_result {
                        return Err(anyhow!(error));
                    }
                    stream = Some(connected);
                    break;
                }
                Err(error) => last_error = Some(error.into()),
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(350));
            }
        }
        let stream = stream.ok_or_else(|| {
            last_error
                .unwrap_or_else(|| anyhow!("无法连接 Luna 媒体会话 {}:{}", self.host, self.port))
        })?;
        stream.set_read_timeout(Some(Duration::from_millis(120)))?;

        let (command_tx, command_rx) = mpsc::channel();
        let worker = thread::spawn(move || luna_auth_worker(stream, command_rx));
        self.command_tx = Some(command_tx);
        self.worker = Some(worker);
        let setup_result = (|| -> anyhow::Result<()> {
            self.execute(CAMERA_SESSION_INIT_COMMAND, &[], Duration::from_secs(6))
                .context("初始化 Luna 媒体会话失败")?;
            self.execute(
                CAMERA_STATUS_QUERY_COMMAND,
                CAMERA_FULL_STATUS_QUERY_BODY,
                Duration::from_secs(8),
            )
            .context("读取 Luna 媒体能力失败")?;
            self.execute(
                CAMERA_CLIENT_REGISTER_COMMAND,
                &build_camera_client_registration_body(),
                Duration::from_secs(6),
            )
            .context("登记 Luna Studio 媒体客户端失败")?;
            let epoch_seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("读取系统时间失败")?
                .as_secs();
            self.execute(
                CAMERA_TIME_SYNC_COMMAND,
                &build_camera_time_sync_body(epoch_seconds),
                Duration::from_secs(6),
            )
            .context("同步 Luna 媒体会话时间失败")?;
            self.execute(
                CAMERA_STATUS_QUERY_COMMAND,
                CAMERA_STATUS_QUERY_BODY,
                Duration::from_secs(6),
            )
            .context("读取 Luna 媒体会话状态失败")?;
            Ok(())
        })();
        if let Err(error) = setup_result {
            self.close();
            return Err(error);
        }
        Ok(())
    }

    pub fn list_files_for_storage(&mut self, storage_id: &str) -> anyhow::Result<Vec<LunaFile>> {
        self.refresh()?;
        let host = self.host.clone();
        list_files_via_ucd2(&host, storage_id, |body| {
            self.execute(CAMERA_FILE_LIST_COMMAND, body, Duration::from_secs(8))
        })
    }

    fn execute(
        &self,
        command_id: u16,
        body: &[u8],
        timeout: Duration,
    ) -> anyhow::Result<CameraControlResponse> {
        let command_tx = self
            .command_tx
            .as_ref()
            .ok_or_else(|| anyhow!("Luna 媒体会话尚未建立"))?;
        let (reply_tx, reply_rx) = mpsc::channel();
        command_tx
            .send(LunaAuthWorkerCommand::Execute {
                command_id,
                body: body.to_vec(),
                timeout,
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("Luna 媒体会话已经关闭"))?;
        receive_camera_response(reply_rx, timeout + Duration::from_secs(1))
    }

    pub fn close(&mut self) {
        if let Some(command_tx) = self.command_tx.take() {
            let _ = command_tx.send(LunaAuthWorkerCommand::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for LunaAuthSession {
    fn drop(&mut self) {
        self.close();
    }
}

fn luna_auth_worker(mut stream: TcpStream, command_rx: Receiver<LunaAuthWorkerCommand>) {
    let mut next_sequence = 0x11u8;
    let mut next_request_id = 0x8000_0002u32;
    let mut last_heartbeat = Instant::now();
    let mut read_buffer = vec![0u8; 64 * 1024];
    let mut receive_buffer = Vec::with_capacity(128 * 1024);
    let mut pending: HashMap<u32, PendingControl> = HashMap::new();

    loop {
        loop {
            match command_rx.try_recv() {
                Ok(LunaAuthWorkerCommand::Execute {
                    command_id,
                    body,
                    timeout,
                    reply,
                }) => {
                    let request_id = next_request_id;
                    next_request_id = next_request_id.wrapping_add(1);
                    let payload =
                        build_internal_packet_with_request_id(command_id, 0x02, request_id, &body);
                    let packet = build_ucd2_frame(0x04, next_sequence, &payload);
                    next_sequence = next_sequence.wrapping_add(1);

                    if let Err(error) = stream.write_all(&packet).and_then(|_| stream.flush()) {
                        let _ = reply.send(Err(format!("发送相册列表命令失败：{error}")));
                        fail_pending(&mut pending, "Luna 媒体会话已断开");
                        return;
                    }

                    pending.insert(
                        request_id,
                        PendingControl {
                            command_id,
                            deadline: Instant::now() + timeout,
                            reply,
                        },
                    );
                }
                Ok(LunaAuthWorkerCommand::Shutdown) => {
                    fail_pending(&mut pending, "Luna 媒体会话已关闭");
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    fail_pending(&mut pending, "Luna 媒体会话已关闭");
                    return;
                }
            }
        }

        if last_heartbeat.elapsed() >= Duration::from_millis(1500) {
            if send_luna_auth_heartbeat(&mut stream, &mut next_sequence).is_err() {
                fail_pending(&mut pending, "Luna 媒体会话心跳失败");
                return;
            }
            last_heartbeat = Instant::now();
        }

        match stream.read(&mut read_buffer) {
            Ok(0) => {
                fail_pending(&mut pending, "Luna 已关闭媒体会话");
                return;
            }
            Ok(count) => {
                receive_buffer.extend_from_slice(&read_buffer[..count]);
                for frame in extract_complete_ucd2_frames(&mut receive_buffer) {
                    let frame_type = frame[6];
                    let header_len = frame[5] as usize;
                    let payload_len =
                        u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
                    let payload = &frame[header_len..header_len + payload_len];
                    if frame_type != 0x04 || payload.len() < 9 {
                        continue;
                    }

                    let status = u16::from_le_bytes([payload[0], payload[1]]);
                    let request_id =
                        u32::from_le_bytes([payload[3], payload[4], payload[5], payload[6]]);
                    let Some(waiter) = pending.remove(&request_id) else {
                        continue;
                    };
                    if status == 0x00c8 {
                        let body = payload[9..].to_vec();
                        let response = CameraControlResponse {
                            command_id: waiter.command_id,
                            request_id,
                            body_hex: bytes_to_hex(&body),
                            media_path: first_camera_media_path(&body),
                            body,
                        };
                        let _ = waiter.reply.send(Ok(response));
                    } else {
                        let _ = waiter
                            .reply
                            .send(Err(format!("相机返回状态 0x{status:04x}")));
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => {
                fail_pending(&mut pending, "读取 Luna 媒体会话失败");
                return;
            }
        }

        let expired: Vec<u32> = pending
            .iter()
            .filter_map(|(request_id, waiter)| {
                (Instant::now() >= waiter.deadline).then_some(*request_id)
            })
            .collect();
        for request_id in expired {
            if let Some(waiter) = pending.remove(&request_id) {
                let _ = waiter.reply.send(Err("等待相册列表响应超时".to_string()));
            }
        }
    }
}

fn send_luna_auth_heartbeat(stream: &mut TcpStream, next_sequence: &mut u8) -> Result<(), String> {
    let heartbeat = build_ucd2_frame(0x05, *next_sequence, &[]);
    *next_sequence = next_sequence.wrapping_add(1);
    stream
        .write_all(&heartbeat)
        .and_then(|_| stream.flush())
        .map_err(|error| format!("Luna 媒体会话心跳失败：{error}"))
}

impl Ucd2RawSession {
    pub fn open(host: &str) -> anyhow::Result<Self> {
        let stream = TcpStream::connect((host, 6666))
            .with_context(|| format!("failed to connect APK-derived UCD2 port {host}:6666"))?;

        stream.set_read_timeout(Some(Duration::from_millis(350)))?;

        stream.set_write_timeout(Some(Duration::from_secs(3)))?;

        Ok(Self {
            host: host.to_string(),

            stream,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn send_apk_auth(&mut self) -> anyhow::Result<Ucd2ProbeResult> {
        let packets: Vec<Vec<u8>> = AUTH_PAYLOADS
            .iter()
            .map(|payload| payload.to_vec())
            .collect();

        let notes = vec![

            "Persistent UCD2 session: the socket stays open until Disconnect Luna.".to_string(),

            "APK evidence: z03 basic/business.json has ucd2=true".to_string(),

            "APK evidence: classes.dex contains UCD2, UCD2-ENCRYPT, UCD2-XOR-KEY-001".to_string(),

            "APK evidence: this probe only sends the two captured UCD2 auth packets already present in this project from APK analysis".to_string(),

        ];

        self.send_packets(&packets, notes)
    }

    pub fn send_packets(
        &mut self,

        packets: &[Vec<u8>],

        notes: Vec<String>,
    ) -> anyhow::Result<Ucd2ProbeResult> {
        let mut result = Ucd2ProbeResult {
            host: self.host.clone(),

            sent_packets: Vec::new(),

            received_packets: Vec::new(),

            sent_frames: Vec::new(),

            received_frames: Vec::new(),

            notes,
        };

        read_available_hex(
            &mut self.stream,
            &mut result.received_packets,
            &mut result.received_frames,
        )?;

        for payload in packets {
            self.stream.write_all(payload)?;

            self.stream.flush()?;

            result.sent_packets.push(bytes_to_hex(payload));

            result.sent_frames.extend(parse_ucd2_frames(payload));

            std::thread::sleep(Duration::from_millis(120));

            read_available_hex(
                &mut self.stream,
                &mut result.received_packets,
                &mut result.received_frames,
            )?;
        }

        Ok(result)
    }

    pub fn poll_pending(&mut self) -> anyhow::Result<Ucd2ProbeResult> {
        let mut result = Ucd2ProbeResult {
            host: self.host.clone(),

            sent_packets: Vec::new(),

            received_packets: Vec::new(),

            sent_frames: Vec::new(),

            received_frames: Vec::new(),

            notes: vec![
                "Persistent UCD2 session poll: no bytes were sent.".to_string(),
                "This only reads frames already queued on the existing socket.".to_string(),
            ],
        };

        read_available_hex(
            &mut self.stream,
            &mut result.received_packets,
            &mut result.received_frames,
        )?;

        Ok(result)
    }

    pub fn collect_heartbeats(&mut self) -> anyhow::Result<Ucd2ProbeResult> {
        let mut result = Ucd2ProbeResult {

            host: self.host.clone(),

            sent_packets: Vec::new(),

            received_packets: Vec::new(),

            sent_frames: Vec::new(),

            received_frames: Vec::new(),

            notes: vec![

                "Persistent UCD2 heartbeat collection: no bytes were sent.".to_string(),

                "This loops over the existing socket for about 8 seconds and deduplicates frames by frame_hex.".to_string(),

            ],

        };

        let started = Instant::now();

        let mut seen = HashSet::new();

        while started.elapsed() < Duration::from_secs(8) {
            let mut packets = Vec::new();

            let mut frames = Vec::new();

            read_available_hex(&mut self.stream, &mut packets, &mut frames)?;

            result.received_packets.extend(packets);

            for frame in frames {
                if seen.insert(frame.frame_hex.clone()) {
                    result.received_frames.push(frame);
                }
            }

            std::thread::sleep(Duration::from_millis(350));
        }

        Ok(result)
    }

    pub fn read_device_info(&mut self) -> anyhow::Result<Ucd2ProbeResult> {
        let packet = build_ucd2_frame(0x04, 0x10, DEVICE_INFO_PAYLOAD);

        self.send_packets(

            &[packet],

            vec![

                "Persistent UCD2 session: the socket stays open until Disconnect Luna.".to_string(),

                "APK-derived device info command: generated with recovered UCD2 header and checksum algorithm.".to_string(),

            ],

        )
    }

    pub fn send_negotiation_sync(&mut self) -> anyhow::Result<Ucd2ProbeResult> {
        self.send_packets(
            &[UCD2_NEGOTIATION_SYNC.to_vec()],
            vec![
                "Persistent UCD2 session: the socket stays open until Disconnect Luna.".to_string(),
                "APK-derived negotiation probe: seg12 0x563fe8 field@7059 inline bytes `syNceNdinS`.".to_string(),
                "This is not a UCD2 frame; it is the lower negotiation sync buffer sent by seg12 0x563e48 through ee92/f975/ee50.".to_string(),
            ],
        )
    }

    pub fn send_stop_capture_candidate(
        &mut self,
        variant: &str,
    ) -> anyhow::Result<Ucd2ProbeResult> {
        let (command_id, body, variant_note) = match variant {
            "base" => (
                0x0008,
                STOP_CAPTURE_BASE_BODY,
                "APK-derived StopCapture candidate: base branch body b2 0003 59 c7 00000005 57 b0.",
            ),
            "a03f" => (
                0x0008,
                STOP_CAPTURE_A03F_BODY,
                "APK-derived StopCapture candidate: a03f branch body b2 0003 59 c7 00000009 57 59 b3 0003 b0.",
            ),
            "full_base" => (
                0x0008,
                STOP_CAPTURE_FULL_BASE_BODY,
                "APK-derived StopCapture candidate: full 0x471b8c builder node, base branch, no 9d58 extension.",
            ),
            "full_a03f" => (
                0x0008,
                STOP_CAPTURE_FULL_A03F_BODY,
                "APK-derived StopCapture candidate: full 0x471b8c builder node, a03f branch, no 9d58 extension.",
            ),
            "full_base_9d58" => (
                0x0008,
                STOP_CAPTURE_FULL_BASE_9D58_BODY,
                "APK-derived StopCapture candidate: full 0x471b8c builder node, base branch, with 9d58 extension.",
            ),
            "full_a03f_9d58" => (
                0x0008,
                STOP_CAPTURE_FULL_A03F_9D58_BODY,
                "APK-derived StopCapture candidate: full 0x471b8c builder node, a03f branch, with 9d58 extension.",
            ),
            "seq_base" => (
                0x0008,
                STOP_CAPTURE_APK_SEQUENCE_BASE_BODY,
                "APK-derived StopCapture action-list candidate: a03c marker 4234 plus base builder node; not full root.9cd6 output.",
            ),
            "seq_a03f" => (
                0x0008,
                STOP_CAPTURE_APK_SEQUENCE_A03F_BODY,
                "APK-derived StopCapture action-list candidate: a03c marker 4234 plus a03f builder node; not full root.9cd6 output.",
            ),
            "seq_4121" => (
                0x0008,
                STOP_CAPTURE_APK_SEQUENCE_4121_BODY,
                "APK-derived StopCapture action-list candidate: a066 marker 4121 plus mandatory 9d58 builder node; not full root.9cd6 output.",
            ),
            "seq_wrapped_a03f" => (
                0x0008,
                STOP_CAPTURE_APK_WRAPPED_SEQUENCE_A03F_BODY,
                "APK-derived StopCapture action-list candidate: 480fcc/a0e4 wrapper marker 4233, then a03c marker 4234, then a03f builder node; not full root.9cd6 output.",
            ),
            "command199_empty" => (
                0x00c7,
                STOP_CAPTURE_COMMAND199_EMPTY_BODY,
                "APK-derived StopCapture candidate: seg08 selector 199 used as ee91 command id, seg07 StopCapture{} empty body.",
            ),
            "command199_selector" => (
                0x00c7,
                STOP_CAPTURE_COMMAND199_SELECTOR_BODY,
                "APK-derived StopCapture candidate: seg08 selector 199 used as ee91 command id with recovered StopCapture selector body.",
            ),
            other => anyhow::bail!("unknown StopCapture candidate variant: {other}"),
        };

        let payload = build_internal_packet(command_id, 0x02, body);
        let packet = build_ucd2_frame(0x04, 0x10, &payload);

        self.send_packets(
            &[packet],
            vec![
                "Persistent UCD2 session: the socket stays open until Disconnect Luna.".to_string(),
                variant_note.to_string(),
                "APK-only evidence: seg08 StopCapture template plus seg12 Packet/UCD2 builder; still requires device-response verification.".to_string(),
            ],
        )
    }
}

impl CameraControlSession {
    pub fn open(host: &str, preview_tx: SyncSender<LivePreviewChunk>) -> anyhow::Result<Self> {
        let (command_tx, command_rx) = mpsc::channel();
        let (startup_tx, startup_rx) = mpsc::sync_channel(1);
        let worker_host = host.to_string();
        let worker = thread::spawn(move || {
            run_camera_worker(worker_host, command_rx, preview_tx, startup_tx)
        });

        match startup_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                let mut session = Self {
                    host: host.to_string(),
                    command_tx,
                    worker: Some(worker),
                    capture_mode: None,
                    zoom: None,
                    recording: false,
                };
                session
                    .execute(CAMERA_SESSION_INIT_COMMAND, &[])
                    .context("初始化 Luna Ultra 相机会话失败")?;
                let full_status = session
                    .execute(CAMERA_STATUS_QUERY_COMMAND, CAMERA_FULL_STATUS_QUERY_BODY)
                    .context("读取 Luna Ultra 完整相机能力失败")?;
                session.capture_mode = capture_mode_from_full_status_body(&full_status.body);
                if session.capture_mode.is_none() {
                    log::warn!("Luna Ultra 完整状态中没有可识别的当前拍摄模式");
                }
                let registration_body = build_camera_client_registration_body();
                session
                    .execute(CAMERA_CLIENT_REGISTER_COMMAND, &registration_body)
                    .context("登记 Luna Studio 相机客户端失败")?;
                let epoch_seconds = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .context("读取系统时间失败")?
                    .as_secs();
                let time_sync_body = build_camera_time_sync_body(epoch_seconds);
                session
                    .execute(CAMERA_TIME_SYNC_COMMAND, &time_sync_body)
                    .context("同步 Luna Ultra 相机时间失败")?;
                session
                    .execute(CAMERA_STATUS_QUERY_COMMAND, CAMERA_STATUS_QUERY_BODY)
                    .context("读取 Luna Ultra 相机状态失败")?;
                session
                    .execute(CAMERA_EVENT_SUBSCRIBE_COMMAND, CAMERA_EVENT_SUBSCRIBE_BODY)
                    .context("订阅 Luna Ultra 相机状态事件失败")?;
                if let Err(error) = session.refresh_zoom_state() {
                    if !session.is_active() {
                        return Err(error.context("读取变焦状态时相机控制会话已断开"));
                    }
                    log::warn!("Luna Ultra 未返回当前变焦状态：{error:#}");
                }
                Ok(session)
            }
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(anyhow!(error))
            }
            Err(_) => {
                let _ = command_tx.send(CameraWorkerCommand::Shutdown);
                let _ = worker.join();
                Err(anyhow!("连接 Luna Ultra 控制会话超时"))
            }
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn is_active(&self) -> bool {
        self.worker
            .as_ref()
            .map(|worker| !worker.is_finished())
            .unwrap_or(false)
    }

    pub fn capture_mode(&self) -> Option<CameraCaptureMode> {
        self.capture_mode
    }

    pub fn zoom(&self) -> Option<f64> {
        self.zoom
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn list_files_for_storage(&self, storage_id: &str) -> anyhow::Result<Vec<LunaFile>> {
        list_files_via_ucd2(&self.host, storage_id, |body| {
            self.execute_with_timeout(CAMERA_FILE_LIST_COMMAND, body, Duration::from_secs(8))
        })
    }

    pub fn switch_capture_mode(
        &mut self,
        mode: CameraCaptureMode,
    ) -> anyhow::Result<CameraControlResponse> {
        if self.recording {
            anyhow::bail!("请先停止录像，再切换拍摄模式");
        }
        let mode_waiter = self.prepare_capture_mode_wait(mode)?;
        let body = build_capture_mode_body(mode);
        let context_body = build_capture_context_body(CameraCaptureMode::Photo);
        let mode_response = self.queue_execute(0x0007, &body, Duration::from_secs(6))?;
        let context_response = self.queue_execute(0x000a, &context_body, Duration::from_secs(6))?;
        let response = receive_camera_response(mode_response, Duration::from_secs(7))?;
        receive_camera_response(context_response, Duration::from_secs(7))?;
        receive_camera_state(mode_waiter, Duration::from_secs(6))?;
        self.capture_mode = Some(mode);
        if let Err(error) = self.refresh_capture_mode_context(mode) {
            if !self.is_active() {
                return Err(error.context("切换拍摄模式后控制会话已断开"));
            }
            log::warn!("拍摄模式已经切换，但辅助状态刷新未全部完成：{error:#}");
        }
        self.set_zoom(1.0)
            .context("切换拍摄模式后初始化 1× 变焦失败")?;
        Ok(response)
    }

    pub fn set_zoom(&mut self, zoom: f64) -> anyhow::Result<CameraControlResponse> {
        let mode = self.capture_mode.context("请先选择拍照或录像模式")?;
        let body = build_zoom_body(zoom, mode)?;
        let response = self
            .execute(CAMERA_SET_OPTION_COMMAND, &body)
            .context("设置 Luna Ultra 变焦失败")?;
        self.zoom = None;
        self.refresh_zoom_state()
            .context("相机已收到变焦设置，但读取实际变焦值失败")?;
        Ok(response)
    }

    pub fn set_video_profile(
        &self,
        profile: CameraVideoProfile,
    ) -> anyhow::Result<CameraControlResponse> {
        if self.capture_mode != Some(CameraCaptureMode::Video) {
            anyhow::bail!("请先切换到录像模式");
        }
        if self.recording {
            anyhow::bail!("请先停止录像，再调整录像规格");
        }
        let body = build_video_profile_body(profile);
        self.execute(CAMERA_SET_OPTION_COMMAND, &body)
            .context("设置 Luna Ultra 录像规格失败")
    }

    fn refresh_capture_mode_context(&self, mode: CameraCaptureMode) -> anyhow::Result<()> {
        let context_response = self.queue_execute(
            CAMERA_CAPTURE_CONTEXT_COMMAND,
            &build_capture_context_body(mode),
            Duration::from_secs(6),
        )?;
        let ready_response = self.queue_execute(
            CAMERA_STATUS_QUERY_COMMAND,
            CAMERA_CAPTURE_READY_QUERY_BODY,
            Duration::from_secs(6),
        )?;
        let detail_response = self.queue_execute(
            CAMERA_CAPTURE_CONTEXT_COMMAND,
            &build_capture_detail_body(mode),
            Duration::from_secs(6),
        )?;
        let combined_response = self.queue_execute(
            CAMERA_CAPTURE_CONTEXT_COMMAND,
            &build_capture_combined_context_body(mode),
            Duration::from_secs(6),
        )?;

        receive_camera_response(context_response, Duration::from_secs(7))
            .context("刷新相机拍摄上下文失败")?;
        receive_camera_response(ready_response, Duration::from_secs(7))
            .context("读取相机拍摄准备状态失败")?;
        receive_camera_response(detail_response, Duration::from_secs(7))
            .context("读取相机拍摄模式详情失败")?;
        receive_camera_response(combined_response, Duration::from_secs(7))
            .context("确认相机拍摄上下文失败")?;
        Ok(())
    }

    fn refresh_zoom_state(&mut self) -> anyhow::Result<()> {
        let mode = self.capture_mode.context("相机尚未返回当前拍摄模式")?;
        let response = self
            .execute(
                CAMERA_CAPTURE_CONTEXT_COMMAND,
                &build_capture_settings_query_body(mode),
            )
            .context("读取 Luna Ultra 拍摄设置失败")?;
        self.zoom = zoom_from_capture_settings_body(&response.body);
        if self.zoom.is_none() {
            anyhow::bail!("相机拍摄设置中没有有效变焦值");
        }
        Ok(())
    }

    pub fn take_photo(&mut self) -> anyhow::Result<CameraControlResponse> {
        if self.capture_mode != Some(CameraCaptureMode::Photo) {
            anyhow::bail!("请先切换到拍照模式");
        }
        if self.recording {
            anyhow::bail!("录像过程中不能拍照");
        }
        self.execute(CAMERA_TAKE_PHOTO_COMMAND, CAMERA_TAKE_PHOTO_BODY)
    }

    pub fn start_recording(&mut self) -> anyhow::Result<CameraControlResponse> {
        if self.capture_mode != Some(CameraCaptureMode::Video) {
            anyhow::bail!("请先切换到录像模式");
        }
        if self.recording {
            anyhow::bail!("录像已经开始");
        }
        let response = self.execute(CAMERA_START_RECORD_COMMAND, CAMERA_START_RECORD_BODY)?;
        self.wait_for_recording_state(true)?;
        self.recording = true;
        Ok(response)
    }

    pub fn stop_recording(&mut self) -> anyhow::Result<CameraControlResponse> {
        if !self.recording {
            anyhow::bail!("当前没有正在进行的录像");
        }
        let response = self.execute(CAMERA_STOP_RECORD_COMMAND, CAMERA_STOP_RECORD_BODY)?;
        self.wait_for_recording_state(false)?;
        self.recording = false;
        Ok(response)
    }

    pub fn start_preview(&self) -> anyhow::Result<CameraControlResponse> {
        self.execute(CAMERA_PREVIEW_PREPARE_COMMAND, CAMERA_PREVIEW_PREPARE_BODY)
            .context("准备 Luna Ultra 实时预览失败")?;
        self.execute(CAMERA_PREVIEW_ACTIVATE_COMMAND, &[])
            .context("激活 Luna Ultra 实时预览失败")?;
        self.execute(
            0x0001,
            &[
                0x10, 0x01, 0x30, 0x28, 0x38, 0x2c, 0x40, 0x01, 0x48, 0x28, 0x50, 0x22,
            ],
        )
    }

    pub fn stop_preview(&self) -> anyhow::Result<CameraControlResponse> {
        self.execute(0x0002, &[])
    }

    pub fn move_gimbal(
        &self,
        horizontal: i16,
        vertical: i16,
    ) -> anyhow::Result<CameraControlResponse> {
        let (device_x, device_y) = gimbal_device_axes_from_ui(horizontal, vertical)?;
        let body = build_gimbal_move_body(device_x, device_y)?;
        self.execute_with_timeout(0x00e2, &body, Duration::from_secs(2))
    }

    pub fn set_gimbal_speed(&self, level: u8) -> anyhow::Result<CameraControlResponse> {
        let body = build_gimbal_speed_body(level)?;
        let response = self.execute(0x0009, &body)?;
        self.execute(
            0x000a,
            &build_capture_context_body(CameraCaptureMode::Photo),
        )
        .context("刷新 Luna Ultra 云台速度状态失败")?;
        self.wait_for_gimbal_speed(level)?;
        Ok(response)
    }

    pub fn delete_media_urls(&self, urls: &[String]) -> anyhow::Result<Vec<String>> {
        let mut paths = Vec::new();
        let mut seen = HashSet::new();
        for url in urls {
            let path = camera_path_from_url(&self.host, url)?;
            if seen.insert(path.clone()) {
                paths.push(path);
            }
        }
        if paths.is_empty() {
            anyhow::bail!("没有可删除的相机素材");
        }

        for batch in paths.chunks(50) {
            let body = build_delete_files_body(batch)?;
            self.execute_with_timeout(0x000c, &body, Duration::from_secs(20))?;
        }
        Ok(paths)
    }

    fn execute(&self, command_id: u16, body: &[u8]) -> anyhow::Result<CameraControlResponse> {
        self.execute_with_timeout(command_id, body, Duration::from_secs(6))
    }

    fn execute_with_timeout(
        &self,
        command_id: u16,
        body: &[u8],
        timeout: Duration,
    ) -> anyhow::Result<CameraControlResponse> {
        let reply_rx = self.queue_execute(command_id, body, timeout)?;
        receive_camera_response(reply_rx, timeout + Duration::from_secs(1))
    }

    fn queue_execute(
        &self,
        command_id: u16,
        body: &[u8],
        timeout: Duration,
    ) -> anyhow::Result<Receiver<Result<CameraControlResponse, String>>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(CameraWorkerCommand::Execute {
                command_id,
                body: body.to_vec(),
                timeout,
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("Luna Ultra 控制会话已经关闭"))?;
        Ok(reply_rx)
    }

    fn wait_for_recording_state(&self, recording: bool) -> anyhow::Result<()> {
        let timeout = Duration::from_secs(3);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(CameraWorkerCommand::WaitForRecordingState {
                recording,
                timeout,
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("Luna Ultra 控制会话已经关闭"))?;

        match reply_rx.recv_timeout(timeout + Duration::from_secs(1)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(anyhow!(error)),
            Err(_) => Err(anyhow!("等待相机录像状态超时")),
        }
    }

    fn wait_for_gimbal_speed(&self, level: u8) -> anyhow::Result<()> {
        let timeout = Duration::from_secs(3);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(CameraWorkerCommand::WaitForGimbalSpeed {
                level,
                timeout,
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("Luna Ultra 控制会话已经关闭"))?;

        match reply_rx.recv_timeout(timeout + Duration::from_secs(1)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(anyhow!(error)),
            Err(_) => Err(anyhow!("等待相机云台速度状态超时")),
        }
    }

    fn prepare_capture_mode_wait(
        &self,
        mode: CameraCaptureMode,
    ) -> anyhow::Result<Receiver<Result<(), String>>> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(CameraWorkerCommand::WaitForCaptureMode {
                mode,
                timeout: Duration::from_secs(5),
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("Luna Ultra 控制会话已经关闭"))?;
        Ok(reply_rx)
    }
}

fn receive_camera_response(
    reply_rx: Receiver<Result<CameraControlResponse, String>>,
    timeout: Duration,
) -> anyhow::Result<CameraControlResponse> {
    match reply_rx.recv_timeout(timeout) {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => Err(anyhow!(error)),
        Err(_) => Err(anyhow!("等待相机响应超时")),
    }
}

fn receive_camera_state(
    reply_rx: Receiver<Result<(), String>>,
    timeout: Duration,
) -> anyhow::Result<()> {
    match reply_rx.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(anyhow!(error)),
        Err(_) => Err(anyhow!("等待相机状态超时")),
    }
}

impl Drop for CameraControlSession {
    fn drop(&mut self) {
        let _ = self.command_tx.send(CameraWorkerCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_camera_worker(
    host: String,
    command_rx: Receiver<CameraWorkerCommand>,
    preview_tx: SyncSender<LivePreviewChunk>,
    startup_tx: SyncSender<Result<(), String>>,
) {
    let mut stream = match TcpStream::connect((&*host, 6666)) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = startup_tx.send(Err(format!("无法连接 Luna Ultra 控制端口：{error}")));
            return;
        }
    };

    if let Err(error) = stream.set_read_timeout(Some(Duration::from_millis(35))) {
        let _ = startup_tx.send(Err(format!("无法设置相机读取超时：{error}")));
        return;
    }
    if let Err(error) = stream.set_write_timeout(Some(Duration::from_secs(3))) {
        let _ = startup_tx.send(Err(format!("无法设置相机写入超时：{error}")));
        return;
    }

    for packet in AUTH_PAYLOADS {
        if let Err(error) = stream.write_all(packet).and_then(|_| stream.flush()) {
            let _ = startup_tx.send(Err(format!("Luna Ultra 会话初始化失败：{error}")));
            return;
        }
        thread::sleep(Duration::from_millis(35));
    }

    if let Err(error) = wait_for_camera_session_ready(&mut stream) {
        let _ = startup_tx.send(Err(error));
        return;
    }

    let _ = startup_tx.send(Ok(()));

    let mut next_sequence = 0x11u8;
    let mut next_request_id = 0x8000_0002u32;
    let mut receive_buffer = Vec::with_capacity(128 * 1024);
    let mut pending: HashMap<u32, PendingControl> = HashMap::new();
    let mut pending_recording_state: Option<PendingRecordingState> = None;
    let mut pending_gimbal_speed: Option<PendingGimbalSpeedState> = None;
    let mut pending_capture_mode: Option<PendingCaptureModeState> = None;
    let mut reported_recording = None;
    let mut reported_gimbal_speed = None;
    let mut preview_active = false;
    let mut wait_for_keyframe = false;
    let mut last_heartbeat = Instant::now();
    let mut read_buffer = vec![0u8; 64 * 1024];

    loop {
        loop {
            match command_rx.try_recv() {
                Ok(CameraWorkerCommand::Execute {
                    command_id,
                    body,
                    timeout,
                    reply,
                }) => {
                    let request_id = next_request_id;
                    next_request_id = next_request_id.wrapping_add(1);
                    let payload =
                        build_internal_packet_with_request_id(command_id, 0x02, request_id, &body);
                    let packet = build_ucd2_frame(0x04, next_sequence, &payload);
                    next_sequence = next_sequence.wrapping_add(1);

                    if let Err(error) = stream.write_all(&packet).and_then(|_| stream.flush()) {
                        let _ = reply.send(Err(format!("发送相机命令失败：{error}")));
                        fail_pending(&mut pending, "Luna Ultra 控制连接已断开");
                        fail_recording_waiter(
                            &mut pending_recording_state,
                            "Luna Ultra 控制连接已断开",
                        );
                        fail_gimbal_speed_waiter(
                            &mut pending_gimbal_speed,
                            "Luna Ultra 控制连接已断开",
                        );
                        fail_capture_mode_waiter(
                            &mut pending_capture_mode,
                            "Luna Ultra 控制连接已断开",
                        );
                        return;
                    }

                    if command_id == 0x0001 {
                        preview_active = true;
                        wait_for_keyframe = false;
                    } else if command_id == 0x0002 {
                        preview_active = false;
                    }

                    pending.insert(
                        request_id,
                        PendingControl {
                            command_id,
                            deadline: Instant::now() + timeout,
                            reply,
                        },
                    );
                }
                Ok(CameraWorkerCommand::WaitForRecordingState {
                    recording,
                    timeout,
                    reply,
                }) => {
                    if reported_recording == Some(recording) {
                        let _ = reply.send(Ok(()));
                    } else {
                        if let Some(waiter) = pending_recording_state.take() {
                            let _ = waiter
                                .reply
                                .send(Err("录像状态等待已被新的操作替代".to_string()));
                        }
                        pending_recording_state = Some(PendingRecordingState {
                            recording,
                            deadline: Instant::now() + timeout,
                            reply,
                        });
                    }
                }
                Ok(CameraWorkerCommand::WaitForGimbalSpeed {
                    level,
                    timeout,
                    reply,
                }) => {
                    if reported_gimbal_speed == Some(level) {
                        let _ = reply.send(Ok(()));
                    } else {
                        if let Some(waiter) = pending_gimbal_speed.take() {
                            let _ = waiter
                                .reply
                                .send(Err("云台速度等待已被新的操作替代".to_string()));
                        }
                        pending_gimbal_speed = Some(PendingGimbalSpeedState {
                            level,
                            deadline: Instant::now() + timeout,
                            reply,
                        });
                    }
                }
                Ok(CameraWorkerCommand::WaitForCaptureMode {
                    mode,
                    timeout,
                    reply,
                }) => {
                    if let Some(waiter) = pending_capture_mode.take() {
                        let _ = waiter
                            .reply
                            .send(Err("拍摄模式等待已被新的操作替代".to_string()));
                    }
                    pending_capture_mode = Some(PendingCaptureModeState {
                        mode,
                        deadline: Instant::now() + timeout,
                        reply,
                    });
                }
                Ok(CameraWorkerCommand::Shutdown) => {
                    fail_pending(&mut pending, "Luna Ultra 控制会话已关闭");
                    fail_recording_waiter(
                        &mut pending_recording_state,
                        "Luna Ultra 控制会话已关闭",
                    );
                    fail_gimbal_speed_waiter(
                        &mut pending_gimbal_speed,
                        "Luna Ultra 控制会话已关闭",
                    );
                    fail_capture_mode_waiter(
                        &mut pending_capture_mode,
                        "Luna Ultra 控制会话已关闭",
                    );
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    fail_pending(&mut pending, "Luna Ultra 控制会话已关闭");
                    fail_recording_waiter(
                        &mut pending_recording_state,
                        "Luna Ultra 控制会话已关闭",
                    );
                    fail_gimbal_speed_waiter(
                        &mut pending_gimbal_speed,
                        "Luna Ultra 控制会话已关闭",
                    );
                    fail_capture_mode_waiter(
                        &mut pending_capture_mode,
                        "Luna Ultra 控制会话已关闭",
                    );
                    return;
                }
            }
        }

        if last_heartbeat.elapsed() >= Duration::from_millis(1500) {
            let heartbeat = build_ucd2_frame(0x05, next_sequence, &[]);
            next_sequence = next_sequence.wrapping_add(1);
            if stream
                .write_all(&heartbeat)
                .and_then(|_| stream.flush())
                .is_err()
            {
                fail_pending(&mut pending, "Luna Ultra 心跳发送失败");
                fail_recording_waiter(&mut pending_recording_state, "Luna Ultra 心跳发送失败");
                fail_gimbal_speed_waiter(&mut pending_gimbal_speed, "Luna Ultra 心跳发送失败");
                fail_capture_mode_waiter(&mut pending_capture_mode, "Luna Ultra 心跳发送失败");
                return;
            }
            last_heartbeat = Instant::now();
        }

        match stream.read(&mut read_buffer) {
            Ok(0) => {
                fail_pending(&mut pending, "Luna Ultra 已关闭控制连接");
                fail_recording_waiter(&mut pending_recording_state, "Luna Ultra 已关闭控制连接");
                fail_gimbal_speed_waiter(&mut pending_gimbal_speed, "Luna Ultra 已关闭控制连接");
                fail_capture_mode_waiter(&mut pending_capture_mode, "Luna Ultra 已关闭控制连接");
                return;
            }
            Ok(count) => {
                receive_buffer.extend_from_slice(&read_buffer[..count]);
                for frame in extract_complete_ucd2_frames(&mut receive_buffer) {
                    let frame_type = frame[6];
                    let header_len = frame[5] as usize;
                    let payload_len =
                        u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
                    let payload = &frame[header_len..header_len + payload_len];

                    if frame_type == 0x04 && payload.len() >= 9 {
                        let status = u16::from_le_bytes([payload[0], payload[1]]);
                        let request_id =
                            u32::from_le_bytes([payload[3], payload[4], payload[5], payload[6]]);
                        if status == 0x2010 {
                            if let Some(recording) = recording_state_from_event_body(&payload[9..])
                            {
                                reported_recording = Some(recording);
                                if pending_recording_state
                                    .as_ref()
                                    .map(|waiter| waiter.recording == recording)
                                    .unwrap_or(false)
                                {
                                    if let Some(waiter) = pending_recording_state.take() {
                                        let _ = waiter.reply.send(Ok(()));
                                    }
                                }
                            }
                            continue;
                        }
                        if status == 0x206a {
                            if let Some(level) = gimbal_speed_from_event_body(&payload[9..]) {
                                reported_gimbal_speed = Some(level);
                                if pending_gimbal_speed
                                    .as_ref()
                                    .map(|waiter| waiter.level == level)
                                    .unwrap_or(false)
                                {
                                    if let Some(waiter) = pending_gimbal_speed.take() {
                                        let _ = waiter.reply.send(Ok(()));
                                    }
                                }
                            }
                            continue;
                        }
                        if status == 0x2053 {
                            if let Some(mode) = capture_mode_from_event_body(&payload[9..]) {
                                if pending_capture_mode
                                    .as_ref()
                                    .map(|waiter| waiter.mode == mode)
                                    .unwrap_or(false)
                                {
                                    if let Some(waiter) = pending_capture_mode.take() {
                                        let _ = waiter.reply.send(Ok(()));
                                    }
                                }
                            }
                            continue;
                        }
                        if let Some(waiter) = pending.remove(&request_id) {
                            if status == 0x00c8 {
                                let body = &payload[9..];
                                let response = CameraControlResponse {
                                    command_id: waiter.command_id,
                                    request_id,
                                    body_hex: bytes_to_hex(body),
                                    media_path: first_camera_media_path(body),
                                    body: body.to_vec(),
                                };
                                let _ = waiter.reply.send(Ok(response));
                            } else {
                                let _ = waiter
                                    .reply
                                    .send(Err(format!("相机返回状态 0x{status:04x}")));
                            }
                        }
                    } else if frame_type == 0x01
                        && preview_active
                        && payload.len() > 9
                        && payload[0] == 0x20
                    {
                        let data = payload[9..].to_vec();
                        let key = is_hevc_keyframe(&data);
                        if wait_for_keyframe && !key {
                            continue;
                        }
                        let chunk = LivePreviewChunk { data };
                        match preview_tx.try_send(chunk) {
                            Ok(()) => wait_for_keyframe = false,
                            Err(TrySendError::Full(_)) => wait_for_keyframe = true,
                            Err(TrySendError::Disconnected(_)) => preview_active = false,
                        }
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => {
                fail_pending(&mut pending, "读取 Luna Ultra 控制连接失败");
                fail_recording_waiter(&mut pending_recording_state, "读取 Luna Ultra 控制连接失败");
                fail_gimbal_speed_waiter(&mut pending_gimbal_speed, "读取 Luna Ultra 控制连接失败");
                fail_capture_mode_waiter(&mut pending_capture_mode, "读取 Luna Ultra 控制连接失败");
                return;
            }
        }

        let expired: Vec<u32> = pending
            .iter()
            .filter_map(|(request_id, waiter)| {
                (Instant::now() >= waiter.deadline).then_some(*request_id)
            })
            .collect();
        for request_id in expired {
            if let Some(waiter) = pending.remove(&request_id) {
                let _ = waiter.reply.send(Err("等待相机响应超时".to_string()));
            }
        }
        if pending_recording_state
            .as_ref()
            .map(|waiter| Instant::now() >= waiter.deadline)
            .unwrap_or(false)
        {
            fail_recording_waiter(&mut pending_recording_state, "相机没有进入预期的录像状态");
        }
        if pending_gimbal_speed
            .as_ref()
            .map(|waiter| Instant::now() >= waiter.deadline)
            .unwrap_or(false)
        {
            fail_gimbal_speed_waiter(&mut pending_gimbal_speed, "相机没有应用目标云台速度档位");
        }
        if pending_capture_mode
            .as_ref()
            .map(|waiter| Instant::now() >= waiter.deadline)
            .unwrap_or(false)
        {
            fail_capture_mode_waiter(&mut pending_capture_mode, "相机没有确认目标拍摄模式");
        }
    }
}

fn fail_pending(pending: &mut HashMap<u32, PendingControl>, message: &str) {
    for (_, waiter) in pending.drain() {
        let _ = waiter.reply.send(Err(message.to_string()));
    }
}

fn fail_recording_waiter(waiter: &mut Option<PendingRecordingState>, message: &str) {
    if let Some(waiter) = waiter.take() {
        let _ = waiter.reply.send(Err(message.to_string()));
    }
}

fn fail_gimbal_speed_waiter(waiter: &mut Option<PendingGimbalSpeedState>, message: &str) {
    if let Some(waiter) = waiter.take() {
        let _ = waiter.reply.send(Err(message.to_string()));
    }
}

fn fail_capture_mode_waiter(waiter: &mut Option<PendingCaptureModeState>, message: &str) {
    if let Some(waiter) = waiter.take() {
        let _ = waiter.reply.send(Err(message.to_string()));
    }
}

fn recording_state_from_event_body(body: &[u8]) -> Option<bool> {
    match protobuf_varint_field(body, 1)? {
        0 => Some(false),
        1 if protobuf_varint_field(body, 7) == Some(2) => Some(true),
        _ => None,
    }
}

fn gimbal_speed_from_event_body(body: &[u8]) -> Option<u8> {
    protobuf_varint_field(body, 2).and_then(|value| u8::try_from(value).ok())
}

fn capture_mode_from_event_body(body: &[u8]) -> Option<CameraCaptureMode> {
    match (
        protobuf_varint_field(body, 1)?,
        protobuf_varint_field(body, 2)?,
        protobuf_varint_field(body, 3)?,
    ) {
        (0, 100, 3) => Some(CameraCaptureMode::Photo),
        (100, 0, 3) => Some(CameraCaptureMode::Video),
        _ => None,
    }
}

fn capture_mode_from_full_status_body(body: &[u8]) -> Option<CameraCaptureMode> {
    let status = protobuf_length_delimited_field(body, 2)?;
    match (
        protobuf_varint_field(status, 40)?,
        protobuf_varint_field(status, 41)?,
    ) {
        (0, 100) => Some(CameraCaptureMode::Photo),
        (100, 0) => Some(CameraCaptureMode::Video),
        _ => None,
    }
}

fn zoom_from_capture_settings_body(body: &[u8]) -> Option<f64> {
    let status = protobuf_length_delimited_field(body, 2)?;
    let zoom = f64::from_bits(protobuf_fixed64_field(status, 53)?);
    (zoom.is_finite() && zoom > 0.0 && zoom <= 100.0).then_some(zoom)
}

fn wait_for_camera_session_ready(stream: &mut TcpStream) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut buffer = Vec::with_capacity(8192);
    let mut read_buffer = [0u8; 8192];

    while Instant::now() < deadline {
        match stream.read(&mut read_buffer) {
            Ok(0) => return Err("Luna Ultra 在初始化时关闭了连接".to_string()),
            Ok(count) => {
                buffer.extend_from_slice(&read_buffer[..count]);
                for frame in extract_complete_ucd2_frames(&mut buffer) {
                    let header_len = frame[5] as usize;
                    let payload_len =
                        u32::from_le_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
                    let payload = &frame[header_len..header_len + payload_len];
                    if frame[6] == 0x04
                        && payload.len() >= 9
                        && u16::from_le_bytes([payload[0], payload[1]]) == 0x00c8
                        && u32::from_le_bytes([payload[3], payload[4], payload[5], payload[6]])
                            == 0x8000_0001
                    {
                        return Ok(());
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => return Err(format!("读取 Luna Ultra 初始化响应失败：{error}")),
        }
    }

    Err("等待 Luna Ultra 初始化响应超时".to_string())
}

fn extract_complete_ucd2_frames(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();

    loop {
        let Some(magic_offset) = buffer.windows(4).position(|window| window == b"UCD2") else {
            let keep = buffer.len().min(3);
            if buffer.len() > keep {
                buffer.drain(..buffer.len() - keep);
            }
            break;
        };
        if magic_offset > 0 {
            buffer.drain(..magic_offset);
        }
        if buffer.len() < 12 {
            break;
        }

        let header_len = buffer[5] as usize;
        if !(12..=96).contains(&header_len) {
            buffer.drain(..1);
            continue;
        }
        let payload_len =
            u32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]) as usize;
        let total_len = header_len.saturating_add(payload_len).saturating_add(4);
        if total_len > 64 * 1024 * 1024 {
            buffer.drain(..1);
            continue;
        }
        if buffer.len() < total_len {
            break;
        }
        frames.push(buffer.drain(..total_len).collect());
    }

    frames
}

fn is_hevc_keyframe(data: &[u8]) -> bool {
    let mut offset = 0usize;
    while offset + 5 <= data.len() {
        let start_len = if data[offset..].starts_with(&[0, 0, 0, 1]) {
            4
        } else if data[offset..].starts_with(&[0, 0, 1]) {
            3
        } else {
            offset += 1;
            continue;
        };
        let nal_type = (data[offset + start_len] >> 1) & 0x3f;
        if (16..=23).contains(&nal_type) || nal_type == 32 {
            return true;
        }
        offset += start_len + 1;
    }
    false
}

fn first_camera_media_path(body: &[u8]) -> Option<String> {
    let marker = b"/DCIM/";
    let start = body
        .windows(marker.len())
        .position(|window| window == marker)?;
    let end = body[start..]
        .iter()
        .position(|byte| !byte.is_ascii_graphic())
        .map(|length| start + length)
        .unwrap_or(body.len());
    String::from_utf8(body[start..end].to_vec()).ok()
}

pub(crate) fn camera_path_from_url(host: &str, url_text: &str) -> anyhow::Result<String> {
    let lower_url = url_text.to_ascii_lowercase();
    if lower_url.contains("%2e") || lower_url.contains("%2f") || lower_url.contains("%5c") {
        anyhow::bail!("素材路径包含不允许的编码路径段");
    }
    let expected =
        reqwest::Url::parse(&format!("http://{host}")).context("相机地址无效，请检查连接设置")?;
    let url = reqwest::Url::parse(url_text).context("素材地址无效，请刷新相册后重试")?;
    if !matches!(url.scheme(), "http" | "https") {
        anyhow::bail!("素材地址不是相机可访问地址");
    }
    if url.host_str() != expected.host_str()
        || url.port_or_known_default() != expected.port_or_known_default()
    {
        anyhow::bail!("素材不属于当前连接的相机");
    }

    let path = percent_decode_path(url.path())?;
    const ALLOWED_ROOTS: [&str; 3] = ["/storage_internal/DCIM/", "/sdcard/DCIM/", "/DCIM/"];
    if path.ends_with('/')
        || path.contains('\0')
        || path.contains('\\')
        || path
            .split('/')
            .skip(1)
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !ALLOWED_ROOTS.iter().any(|root| path.starts_with(root))
    {
        anyhow::bail!("素材路径不在相机媒体目录中");
    }
    Ok(path)
}

fn percent_decode_path(path: &str) -> anyhow::Result<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes[offset] == b'%' {
            if offset + 2 >= bytes.len() {
                anyhow::bail!("素材路径包含无效的百分号编码");
            }
            let high = hex_digit(bytes[offset + 1])?;
            let low = hex_digit(bytes[offset + 2])?;
            decoded.push((high << 4) | low);
            offset += 3;
        } else {
            decoded.push(bytes[offset]);
            offset += 1;
        }
    }
    String::from_utf8(decoded).context("素材路径不是有效 UTF-8")
}

fn hex_digit(byte: u8) -> anyhow::Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => anyhow::bail!("素材路径包含无效的百分号编码"),
    }
}

fn build_delete_files_body(paths: &[String]) -> anyhow::Result<Vec<u8>> {
    if paths.is_empty() {
        anyhow::bail!("没有可删除的相机素材");
    }
    let mut body = Vec::new();
    for path in paths {
        body.push(0x0a);
        encode_varint(path.len() as u64, &mut body);
        body.extend_from_slice(path.as_bytes());
    }
    Ok(body)
}

fn build_capture_mode_body(mode: CameraCaptureMode) -> Vec<u8> {
    match mode {
        CameraCaptureMode::Photo => vec![0x08, 0x28, 0x12, 0x00],
        CameraCaptureMode::Video => vec![0x08, 0x29, 0x12, 0x00],
    }
}

fn build_zoom_body(zoom: f64, mode: CameraCaptureMode) -> anyhow::Result<Vec<u8>> {
    if ![1.0, 2.0, 3.0, 6.0].contains(&zoom) {
        anyhow::bail!("当前只开放抓包确认的 1×、2×、3× 和 6× 快捷变焦");
    }
    let mut body = vec![0x08, 0x35, 0x12, 0x0a, 0xa9, 0x03];
    body.extend_from_slice(&zoom.to_le_bytes());
    body.extend_from_slice(&[0x18, capture_context_value(mode)]);
    Ok(body)
}

fn build_video_profile_body(profile: CameraVideoProfile) -> Vec<u8> {
    let mut profile_body = vec![0xf8, 0x01];
    encode_varint(profile.protocol_value.into(), &mut profile_body);

    let mut body = vec![0x08, 0x1f, 0x12];
    encode_varint(profile_body.len() as u64, &mut body);
    body.extend_from_slice(&profile_body);
    body.extend_from_slice(&[0x18, capture_context_value(CameraCaptureMode::Video)]);
    body
}

fn build_capture_context_body(mode: CameraCaptureMode) -> Vec<u8> {
    vec![0x08, 0x63, 0x10, capture_context_value(mode)]
}

fn build_capture_detail_body(mode: CameraCaptureMode) -> Vec<u8> {
    vec![0x08, 0x57, 0x10, capture_context_value(mode)]
}

fn build_capture_combined_context_body(mode: CameraCaptureMode) -> Vec<u8> {
    vec![0x08, 0x29, 0x08, 0x63, 0x10, capture_context_value(mode)]
}

fn build_capture_settings_query_body(mode: CameraCaptureMode) -> Vec<u8> {
    let mut body = Vec::with_capacity(CAMERA_CAPTURE_SETTING_IDS.len() * 2 + 2);
    for setting_id in CAMERA_CAPTURE_SETTING_IDS {
        body.push(0x08);
        encode_varint(*setting_id, &mut body);
    }
    body.extend_from_slice(&[0x10, capture_context_value(mode)]);
    body
}

fn capture_context_value(mode: CameraCaptureMode) -> u8 {
    match mode {
        CameraCaptureMode::Photo => 0x06,
        CameraCaptureMode::Video => 0x07,
    }
}

fn build_camera_client_registration_body() -> Vec<u8> {
    let client_id = CAMERA_CLIENT_ID.as_bytes();
    let mut body = Vec::with_capacity(client_id.len() + 4);
    body.push(0x0a);
    encode_varint(client_id.len() as u64, &mut body);
    body.extend_from_slice(client_id);
    body.extend_from_slice(&[0x10, 0x02]);
    body
}

fn build_camera_time_sync_body(epoch_seconds: u64) -> Vec<u8> {
    const TIME_ZONE: &[u8] = b"Asia/Shanghai";

    let mut values = Vec::with_capacity(24);
    values.push(0x60);
    encode_varint(epoch_seconds, &mut values);
    values.push(0x68);
    encode_varint(28_800, &mut values);
    values.extend_from_slice(&[0xfa, 0x0a]);
    encode_varint(TIME_ZONE.len() as u64, &mut values);
    values.extend_from_slice(TIME_ZONE);

    let mut body = vec![0x08, 0x0c, 0x08, 0x0d, 0x08, 0xaf, 0x01, 0x12];
    encode_varint(values.len() as u64, &mut body);
    body.extend_from_slice(&values);
    body
}

fn gimbal_device_axes_from_ui(horizontal: i16, vertical: i16) -> anyhow::Result<(i16, i16)> {
    if !(-100..=100).contains(&horizontal) || !(-100..=100).contains(&vertical) {
        anyhow::bail!("云台控制值必须在 -100 到 100 之间");
    }
    Ok((-vertical, horizontal))
}

fn build_gimbal_move_body(x: i16, y: i16) -> anyhow::Result<Vec<u8>> {
    if !(-100..=100).contains(&x) || !(-100..=100).contains(&y) {
        anyhow::bail!("云台控制值必须在 -100 到 100 之间");
    }

    let mut coordinates = Vec::with_capacity(6);
    if x != 0 {
        coordinates.push(0x08);
        encode_varint(zigzag_i16(x), &mut coordinates);
    }
    if y != 0 {
        coordinates.push(0x10);
        encode_varint(zigzag_i16(y), &mut coordinates);
    }

    let mut body = vec![0x08, 0x01, 0x12];
    encode_varint(coordinates.len() as u64, &mut body);
    body.extend_from_slice(&coordinates);
    Ok(body)
}

fn build_gimbal_speed_body(level: u8) -> anyhow::Result<Vec<u8>> {
    if !(1..=3).contains(&level) {
        anyhow::bail!("云台速度档位必须在 1 到 3 之间");
    }
    Ok(vec![
        0x08, 0x55, 0x12, 0x05, 0xaa, 0x05, 0x02, 0x10, level, 0x18, 0x06,
    ])
}

fn protobuf_varint_field(message: &[u8], target_field: u64) -> Option<u64> {
    let mut offset = 0usize;
    while offset < message.len() {
        let (tag, tag_len) = decode_varint(&message[offset..])?;
        offset += tag_len;
        let field = tag >> 3;
        match tag & 0x07 {
            0 => {
                let (value, value_len) = decode_varint(&message[offset..])?;
                offset += value_len;
                if field == target_field {
                    return Some(value);
                }
            }
            1 => offset = offset.checked_add(8)?,
            2 => {
                let (length, length_len) = decode_varint(&message[offset..])?;
                offset += length_len;
                offset = offset.checked_add(length as usize)?;
            }
            5 => offset = offset.checked_add(4)?,
            _ => return None,
        }
        if offset > message.len() {
            return None;
        }
    }
    None
}

fn protobuf_length_delimited_field(message: &[u8], target_field: u64) -> Option<&[u8]> {
    let mut offset = 0usize;
    while offset < message.len() {
        let (tag, tag_len) = decode_varint(&message[offset..])?;
        offset += tag_len;
        let field = tag >> 3;
        match tag & 0x07 {
            0 => {
                let (_, value_len) = decode_varint(&message[offset..])?;
                offset += value_len;
            }
            1 => offset = offset.checked_add(8)?,
            2 => {
                let (length, length_len) = decode_varint(&message[offset..])?;
                offset += length_len;
                let end = offset.checked_add(length as usize)?;
                if end > message.len() {
                    return None;
                }
                if field == target_field {
                    return Some(&message[offset..end]);
                }
                offset = end;
            }
            5 => offset = offset.checked_add(4)?,
            _ => return None,
        }
        if offset > message.len() {
            return None;
        }
    }
    None
}

fn protobuf_fixed64_field(message: &[u8], target_field: u64) -> Option<u64> {
    let mut offset = 0usize;
    while offset < message.len() {
        let (tag, tag_len) = decode_varint(&message[offset..])?;
        offset += tag_len;
        let field = tag >> 3;
        match tag & 0x07 {
            0 => {
                let (_, value_len) = decode_varint(&message[offset..])?;
                offset += value_len;
            }
            1 => {
                let end = offset.checked_add(8)?;
                let value = u64::from_le_bytes(message.get(offset..end)?.try_into().ok()?);
                if field == target_field {
                    return Some(value);
                }
                offset = end;
            }
            2 => {
                let (length, length_len) = decode_varint(&message[offset..])?;
                offset += length_len;
                offset = offset.checked_add(length as usize)?;
            }
            5 => offset = offset.checked_add(4)?,
            _ => return None,
        }
        if offset > message.len() {
            return None;
        }
    }
    None
}

fn zigzag_i16(value: i16) -> u64 {
    let value = i32::from(value);
    ((value << 1) ^ (value >> 31)) as u32 as u64
}

fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
    while value > 0x7f {
        output.push(((value as u8) & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn decode_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().copied().take(10).enumerate() {
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
    }
    None
}

pub fn build_ucd2_frame(group: u8, code: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(12 + payload.len() + 4);

    frame.extend_from_slice(b"UCD2");
    frame.push(0x01);
    frame.push(0x0c);
    frame.push(group);
    frame.push(code);
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(payload);

    let checksum = ucd2_checksum(&frame);
    frame.extend_from_slice(&checksum.to_le_bytes());

    frame
}

pub fn build_internal_packet(command_id: u16, method_id: u8, body: &[u8]) -> Vec<u8> {
    build_internal_packet_with_request_id(command_id, method_id, 0x8000_0001, body)
}

pub fn build_internal_packet_with_request_id(
    command_id: u16,
    method_id: u8,
    request_id: u32,
    body: &[u8],
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(9 + body.len());

    payload.extend_from_slice(&command_id.to_le_bytes());
    payload.push(method_id);
    payload.extend_from_slice(&request_id.to_le_bytes());
    payload.extend_from_slice(&[0x00, 0x00]);
    payload.extend_from_slice(body);

    payload
}

pub fn ucd2_checksum(bytes: &[u8]) -> u32 {
    let table = ucd2_checksum_table();
    let mut crc = 0xffff_ffffu32;

    for byte in bytes {
        crc ^= u32::from(*byte);

        for _ in 0..4 {
            let index = (crc >> 24) as usize;
            crc = (crc << 8) ^ table[index];
        }
    }

    crc
}

fn ucd2_checksum_table() -> [u32; 256] {
    let mut table = [0u32; 256];

    for (index, item) in table.iter_mut().enumerate() {
        let mut value = (index as u32) << 24;

        for _ in 0..8 {
            if value & 0x8000_0000 != 0 {
                value = (value << 1) ^ 0x04c1_1db7;
            } else {
                value <<= 1;
            }
        }

        *item = value;
    }

    table
}

pub fn check_status(host: &str, touch_control: bool) -> LunaStatus {
    let http_ok = TcpStream::connect((host, 80)).is_ok();

    let control_ok = if touch_control {
        TcpStream::connect((host, 6666)).is_ok()
    } else {
        false
    };

    let message = match (http_ok, control_ok) {
        (true, true) => "Detected Luna HTTP and UCD2 control ports".to_string(),

        (true, false) if touch_control => {
            "HTTP reachable, but UCD2 control port 6666 is unavailable".to_string()
        }

        (true, false) => {
            "HTTP reachable. Control port not touched; use List media to open UCD2 session."
                .to_string()
        }

        (false, true) => "UCD2 control reachable, but HTTP port 80 is unavailable".to_string(),

        (false, false) => "Camera not reachable on ports 80/6666".to_string(),
    };

    LunaStatus {
        host: host.to_string(),

        http_ok,

        control_ok,

        message,
    }
}

pub fn apk_auth_probe(host: &str) -> anyhow::Result<Ucd2ProbeResult> {
    let mut stream = TcpStream::connect((host, 6666))
        .with_context(|| format!("failed to connect APK-derived UCD2 port {host}:6666"))?;

    stream.set_read_timeout(Some(Duration::from_millis(250)))?;

    stream.set_write_timeout(Some(Duration::from_secs(3)))?;

    let mut result = Ucd2ProbeResult {

        host: host.to_string(),

        sent_packets: Vec::new(),

        received_packets: Vec::new(),

        sent_frames: Vec::new(),

        received_frames: Vec::new(),

        notes: vec![

            "APK evidence: z03 basic/business.json has ucd2=true".to_string(),

            "APK evidence: classes.dex contains UCD2, UCD2-ENCRYPT, UCD2-XOR-KEY-001".to_string(),

            "APK evidence: this probe only sends the two captured UCD2 auth packets already present in this project from APK analysis".to_string(),

        ],

    };

    read_available_hex(
        &mut stream,
        &mut result.received_packets,
        &mut result.received_frames,
    )?;

    for payload in AUTH_PAYLOADS {
        stream.write_all(payload)?;

        stream.flush()?;

        result.sent_packets.push(bytes_to_hex(payload));

        result.sent_frames.extend(parse_ucd2_frames(payload));

        std::thread::sleep(Duration::from_millis(80));

        read_available_hex(
            &mut stream,
            &mut result.received_packets,
            &mut result.received_frames,
        )?;
    }

    Ok(result)
}

pub fn raw_ucd2_probe(host: &str, packets: &[Vec<u8>]) -> anyhow::Result<Ucd2ProbeResult> {
    let mut stream = TcpStream::connect((host, 6666))
        .with_context(|| format!("failed to connect APK-derived UCD2 port {host}:6666"))?;

    stream.set_read_timeout(Some(Duration::from_millis(250)))?;

    stream.set_write_timeout(Some(Duration::from_secs(3)))?;

    let mut result = Ucd2ProbeResult {
        host: host.to_string(),

        sent_packets: Vec::new(),

        received_packets: Vec::new(),

        sent_frames: Vec::new(),

        received_frames: Vec::new(),

        notes: vec![
            "Raw UCD2 probe. Use only packets derived from APK or captured from the official app."
                .to_string(),
        ],
    };

    read_available_hex(
        &mut stream,
        &mut result.received_packets,
        &mut result.received_frames,
    )?;

    for payload in packets {
        stream.write_all(payload)?;

        stream.flush()?;

        result.sent_packets.push(bytes_to_hex(payload));

        result.sent_frames.extend(parse_ucd2_frames(payload));

        std::thread::sleep(Duration::from_millis(100));

        read_available_hex(
            &mut stream,
            &mut result.received_packets,
            &mut result.received_frames,
        )?;
    }

    Ok(result)
}

fn read_available_hex(
    stream: &mut TcpStream,

    out: &mut Vec<String>,

    frames: &mut Vec<Ucd2Frame>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,

            Ok(n) => {
                let chunk = &buf[..n];

                out.push(bytes_to_hex(chunk));

                frames.extend(parse_ucd2_frames(chunk));
            }

            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }

            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

fn parse_ucd2_frames(bytes: &[u8]) -> Vec<Ucd2Frame> {
    let mut frames = Vec::new();

    let mut offset = 0;

    while offset + 16 <= bytes.len() {
        let Some(relative_magic) = bytes[offset..]
            .windows(4)
            .position(|window| window == b"UCD2")
        else {
            break;
        };

        offset += relative_magic;

        if offset + 16 > bytes.len() {
            break;
        }

        let payload_len = u32::from_le_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
        ]);

        let total_len = 12usize
            .saturating_add(payload_len as usize)
            .saturating_add(4);

        if offset + total_len > bytes.len() {
            break;
        }

        let frame = &bytes[offset..offset + total_len];

        let payload = &bytes[offset + 12..offset + 12 + payload_len as usize];

        let tail = &bytes[offset + 12 + payload_len as usize..offset + total_len];

        frames.push(Ucd2Frame {
            offset,

            frame_hex: bytes_to_hex(frame),

            version: bytes[offset + 4],

            header_len: bytes[offset + 5],

            message_type: format!("{:02x} {:02x}", bytes[offset + 6], bytes[offset + 7]),

            message_hint: message_hint(bytes[offset + 6], bytes[offset + 7], payload_len),

            payload_len,

            payload_hex: bytes_to_hex(payload),

            payload_ascii: printable_ascii(payload),

            payload_strings: printable_strings(payload),

            tail_hex: bytes_to_hex(tail),
        });

        offset += total_len;
    }

    frames
}

fn message_hint(group: u8, code: u8, payload_len: u32) -> String {
    match (group, code, payload_len) {
        (0x04, 0xf2, _) => "\u{8bbe}\u{5907}\u{4e3b}\u{52a8}\u{901a}\u{77e5}/\u{72b6}\u{6001}\u{5019}\u{9009}\u{ff1a}\u{63e1}\u{624b}\u{540e}\u{53ef}\u{80fd}\u{7531}\u{76f8}\u{673a}\u{63a8}\u{9001}".to_string(),
        (0x04, 0x10, _) => "\u{8bbe}\u{5907}\u{4fe1}\u{606f}\u{8bf7}\u{6c42}\u{5019}\u{9009}\u{ff1a}\u{5df2}\u{89c2}\u{5bdf}\u{5230}\u{8fd4}\u{56de} 04 xx \u{52a8}\u{6001}\u{5e8f}\u{53f7}\u{54cd}\u{5e94}".to_string(),
        (0x04, _, len) if len >= 40 => format!(
            "\u{8bbe}\u{5907}\u{4fe1}\u{606f}\u{54cd}\u{5e94}\u{5019}\u{9009}\u{ff1a}04 10 \u{8bf7}\u{6c42}\u{540e}\u{8fd4}\u{56de}\u{ff1b}\u{7b2c}\u{4e8c}\u{5b57}\u{8282}\u{50cf}\u{4f1a}\u{8bdd}\u{5e8f}\u{53f7}/\u{54cd}\u{5e94}\u{5e8f}\u{53f7} {}\u{ff1b}payload \u{5185}\u{542b}\u{5e8f}\u{5217}\u{53f7}/\u{8bbe}\u{5907}\u{540d}",
            code
        ),
        (0x05, _, 0) => format!(
            "\u{7a7a} payload \u{4f1a}\u{8bdd}\u{63a7}\u{5236}\u{5e27}\u{5019}\u{9009}\u{ff1a}\u{5df2}\u{89c2}\u{5bdf}\u{5230} 05 01/03/05..\u{ff0c}\u{53ef}\u{80fd}\u{662f} ACK\u{3001}\u{5fc3}\u{8df3}\u{6216}\u{5e8f}\u{53f7} {}",
            code
        ),
        _ => "\u{672a}\u{77e5} UCD2 \u{5e27}".to_string(),
    }
}

fn printable_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            }
        })
        .collect()
}

fn printable_strings(bytes: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();

    let mut current = Vec::new();

    for byte in bytes {
        if byte.is_ascii_graphic() || *byte == b' ' {
            current.push(*byte);
        } else {
            if current.len() >= 4 {
                strings.push(String::from_utf8_lossy(&current).trim().to_string());
            }

            current.clear();
        }
    }

    if current.len() >= 4 {
        strings.push(String::from_utf8_lossy(&current).trim().to_string());
    }

    strings.retain(|item| !item.is_empty());

    strings
}

pub fn parse_hex_packets(input: &str) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut packets = Vec::new();

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let clean: String = line
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace() && *ch != '-' && *ch != ':')
            .collect();

        if clean.len() % 2 != 0 {
            anyhow::bail!("hex packet has odd length: {line}");
        }

        let mut packet = Vec::with_capacity(clean.len() / 2);

        for idx in (0..clean.len()).step_by(2) {
            let byte = u8::from_str_radix(&clean[idx..idx + 2], 16)
                .with_context(|| format!("invalid hex byte in packet: {line}"))?;

            packet.push(byte);
        }

        packets.push(packet);
    }

    Ok(packets)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        CAMERA_CLIENT_REGISTER_COMMAND, CAMERA_EVENT_SUBSCRIBE_BODY,
        CAMERA_EVENT_SUBSCRIBE_COMMAND, CAMERA_FULL_STATUS_QUERY_BODY, CAMERA_SESSION_INIT_COMMAND,
        CAMERA_START_RECORD_BODY, CAMERA_START_RECORD_COMMAND, CAMERA_STATUS_QUERY_COMMAND,
        CAMERA_STOP_RECORD_BODY, CAMERA_STOP_RECORD_COMMAND, CAMERA_TAKE_PHOTO_BODY,
        CAMERA_TAKE_PHOTO_COMMAND, CAMERA_VIDEO_FORMATS, CameraCaptureMode, INTERNAL_STORAGE,
        LunaAuthSession, SDCARD_STORAGE, STOP_CAPTURE_APK_SEQUENCE_4121_BODY,
        STOP_CAPTURE_APK_SEQUENCE_A03F_BODY, STOP_CAPTURE_APK_SEQUENCE_BASE_BODY,
        STOP_CAPTURE_APK_WRAPPED_SEQUENCE_A03F_BODY, STOP_CAPTURE_COMMAND199_EMPTY_BODY,
        STOP_CAPTURE_COMMAND199_SELECTOR_BODY, STOP_CAPTURE_FULL_A03F_9D58_BODY,
        STOP_CAPTURE_FULL_A03F_BODY, STOP_CAPTURE_FULL_BASE_9D58_BODY, STOP_CAPTURE_FULL_BASE_BODY,
        build_camera_client_registration_body, build_camera_time_sync_body,
        build_capture_combined_context_body, build_capture_context_body, build_capture_detail_body,
        build_capture_mode_body, build_capture_settings_query_body, build_delete_files_body,
        build_file_list_body, build_gimbal_move_body, build_gimbal_speed_body,
        build_internal_packet, build_internal_packet_with_request_id, build_ucd2_frame,
        build_video_profile_body, build_zoom_body, camera_file_from_path, camera_path_from_url,
        capture_mode_from_event_body, capture_mode_from_full_status_body, encode_varint,
        extract_complete_ucd2_frames, gimbal_device_axes_from_ui, gimbal_speed_from_event_body,
        is_hevc_keyframe, parse_camera_subdirs, parse_file_list_paths, parse_index,
        recording_state_from_event_body, resolve_camera_video_profile, ucd2_checksum,
        zoom_from_capture_settings_body,
    };

    fn hex_bytes(input: &str) -> Vec<u8> {
        input
            .split_whitespace()
            .map(|part| u8::from_str_radix(part, 16).expect("valid hex test byte"))
            .collect()
    }

    #[test]
    fn recovered_ucd2_checksum_matches_empty_control_frame() {
        let header = [
            0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x05, 0x0f, 0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(
            ucd2_checksum(&header).to_le_bytes(),
            [0x37, 0x05, 0x47, 0x7c]
        );
        assert_eq!(
            build_ucd2_frame(0x05, 0x0f, &[]),
            [
                0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x05, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x37, 0x05,
                0x47, 0x7c,
            ]
        );
        assert_eq!(
            build_ucd2_frame(0x05, 0x11, &[]),
            [
                0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x05, 0x11, 0x00, 0x00, 0x00, 0x00, 0x76, 0x20,
                0xc6, 0xcb,
            ]
        );
    }

    #[test]
    fn recovered_ucd2_checksum_matches_device_info_frame() {
        let payload = [
            0x08, 0x00, 0x02, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0x08, 0x30, 0x08, 0x0f, 0x08,
            0x0b,
        ];

        assert_eq!(
            build_ucd2_frame(0x04, 0x10, &payload),
            [
                0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x04, 0x10, 0x0f, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x02, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0x08, 0x30, 0x08, 0x0f, 0x08, 0x0b, 0x7c,
                0x00, 0x8e, 0x7c,
            ]
        );
    }

    #[test]
    fn apk_and_pcap_luna_capture_modes_are_reproducible() {
        assert_eq!(
            build_capture_mode_body(CameraCaptureMode::Photo),
            hex_bytes("08 28 12 00")
        );
        assert_eq!(
            build_capture_mode_body(CameraCaptureMode::Video),
            hex_bytes("08 29 12 00")
        );
        assert_eq!(
            build_capture_context_body(CameraCaptureMode::Photo),
            hex_bytes("08 63 10 06")
        );
        assert_eq!(
            build_capture_context_body(CameraCaptureMode::Video),
            hex_bytes("08 63 10 07")
        );
        assert_eq!(
            build_capture_detail_body(CameraCaptureMode::Photo),
            hex_bytes("08 57 10 06")
        );
        assert_eq!(
            build_capture_detail_body(CameraCaptureMode::Video),
            hex_bytes("08 57 10 07")
        );
        assert_eq!(
            build_capture_combined_context_body(CameraCaptureMode::Photo),
            hex_bytes("08 29 08 63 10 06")
        );
        assert_eq!(
            build_capture_combined_context_body(CameraCaptureMode::Video),
            hex_bytes("08 29 08 63 10 07")
        );
    }

    #[test]
    fn captured_shutter_commands_are_reproducible() {
        assert_eq!(CAMERA_TAKE_PHOTO_COMMAND, 0x0003);
        assert_eq!(CAMERA_TAKE_PHOTO_BODY, hex_bytes("30 03"));
        assert_eq!(CAMERA_START_RECORD_COMMAND, 0x0004);
        assert_eq!(CAMERA_START_RECORD_BODY, hex_bytes("08 01"));
        assert_eq!(CAMERA_STOP_RECORD_COMMAND, 0x0005);
        assert_eq!(CAMERA_STOP_RECORD_BODY, hex_bytes("10 01"));
    }

    #[test]
    fn ui_gimbal_directions_map_to_device_axes() {
        assert_eq!(gimbal_device_axes_from_ui(0, -72).unwrap(), (72, 0));
        assert_eq!(gimbal_device_axes_from_ui(0, 72).unwrap(), (-72, 0));
        assert_eq!(gimbal_device_axes_from_ui(-72, 0).unwrap(), (0, -72));
        assert_eq!(gimbal_device_axes_from_ui(72, 0).unwrap(), (0, 72));
        assert!(gimbal_device_axes_from_ui(101, 0).is_err());
    }

    #[test]
    fn captured_recording_state_events_are_recognized() {
        assert_eq!(
            recording_state_from_event_body(&hex_bytes("08 01 10 00 38 00")),
            None
        );
        assert_eq!(
            recording_state_from_event_body(&hex_bytes("08 00 10 00 38 00")),
            Some(false)
        );
        assert_eq!(
            recording_state_from_event_body(&hex_bytes("08 01 10 00 38 02")),
            Some(true)
        );
        assert_eq!(recording_state_from_event_body(&hex_bytes("10 01")), None);
    }

    #[test]
    fn captured_camera_event_subscription_is_reproducible() {
        assert_eq!(
            build_internal_packet_with_request_id(
                CAMERA_EVENT_SUBSCRIBE_COMMAND,
                0x02,
                0x8000_0002,
                CAMERA_EVENT_SUBSCRIBE_BODY,
            ),
            hex_bytes("11 00 02 02 00 00 80 00 00 08 01")
        );
    }

    #[test]
    fn captured_camera_session_setup_is_reproducible() {
        assert_eq!(
            build_internal_packet_with_request_id(
                CAMERA_SESSION_INIT_COMMAND,
                0x02,
                0x8000_0002,
                &[],
            ),
            hex_bytes("0f 00 02 02 00 00 80 00 00")
        );
        assert_eq!(CAMERA_FULL_STATUS_QUERY_BODY.len(), 183);
        assert_eq!(
            build_internal_packet_with_request_id(
                CAMERA_STATUS_QUERY_COMMAND,
                0x02,
                0x8000_0003,
                CAMERA_FULL_STATUS_QUERY_BODY,
            ),
            hex_bytes(
                "08 00 02 03 00 00 80 00 00 08 01 08 03 08 02 08 4c 08 06 08 4e 08 4f 08 0b \
                 08 55 08 0c 08 0d 08 af 01 08 0e 08 0f 08 13 08 37 08 11 08 14 08 1e 08 24 \
                 08 6e 08 72 08 75 08 59 08 74 08 73 08 25 08 26 08 2a 08 28 08 29 08 30 \
                 08 31 08 32 08 42 08 84 01 08 3a 08 3b 08 3c 08 43 08 44 08 5d 08 53 08 52 \
                 08 46 08 58 08 67 08 10 08 61 08 85 01 08 86 01 08 77 08 7a 08 7b 08 7c \
                 08 80 01 08 81 01 08 87 01 08 96 01 08 95 01 08 93 01 08 9b 01 08 9d 01 \
                 08 9e 01 08 a0 01 08 b3 01 08 a1 01 08 16 08 50 08 51 08 a7 01 08 a9 01 \
                 08 ad 01 08 b4 01 08 b0 01 08 b1 01 08 78 08 6f 08 79 08 ac 01",
            )
        );
        assert_eq!(
            build_internal_packet_with_request_id(
                CAMERA_CLIENT_REGISTER_COMMAND,
                0x02,
                0x8000_0004,
                &build_camera_client_registration_body(),
            ),
            hex_bytes(
                "27 00 02 04 00 00 80 00 00 0a 24 66 66 66 66 66 66 66 66 2d 64 33 38 39 2d 34 65 33 35 2d 66 66 66 66 2d 66 66 66 66 65 66 30 35 61 63 34 61 10 02"
            )
        );
        assert_eq!(
            build_camera_time_sync_body(1_784_719_725),
            hex_bytes(
                "08 0c 08 0d 08 af 01 12 1a 60 ed d2 82 d3 06 68 80 e1 01 fa 0a 0d 41 73 69 61 2f 53 68 61 6e 67 68 61 69"
            )
        );
    }

    #[test]
    fn captured_gimbal_zigzag_points_are_reproducible() {
        let cases = [
            ((0, 0), "08 01 12 00"),
            ((36, 0), "08 01 12 02 08 48"),
            ((59, 0), "08 01 12 02 08 76"),
            ((90, 10), "08 01 12 05 08 b4 01 10 14"),
            ((98, 13), "08 01 12 05 08 c4 01 10 1a"),
            ((-21, 96), "08 01 12 05 08 29 10 c0 01"),
        ];

        for ((x, y), expected) in cases {
            assert_eq!(
                build_gimbal_move_body(x, y).expect("valid gimbal point"),
                hex_bytes(expected)
            );
        }
        assert!(build_gimbal_move_body(-101, 0).is_err());
        assert!(build_gimbal_move_body(0, 101).is_err());
    }

    #[test]
    fn captured_hardware_gimbal_speed_levels_are_reproducible() {
        assert_eq!(
            build_gimbal_speed_body(2).unwrap(),
            hex_bytes("08 55 12 05 aa 05 02 10 02 18 06")
        );
        assert_eq!(
            build_gimbal_speed_body(1).unwrap(),
            hex_bytes("08 55 12 05 aa 05 02 10 01 18 06")
        );
        assert_eq!(
            build_gimbal_speed_body(3).unwrap(),
            hex_bytes("08 55 12 05 aa 05 02 10 03 18 06")
        );
        assert!(build_gimbal_speed_body(0).is_err());
        assert!(build_gimbal_speed_body(4).is_err());
        assert_eq!(
            gimbal_speed_from_event_body(&hex_bytes("08 00 10 03")),
            Some(3)
        );
        assert_eq!(
            gimbal_speed_from_event_body(&hex_bytes("08 00 10 02")),
            Some(2)
        );
        assert_eq!(
            gimbal_speed_from_event_body(&hex_bytes("08 00 10 01")),
            Some(1)
        );
    }

    #[test]
    fn captured_capture_mode_events_are_decoded() {
        assert_eq!(
            capture_mode_from_event_body(&hex_bytes("08 00 10 64 18 03")),
            Some(CameraCaptureMode::Photo)
        );
        assert_eq!(
            capture_mode_from_event_body(&hex_bytes("08 64 10 00 18 03")),
            Some(CameraCaptureMode::Video)
        );
        assert_eq!(
            capture_mode_from_event_body(&hex_bytes("08 08 10 64 18 01")),
            None
        );
        assert_eq!(
            capture_mode_from_event_body(&hex_bytes("10 64 18 03")),
            None
        );
    }

    #[test]
    fn captured_full_status_reports_current_capture_mode() {
        assert_eq!(
            capture_mode_from_full_status_body(&hex_bytes("08 28 08 29 12 06 c0 02 00 c8 02 64")),
            Some(CameraCaptureMode::Photo)
        );
        assert_eq!(
            capture_mode_from_full_status_body(&hex_bytes("08 28 08 29 12 06 c0 02 64 c8 02 00")),
            Some(CameraCaptureMode::Video)
        );
        assert_eq!(
            capture_mode_from_full_status_body(&hex_bytes("08 28 08 29 12 06 c0 02 01 c8 02 63")),
            None
        );
        assert_eq!(
            capture_mode_from_full_status_body(&hex_bytes("08 28 08 29")),
            None
        );
    }

    #[test]
    fn captured_zoom_options_are_reproducible() {
        assert_eq!(
            build_zoom_body(3.0, CameraCaptureMode::Photo).unwrap(),
            hex_bytes("08 35 12 0a a9 03 00 00 00 00 00 00 08 40 18 06")
        );
        assert_eq!(
            build_zoom_body(1.0, CameraCaptureMode::Video).unwrap(),
            hex_bytes("08 35 12 0a a9 03 00 00 00 00 00 00 f0 3f 18 07")
        );
        assert_eq!(
            build_zoom_body(2.0, CameraCaptureMode::Video).unwrap(),
            hex_bytes("08 35 12 0a a9 03 00 00 00 00 00 00 00 40 18 07")
        );
        assert_eq!(
            build_zoom_body(3.0, CameraCaptureMode::Video).unwrap(),
            hex_bytes("08 35 12 0a a9 03 00 00 00 00 00 00 08 40 18 07")
        );
        assert_eq!(
            build_zoom_body(6.0, CameraCaptureMode::Video).unwrap(),
            hex_bytes("08 35 12 0a a9 03 00 00 00 00 00 00 18 40 18 07")
        );
        assert!(build_zoom_body(4.0, CameraCaptureMode::Video).is_err());
        assert!(build_zoom_body(f64::NAN, CameraCaptureMode::Video).is_err());
    }

    #[test]
    fn captured_zoom_state_query_and_response_are_reproducible() {
        assert_eq!(
            build_capture_settings_query_body(CameraCaptureMode::Photo),
            hex_bytes(
                "08 01 08 02 08 03 08 04 08 05 08 06 08 07 08 08 08 09 08 0a 08 0b 08 0c \
                 08 0d 08 27 08 0e 08 0f 08 12 08 13 08 14 08 15 08 16 08 17 08 18 08 19 \
                 08 1a 08 1b 08 1c 08 1d 08 1e 08 1f 08 28 08 21 08 20 08 22 08 3a 08 3b \
                 08 2b 08 37 08 38 08 23 08 64 08 24 08 25 08 26 08 29 08 63 08 2a 08 2c \
                 08 2d 08 2e 08 33 08 34 08 35 08 36 08 3d 08 3e 08 3f 08 46 08 48 08 49 \
                 08 53 08 4a 08 4b 08 4c 08 54 08 4e 08 4f 08 50 08 4d 08 47 08 51 08 52 \
                 08 56 08 57 08 55 08 58 08 59 08 5a 08 5b 08 62 08 5d 08 5e 08 6b 10 06"
            )
        );
        assert_eq!(
            zoom_from_capture_settings_body(&hex_bytes(
                "08 35 12 0a a9 03 00 00 00 00 00 00 f0 3f"
            )),
            Some(1.0)
        );
        assert_eq!(
            zoom_from_capture_settings_body(&hex_bytes(
                "08 35 12 0a a9 03 00 00 00 00 00 00 04 40"
            )),
            Some(2.5)
        );
        assert_eq!(
            zoom_from_capture_settings_body(&hex_bytes(
                "08 35 12 0a a9 03 00 00 00 00 00 00 f8 7f"
            )),
            None
        );
        assert_eq!(zoom_from_capture_settings_body(&hex_bytes("08 35")), None);
    }

    #[test]
    fn captured_video_profiles_are_reproducible() {
        let hd_60 = resolve_camera_video_profile("1080p_16_9", 60).unwrap();
        assert_eq!(
            build_video_profile_body(hd_60),
            hex_bytes("08 1f 12 03 f8 01 28 18 07")
        );
        let hd_48 = resolve_camera_video_profile("1080p_16_9", 48).unwrap();
        assert_eq!(
            build_video_profile_body(hd_48),
            hex_bytes("08 1f 12 04 f8 01 84 02 18 07")
        );
    }

    #[test]
    fn apk_video_resolution_table_covers_all_normal_luna_profiles() {
        assert_eq!(
            CAMERA_VIDEO_FORMATS
                .iter()
                .map(|format| format.fps_values.len())
                .sum::<usize>(),
            64
        );
        assert_eq!(
            resolve_camera_video_profile("8k_16_9", 30)
                .unwrap()
                .protocol_value,
            154
        );
        assert_eq!(
            resolve_camera_video_profile("4k_2_35_1", 120)
                .unwrap()
                .protocol_value,
            433
        );
        assert_eq!(
            resolve_camera_video_profile("2_7k_9_16", 48)
                .unwrap()
                .protocol_value,
            468
        );
        assert_eq!(
            resolve_camera_video_profile("1080p_16_9", 240)
                .unwrap()
                .protocol_value,
            27
        );
        assert!(resolve_camera_video_profile("8k_16_9", 60).is_none());
        assert!(resolve_camera_video_profile("1080p_9_16", 120).is_none());
        assert!(resolve_camera_video_profile("unknown", 30).is_none());
    }

    #[test]
    fn captured_daily_control_frames_are_reproducible() {
        let cases = [
            (
                0x62,
                0x0001,
                0x8000_002c,
                "10 01 30 28 38 2c 40 01 48 28 50 22",
                "55 43 44 32 01 0c 04 62 15 00 00 00 01 00 02 2c 00 00 80 00 00 10 01 30 28 38 2c 40 01 48 28 50 22 22 8e a7 ba",
            ),
            (
                0x74,
                0x0004,
                0x8000_0034,
                "08 01",
                "55 43 44 32 01 0c 04 74 0b 00 00 00 04 00 02 34 00 00 80 00 00 08 01 91 49 44 a1",
            ),
            (
                0x81,
                0x0005,
                0x8000_003a,
                "10 01",
                "55 43 44 32 01 0c 04 81 0b 00 00 00 05 00 02 3a 00 00 80 00 00 10 01 61 d9 33 28",
            ),
            (
                0xa0,
                0x0003,
                0x8000_0054,
                "30 03",
                "55 43 44 32 01 0c 04 a0 0b 00 00 00 03 00 02 54 00 00 80 00 00 30 03 c8 bf 2d f1",
            ),
            (
                0x2c,
                0x0002,
                0x8000_01ba,
                "",
                "55 43 44 32 01 0c 04 2c 09 00 00 00 02 00 02 ba 01 00 80 00 00 e1 c3 0d 10",
            ),
        ];

        for (sequence, command_id, request_id, body, expected) in cases {
            let payload = build_internal_packet_with_request_id(
                command_id,
                0x02,
                request_id,
                &hex_bytes(body),
            );
            assert_eq!(
                build_ucd2_frame(0x04, sequence, &payload),
                hex_bytes(expected)
            );
        }
    }

    #[test]
    fn preview_stream_helpers_handle_keyframes_and_fragmentation() {
        assert!(is_hevc_keyframe(&hex_bytes("00 00 00 01 40 01 0c 01")));
        assert!(!is_hevc_keyframe(&hex_bytes("00 00 00 01 02 01 d0 0f")));

        let frame = build_ucd2_frame(0x05, 0x44, &[]);
        let mut buffer = frame[..7].to_vec();
        assert!(extract_complete_ucd2_frames(&mut buffer).is_empty());
        buffer.extend_from_slice(&frame[7..]);
        assert_eq!(extract_complete_ucd2_frames(&mut buffer), vec![frame]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn media_index_discovers_camera_directories_and_marks_storage() {
        let root = r#"<a href="../">../</a>
<a href="Camera01/">Camera01/</a> 22-Jul-2026 19:20 -
<a href="Camera02/">Camera02/</a> 22-Jul-2026 19:21 -"#;
        assert_eq!(
            parse_camera_subdirs(root).expect("camera directories"),
            vec!["Camera01", "Camera02"]
        );

        let file_index = r#"<a href="../">../</a>
<a href="IMG_20260722_193028_206.jpg">IMG_20260722_193028_206.jpg</a> 22-Jul-2026 19:30 4.5M"#;
        let internal = parse_index(
            file_index,
            "http://192.168.42.1/storage_internal/DCIM/Camera01/",
            INTERNAL_STORAGE,
        )
        .expect("internal index");
        let sdcard = parse_index(
            file_index,
            "http://192.168.42.1/DCIM/Camera01/",
            SDCARD_STORAGE,
        )
        .expect("sdcard index");

        assert_eq!(internal[0].storage_id, "storage_internal");
        assert_eq!(internal[0].storage_label, "内部存储");
        assert_eq!(sdcard[0].storage_id, "sdcard");
        assert_eq!(sdcard[0].storage_label, "SD 卡");
        assert!(internal[0].url.contains("/storage_internal/DCIM/Camera01/"));
        assert!(sdcard[0].url.contains("/DCIM/Camera01/"));
    }

    #[test]
    fn captured_ucd2_file_list_requests_are_reproducible() {
        let internal = build_file_list_body(2, 2, 0);
        let sdcard = build_file_list_body(2, 3, 0);
        let sdcard_page_two = build_file_list_body(2, 3, 100);
        let secondary_internal = build_file_list_body(3, 2, 0);

        assert_eq!(internal, hex_bytes("08 02 18 64 20 02"));
        assert_eq!(sdcard, hex_bytes("08 02 18 64 20 03"));
        assert_eq!(sdcard_page_two, hex_bytes("08 02 10 64 18 64 20 03"));
        assert_eq!(secondary_internal, hex_bytes("08 03 18 64 20 02"));
        assert_eq!(
            build_internal_packet_with_request_id(0x000d, 0x02, 0x8000_0007, &internal),
            hex_bytes("0d 00 02 07 00 00 80 00 00 08 02 18 64 20 02")
        );
    }

    #[test]
    fn ucd2_file_list_response_builds_daily_media_items() {
        let paths = [
            "/storage_internal/DCIM/Camera01/IMG_20260722_193028_206.jpg",
            "/DCIM/Camera01/VID_20260722_193010_205.mp4",
        ];
        let mut body = Vec::new();
        for path in paths {
            body.push(0x0a);
            encode_varint(path.len() as u64, &mut body);
            body.extend_from_slice(path.as_bytes());
        }
        body.extend_from_slice(&[0x10, 0x00]);

        assert_eq!(parse_file_list_paths(&body).unwrap(), paths);

        let photo = camera_file_from_path("192.168.42.1", paths[0], INTERNAL_STORAGE)
            .unwrap()
            .unwrap();
        assert_eq!(photo.name, "IMG_20260722_193028_206.jpg");
        assert_eq!(photo.date, "2026-07-22");
        assert_eq!(photo.time, "19:30");
        assert_eq!(photo.storage_id, "storage_internal");
        assert_eq!(photo.kind, "JPG");

        let video = camera_file_from_path("192.168.42.1", paths[1], SDCARD_STORAGE)
            .unwrap()
            .unwrap();
        assert_eq!(video.storage_id, "sdcard");
        assert_eq!(video.storage_label, "SD 卡");
        assert_eq!(video.kind, "MP4");
        assert!(
            video
                .url
                .ends_with("/DCIM/Camera01/VID_20260722_193010_205.mp4")
        );
    }

    #[test]
    #[ignore = "connects to the physical Luna Ultra and reads its UCD2 media list"]
    fn connected_camera_lists_media_via_ucd2() {
        let mut session = LunaAuthSession::open("192.168.42.1").expect("open Luna media session");
        let files = session
            .list_files_for_storage("all")
            .expect("read UCD2 media list");
        let internal = files
            .iter()
            .filter(|file| file.storage_id == "storage_internal")
            .count();
        let sdcard = files
            .iter()
            .filter(|file| file.storage_id == "sdcard")
            .count();
        println!("UCD2 media list: {internal} internal, {sdcard} SD-card files");
    }

    #[test]
    fn delete_body_and_camera_path_validation_match_protocol() {
        let first = "/storage_internal/DCIM/Camera01/IMG_20260715_120000.jpg".to_string();
        let second = "/storage_internal/DCIM/Camera01/视频 01.mp4".to_string();
        let body = build_delete_files_body(&[first.clone(), second.clone()]).expect("delete body");
        assert_eq!(body[0], 0x0a);
        assert_eq!(body[1] as usize, first.len());
        assert_eq!(&body[2..2 + first.len()], first.as_bytes());
        let second_offset = 2 + first.len();
        assert_eq!(body[second_offset], 0x0a);
        assert_eq!(body[second_offset + 1] as usize, second.len());

        let long = format!("/storage_internal/DCIM/Camera01/{}.jpg", "a".repeat(130));
        let long_body = build_delete_files_body(&[long.clone()]).expect("long delete body");
        assert_eq!(&long_body[..3], &[0x0a, 0xa6, 0x01]);
        assert_eq!(&long_body[3..], long.as_bytes());

        assert_eq!(
            camera_path_from_url(
                "192.168.42.1",
                "http://192.168.42.1/DCIM/Camera01/%E8%A7%86%E9%A2%91%2001.mp4",
            )
            .expect("valid encoded camera path"),
            "/DCIM/Camera01/视频 01.mp4"
        );
        assert!(camera_path_from_url("192.168.42.1", "http://example.com/DCIM/a.jpg").is_err());
        assert!(camera_path_from_url("192.168.42.1", "http://192.168.42.1/private/a.jpg").is_err());
        assert!(
            camera_path_from_url(
                "192.168.42.1",
                "http://192.168.42.1/DCIM/Camera01/%2e%2e/private.jpg",
            )
            .is_err()
        );
    }

    #[test]
    fn apk_stop_capture_candidates_match_static_reverse_outputs() {
        let base = build_internal_packet(
            0x0008,
            0x02,
            &[
                0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x05, 0x57, 0xb0,
            ],
        );
        let a03f = build_internal_packet(
            0x0008,
            0x02,
            &[
                0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00, 0x00, 0x09, 0x57, 0x59, 0xb3, 0x00, 0x03,
                0xb0,
            ],
        );

        assert_eq!(
            build_ucd2_frame(0x04, 0x10, &base),
            [
                0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x04, 0x10, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x02, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00,
                0x00, 0x05, 0x57, 0xb0, 0x8b, 0x78, 0x6a, 0x3d,
            ]
        );
        assert_eq!(
            build_ucd2_frame(0x04, 0x10, &a03f),
            [
                0x55, 0x43, 0x44, 0x32, 0x01, 0x0c, 0x04, 0x10, 0x18, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x02, 0x01, 0x00, 0x00, 0x80, 0x00, 0x00, 0xb2, 0x00, 0x03, 0x59, 0xc7, 0x00, 0x00,
                0x00, 0x09, 0x57, 0x59, 0xb3, 0x00, 0x03, 0xb0, 0x74, 0xef, 0x99, 0x27,
            ]
        );
    }

    #[test]
    fn apk_stop_capture_full_node_candidates_match_static_reverse_outputs() {
        let cases = [
            (
                STOP_CAPTURE_FULL_BASE_BODY,
                "55 43 44 32 01 0c 04 10 38 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 87 0a 10 6c",
            ),
            (
                STOP_CAPTURE_FULL_A03F_BODY,
                "55 43 44 32 01 0c 04 10 3c 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 e8 78 8f 62",
            ),
            (
                STOP_CAPTURE_FULL_BASE_9D58_BODY,
                "55 43 44 32 01 0c 04 10 42 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 1f 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 01 00 05 00 00 00 04 00 01 fb 0a 00 06 00 00 00 00 00 07 00 00 00 00 36 2a f7 ef",
            ),
            (
                STOP_CAPTURE_FULL_A03F_9D58_BODY,
                "55 43 44 32 01 0c 04 10 46 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 23 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 01 00 05 00 00 00 04 00 01 fb 0e 00 06 00 00 00 00 00 07 00 00 00 00 43 b3 04 30",
            ),
        ];

        for (body, expected) in cases {
            let payload = build_internal_packet(0x0008, 0x02, body);
            assert_eq!(build_ucd2_frame(0x04, 0x10, &payload), hex_bytes(expected));
        }
    }

    #[test]
    fn apk_stop_capture_sequence_candidates_match_static_reverse_outputs() {
        let cases = [
            (
                STOP_CAPTURE_APK_SEQUENCE_BASE_BODY,
                "55 43 44 32 01 0c 04 10 48 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 06 59 c7 00 00 00 05 57 b0 00 00 00 08 00 00 00 00 00 03 00 00 00 00 fd 21 04 26",
            ),
            (
                STOP_CAPTURE_APK_SEQUENCE_A03F_BODY,
                "55 43 44 32 01 0c 04 10 4c 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 06 59 c7 00 00 00 09 57 59 b3 00 06 b0 00 00 00 08 00 00 00 00 00 03 00 00 00 00 21 c5 87 ae",
            ),
            (
                STOP_CAPTURE_APK_SEQUENCE_4121_BODY,
                "55 43 44 32 01 0c 04 10 56 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 19 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 23 00 02 00 00 00 00 00 0f b2 00 06 59 c7 00 00 00 09 57 59 b3 00 06 b0 00 01 00 08 00 00 00 04 00 01 fb 0e 00 09 00 00 00 00 00 0a 00 00 00 00 ca 0c 32 2e",
            ),
            (
                STOP_CAPTURE_APK_WRAPPED_SEQUENCE_A03F_BODY,
                "55 43 44 32 01 0c 04 10 5a 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 09 57 59 b3 00 08 b0 00 00 00 0a 00 00 00 00 00 03 00 00 00 00 49 57 f5 6d",
            ),
        ];

        for (body, expected) in cases {
            let payload = build_internal_packet(0x0008, 0x02, body);
            assert_eq!(build_ucd2_frame(0x04, 0x10, &payload), hex_bytes(expected));
        }
    }

    #[test]
    fn apk_stop_capture_command199_candidates_match_static_reverse_outputs() {
        let cases = [
            (
                STOP_CAPTURE_COMMAND199_EMPTY_BODY,
                "55 43 44 32 01 0c 04 10 09 00 00 00 c7 00 02 01 00 00 80 00 00 16 85 b7 5a",
            ),
            (
                STOP_CAPTURE_COMMAND199_SELECTOR_BODY,
                "55 43 44 32 01 0c 04 10 18 00 00 00 c7 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 76 88 5c 4b",
            ),
        ];

        for (body, expected) in cases {
            let payload = build_internal_packet(0x00c7, 0x02, body);
            assert_eq!(build_ucd2_frame(0x04, 0x10, &payload), hex_bytes(expected));
        }
    }
}

#[allow(dead_code)]

pub fn list_files(host: &str) -> anyhow::Result<Vec<LunaFile>> {
    let mut auth = LunaAuthSession::open(host)?;

    list_files_with_session(host, &mut auth)
}

pub fn list_files_with_session(
    host: &str,

    auth: &mut LunaAuthSession,
) -> anyhow::Result<Vec<LunaFile>> {
    if auth.host() != host {
        anyhow::bail!("媒体会话与目标相机不一致");
    }
    auth.list_files_for_storage("storage_internal")
}

pub fn list_files_authenticated(host: &str) -> anyhow::Result<Vec<LunaFile>> {
    let mut auth = LunaAuthSession::open(host)?;
    auth.list_files_for_storage("storage_internal")
}

pub fn list_files_authenticated_for_storage(
    host: &str,
    storage_id: &str,
) -> anyhow::Result<Vec<LunaFile>> {
    let mut auth = LunaAuthSession::open(host)?;
    auth.list_files_for_storage(storage_id)
}

fn list_files_via_ucd2<F>(
    host: &str,
    storage_id: &str,
    mut execute: F,
) -> anyhow::Result<Vec<LunaFile>>
where
    F: FnMut(&[u8]) -> anyhow::Result<CameraControlResponse>,
{
    let storages = match storage_id {
        "all" => vec![INTERNAL_STORAGE, SDCARD_STORAGE],
        "storage_internal" | "internal" => vec![INTERNAL_STORAGE],
        "sdcard" | "sd" => vec![SDCARD_STORAGE],
        other => anyhow::bail!("未知存储位置：{other}"),
    };
    let mut files = Vec::new();
    let mut failures = Vec::new();
    let mut successful_storages = 0usize;

    for storage in storages {
        match query_camera_file_paths(storage, &mut execute).and_then(|paths| {
            let mut storage_files = Vec::new();
            for path in paths {
                if let Some(file) = camera_file_from_path(host, &path, storage)? {
                    storage_files.push(file);
                }
            }
            Ok(storage_files)
        }) {
            Ok(mut storage_files) => {
                successful_storages += 1;
                files.append(&mut storage_files);
            }
            Err(error) => failures.push(format!("{}：{error}", storage.label)),
        }
    }

    if successful_storages == 0 && !failures.is_empty() {
        anyhow::bail!(failures.join("；"));
    }

    let mut seen = HashSet::new();
    files.retain(|file| seen.insert(file.url.clone()));
    Ok(files)
}

fn query_camera_file_paths<F>(storage: MediaStorage, execute: &mut F) -> anyhow::Result<Vec<String>>
where
    F: FnMut(&[u8]) -> anyhow::Result<CameraControlResponse>,
{
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for category in [2u8, 3u8] {
        for page in 0..CAMERA_FILE_LIST_MAX_PAGES {
            let offset = page * CAMERA_FILE_LIST_PAGE_SIZE;
            let body = build_file_list_body(category, storage.selector, offset);
            let response = execute(&body)?;
            let page_paths = parse_file_list_paths(&response.body)?;
            let page_count = page_paths.len() as u32;
            let mut added = 0usize;
            for path in page_paths {
                if seen.insert(path.clone()) {
                    paths.push(path);
                    added += 1;
                }
            }
            if page_count < CAMERA_FILE_LIST_PAGE_SIZE || added == 0 {
                break;
            }
        }
    }
    Ok(paths)
}

fn build_file_list_body(category: u8, storage_selector: u8, offset: u32) -> Vec<u8> {
    let mut body = Vec::with_capacity(12);
    body.push(0x08);
    encode_varint(category as u64, &mut body);
    if offset > 0 {
        body.push(0x10);
        encode_varint(offset as u64, &mut body);
    }
    body.push(0x18);
    encode_varint(CAMERA_FILE_LIST_PAGE_SIZE as u64, &mut body);
    body.push(0x20);
    encode_varint(storage_selector as u64, &mut body);
    body
}

fn parse_file_list_paths(body: &[u8]) -> anyhow::Result<Vec<String>> {
    let mut offset = 0usize;
    let mut paths = Vec::new();
    while offset < body.len() {
        let (tag, tag_len) =
            decode_varint(&body[offset..]).ok_or_else(|| anyhow!("相册列表字段不完整"))?;
        offset += tag_len;
        match tag & 0x07 {
            0 => {
                let (_, value_len) = decode_varint(&body[offset..])
                    .ok_or_else(|| anyhow!("相册列表数值字段不完整"))?;
                offset += value_len;
            }
            1 => {
                offset = offset
                    .checked_add(8)
                    .filter(|end| *end <= body.len())
                    .ok_or_else(|| anyhow!("相册列表固定字段越界"))?;
            }
            2 => {
                let (length, length_len) = decode_varint(&body[offset..])
                    .ok_or_else(|| anyhow!("相册列表字符串长度不完整"))?;
                offset += length_len;
                let end = offset
                    .checked_add(length as usize)
                    .filter(|end| *end <= body.len())
                    .ok_or_else(|| anyhow!("相册列表字符串越界"))?;
                if tag >> 3 == 1 {
                    let path = std::str::from_utf8(&body[offset..end])
                        .context("相册文件路径不是 UTF-8")?
                        .to_string();
                    paths.push(path);
                }
                offset = end;
            }
            5 => {
                offset = offset
                    .checked_add(4)
                    .filter(|end| *end <= body.len())
                    .ok_or_else(|| anyhow!("相册列表固定字段越界"))?;
            }
            _ => anyhow::bail!("相册列表包含不支持的 protobuf 字段"),
        }
    }
    Ok(paths)
}

fn camera_file_from_path(
    host: &str,
    path: &str,
    storage: MediaStorage,
) -> anyhow::Result<Option<LunaFile>> {
    let belongs_to_storage = if storage.id == INTERNAL_STORAGE.id {
        path.starts_with(INTERNAL_MEDIA_ROOT)
    } else {
        path.starts_with(SDCARD_MEDIA_ROOT) || path.starts_with("/sdcard/DCIM/")
    };
    if !belongs_to_storage || path.ends_with('/') {
        return Ok(None);
    }

    let Some(name) = path.rsplit('/').next().filter(|name| !name.is_empty()) else {
        return Ok(None);
    };
    let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "mp4" | "mov" | "lrv" | "jpg" | "jpeg" | "dng" | "insp" | "insv" | "png" | "webp"
    ) {
        return Ok(None);
    }

    let mut url =
        reqwest::Url::parse(&format!("http://{host}")).context("相机地址无效，请检查连接设置")?;
    url.set_path(path);
    let (date, time) = media_timestamp_from_name(name);
    Ok(Some(LunaFile {
        name: name.to_string(),
        url: url.to_string(),
        date,
        time,
        size_text: "大小未知".to_string(),
        bytes: None,
        kind: file_kind(name),
        storage_id: storage.id.to_string(),
        storage_label: storage.label.to_string(),
    }))
}

fn media_timestamp_from_name(name: &str) -> (String, String) {
    let mut parts = name.split('_');
    let _prefix = parts.next();
    let date = parts.next().unwrap_or("");
    let time = parts.next().unwrap_or("");
    let date_bytes = date.as_bytes();
    let time_bytes = time.as_bytes();
    if date.len() == 8
        && time.len() >= 6
        && date_bytes.iter().all(u8::is_ascii_digit)
        && time_bytes[..6].iter().all(u8::is_ascii_digit)
    {
        return (
            format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]),
            format!("{}:{}", &time[..2], &time[2..4]),
        );
    }
    ("日期未知".to_string(), String::new())
}

pub fn resume_download(host: &str, file_url: &str, output: &Path) -> anyhow::Result<()> {
    let mut auth = LunaAuthSession::open(host)?;

    auth.refresh()?;

    resume_download_authenticated(file_url, output)
}

pub fn resume_download_with_session(
    auth: &mut LunaAuthSession,
    file_url: &str,
    output: &Path,
) -> anyhow::Result<()> {
    auth.refresh()?;

    resume_download_authenticated(file_url, output)
}

pub fn resume_download_authenticated(file_url: &str, output: &Path) -> anyhow::Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let partial = output.with_extension(format!(
        "{}part",
        output.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));

    let existing = partial.metadata().map(|m| m.len()).unwrap_or(0);

    let mut req = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .build()?
        .get(file_url)
        .header(USER_AGENT, "Luna Mic Control NG/0.2")
        .header(ACCEPT_ENCODING, "identity");

    if existing > 0 {
        req = req.header(RANGE, format!("bytes={existing}-"));
    }

    let mut response = req.send()?.error_for_status()?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&partial)?;

    std::io::copy(&mut response, &mut file)?;

    fs::rename(partial, output)?;

    Ok(())
}

fn parse_camera_subdirs(html: &str) -> anyhow::Result<Vec<String>> {
    let link_re = Regex::new(r#"(?i)<a\s+href="(?P<href>[^"]+)""#)?;
    let camera_re = Regex::new(r"(?i)^Camera\d+/$")?;
    let mut directories = Vec::new();

    for capture in link_re.captures_iter(html) {
        let href = html_unescape(&capture["href"]);
        if camera_re.is_match(&href) {
            directories.push(href.trim_end_matches('/').to_string());
        }
    }

    directories.sort();
    directories.dedup();
    Ok(directories)
}

fn parse_index(html: &str, base_url: &str, storage: MediaStorage) -> anyhow::Result<Vec<LunaFile>> {
    let re = Regex::new(
        r#"<a\s+href="(?P<href>[^"]+)">(?P<name>[^<]+)</a>\s+(?P<date>\d{2}-[A-Za-z]{3}-\d{4})\s+(?P<time>\d{2}:\d{2})\s+(?P<size>\S+)"#,
    )?;

    let mut files = Vec::new();

    for cap in re.captures_iter(html) {
        let href = html_unescape(&cap["href"]);

        let name = html_unescape(&cap["name"]);

        if href == "../" || name == "../" || href.ends_with('/') {
            continue;
        }

        let url = format!("{}{}", base_url.trim_end_matches('/'), format!("/{href}"));

        let size_text = cap["size"].to_string();

        files.push(LunaFile {
            kind: file_kind(&name),

            storage_id: storage.id.to_string(),

            storage_label: storage.label.to_string(),

            name,

            url,

            date: cap["date"].to_string(),

            time: cap["time"].to_string(),

            bytes: parse_size(&size_text),

            size_text,
        });
    }

    if files.is_empty() && html.contains("401") {
        return Err(anyhow!("camera returned unauthorized directory page"));
    }

    Ok(files)
}

fn parse_size(text: &str) -> Option<u64> {
    let re = Regex::new(r"(?i)^(?P<num>\d+(?:\.\d+)?)(?P<unit>[KMG])?$").ok()?;

    let cap = re.captures(text.trim())?;

    let number: f64 = cap.name("num")?.as_str().parse().ok()?;

    let mul = match cap
        .name("unit")
        .map(|m| m.as_str().to_ascii_uppercase())
        .as_deref()
    {
        Some("K") => 1024.0,

        Some("M") => 1024.0 * 1024.0,

        Some("G") => 1024.0 * 1024.0 * 1024.0,

        _ => 1.0,
    };

    Some((number * mul) as u64)
}

fn file_kind(name: &str) -> String {
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
        .as_str()
    {
        "MP4" => "MP4".to_string(),

        "LRV" => "LRV".to_string(),

        other if !other.is_empty() => other.to_string(),

        _ => "FILE".to_string(),
    }
}

fn html_unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceProfileSummary {
    pub id: String,
    pub display_name: String,
    pub short_name: String,
    pub server_name: Option<String>,
    pub series: Option<String>,
    pub variety: Option<String>,
    pub channels: Vec<String>,
    pub capture_modes: Vec<String>,
    pub lens_modes: Vec<String>,
}

pub fn load_profiles() -> anyhow::Result<BTreeMap<String, Value>> {
    let raw = include_str!("../data/profiles.json");
    serde_json::from_str(raw).context("failed to parse embedded device profiles")
}

pub fn summaries() -> anyhow::Result<Vec<DeviceProfileSummary>> {
    let profiles = load_profiles()?;
    let mut out = Vec::new();
    for (id, p) in profiles {
        let basic = p.get("basic").unwrap_or(&Value::Null);
        let capture = p.get("captureOperation").unwrap_or(&Value::Null);
        out.push(DeviceProfileSummary {
            id: id.clone(),
            display_name: str_value(basic, "display_name")
                .or_else(|| str_value(basic, "device_type"))
                .unwrap_or(id.clone()),
            short_name: str_value(basic, "short_display_name").unwrap_or(id.clone()),
            server_name: str_value(basic, "server_name"),
            series: str_value(basic, "device_series"),
            variety: str_value(basic, "device_variety"),
            channels: str_vec(basic, "connection_channels"),
            capture_modes: str_vec(capture, "camera_enable_highlight_capture_mode_list"),
            lens_modes: str_vec(capture, "camera_lens_mode_list"),
        });
    }
    Ok(out)
}

fn str_value(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(ToOwned::to_owned)
}

fn str_vec(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

use anyhow::Context;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::Manager;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct BleDevice {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BleCharacteristic {
    pub service_uuid: String,
    pub uuid: String,
    pub properties: Vec<String>,
}

pub async fn scan_mic_devices() -> anyhow::Result<Vec<BleDevice>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .into_iter()
        .next()
        .context("no Bluetooth adapter found")?;
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(Duration::from_secs(6)).await;
    let mut out = Vec::new();
    for peripheral in adapter.peripherals().await? {
        let props = peripheral.properties().await?.unwrap_or_default();
        let name = props.local_name.unwrap_or_else(|| "(unnamed)".to_string());
        let lower = name.to_lowercase();
        if lower.contains("mic")
            || lower.contains("trc")
            || lower.contains("insta360")
            || lower.contains("rx")
            || lower.contains("tx")
        {
            out.push(BleDevice {
                name,
                address: props.address.to_string(),
            });
        }
    }
    Ok(out)
}

pub async fn inspect(address: &str) -> anyhow::Result<Vec<BleCharacteristic>> {
    let peripheral = find(address).await?;
    peripheral.connect().await?;
    peripheral.discover_services().await?;
    let mut out = Vec::new();
    for service in peripheral.services() {
        for ch in service.characteristics {
            out.push(BleCharacteristic {
                service_uuid: service.uuid.to_string(),
                uuid: ch.uuid.to_string(),
                properties: ch.properties.iter().map(|p| format!("{p:?}")).collect(),
            });
        }
    }
    let _ = peripheral.disconnect().await;
    Ok(out)
}

pub async fn write_hex(address: &str, characteristic_uuid: &str, hex: &str) -> anyhow::Result<()> {
    let peripheral = find(address).await?;
    peripheral.connect().await?;
    peripheral.discover_services().await?;
    let clean = hex.replace([' ', '-'], "");
    let bytes = hex_to_bytes(&clean)?;
    let target = peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid.to_string().eq_ignore_ascii_case(characteristic_uuid))
        .context("characteristic not found")?;
    peripheral
        .write(&target, &bytes, WriteType::WithResponse)
        .await?;
    let _ = peripheral.disconnect().await;
    Ok(())
}

async fn find(address: &str) -> anyhow::Result<btleplug::platform::Peripheral> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .into_iter()
        .next()
        .context("no Bluetooth adapter found")?;
    adapter.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    for p in adapter.peripherals().await? {
        if let Some(props) = p.properties().await? {
            if props.address.to_string().eq_ignore_ascii_case(address) {
                return Ok(p);
            }
        }
    }
    anyhow::bail!("Bluetooth device not found: {address}")
}

fn hex_to_bytes(hex: &str) -> anyhow::Result<Vec<u8>> {
    if hex.len() % 2 != 0 {
        anyhow::bail!("hex payload length must be even");
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).context("invalid hex payload"))
        .collect()
}

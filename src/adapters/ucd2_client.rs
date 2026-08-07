use anyhow::{Context, anyhow};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use serde_json::{Value, json};

// UCD2 协议常量
const UCD2_MAGIC: [u8; 4] = [0x55, 0x43, 0x44, 0x32]; // "UCD2"
const UCD2_PORT: u16 = 6666;

// UCD2 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Ucd2MessageType {
    // 认证相关
    Auth = 0x0100,
    AuthResp = 0x0101,
    
    // 拍摄模式
    SetSyncCaptureMode = 0x0200,
    GetSyncCaptureMode = 0x0201,
    SetSyncCaptureModeResp = 0x0202,
    GetSyncCaptureModeResp = 0x0203,
    
    // 拍摄控制
    StartCapture = 0x0300,
    StopCapture = 0x0301,
    TakePicture = 0x0302,
    StartCaptureResp = 0x0303,
    StopCaptureResp = 0x0304,
    TakePictureResp = 0x0305,
    
    // 选项设置
    SetOptions = 0x0400,
    GetOptions = 0x0401,
    SetOptionsResp = 0x0402,
    GetOptionsResp = 0x0403,
    
    // 子模式选项
    SetSubmodeOptions = 0x0500,
    GetSubmodeOptions = 0x0501,
    SetSubmodeOptionsResp = 0x0502,
    GetSubmodeOptionsResp = 0x0503,
    
    // 其他
    GetStorage = 0x0600,
    GetBattery = 0x0601,
    GetStorageResp = 0x0602,
    GetBatteryResp = 0x0603,
}

// UCD2 消息头
#[derive(Debug, Clone)]
struct Ucd2Header {
    magic: [u8; 4],
    version: u8,
    msg_type: u16,
    sequence: u32,
    payload_length: u32,
}

impl Ucd2Header {
    fn new(msg_type: Ucd2MessageType, sequence: u32, payload_length: u32) -> Self {
        Self {
            magic: UCD2_MAGIC,
            version: 1,
            msg_type: msg_type as u16,
            sequence,
            payload_length,
        }
    }
    
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.version);
        bytes.extend_from_slice(&self.msg_type.to_le_bytes());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&self.payload_length.to_le_bytes());
        bytes
    }
    
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        if data.len() < 16 {
            return Err(anyhow!("UCD2 header too short"));
        }
        if data[0..4] != UCD2_MAGIC {
            return Err(anyhow!("Invalid UCD2 magic"));
        }
        Ok(Self {
            magic: UCD2_MAGIC,
            version: data[4],
            msg_type: u16::from_le_bytes([data[5], data[6]]),
            sequence: u32::from_le_bytes([data[7], data[8], data[9], data[10]]),
            payload_length: u32::from_le_bytes([data[11], data[12], data[13], data[14]]),
        })
    }
}

// UCD2 客户端
pub struct Ucd2Client {
    host: String,
    stream: Option<TcpStream>,
    sequence: u32,
}

impl Ucd2Client {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            stream: None,
            sequence: 0,
        }
    }
    
    /// 连接到相机
    pub fn connect(&mut self) -> anyhow::Result<()> {
        let stream = TcpStream::connect((&*self.host, UCD2_PORT))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        self.stream = Some(stream);
        self.authenticate()?;
        Ok(())
    }
    
    /// 认证
    fn authenticate(&mut self) -> anyhow::Result<()> {
        // 使用 APK 中的认证格式
        let auth_payload = vec![
            0x55, 0x43, 0x44, 0x32, 0x01, 0x0C, 0x05, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x37, 0x05, 0x47, 0x7C,
        ];
        self.send_raw(&auth_payload)?;
        
        // 读取认证响应
        let _response = self.receive()?;
        Ok(())
    }
    
    /// 发送原始数据
    fn send_raw(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let stream = self.stream.as_mut().context("Not connected")?;
        stream.write_all(data)?;
        stream.flush()?;
        Ok(())
    }
    
    /// 发送 UCD2 消息
    pub fn send_message(&mut self, msg_type: Ucd2MessageType, payload: &[u8]) -> anyhow::Result<()> {
        self.sequence += 1;
        let header = Ucd2Header::new(msg_type, self.sequence, payload.len() as u32);
        let mut message = header.to_bytes();
        message.extend_from_slice(payload);
        self.send_raw(&message)
    }
    
    /// 接收 UCD2 消息
    pub fn receive(&mut self) -> anyhow::Result<(Ucd2Header, Vec<u8>)> {
        let stream = self.stream.as_mut().context("Not connected")?;
        
        // 读取头部
        let mut header_bytes = [0u8; 16];
        stream.read_exact(&mut header_bytes)?;
        let header = Ucd2Header::from_bytes(&header_bytes)?;
        
        // 读取负载
        let mut payload = vec![0u8; header.payload_length as usize];
        if header.payload_length > 0 {
            stream.read_exact(&mut payload)?;
        }
        
        Ok((header, payload))
    }
    
    /// 设置拍摄模式
    pub fn set_capture_mode(&mut self, mode: &str) -> anyhow::Result<Value> {
        // 构建 protobuf 负载
        let payload = self.build_set_capture_mode_payload(mode);
        self.send_message(Ucd2MessageType::SetSyncCaptureMode, &payload)?;
        
        let (header, _response) = self.receive()?;
        match header.msg_type {
            0x0202 => { // SetSyncCaptureModeResp
                Ok(json!({"status": "ok", "mode": mode}))
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 构建设置拍摄模式的 protobuf 负载
    fn build_set_capture_mode_payload(&self, mode: &str) -> Vec<u8> {
        // 简化的 protobuf 编码
        // 字段 1 (mode): string, tag = 0x0A
        let mode_bytes = mode.as_bytes();
        let mut payload = Vec::new();
        payload.push(0x0A); // tag for field 1, type LENGTH_DELIMITED
        payload.push(mode_bytes.len() as u8); // length
        payload.extend_from_slice(mode_bytes);
        payload
    }
    
    /// 开始拍摄
    pub fn start_capture(&mut self) -> anyhow::Result<Value> {
        let payload = vec![]; // 空负载
        self.send_message(Ucd2MessageType::StartCapture, &payload)?;
        
        let (header, _response) = self.receive()?;
        match header.msg_type {
            0x0303 => { // StartCaptureResp
                Ok(json!({"status": "ok", "action": "start_capture"}))
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 停止拍摄
    pub fn stop_capture(&mut self) -> anyhow::Result<Value> {
        let payload = vec![];
        self.send_message(Ucd2MessageType::StopCapture, &payload)?;
        
        let (header, _response) = self.receive()?;
        match header.msg_type {
            0x0304 => { // StopCaptureResp
                Ok(json!({"status": "ok", "action": "stop_capture"}))
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 拍照
    pub fn take_picture(&mut self) -> anyhow::Result<Value> {
        let payload = vec![];
        self.send_message(Ucd2MessageType::TakePicture, &payload)?;
        
        let (header, _response) = self.receive()?;
        match header.msg_type {
            0x0305 => { // TakePictureResp
                Ok(json!({"status": "ok", "action": "take_picture"}))
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 设置选项
    pub fn set_options(&mut self, options: Value) -> anyhow::Result<Value> {
        let payload = self.build_set_options_payload(options);
        self.send_message(Ucd2MessageType::SetOptions, &payload)?;
        
        let (header, _response) = self.receive()?;
        match header.msg_type {
            0x0402 => { // SetOptionsResp
                Ok(json!({"status": "ok", "action": "set_options"}))
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 构建设置选项的 protobuf 负载
    fn build_set_options_payload(&self, options: Value) -> Vec<u8> {
        // 简化的 protobuf 编码
        let mut payload = Vec::new();
        
        if let Some(obj) = options.as_object() {
            for (key, value) in obj {
                // 每个选项是一个字段
                let key_bytes = key.as_bytes();
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                let value_bytes = value_str.as_bytes();
                
                // tag: field 1, type LENGTH_DELIMITED
                payload.push(0x0A);
                // 计算总长度
                let total_len = 2 + key_bytes.len() + 2 + value_bytes.len();
                payload.push(total_len as u8);
                // key
                payload.push(0x0A); // field 1
                payload.push(key_bytes.len() as u8);
                payload.extend_from_slice(key_bytes);
                // value
                payload.push(0x12); // field 2
                payload.push(value_bytes.len() as u8);
                payload.extend_from_slice(value_bytes);
            }
        }
        
        payload
    }
    
    /// 获取选项
    pub fn get_options(&mut self, option_names: &[&str]) -> anyhow::Result<Value> {
        let payload = self.build_get_options_payload(option_names);
        self.send_message(Ucd2MessageType::GetOptions, &payload)?;
        
        let (header, response) = self.receive()?;
        match header.msg_type {
            0x0403 => { // GetOptionsResp
                // 解析响应
                self.parse_get_options_response(&response)
            }
            _ => {
                Err(anyhow!("Unexpected response type: 0x{:04x}", header.msg_type))
            }
        }
    }
    
    /// 构建获取选项的 protobuf 负载
    fn build_get_options_payload(&self, option_names: &[&str]) -> Vec<u8> {
        let mut payload = Vec::new();
        for name in option_names {
            let name_bytes = name.as_bytes();
            payload.push(0x0A); // field 1, type LENGTH_DELIMITED
            payload.push(name_bytes.len() as u8);
            payload.extend_from_slice(name_bytes);
        }
        payload
    }
    
    /// 解析获取选项响应
    fn parse_get_options_response(&self, data: &[u8]) -> anyhow::Result<Value> {
        // 简化解析，实际需要完整的 protobuf 解析
        let mut result = json!({});
        let mut i = 0;
        while i < data.len() {
            if data[i] == 0x0A { // field 1, type LENGTH_DELIMITED
                i += 1;
                if i >= data.len() { break; }
                let len = data[i] as usize;
                i += 1;
                if i + len > data.len() { break; }
                // 解析 key-value 对
                let pair = &data[i..i+len];
                if let Ok(s) = std::str::from_utf8(pair) {
                    if let Some((k, v)) = s.split_once('=') {
                        result[k.trim()] = json!(v.trim());
                    }
                }
                i += len;
            } else {
                i += 1;
            }
        }
        Ok(result)
    }
    
    /// 断开连接
    pub fn disconnect(&mut self) {
        self.stream = None;
    }
}

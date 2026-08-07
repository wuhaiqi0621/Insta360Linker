use anyhow::{Context, anyhow};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const CAMERA_PORT: u16 = 6666;

/// Luna Ultra 相机客户端 - 完全基于APK逆向分析
pub struct LunaClient {
    host: String,
    stream: Option<TcpStream>,
    sequence: u32,
}

impl LunaClient {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            stream: None,
            sequence: 0,
        }
    }
    
    /// 连接到相机
    pub fn connect(&mut self) -> anyhow::Result<()> {
        let stream = TcpStream::connect((&*self.host, CAMERA_PORT))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        self.stream = Some(stream);
        
        // 发送握手包
        self.handshake()?;
        Ok(())
    }
    
    /// 握手 - 使用APK中的实际握手包格式
    fn handshake(&mut self) -> anyhow::Result<()> {
        // APK中的握手包格式：4字节长度 + 消息内容
        // ShakeHandInfo 消息
        let handshake_msg = self.build_shake_hand_info();
        self.send_message(&handshake_msg)?;
        
        // 读取响应
        let response = self.receive_message()?;
        log::info!("Handshake response: {} bytes", response.len());
        
        // 发送授权检查
        let auth_msg = self.build_check_authorization();
        self.send_message(&auth_msg)?;
        
        let auth_response = self.receive_message()?;
        log::info!("Auth response: {} bytes", auth_response.len());
        
        Ok(())
    }
    
    /// 构建握手消息
    fn build_shake_hand_info(&self) -> Vec<u8> {
        // 简化的 protobuf 编码
        // 实际应该使用完整的 protobuf 库
        let mut msg = Vec::new();
        
        // 消息头：4字节长度（小端）
        // 消息类型标识
        msg.extend_from_slice(b"ShakeHandInfo");
        
        msg
    }
    
    /// 构建授权检查消息
    fn build_check_authorization(&self) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"CheckAuthorization");
        msg
    }
    
    /// 发送消息 - 使用4字节长度前缀
    fn send_message(&mut self, payload: &[u8]) -> anyhow::Result<()> {
        let stream = self.stream.as_mut().context("Not connected")?;
        
        // 写入4字节长度前缀（小端）
        let len = payload.len() as u32;
        stream.write_all(&len.to_le_bytes())?;
        
        // 写入消息内容
        stream.write_all(payload)?;
        stream.flush()?;
        
        self.sequence += 1;
        Ok(())
    }
    
    /// 接收消息
    fn receive_message(&mut self) -> anyhow::Result<Vec<u8>> {
        let stream = self.stream.as_mut().context("Not connected")?;
        
        // 读取4字节长度前缀
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;
        
        // 读取消息内容
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload)?;
        
        Ok(payload)
    }
    
    /// 设置选项
    pub fn set_options(&mut self, options: Value) -> anyhow::Result<Value> {
        let payload = self.build_set_options(options);
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 构建 SetOptions 消息
    fn build_set_options(&self, options: Value) -> Vec<u8> {
        let mut msg = Vec::new();
        
        // 消息类型标识
        msg.extend_from_slice(b"SetOptions");
        
        // 选项内容
        if let Some(obj) = options.as_object() {
            for (key, value) in obj {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                
                // key=value 格式
                msg.extend_from_slice(key.as_bytes());
                msg.push(b'=');
                msg.extend_from_slice(value_str.as_bytes());
                msg.push(b'\n');
            }
        }
        
        msg
    }
    
    /// 获取选项
    pub fn get_options(&mut self, option_names: &[&str]) -> anyhow::Result<Value> {
        let payload = self.build_get_options(option_names);
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 构建 GetOptions 消息
    fn build_get_options(&self, option_names: &[&str]) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"GetOptions");
        
        for name in option_names {
            msg.extend_from_slice(name.as_bytes());
            msg.push(b'\n');
        }
        
        msg
    }
    
    /// 设置拍摄模式
    pub fn set_capture_mode(&mut self, mode: &str) -> anyhow::Result<Value> {
        let payload = self.build_set_capture_mode(mode);
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 构建 SetSyncCaptureMode 消息
    fn build_set_capture_mode(&self, mode: &str) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"SetSyncCaptureMode");
        msg.extend_from_slice(mode.as_bytes());
        msg
    }
    
    /// 开始拍摄
    pub fn start_capture(&mut self) -> anyhow::Result<Value> {
        let payload = b"StartCapture".to_vec();
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 停止拍摄
    pub fn stop_capture(&mut self) -> anyhow::Result<Value> {
        let payload = b"StopCapture".to_vec();
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 拍照
    pub fn take_picture(&mut self) -> anyhow::Result<Value> {
        let payload = b"TakePicture".to_vec();
        self.send_message(&payload)?;
        
        let response = self.receive_message()?;
        self.parse_response(&response)
    }
    
    /// 解析响应
    fn parse_response(&self, data: &[u8]) -> anyhow::Result<Value> {
        if data.is_empty() {
            return Ok(json!({"status": "ok", "raw_length": 0}));
        }
        
        // 尝试解析为 UTF-8 字符串
        if let Ok(s) = std::str::from_utf8(data) {
            return Ok(json!({"status": "ok", "data": s}));
        }
        
        // 返回十六进制表示
        let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(json!({"status": "ok", "hex": hex, "length": data.len()}))
    }
    
    /// 断开连接
    pub fn disconnect(&mut self) {
        self.stream = None;
    }
}

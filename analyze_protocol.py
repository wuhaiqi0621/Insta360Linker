# 创建一个Python脚本来抓包分析Luna Ultra通信
import socket
import struct
import time
import json

def analyze_luna_protocol():
    """分析Luna Ultra的实际通信协议"""
    host = "192.168.42.1"
    port = 6666
    
    print(f"尝试连接到 {host}:{port}...")
    
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        sock.connect((host, port))
        print("连接成功！")
        
        # 接收欢迎消息
        try:
            data = sock.recv(1024)
            print(f"收到欢迎消息 ({len(data)} 字节): {data.hex()}")
            if len(data) > 0:
                print(f"  ASCII: {data.decode('ascii', errors='replace')}")
        except socket.timeout:
            print("没有收到欢迎消息")
        
        # 发送探测包
        probe_packets = [
            # UCD2 握手包
            bytes.fromhex("55434432010001000000000100000000"),
            # 另一种可能的握手包
            bytes.fromhex("00000001000000000000000000000000"),
            # 简单的ping
            bytes.fromhex("0000000400000001"),
        ]
        
        for i, packet in enumerate(probe_packets):
            print(f"\n发送探测包 {i+1}: {packet.hex()}")
            sock.send(packet)
            time.sleep(0.5)
            
            try:
                response = sock.recv(1024)
                print(f"收到响应 ({len(response)} 字节): {response.hex()}")
                if len(response) > 20:
                    print(f"  前20字节: {response[:20].hex()}")
            except socket.timeout:
                print("没有收到响应")
        
        sock.close()
        
    except Exception as e:
        print(f"连接失败: {e}")

if __name__ == "__main__":
    analyze_luna_protocol()

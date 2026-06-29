use crate::core::error::AppError;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn read_varint_async(stream: &mut TcpStream) -> Result<i32, AppError> {
    let mut num_read = 0;
    let mut result = 0;
    let mut read = [0u8; 1];
    loop {
        stream.read_exact(&mut read).await?;
        let value = (read[0] & 0x7F) as i32;
        result |= value << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(AppError::Process("VarInt too big".to_string()));
        }
        if (read[0] & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn write_varint_to_buf(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7F) == 0 {
            buf.push(value as u8);
            break;
        }
        buf.push(((value & 0x7F) | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}

fn write_string_to_buf(buf: &mut Vec<u8>, s: &str) {
    write_varint_to_buf(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

async fn write_packet_async(stream: &mut TcpStream, data: &[u8]) -> Result<(), AppError> {
    let mut header = Vec::new();
    write_varint_to_buf(&mut header, data.len() as i32);
    stream.write_all(&header).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn resolve_address_async(
    server_ip: &str,
    server_port: u16,
) -> Result<std::net::SocketAddr, AppError> {
    let addr = format!("{}:{}", server_ip, server_port);
    if let Ok(socket_addr) = addr.parse::<std::net::SocketAddr>() {
        return Ok(socket_addr);
    }
    match tokio::net::lookup_host(&addr).await {
        Ok(mut addrs) => {
            if let Some(socket_addr) = addrs.next() {
                return Ok(socket_addr);
            }
        }
        Err(e) => return Err(AppError::Io(e)),
    }
    Err(AppError::Other(
        "Не удалось разрешить IP-адрес сервера".to_string(),
    ))
}

/// Ping Minecraft server using async TCP protocol to retrieve statistics (players count, motd, ping time)
pub async fn ping_server_async(server_ip: &str, server_port: u16) -> Result<String, AppError> {
    let socket_addr = resolve_address_async(server_ip, server_port).await?;

    let stream_result =
        tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&socket_addr)).await;

    let mut stream = match stream_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(AppError::Io(e)),
        Err(_) => return Err(AppError::Other("Timeout connecting to server".to_string())),
    };

    // Prepare Handshake packet
    let mut handshake = Vec::new();
    handshake.push(0x00); // Packet ID
    write_varint_to_buf(&mut handshake, 340); // Protocol version
    write_string_to_buf(&mut handshake, server_ip);
    handshake.extend_from_slice(&server_port.to_be_bytes());
    write_varint_to_buf(&mut handshake, 1); // Next state: Status

    write_packet_async(&mut stream, &handshake).await?;
    write_packet_async(&mut stream, &[0x00]).await?; // Status request packet (ID = 0x00)

    // Read response with timeout
    let read_result = tokio::time::timeout(Duration::from_secs(3), async {
        let _packet_len = read_varint_async(&mut stream).await?;
        let packet_id = read_varint_async(&mut stream).await?;
        if packet_id != 0x00 {
            return Err(AppError::Process("Неверный ID ответа сервера".to_string()));
        }
        let json_len = read_varint_async(&mut stream).await?;
        let mut json_bytes = vec![0u8; json_len as usize];
        stream.read_exact(&mut json_bytes).await?;

        let json_str = String::from_utf8(json_bytes)
            .map_err(|e| AppError::Process(format!("UTF-8 decoding error: {}", e)))?;
        Ok(json_str)
    })
    .await;

    match read_result {
        Ok(res) => res,
        Err(_) => Err(AppError::Other("Timeout reading from server".to_string())),
    }
}

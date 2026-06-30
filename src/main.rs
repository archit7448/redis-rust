mod resp;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{string, thread};

use crate::resp::RespValue;

struct Entry {
    value: String,
    expires_at: Option<std::time::Instant>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = "127.0.0.1:6379";
    match TcpListener::bind(&port) {
        Ok(listener) => {
            eprintln!("Server listening on port 6379");
            let store = Arc::new(Mutex::new(HashMap::<String, Entry>::new()));
            loop {
                match listener.accept() {
                    Ok((mut stream, address)) => {
                        eprintln!("{}", address);
                        let store_for_this_thread = Arc::clone(&store);
                        thread::spawn(move || {
                            let mut buf = [0u8; 512];
                            let mut connection_buffer = Vec::new();
                            loop {
                                let n = stream.read(&mut buf).unwrap();
                                if n == 0 {
                                    break;
                                }
                                connection_buffer.extend_from_slice(&buf[..n]);
                                let tcp_stream_data = String::from_utf8_lossy(&connection_buffer);
                                let response = match resp::parse(&tcp_stream_data) {
                                    Ok((command, _lines_consumed)) => {
                                        let bytes_consumed =
                                            lines_to_bytes(&tcp_stream_data, _lines_consumed);
                                        connection_buffer.drain(..bytes_consumed);
                                        Some(handle_command(command, &store_for_this_thread))
                                    }
                                    Err(message) if message == "unexpected end of input" => None,
                                    Err(message) => {
                                        connection_buffer.clear();
                                        Some(RespValue::Error(message))
                                    }
                                };

                                if let Some(actual_response) = response {
                                    let response_bytes = resp::serialize(&actual_response);
                                    if let Err(e) = stream.write_all(response_bytes.as_bytes()) {
                                        eprintln!("Write error: {}", e);
                                        break;
                                    }
                                    stream.flush().unwrap();
                                }
                            }
                        });
                    }
                    Err(e) => eprintln!("{}: tcp connection failed", e),
                }
            }
        }
        Err(e) => eprintln!("{}: tcp connection failed", e),
    }
    Ok(())
}

fn handle_command(
    command: resp::RespValue,
    store: &Arc<Mutex<HashMap<String, Entry>>>,
) -> resp::RespValue {
    match command {
        RespValue::Array(items) => {
            if items.is_empty() {
                return RespValue::Null;
            }
            let cmd = if let RespValue::BulkString(s) = &items[0] {
                s.to_uppercase()
            } else {
                return RespValue::Error("Expected string".to_string());
            };

            match cmd.as_str() {
                "GET" => {
                    if items.len() != 2 {
                        return RespValue::Error(
                            "ERR wrong number of arguments for GET".to_string(),
                        );
                    }
                    let key = match as_bulk_string(&items[1]) {
                        Some(key) => key,
                        None => {
                            return RespValue::Error("ERR key must be a bulk string".to_string());
                        }
                    };

                    let map = store.lock().unwrap();
                    match map.get(key) {
                        Some(data) => {
                            match data.expires_at {
                                Some(time) if time <= Instant::now() => RespValue::Null,
                                Some(_) => RespValue::Null,
                                None => RespValue::SimpleString("Expire is not set".to_string()),
                            };

                            RespValue::BulkString(data.value.clone())
                        }
                        None => RespValue::Null,
                    }
                }
                "SET" => {
                    if items.len() != 3 {
                        return RespValue::Error(
                            "ERR wrong number of arguments for SET".to_string(),
                        );
                    }

                    let key = match as_bulk_string(&items[1]) {
                        Some(key) => key,
                        None => {
                            return RespValue::Error("ERR key must be a bulk string".to_string());
                        }
                    };

                    let value = match as_bulk_string(&items[2]) {
                        Some(value) => value,
                        None => {
                            return RespValue::Error("ERR value must be a bulk string".to_string());
                        }
                    };

                    let mut map: std::sync::MutexGuard<'_, HashMap<String, Entry>> =
                        store.lock().unwrap();
                    map.insert(
                        key.to_string(),
                        Entry {
                            value: value.to_string(),
                            expires_at: Some(Instant::now() + Duration::from_secs(30)),
                        },
                    );
                    RespValue::SimpleString("OK".to_string())
                }
                "DEL" => {
                    if items.len() != 2 {
                        return RespValue::Error(
                            "ERR wrong number of arguments for Del".to_string(),
                        );
                    }

                    let key = match as_bulk_string(&items[1]) {
                        Some(key) => key,
                        None => {
                            return RespValue::Error("ERR key must be a bulk string".to_string());
                        }
                    };

                    let mut map: std::sync::MutexGuard<'_, HashMap<String, Entry>> =
                        store.lock().unwrap();
                    match map.remove(key) {
                        Some(_) => RespValue::Integer(1),
                        None => RespValue::Integer(0),
                    }
                }
                _ => RespValue::Error("unknown command".to_string()),
            }
        }
        _ => resp::RespValue::Error("expected array".to_string()),
    }
}

fn as_bulk_string(value: &RespValue) -> Option<&str> {
    match value {
        RespValue::BulkString(s) => Some(s.as_str()),
        _ => None,
    }
}

fn lines_to_bytes(input: &str, line_count: usize) -> usize {
    let mut seen = 0;
    for (byte_idx, ch) in input.char_indices() {
        if ch == '\n' {
            seen += 1;
            if seen == line_count {
                return byte_idx + 1;
            }
        }
    }
    input.len()
}

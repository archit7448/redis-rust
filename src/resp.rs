#[derive(Debug, PartialEq, Clone)]
pub enum RespValue {
    // 'pub' added — fixes the privacy warning
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Null,
    Array(Vec<RespValue>),
}

pub fn parse(input: &str) -> Result<RespValue,String> {
    let lines: Vec<&str> = input.lines().collect();
    let mut pos = 0;
    parse_value(&lines, &mut pos)
}

fn parse_value(lines: &[&str], pos: &mut usize) -> Result<RespValue,String> {
    let line =  match  lines.get(*pos) {
      Some(line) => *line,
      None => return Err("unexpected end of input".to_string()),
    };

    *pos += 1;

    match line.chars().next() {
        Some('+') => Ok(RespValue::SimpleString(line[1..].to_string())),
        Some('-') => Ok(RespValue::Error(line[1..].to_string())),
        Some(':') => {
            let n  = match line[1..].parse::<i64>(){
                Ok(n) => n, 
                Err(_) => return Err("invalid integer".to_string()),
            };

            Ok(RespValue::Integer(n))
        },
        Some('$') => {
            let length: i64 =  match line[1..].parse::<i64>() {
                Ok(n) => n, 
                Err(_) => return Err("invalid integer".to_string()),
            }; 

            if length == -1 {
                Ok(RespValue::Null)
            } else {
              let data = match lines.get(*pos) {
               Some(data) => *data,
               None => return Err("unexpected end of input".to_string()),
               };
                *pos += 1;
                Ok(RespValue::BulkString(data.to_string()))
            }
        }
        Some('*') => {
            let count: usize =  match line[1..].parse::<usize>(){
                Ok(n) => n,
                Err(_) => return Err("invalid integer".to_string()),
            };
            
            let mut items = Vec::new();
            for _ in 0..count {
                items.push(parse_value(lines, pos)?);
            }
            Ok(RespValue::Array(items))
        }
        _ => Err(format!("unknown RESP type: {}", line)),
    }
}

// ── Serializer ───────────────────────────────────────────────────────────────
// Takes a RespValue and produces the RESP wire format string.
// This is the exact reverse of parse().
pub fn serialize(value: &RespValue) -> String {
    match value {
        RespValue::SimpleString(s) => format!("+{}\r\n", s),
        RespValue::Error(s) => format!("-{}\r\n", s),
        RespValue::Integer(n) => format!(":{}\r\n", n),
        RespValue::Null => "$-1\r\n".to_string(),
        RespValue::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
        RespValue::Array(items) => {
            let mut result = format!("*{}\r\n", items.len());
            for item in items {
                result.push_str(&serialize(item)); // recursive: each item serialized
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum construction tests (Step 2.1) ──────────────────────────────────

    #[test]
    fn test_simple_string_variant() {
        let v = RespValue::SimpleString("Rust".to_string());
        assert_eq!(v, RespValue::SimpleString("Rust".to_string()));
    }

    #[test]
    fn test_null_variant() {
        let v = RespValue::Null;
        assert_eq!(v, RespValue::Null);
    }

    #[test]
    fn test_integer_variant() {
        let v = RespValue::Integer(42);
        assert_eq!(v, RespValue::Integer(42));
    }

    #[test]
    fn test_array_variant() {
        let v = RespValue::Array(vec![
            RespValue::BulkString("SET".to_string()),
            RespValue::BulkString("foo".to_string()),
            RespValue::BulkString("bar".to_string()),
        ]);
        if let RespValue::Array(items) = v {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], RespValue::BulkString("SET".to_string()));
        }
    }

    // ── Parser tests (Step 2.2) ──────────────────────────────────────────────

    #[test]
    fn test_parse_simple_string() {
        assert_eq!(parse("+OK\r\n"), Ok(RespValue::SimpleString("OK".to_string())));
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(
            parse("-ERR unknown command\r\n"),
            Ok(RespValue::Error("ERR unknown command".to_string()))
        );
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(parse(":42\r\n"), Ok(RespValue::Integer(42)));
    }

    #[test]
    fn test_parse_null() {
        assert_eq!(parse("$-1\r\n"), Ok(RespValue::Null));
    }

    #[test]
    fn test_parse_bulk_string() {
        assert_eq!(
            parse("$5\r\nhello\r\n"),
            Ok(RespValue::BulkString("hello".to_string()))
        );
    }

    #[test]
    fn test_parse_array() {
        assert_eq!(
            parse("*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"),
            Ok(RespValue::Array(vec![
                RespValue::BulkString("SET".to_string()),
                RespValue::BulkString("foo".to_string()),
                RespValue::BulkString("bar".to_string()),
            ]))
        );
    }

    // ── Serializer tests (Step 2.3) ──────────────────────────────────────────

    #[test]
    fn test_serialize_simple_string() {
        assert_eq!(
            serialize(&RespValue::SimpleString("OK".to_string())),
            "+OK\r\n"
        );
    }

    #[test]
    fn test_serialize_error() {
        assert_eq!(
            serialize(&RespValue::Error("ERR bad".to_string())),
            "-ERR bad\r\n"
        );
    }

    #[test]
    fn test_serialize_integer() {
        assert_eq!(serialize(&RespValue::Integer(42)), ":42\r\n");
    }

    #[test]
    fn test_serialize_null() {
        assert_eq!(serialize(&RespValue::Null), "$-1\r\n");
    }

    #[test]
    fn test_serialize_bulk_string() {
        assert_eq!(
            serialize(&RespValue::BulkString("hello".to_string())),
            "$5\r\nhello\r\n"
        );
    }

    #[test]
    fn test_serialize_array() {
        let v = RespValue::Array(vec![
            RespValue::BulkString("foo".to_string()),
            RespValue::BulkString("bar".to_string()),
        ]);
        assert_eq!(serialize(&v), "*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    // ── Roundtrip test: parse then serialize must give back the original ─────

    #[test]
    fn test_roundtrip() {
        let input = "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let value = parse(input).unwrap();
        assert_eq!(serialize(&value), input);
    }
}

use crate::{Command, CommandError, SetCond};
use bytes::Bytes;
use protocol::RespValue;
use std::time::Duration;

pub fn parse_command(value: RespValue) -> Result<Command, CommandError> {
    let items = match value {
        RespValue::Array(Some(items)) => items,
        _ => return Err(CommandError::BadFraming),
    };
    let mut bulks: Vec<Bytes> = items
        .into_iter()
        .map(|v| match v {
            RespValue::BulkString(Some(b)) => Ok(b),
            _ => Err(CommandError::BadFraming),
        })
        .collect::<Result<_, _>>()?;

    if bulks.is_empty() {
        return Err(CommandError::BadFraming);
    }
    let cmd_name = std::str::from_utf8(&bulks[0])
        .map_err(|_| CommandError::BadFraming)?
        .to_ascii_uppercase();
    bulks.remove(0); // consume name

    match cmd_name.as_str() {
        "PING" => match bulks.len() {
            0 => Ok(Command::Ping(None)),
            1 => Ok(Command::Ping(Some(bulks.remove(0)))),
            _ => Err(CommandError::WrongArity("PING".into())),
        },
        "ECHO" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("ECHO".into()));
            }
            Ok(Command::Echo(bulks.remove(0)))
        }
        "GET" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("GET".into()));
            }
            Ok(Command::Get(bulks.remove(0)))
        }
        "SET" => parse_set(bulks),
        "DEL" => {
            if bulks.is_empty() {
                return Err(CommandError::WrongArity("DEL".into()));
            }
            Ok(Command::Del(bulks))
        }
        "EXISTS" => {
            if bulks.is_empty() {
                return Err(CommandError::WrongArity("EXISTS".into()));
            }
            Ok(Command::Exists(bulks))
        }
        "INCR" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("INCR".into()));
            }
            Ok(Command::Incr(bulks.remove(0)))
        }
        "DECR" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("DECR".into()));
            }
            Ok(Command::Decr(bulks.remove(0)))
        }
        "EXPIRE" => {
            if bulks.len() != 2 {
                return Err(CommandError::WrongArity("EXPIRE".into()));
            }
            let secs = parse_i64(&bulks[1])?;
            if secs < 0 {
                return Err(CommandError::InvalidArg("seconds must be >= 0"));
            }
            Ok(Command::Expire {
                key: bulks.remove(0),
                ttl: Duration::from_secs(secs as u64),
            })
        }
        "TTL" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("TTL".into()));
            }
            Ok(Command::Ttl(bulks.remove(0)))
        }
        "KEYS" => {
            if bulks.len() != 1 {
                return Err(CommandError::WrongArity("KEYS".into()));
            }
            Ok(Command::Keys(bulks.remove(0)))
        }
        "INFO" => Ok(Command::Info),
        "REPLICAOF" => {
            if bulks.len() != 2 {
                return Err(CommandError::WrongArity("REPLICAOF".into()));
            }
            let host = std::str::from_utf8(&bulks[0])
                .map_err(|_| CommandError::InvalidArg("host"))?
                .to_string();
            let port_str =
                std::str::from_utf8(&bulks[1]).map_err(|_| CommandError::InvalidArg("port"))?;
            if host.eq_ignore_ascii_case("NO") && port_str.eq_ignore_ascii_case("ONE") {
                return Ok(Command::ReplicaOf(None));
            }
            let port: u16 = port_str
                .parse()
                .map_err(|_| CommandError::InvalidArg("port not u16"))?;
            Ok(Command::ReplicaOf(Some((host, port))))
        }
        other => Err(CommandError::Unknown(other.to_string())),
    }
}

fn parse_set(mut args: Vec<Bytes>) -> Result<Command, CommandError> {
    if args.len() < 2 {
        return Err(CommandError::WrongArity("SET".into()));
    }
    let key = args.remove(0);
    let value = args.remove(0);
    let mut ttl = None;
    let mut cond = SetCond::Always;
    let mut i = 0;
    while i < args.len() {
        let token = std::str::from_utf8(&args[i])
            .map_err(|_| CommandError::InvalidArg("SET option"))?
            .to_ascii_uppercase();
        match token.as_str() {
            "EX" => {
                if i + 1 >= args.len() {
                    return Err(CommandError::WrongArity("SET".into()));
                }
                let secs = parse_i64(&args[i + 1])?;
                if secs <= 0 {
                    return Err(CommandError::InvalidArg("EX must be > 0"));
                }
                ttl = Some(Duration::from_secs(secs as u64));
                i += 2;
            }
            "PX" => {
                if i + 1 >= args.len() {
                    return Err(CommandError::WrongArity("SET".into()));
                }
                let ms = parse_i64(&args[i + 1])?;
                if ms <= 0 {
                    return Err(CommandError::InvalidArg("PX must be > 0"));
                }
                ttl = Some(Duration::from_millis(ms as u64));
                i += 2;
            }
            "NX" => {
                cond = SetCond::IfAbsent;
                i += 1;
            }
            "XX" => {
                cond = SetCond::IfExists;
                i += 1;
            }
            _ => return Err(CommandError::InvalidArg("unknown SET option")),
        }
    }
    Ok(Command::Set {
        key,
        value,
        ttl,
        cond,
    })
}

fn parse_i64(b: &[u8]) -> Result<i64, CommandError> {
    std::str::from_utf8(b)
        .map_err(|_| CommandError::NotInteger)?
        .parse::<i64>()
        .map_err(|_| CommandError::NotInteger)
}

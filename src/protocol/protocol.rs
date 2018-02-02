/*
 * This submodule contains implementation of basic
 * elements of the TWS protocol.
 * TODO: Better documentation
 * TODO: Randomize packet length
 *      or try to add random meaningless
 *      packets during the session.
 */
use errors::*;
use base64::encode;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::net::SocketAddr;
use std::str;
use protocol::util;

/*
 * HMAC_SHA256 authentication wrapper
 * This is used for HANDSHAKE and CONNECT packets
 */
pub fn hmac_sha256(passwd: &str, data: &str) -> Result<String> {
    Hmac::<Sha256>::new(passwd.as_bytes())
        .and_then(|mut mac| {
            mac.input(data.as_bytes());
            Ok(encode(mac.result().code().as_slice()))
        })
        .map_err(|_| "HMAC_SHA256 failed".into())
}

fn build_authenticated_packet(passwd: &str, msg: &str) -> Result<String> {
    hmac_sha256(passwd, msg)
        .and_then(|auth| Ok(format!("AUTH {}\n{}", auth, msg)))
}

fn parse_authenticated_packet(passwd: &str, packet: &[u8]) -> Result<Vec<String>> {
    if packet[0..4] != "AUTH".as_bytes()[0..4] {
        return Err("Not a proper authenticated packet.".into());
    }

    str::from_utf8(packet)
        .map_err(|_| "Illegal packet".into())
        .and_then(|packet_str| {
            let lines = packet_str.lines()
                .map(|s| String::from(s))
                .collect::<Vec<String>>();

            hmac_sha256(passwd, &lines[1..].join("\n"))
                .and_then(|auth| Ok((lines, auth)))
        })
        .and_then(|(lines, auth)| {
            if lines[0] == format!("AUTH {}", auth) {
                Ok(lines[1..].to_vec())
            } else {
                Err("Illegal packet".into())
            }
        })
}

/*
 * Handshake packet
 * 
 * > AUTH [authentication code]
 * > NOW [current timestamp (UTC)]
 * > TARGET [targetHost]:[targetPort]
 * 
 * [authentication code] is the HMAC_SHA256 value
 * based on the pre-shared password and
 * the full message without the AUTH line.
 */
pub fn handshake_build(passwd: &str, target: SocketAddr) -> Result<String> {
    _handshake_build(passwd, util::time_ms(), target)
}

fn _handshake_build(passwd: &str, time: i64, target: SocketAddr) -> Result<String> {
    build_authenticated_packet(
        passwd,
        &format!("NOW {}\nTARGET {}", time, util::addr_to_str(target))
    )
}

pub fn handshake_parse(passwd: &str, packet: &[u8]) -> Result<SocketAddr> {
    _handshake_parse(passwd, util::time_ms(), packet)
}

fn _handshake_parse(passwd: &str, time: i64, packet: &[u8]) -> Result<SocketAddr> {
    parse_authenticated_packet(passwd, packet)
        .and_then(|lines| {
            if lines.len() < 2 {
                return Err("Not a handshake packet".into());
            }
            if !(lines[0].starts_with("NOW ") && lines[0].len() > 4) {
                return Err("Not a handshake packet".into());
            }
            if !(lines[1].starts_with("TARGET ") && lines[1].len() > 7) {
                return Err("Not a handshake packet".into());
            }
            lines[0][4..].parse::<i64>()
                .chain_err(|| "Illegal handshake packet")
                .and_then(|packet_time| Ok((packet_time, lines)))
        })
        .and_then(|(packet_time, lines)| {
            if time - packet_time > 5 * 1000 {
                return Err("Protocol handshake timed out".into());
            }
            util::str_to_addr(&lines[1][7..])
                .chain_err(|| "Illegal host name")
        })
}

/*
 * Connect packet
 * 
 * > AUTH [authentication code]
 * > NEW CONNECTION [conn id]
 * 
 * [conn id] should be a random 6-char string
 * generated by the client side.
 * TODO: Should we make authentication for this
 *  kind of packets more strict? i.e. include time
 */
fn connect_build(passwd: &str) -> Result<(String, String)> {
    let conn_id = util::rand_str(6);
    _connect_build(passwd, &conn_id)
        .and_then(|packet| Ok((conn_id, packet)))
}

fn _connect_build(passwd: &str, conn_id: &str) -> Result<String> {
    build_authenticated_packet(
        passwd,
        &format!("NEW CONNECTION {}", conn_id)
    )
}

fn connect_parse(passwd: &str, packet: &[u8]) -> Result<String> {
    parse_authenticated_packet(passwd, packet)
        .and_then(|lines| {
            if lines.len() < 1 {
                return Err("Not a Connect packet".into());
            }

            if !(lines[0].starts_with("NEW CONNECTION ") && lines[0].len() == 21) {
                return Err("Not a Connect packet".into());
            }

            Ok(String::from(&lines[0][15..]))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_should_work_1() {
        assert_eq!("pOWtIY65MVjolOXjrIkpNH72V95kfBGN9zL1OJdUZOY=", hmac_sha256("testpasswd", "testdata").unwrap());
    }

    #[test]
    fn hmac_sha256_should_work_2() {
        assert_eq!("3c/Z/9/7ZqSfddwILUTheauyZe7YdDCRRtOArSRo9bc=", hmac_sha256("testpasswd2", "testdata2").unwrap());
    }

    #[test]
    fn handshake_build_1() {
        assert_eq!(
            "AUTH s4V0i9Lwlm6eve7JftwGEgKN20mgtbSW3uacxIuh0Fo=\nNOW 1517476212983\nTARGET 192.168.1.1:443",
            _handshake_build("bscever", 1517476212983, util::str_to_addr("192.168.1.1:443").unwrap()).unwrap()
        );
    }

    #[test]
    fn handshake_build_2() {
        assert_eq!(
            "AUTH wrhyAKqrQKln+Jj9rSlpiDC1+/gw8vi5o6yIMnB5oOM=\nNOW 1517476367329\nTARGET 8.8.4.4:62311",
            _handshake_build("0o534hn045", 1517476367329, util::str_to_addr("8.8.4.4:62311").unwrap()).unwrap()
        );
    }

    #[test]
    fn handshake_build_parse_1() {
        let t = util::time_ms();
        let handshake = _handshake_build("evbie", t, util::str_to_addr("233.233.233.233:456").unwrap()).unwrap();
        assert_eq!("233.233.233.233:456", util::addr_to_str(_handshake_parse("evbie", t, handshake.as_bytes()).unwrap()));
    }

    #[test]
    fn handshake_build_parse_2() {
        let t = util::time_ms();
        let handshake = _handshake_build("43g,poe3w", t, util::str_to_addr("fe80::dead:beef:2333:8080").unwrap()).unwrap();
        assert_eq!("fe80::dead:beef:2333:8080", util::addr_to_str(_handshake_parse("43g,poe3w", t, handshake.as_bytes()).unwrap()));
    }

    #[test]
    fn connect_build_1() {
        assert_eq!(
            "AUTH +cdQQVGtyqj7KxTS5mPEwvpRGhRuctCM3pa9GsTYGZA=\nNEW CONNECTION XnjEa2",
            _connect_build("eeovgrg", "XnjEa2").unwrap()
        );
    }

    #[test]
    fn connect_parse_1() {
        assert_eq!(
            "37keeU",
            connect_parse("fneo0ivb", b"AUTH +l0yOYsTR0oqvj7//0iO24WjmdxRKNmMwVhXZpVLwvY=\nNEW CONNECTION 37keeU").unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn connect_parse_fail_1() {
        connect_parse("fneo0ib", b"AUTH +l0yOYsTR0oqvj7//0iO24WjmdxRKNmMwVhXZpVLwvY=\nNEW CONNECTION 37keeU").unwrap();
    }

    #[test]
    #[should_panic]
    fn connect_parse_fail_2() {
        connect_parse("fneo0ivb", b"AUTH +l0yOYsTR0oqvj77/0iO24WjmdxRKNmMwVhXZpVLwvY=\nNEW CONNECTION 37keeU").unwrap();
    }
}
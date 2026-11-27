use std::net::{Ipv4Addr, SocketAddrV4};

use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    TCP_TABLE_OWNER_PID_CONNECTIONS,
};
use windows::Win32::Networking::WinSock::AF_INET;

/// Attempts before giving up when connections open faster than the table is read.
const ATTEMPTS: usize = 3;

/// Id of the process that owns the client side of a local IPv4 TCP
/// connection to `server_port`.
#[must_use]
pub fn tcp_client_process(client: SocketAddrV4, server_port: u16) -> Option<u32> {
    let table = connection_table()?;

    let rows = unsafe {
        let table = table.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>();
        std::slice::from_raw_parts((*table).table.as_ptr(), (*table).dwNumEntries as usize)
    };

    rows.iter()
        .find(|row| row_matches(row, client, server_port))
        .map(|row| row.dwOwningPid)
}

fn connection_table() -> Option<Vec<u32>> {
    let mut size = 0u32;
    let mut buffer: Vec<u32> = Vec::new();

    for _ in 0..ATTEMPTS {
        // u32 elements keep the table aligned for its u32 fields.
        buffer.resize((size as usize).div_ceil(size_of::<u32>()), 0);
        let result = unsafe {
            GetExtendedTcpTable(
                Some(buffer.as_mut_ptr().cast()),
                &raw mut size,
                false,
                u32::from(AF_INET.0),
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            )
        };

        match result {
            code if code == NO_ERROR.0 => return Some(buffer),
            code if code == ERROR_INSUFFICIENT_BUFFER.0 => {}
            _ => return None,
        }
    }

    None
}

/// Addresses and ports in the table are in network byte order.
fn row_matches(row: &MIB_TCPROW_OWNER_PID, client: SocketAddrV4, server_port: u16) -> bool {
    Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()) == *client.ip()
        && port(row.dwLocalPort) == client.port()
        && port(row.dwRemotePort) == server_port
}

/// The port sits in the first two bytes of the field, most significant first.
fn port(raw: u32) -> u16 {
    let [high, low, ..] = raw.to_ne_bytes();

    u16::from_be_bytes([high, low])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_matches_the_client_side_of_a_connection() {
        let client = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 50123);
        let row = MIB_TCPROW_OWNER_PID {
            dwLocalAddr: u32::from_ne_bytes([127, 0, 0, 1]),
            dwLocalPort: network_port(50123),
            dwRemotePort: network_port(47823),
            dwOwningPid: 42,
            ..Default::default()
        };

        assert!(row_matches(&row, client, 47823));
        assert!(!row_matches(&row, client, 8080));
    }

    #[test]
    fn the_own_connection_is_found_in_the_system_table() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server_port = listener.local_addr().unwrap().port();
        let stream = std::net::TcpStream::connect(("127.0.0.1", server_port)).unwrap();
        let std::net::SocketAddr::V4(client) = stream.local_addr().unwrap() else {
            panic!("expected an IPv4 address");
        };

        let owner = tcp_client_process(client, server_port);

        assert_eq!(owner, Some(std::process::id()));
    }

    fn network_port(port: u16) -> u32 {
        let [high, low] = port.to_be_bytes();

        u32::from_ne_bytes([high, low, 0, 0])
    }
}

use crate::tracing::error::TraceResult;
use crate::tracing::error::TracerError::InvalidSourceAddr;
use crate::tracing::probe::ProbeResponse;
use crate::tracing::types::Port;
use crate::tracing::util::Required;
use crate::tracing::{PortDirection, Probe, TracerAddrFamily};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, SocketAddr};

/// IPv4 implementation.
mod ipv4;

/// IPv6 implementation.
mod ipv6;

/// Platform specific network code.
mod platform;

/// A channel for sending and receiving probes.
pub mod channel;

/// An abstraction over a network interface for tracing.
pub trait Network {
    /// Send a `Probe`.
    fn send_probe(&mut self, probe: Probe) -> TraceResult<()>;

    /// Receive the next Icmp packet and return a `ProbeResponse`.
    ///
    /// Returns `None` if the read times out or the packet read is not one of the types expected.
    fn recv_probe(&mut self) -> TraceResult<Option<ProbeResponse>>;
}

/// The port used for local address discovery if not dest port is available.
const DISCOVERY_PORT: Port = Port(80);

/// Validate, Lookup or discover the source `IpAddr`.
fn make_src_addr(
    source_addr: Option<IpAddr>,
    target_addr: IpAddr,
    port_direction: PortDirection,
    interface: Option<&str>,
    addr_family: TracerAddrFamily,
) -> TraceResult<IpAddr> {
    match (source_addr, interface.as_ref()) {
        (Some(addr), None) => validate_local_addr(addr_family, addr),
        (None, Some(interface)) => lookup_interface_addr(addr_family, interface),
        (None, None) => discover_local_addr(
            addr_family,
            target_addr,
            port_direction.dest().unwrap_or(DISCOVERY_PORT).0,
        ),
        (Some(_), Some(_)) => unreachable!(),
    }
}

/// Lookup the address for a named interface.
fn lookup_interface_addr(addr_family: TracerAddrFamily, name: &str) -> TraceResult<IpAddr> {
    match addr_family {
        TracerAddrFamily::Ipv4 => ipv4::lookup_interface_addr(name),
        TracerAddrFamily::Ipv6 => ipv6::lookup_interface_addr(name),
    }
}

/// Discover the local `IpAddr` that will be used to communicate with the given target `IpAddr`.
///
/// Note that no packets are transmitted by this method.
fn discover_local_addr(
    addr_family: TracerAddrFamily,
    target: IpAddr,
    port: u16,
) -> TraceResult<IpAddr> {
    let socket = udp_socket_for_addr_family(addr_family)?;
    socket.connect(&SockAddr::from(SocketAddr::new(target, port)))?;
    Ok(socket.local_addr()?.as_socket().req()?.ip())
}

/// Validate that we can bind to the source address.
fn validate_local_addr(addr_family: TracerAddrFamily, source_addr: IpAddr) -> TraceResult<IpAddr> {
    let socket = udp_socket_for_addr_family(addr_family)?;
    let addr = SocketAddr::new(source_addr, 0);
    match socket.bind(&SockAddr::from(addr)) {
        Ok(_) => Ok(source_addr),
        Err(_) => Err(InvalidSourceAddr(addr.ip())),
    }
}

/// Create a socket suitable for a given address.
fn udp_socket_for_addr_family(addr_family: TracerAddrFamily) -> TraceResult<Socket> {
    Ok(match addr_family {
        TracerAddrFamily::Ipv4 => Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?,
        TracerAddrFamily::Ipv6 => Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?,
    })
}

/// Make a socket for sending raw `ICMP` packets.
fn make_icmp_send_socket(addr_family: TracerAddrFamily) -> TraceResult<Socket> {
    match addr_family {
        TracerAddrFamily::Ipv4 => ipv4::make_icmp_send_socket(),
        TracerAddrFamily::Ipv6 => ipv6::make_icmp_send_socket(),
    }
}

/// Make a socket for sending `UDP` packets.
fn make_udp_send_socket(addr_family: TracerAddrFamily) -> TraceResult<Socket> {
    match addr_family {
        TracerAddrFamily::Ipv4 => ipv4::make_udp_send_socket(),
        TracerAddrFamily::Ipv6 => ipv6::make_udp_send_socket(),
    }
}

/// Make a socket for receiving raw `ICMP` packets.
fn make_recv_socket(addr_family: TracerAddrFamily) -> TraceResult<Socket> {
    match addr_family {
        TracerAddrFamily::Ipv4 => ipv4::make_recv_socket(),
        TracerAddrFamily::Ipv6 => ipv6::make_recv_socket(),
    }
}

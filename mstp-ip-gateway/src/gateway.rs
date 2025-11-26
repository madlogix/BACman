//! BACnet Gateway - Routes messages between MS/TP and BACnet/IP networks

use log::{debug, info, trace};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// BACnet/IP BVLC function codes
const BVLC_ORIGINAL_UNICAST: u8 = 0x0A;
const BVLC_ORIGINAL_BROADCAST: u8 = 0x0B;
const BVLC_FORWARDED_NPDU: u8 = 0x04;

/// Default address table entry age (1 hour)
const DEFAULT_ADDRESS_AGE: Duration = Duration::from_secs(3600);

/// Address table entry with timestamp for aging
#[derive(Debug, Clone)]
struct AddressEntry<T> {
    address: T,
    last_seen: Instant,
}

impl<T> AddressEntry<T> {
    fn new(address: T) -> Self {
        Self {
            address,
            last_seen: Instant::now(),
        }
    }

    fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        self.last_seen.elapsed() > max_age
    }
}

/// BACnet Gateway
pub struct BacnetGateway {
    // Network configuration
    mstp_network: u16,
    ip_network: u16,

    // Address translation tables with aging
    mstp_to_ip: HashMap<u8, AddressEntry<SocketAddr>>,
    ip_to_mstp: HashMap<SocketAddr, AddressEntry<u8>>,

    // Address aging configuration
    address_max_age: Duration,

    // Pending transmissions for IP side
    ip_send_queue: Vec<(Vec<u8>, SocketAddr)>,

    // Statistics
    stats: GatewayStats,

    // UDP socket for sending (will be set externally)
    ip_socket: Option<UdpSocket>,
}

/// Gateway statistics
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct GatewayStats {
    pub mstp_to_ip_packets: u64,
    pub ip_to_mstp_packets: u64,
    pub routing_errors: u64,
    pub last_activity: Option<Instant>,
}

#[allow(dead_code)]
impl BacnetGateway {
    /// Create a new gateway
    pub fn new(mstp_network: u16, ip_network: u16) -> Self {
        info!(
            "Creating BACnet gateway: MS/TP network {} <-> IP network {}",
            mstp_network, ip_network
        );

        Self {
            mstp_network,
            ip_network,
            mstp_to_ip: HashMap::new(),
            ip_to_mstp: HashMap::new(),
            address_max_age: DEFAULT_ADDRESS_AGE,
            ip_send_queue: Vec::new(),
            stats: GatewayStats::default(),
            ip_socket: None,
        }
    }

    /// Set custom address aging timeout
    pub fn set_address_max_age(&mut self, max_age: Duration) {
        self.address_max_age = max_age;
    }

    /// Learn/update an MS/TP to IP address mapping
    fn learn_mstp_address(&mut self, mstp_addr: u8, ip_addr: SocketAddr) {
        if let Some(entry) = self.mstp_to_ip.get_mut(&mstp_addr) {
            entry.address = ip_addr;
            entry.touch();
            trace!("Updated MS/TP address {} -> {}", mstp_addr, ip_addr);
        } else {
            self.mstp_to_ip.insert(mstp_addr, AddressEntry::new(ip_addr));
            debug!("Learned MS/TP address {} -> {}", mstp_addr, ip_addr);
        }
    }

    /// Learn/update an IP to MS/TP address mapping
    fn learn_ip_address(&mut self, ip_addr: SocketAddr, mstp_addr: u8) {
        if let Some(entry) = self.ip_to_mstp.get_mut(&ip_addr) {
            entry.address = mstp_addr;
            entry.touch();
            trace!("Updated IP address {} -> MS/TP {}", ip_addr, mstp_addr);
        } else {
            self.ip_to_mstp.insert(ip_addr, AddressEntry::new(mstp_addr));
            debug!("Learned IP address {} -> MS/TP {}", ip_addr, mstp_addr);
        }
    }

    /// Set the IP socket for sending
    pub fn set_ip_socket(&mut self, socket: UdpSocket) {
        self.ip_socket = Some(socket);
    }

    /// Route a frame from MS/TP to IP
    pub fn route_from_mstp(&mut self, data: &[u8], source_addr: u8) -> Result<(), GatewayError> {
        if data.len() < 2 {
            return Err(GatewayError::InvalidFrame);
        }

        // Parse NPDU
        let (npdu, _npdu_len) = parse_npdu(data)?;

        debug!(
            "Routing MS/TP->IP: src={} network_msg={} dest_present={}",
            source_addr, npdu.network_message, npdu.destination_present
        );

        // Determine destination
        let dest_addr = if let Some(ref dest) = npdu.destination {
            if dest.network == self.ip_network {
                // Specific device on IP network
                self.resolve_ip_address(&dest.address)?
            } else if dest.network == 0xFFFF {
                // Global broadcast
                "255.255.255.255:47808".parse().unwrap()
            } else {
                // Unknown network
                return Err(GatewayError::NetworkUnreachable(dest.network));
            }
        } else {
            // Local network broadcast - forward to IP broadcast
            "255.255.255.255:47808".parse().unwrap()
        };

        // Build NPDU with source network info
        let routed_npdu = build_routed_npdu(
            data,
            self.mstp_network,
            &[source_addr],
            &npdu,
        )?;

        // Wrap in BVLC - check if broadcast (only IPv4 has is_broadcast)
        let is_broadcast = match dest_addr.ip() {
            IpAddr::V4(addr) => addr.is_broadcast(),
            IpAddr::V6(_) => false,
        };
        let bvlc = build_bvlc(&routed_npdu, is_broadcast);

        // Send via IP
        if let Some(ref socket) = self.ip_socket {
            socket.send_to(&bvlc, dest_addr)
                .map_err(|e| GatewayError::IoError(e.to_string()))?;
        } else {
            // Queue for later
            self.ip_send_queue.push((bvlc, dest_addr));
        }

        self.stats.mstp_to_ip_packets += 1;
        self.stats.last_activity = Some(Instant::now());

        Ok(())
    }

    /// Route a frame from IP to MS/TP
    /// Returns the data and destination address for MS/TP
    pub fn route_from_ip(
        &mut self,
        data: &[u8],
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        if data.len() < 4 {
            return Err(GatewayError::InvalidFrame);
        }

        // Parse BVLC header
        if data[0] != 0x81 {
            return Err(GatewayError::InvalidFrame);
        }

        let bvlc_function = data[1];
        let bvlc_length = ((data[2] as usize) << 8) | (data[3] as usize);

        if data.len() != bvlc_length {
            return Err(GatewayError::InvalidFrame);
        }

        // Extract NPDU based on BVLC function
        let npdu_data = match bvlc_function {
            BVLC_ORIGINAL_UNICAST | BVLC_ORIGINAL_BROADCAST => &data[4..],
            BVLC_FORWARDED_NPDU => {
                if data.len() < 10 {
                    return Err(GatewayError::InvalidFrame);
                }
                &data[10..] // Skip original source address
            }
            _ => {
                // Other BVLC functions (control messages)
                return Ok(None);
            }
        };

        if npdu_data.len() < 2 {
            return Err(GatewayError::InvalidFrame);
        }

        // Parse NPDU
        let (npdu, _npdu_len) = parse_npdu(npdu_data)?;

        debug!(
            "Routing IP->MS/TP: src={} network_msg={} dest_present={}",
            source_addr, npdu.network_message, npdu.destination_present
        );

        // Determine MS/TP destination
        let mstp_dest = if let Some(ref dest) = npdu.destination {
            if dest.network == self.mstp_network {
                // Specific device on MS/TP network
                if dest.address.is_empty() {
                    255 // Broadcast on MS/TP network
                } else {
                    dest.address[0]
                }
            } else if dest.network == 0xFFFF {
                // Global broadcast
                255
            } else {
                // Not for MS/TP network
                return Ok(None);
            }
        } else {
            // Local delivery - check if it's for us or broadcast
            255
        };

        // Build NPDU with source network info
        let routed_npdu = build_routed_npdu(
            npdu_data,
            self.ip_network,
            &ip_to_mac(&source_addr),
            &npdu,
        )?;

        self.stats.ip_to_mstp_packets += 1;
        self.stats.last_activity = Some(Instant::now());

        // Update address translation table with aging
        if let Some(ref src) = npdu.source {
            if !src.address.is_empty() {
                self.learn_ip_address(source_addr, src.address[0]);
            }
        }

        Ok(Some((routed_npdu, mstp_dest)))
    }

    /// Resolve an IP address from BACnet MAC address
    fn resolve_ip_address(&self, mac: &[u8]) -> Result<SocketAddr, GatewayError> {
        if mac.len() == 6 {
            // 6-byte BACnet/IP address: 4 bytes IP + 2 bytes port
            let ip = std::net::Ipv4Addr::new(mac[0], mac[1], mac[2], mac[3]);
            let port = ((mac[4] as u16) << 8) | (mac[5] as u16);
            Ok(SocketAddr::new(ip.into(), port))
        } else {
            Err(GatewayError::InvalidAddress)
        }
    }

    /// Process periodic housekeeping tasks
    pub fn process_housekeeping(&mut self) {
        // Clean up old address mappings
        let max_age = self.address_max_age;

        // Count entries before cleanup
        let mstp_before = self.mstp_to_ip.len();
        let ip_before = self.ip_to_mstp.len();

        // Remove expired MS/TP to IP mappings
        self.mstp_to_ip.retain(|addr, entry| {
            let keep = !entry.is_expired(max_age);
            if !keep {
                debug!("Aged out MS/TP address {} -> {}", addr, entry.address);
            }
            keep
        });

        // Remove expired IP to MS/TP mappings
        self.ip_to_mstp.retain(|addr, entry| {
            let keep = !entry.is_expired(max_age);
            if !keep {
                debug!("Aged out IP address {} -> MS/TP {}", addr, entry.address);
            }
            keep
        });

        // Log if any entries were removed
        let mstp_removed = mstp_before - self.mstp_to_ip.len();
        let ip_removed = ip_before - self.ip_to_mstp.len();
        if mstp_removed > 0 || ip_removed > 0 {
            info!(
                "Address table cleanup: removed {} MS/TP and {} IP entries",
                mstp_removed, ip_removed
            );
        }
    }

    /// Get gateway statistics
    pub fn get_stats(&self) -> &GatewayStats {
        &self.stats
    }
}

/// Gateway error types
#[derive(Debug)]
pub enum GatewayError {
    InvalidFrame,
    InvalidAddress,
    NetworkUnreachable(u16),
    IoError(String),
    NpduError(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::InvalidFrame => write!(f, "Invalid frame"),
            GatewayError::InvalidAddress => write!(f, "Invalid address"),
            GatewayError::NetworkUnreachable(n) => write!(f, "Network {} unreachable", n),
            GatewayError::IoError(s) => write!(f, "I/O error: {}", s),
            GatewayError::NpduError(s) => write!(f, "NPDU error: {}", s),
        }
    }
}

/// Parsed NPDU information
#[allow(dead_code)]
struct NpduInfo {
    network_message: bool,
    destination_present: bool,
    source_present: bool,
    expecting_reply: bool,
    priority: u8,
    destination: Option<NetworkAddress>,
    source: Option<NetworkAddress>,
    hop_count: Option<u8>,
}

/// Network address
struct NetworkAddress {
    network: u16,
    address: Vec<u8>,
}

/// Parse NPDU header
fn parse_npdu(data: &[u8]) -> Result<(NpduInfo, usize), GatewayError> {
    if data.len() < 2 {
        return Err(GatewayError::InvalidFrame);
    }

    let version = data[0];
    if version != 1 {
        return Err(GatewayError::NpduError(format!("Invalid version: {}", version)));
    }

    let control = data[1];
    let network_message = (control & 0x80) != 0;
    let destination_present = (control & 0x20) != 0;
    let source_present = (control & 0x08) != 0;
    let expecting_reply = (control & 0x04) != 0;
    let priority = control & 0x03;

    let mut pos = 2;

    // Parse destination
    let destination = if destination_present {
        if pos + 3 > data.len() {
            return Err(GatewayError::InvalidFrame);
        }
        let network = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        let addr_len = data[pos + 2] as usize;
        pos += 3;

        if pos + addr_len > data.len() {
            return Err(GatewayError::InvalidFrame);
        }
        let address = data[pos..pos + addr_len].to_vec();
        pos += addr_len;

        Some(NetworkAddress { network, address })
    } else {
        None
    };

    // Parse source
    let source = if source_present {
        if pos + 3 > data.len() {
            return Err(GatewayError::InvalidFrame);
        }
        let network = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        let addr_len = data[pos + 2] as usize;
        pos += 3;

        if pos + addr_len > data.len() {
            return Err(GatewayError::InvalidFrame);
        }
        let address = data[pos..pos + addr_len].to_vec();
        pos += addr_len;

        Some(NetworkAddress { network, address })
    } else {
        None
    };

    // Parse hop count
    let hop_count = if destination_present {
        if pos >= data.len() {
            return Err(GatewayError::InvalidFrame);
        }
        let hc = data[pos];
        pos += 1;
        Some(hc)
    } else {
        None
    };

    Ok((
        NpduInfo {
            network_message,
            destination_present,
            source_present,
            expecting_reply,
            priority,
            destination,
            source,
            hop_count,
        },
        pos,
    ))
}

/// Build a routed NPDU with source network information
fn build_routed_npdu(
    original_data: &[u8],
    source_network: u16,
    source_address: &[u8],
    npdu: &NpduInfo,
) -> Result<Vec<u8>, GatewayError> {
    let mut result = Vec::new();

    // Version
    result.push(1);

    // Build control byte
    let mut control = npdu.priority;
    if npdu.network_message {
        control |= 0x80;
    }
    if npdu.destination.is_some() {
        control |= 0x20;
    }
    // Always set source present since we're routing
    control |= 0x08;
    if npdu.expecting_reply {
        control |= 0x04;
    }
    result.push(control);

    // Destination (if present)
    if let Some(ref dest) = npdu.destination {
        result.push((dest.network >> 8) as u8);
        result.push((dest.network & 0xFF) as u8);
        result.push(dest.address.len() as u8);
        result.extend_from_slice(&dest.address);
    }

    // Source (always add for routing)
    result.push((source_network >> 8) as u8);
    result.push((source_network & 0xFF) as u8);
    result.push(source_address.len() as u8);
    result.extend_from_slice(source_address);

    // Hop count (if destination present)
    if npdu.destination.is_some() {
        let hc = npdu.hop_count.unwrap_or(255).saturating_sub(1);
        result.push(hc);
    }

    // Copy APDU (everything after NPDU header)
    let (_, npdu_len) = parse_npdu(original_data)?;
    if npdu_len < original_data.len() {
        result.extend_from_slice(&original_data[npdu_len..]);
    }

    Ok(result)
}

/// Build BVLC wrapper for NPDU
fn build_bvlc(npdu: &[u8], broadcast: bool) -> Vec<u8> {
    let mut result = Vec::with_capacity(4 + npdu.len());

    // BVLC header
    result.push(0x81); // BVLC type
    result.push(if broadcast {
        BVLC_ORIGINAL_BROADCAST
    } else {
        BVLC_ORIGINAL_UNICAST
    });

    let length = 4 + npdu.len();
    result.push((length >> 8) as u8);
    result.push((length & 0xFF) as u8);

    // NPDU
    result.extend_from_slice(npdu);

    result
}

/// Convert IP address to BACnet MAC format (6 bytes)
fn ip_to_mac(addr: &SocketAddr) -> Vec<u8> {
    match addr {
        SocketAddr::V4(v4) => {
            let ip = v4.ip().octets();
            let port = v4.port();
            vec![
                ip[0], ip[1], ip[2], ip[3],
                (port >> 8) as u8,
                (port & 0xFF) as u8,
            ]
        }
        SocketAddr::V6(_) => vec![],
    }
}

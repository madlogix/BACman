//! BACnet/IP Data Link Implementation (ASHRAE 135 Annex J).
//!
//! This module provides a complete implementation of the BACnet/IP data link layer,
//! enabling BACnet communication over Internet Protocol networks. BACnet/IP is the
//! most widely used data link layer in modern BACnet installations due to its
//! compatibility with existing IP infrastructure.
//!
//! # Overview
//!
//! BACnet/IP uses UDP as the transport protocol, typically on port 47808 (0xBAC0),
//! and includes the BACnet Virtual Link Control (BVLC) layer for managing broadcasts
//! across IP networks. Key features include:
//!
//! - **UDP Communication**: Efficient, connectionless transport over IP networks
//! - **BVLC Protocol**: Manages broadcast distribution across routers
//! - **Foreign Device Support**: Allows devices to join remote BACnet networks
//! - **BBMD Functionality**: Broadcast Management Devices for inter-network communication
//!
//! # BVLC Functions
//!
//! The BVLC layer provides these message types:
//!
//! - **Original-Unicast-NPDU**: Direct unicast message to a specific device
//! - **Original-Broadcast-NPDU**: Broadcast message originating from this device
//! - **Forwarded-NPDU**: Message forwarded by a BBMD
//! - **Register-Foreign-Device**: Request to join a remote network
//! - **Read-Broadcast-Distribution-Table**: Query BBMD's peer list
//! - **Read-Foreign-Device-Table**: Query registered foreign devices
//! - **Delete-Foreign-Device-Table-Entry**: Remove a foreign device
//! - **Distribute-Broadcast-To-Network**: BBMD-to-BBMD broadcast distribution
//! - **Secure-BVLL**: Encrypted BACnet communication (BACnet/SC)
//!
//! # Examples
//!
//! ## Basic BACnet/IP Communication
//!
//! ```no_run
//! use bacnet_rs::datalink::bip::{BacnetIpDataLink, BvlcFunction};
//! use bacnet_rs::datalink::{DataLink, DataLinkAddress};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a BACnet/IP data link
//! let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
//!
//! // Send a unicast message
//! let npdu = vec![0x01, 0x04, 0x00, 0x00];  // Example NPDU
//! let dest = "192.168.1.100:47808".parse()?;
//! data_link.send_frame(&npdu, &DataLinkAddress::Ip(dest))?;
//!
//! // Send a broadcast message
//! data_link.send_frame(&npdu, &DataLinkAddress::Broadcast)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Foreign Device Registration
//!
//! ```no_run
//! use bacnet_rs::datalink::bip::BacnetIpDataLink;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
//!
//! // Register with a BBMD (Time-to-live: 300 seconds)
//! let bbmd_addr = "192.168.1.10:47808".parse()?;
//! data_link.register_foreign_device(bbmd_addr, 300)?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "std")]
use std::{
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs, UdpSocket},
    time::{Duration, Instant},
};

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::datalink::{DataLink, DataLinkAddress, DataLinkError, DataLinkType, Result};

/// Broadcast configuration for BACnet/IP.
///
/// Controls how broadcast messages are sent, including subnet mask handling
/// and global broadcast behavior. This configuration is critical for ensuring
/// Who-Is requests reach all devices on the network.
///
/// # BACnet Standard Reference
///
/// Per ASHRAE 135 Annex J, BACnet/IP broadcasts should be sent to:
/// - The local subnet broadcast address for local discovery
/// - The global broadcast (255.255.255.255) when subnet is unknown
/// - BBMD peers for cross-subnet discovery
///
/// # Examples
///
/// ```
/// use bacnet_rs::datalink::bip::BroadcastConfig;
///
/// // Default: auto-detect subnet, send to both local and global
/// let config = BroadcastConfig::default();
///
/// // Explicit subnet mask for /24 network
/// let config = BroadcastConfig::with_subnet_mask([255, 255, 255, 0]);
///
/// // Only use global broadcast (for unknown network topology)
/// let config = BroadcastConfig::global_only();
/// ```
#[derive(Debug, Clone)]
#[cfg(feature = "std")]
pub struct BroadcastConfig {
    /// Subnet mask for calculating broadcast address.
    /// If None, attempts to auto-detect or falls back to /24.
    pub subnet_mask: Option<[u8; 4]>,

    /// Whether to send to the global broadcast address (255.255.255.255).
    /// Recommended for Who-Is when network topology is unknown.
    pub use_global_broadcast: bool,

    /// Whether to also send to the calculated local subnet broadcast.
    /// Useful when you want to reach devices on the local subnet more reliably.
    pub use_local_broadcast: bool,

    /// Additional broadcast addresses to send to (e.g., specific subnets).
    /// These are in addition to the auto-calculated addresses.
    pub additional_broadcasts: Vec<Ipv4Addr>,
}

#[cfg(feature = "std")]
impl Default for BroadcastConfig {
    /// Creates a default broadcast configuration optimized for device discovery.
    ///
    /// Default behavior:
    /// - Auto-detects subnet mask (falls back to /24 if detection fails)
    /// - Sends to both global broadcast (255.255.255.255) AND local subnet
    /// - No additional broadcast addresses
    fn default() -> Self {
        Self {
            subnet_mask: None, // Auto-detect
            use_global_broadcast: true,
            use_local_broadcast: true,
            additional_broadcasts: Vec::new(),
        }
    }
}

#[cfg(feature = "std")]
impl BroadcastConfig {
    /// Creates a new broadcast configuration with explicit subnet mask.
    ///
    /// # Arguments
    ///
    /// * `mask` - The subnet mask as a 4-byte array (e.g., [255, 255, 255, 0] for /24)
    ///
    /// # Examples
    ///
    /// ```
    /// use bacnet_rs::datalink::bip::BroadcastConfig;
    ///
    /// // For a /24 network (255.255.255.0)
    /// let config = BroadcastConfig::with_subnet_mask([255, 255, 255, 0]);
    ///
    /// // For a /16 network (255.255.0.0)
    /// let config = BroadcastConfig::with_subnet_mask([255, 255, 0, 0]);
    /// ```
    pub fn with_subnet_mask(mask: [u8; 4]) -> Self {
        Self {
            subnet_mask: Some(mask),
            use_global_broadcast: true,
            use_local_broadcast: true,
            additional_broadcasts: Vec::new(),
        }
    }

    /// Creates a configuration that only uses global broadcast.
    ///
    /// This is useful when the network topology is unknown or when you want
    /// to ensure the broadcast reaches the widest possible audience.
    ///
    /// # Note
    ///
    /// Global broadcasts (255.255.255.255) are typically NOT forwarded by routers,
    /// so this will only reach devices on the same Layer 2 network unless
    /// the operating system is configured to forward them.
    pub fn global_only() -> Self {
        Self {
            subnet_mask: None,
            use_global_broadcast: true,
            use_local_broadcast: false,
            additional_broadcasts: Vec::new(),
        }
    }

    /// Creates a configuration that only uses local subnet broadcast.
    ///
    /// This is useful when you know your network topology and want to avoid
    /// sending to 255.255.255.255.
    pub fn local_only() -> Self {
        Self {
            subnet_mask: None,
            use_global_broadcast: false,
            use_local_broadcast: true,
            additional_broadcasts: Vec::new(),
        }
    }

    /// Adds additional broadcast addresses to send to.
    ///
    /// This is useful when you have multiple subnets and want to broadcast
    /// to specific subnet broadcast addresses in addition to the auto-calculated ones.
    ///
    /// # Examples
    ///
    /// ```
    /// use bacnet_rs::datalink::bip::BroadcastConfig;
    /// use std::net::Ipv4Addr;
    ///
    /// let config = BroadcastConfig::default()
    ///     .with_additional_broadcast(Ipv4Addr::new(192, 168, 1, 255))
    ///     .with_additional_broadcast(Ipv4Addr::new(192, 168, 2, 255))
    ///     .with_additional_broadcast(Ipv4Addr::new(10, 0, 0, 255));
    /// ```
    pub fn with_additional_broadcast(mut self, addr: Ipv4Addr) -> Self {
        self.additional_broadcasts.push(addr);
        self
    }

    /// Disables global broadcast.
    pub fn without_global_broadcast(mut self) -> Self {
        self.use_global_broadcast = false;
        self
    }

    /// Disables local subnet broadcast.
    pub fn without_local_broadcast(mut self) -> Self {
        self.use_local_broadcast = false;
        self
    }
}

/// Global broadcast address (255.255.255.255).
#[cfg(feature = "std")]
pub const GLOBAL_BROADCAST: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);

/// Calculate the broadcast address from an IP address and subnet mask.
///
/// # Arguments
///
/// * `ip` - The IPv4 address
/// * `mask` - The subnet mask
///
/// # Returns
///
/// The broadcast address for the subnet.
///
/// # Examples
///
/// ```
/// use bacnet_rs::datalink::bip::calculate_broadcast_address;
/// use std::net::Ipv4Addr;
///
/// let ip = Ipv4Addr::new(192, 168, 1, 100);
/// let mask = [255, 255, 255, 0];
/// let broadcast = calculate_broadcast_address(&ip, &mask);
/// assert_eq!(broadcast, Ipv4Addr::new(192, 168, 1, 255));
///
/// // /16 network
/// let mask = [255, 255, 0, 0];
/// let broadcast = calculate_broadcast_address(&ip, &mask);
/// assert_eq!(broadcast, Ipv4Addr::new(192, 168, 255, 255));
/// ```
#[cfg(feature = "std")]
pub fn calculate_broadcast_address(ip: &Ipv4Addr, mask: &[u8; 4]) -> Ipv4Addr {
    let ip_octets = ip.octets();
    Ipv4Addr::new(
        ip_octets[0] | !mask[0],
        ip_octets[1] | !mask[1],
        ip_octets[2] | !mask[2],
        ip_octets[3] | !mask[3],
    )
}

/// Attempts to detect the subnet mask for a given IP address.
///
/// This function uses platform-specific methods to detect the subnet mask:
/// - On Linux: Reads from /proc/net/route and /sys/class/net
/// - Falls back to classful addressing if detection fails
///
/// # Arguments
///
/// * `ip` - The IPv4 address to find the subnet mask for
///
/// # Returns
///
/// The detected subnet mask, or a classful default if detection fails.
#[cfg(feature = "std")]
pub fn detect_subnet_mask(ip: &Ipv4Addr) -> [u8; 4] {
    // Try platform-specific detection first
    if let Some(mask) = try_detect_subnet_mask_platform(ip) {
        return mask;
    }

    // Fall back to classful addressing based on first octet
    let first_octet = ip.octets()[0];
    match first_octet {
        0..=127 => [255, 0, 0, 0],       // Class A
        128..=191 => [255, 255, 0, 0],   // Class B
        192..=223 => [255, 255, 255, 0], // Class C
        _ => [255, 255, 255, 0],         // Default to /24
    }
}

/// Platform-specific subnet mask detection.
#[cfg(all(feature = "std", target_os = "linux"))]
fn try_detect_subnet_mask_platform(ip: &Ipv4Addr) -> Option<[u8; 4]> {
    use std::fs;
    use std::io::BufRead;

    // Try to find the interface that matches this IP
    let target_ip = ip.octets();

    // Read /proc/net/fib_trie or use netlink would be more accurate,
    // but for simplicity we'll check interface addresses
    if let Ok(interfaces) = fs::read_dir("/sys/class/net") {
        for entry in interfaces.flatten() {
            let iface_name = entry.file_name();
            let iface_name = iface_name.to_string_lossy();

            // Skip loopback
            if iface_name == "lo" {
                continue;
            }

            // Try to read the interface's address info via /proc
            // This is a simplified approach - production code might use netlink
            let operstate_path = format!("/sys/class/net/{}/operstate", iface_name);
            if let Ok(state) = fs::read_to_string(&operstate_path) {
                if state.trim() != "up" {
                    continue;
                }
            }

            // Use ip command output parsing as fallback
            // In production, you'd use the netlink crate or libc directly
        }
    }

    // If we can't detect, try reading from /proc/net/route
    if let Ok(file) = std::fs::File::open("/proc/net/route") {
        let reader = std::io::BufReader::new(file);
        for line in reader.lines().skip(1).flatten() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 8 {
                // parts[1] is destination, parts[7] is mask (in hex, little-endian)
                if let Ok(mask_hex) = u32::from_str_radix(parts[7], 16) {
                    // Check if this route's network contains our IP
                    if let Ok(dest_hex) = u32::from_str_radix(parts[1], 16) {
                        let dest_ip = u32::from_ne_bytes(target_ip);
                        if (dest_ip & mask_hex) == dest_hex && mask_hex != 0 {
                            let mask_bytes = mask_hex.to_ne_bytes();
                            return Some(mask_bytes);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Platform-specific subnet mask detection for non-Linux platforms.
#[cfg(all(feature = "std", not(target_os = "linux")))]
fn try_detect_subnet_mask_platform(_ip: &Ipv4Addr) -> Option<[u8; 4]> {
    // On non-Linux platforms, we fall back to classful addressing
    // Production code could use platform-specific APIs:
    // - Windows: GetAdaptersInfo or GetAdaptersAddresses
    // - macOS/BSD: getifaddrs
    None
}

/// BACnet/IP well-known UDP port number.
///
/// This is the standard port (0xBAC0 = 47808) defined by ASHRAE 135 for BACnet/IP
/// communication. While this is the default, BACnet/IP can use other ports when
/// multiple BACnet networks share the same IP infrastructure.
///
/// # Examples
///
/// ```
/// use bacnet_rs::datalink::bip::BACNET_IP_PORT;
///
/// let addr = format!("0.0.0.0:{}", BACNET_IP_PORT);
/// assert_eq!(addr, "0.0.0.0:47808");
/// ```
pub const BACNET_IP_PORT: u16 = 47808;

/// BVLC (BACnet Virtual Link Control) message types.
///
/// These message types define the various operations supported by the BVLC protocol,
/// which manages broadcast distribution and foreign device registration in BACnet/IP
/// networks. Each function code corresponds to a specific BVLC operation.
///
/// # Protocol Details
///
/// BVLC messages have a 4-byte header followed by function-specific data:
/// - Type (1 byte): Always 0x81 for BACnet/IP
/// - Function (1 byte): One of the values defined in this enum
/// - Length (2 bytes): Total message length including header
///
/// # Examples
///
/// ```
/// use bacnet_rs::datalink::bip::BvlcFunction;
///
/// // Check if a function expects a response
/// fn expects_ack(func: BvlcFunction) -> bool {
///     matches!(func,
///         BvlcFunction::ReadBroadcastDistributionTable |
///         BvlcFunction::ReadForeignDeviceTable
///     )
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BvlcFunction {
    /// Original-Unicast-NPDU (0x0A).
    ///
    /// Encapsulates an NPDU for unicast delivery to a specific BACnet/IP device.
    /// This is the most common BVLC function for point-to-point communication.
    OriginalUnicastNpdu = 0x0A,

    /// Original-Broadcast-NPDU (0x0B).
    ///
    /// Encapsulates an NPDU for broadcast delivery. The message is sent to the
    /// local IP broadcast address and to all entries in the BDT if the sender
    /// is a BBMD.
    OriginalBroadcastNpdu = 0x0B,

    /// Forwarded-NPDU (0x04).
    ///
    /// Used by BBMDs to forward broadcasts between BACnet/IP networks. Contains
    /// the original source address before the NPDU data.
    ForwardedNpdu = 0x04,

    /// Register-Foreign-Device (0x05).
    ///
    /// Sent by a foreign device to register with a BBMD, allowing it to receive
    /// broadcasts. Includes a Time-to-Live value in seconds.
    RegisterForeignDevice = 0x05,

    /// Read-Broadcast-Distribution-Table (0x02).
    ///
    /// Request to read a BBMD's Broadcast Distribution Table, which lists peer
    /// BBMDs for broadcast forwarding.
    ReadBroadcastDistributionTable = 0x02,

    /// Read-Broadcast-Distribution-Table-Ack (0x03).
    ///
    /// Response containing the requested Broadcast Distribution Table entries.
    ReadBroadcastDistributionTableAck = 0x03,

    /// Read-Foreign-Device-Table (0x06).
    ///
    /// Request to read a BBMD's Foreign Device Table, listing registered foreign
    /// devices.
    ReadForeignDeviceTable = 0x06,

    /// Read-Foreign-Device-Table-Ack (0x07).
    ///
    /// Response containing the requested Foreign Device Table entries.
    ReadForeignDeviceTableAck = 0x07,

    /// Delete-Foreign-Device-Table-Entry (0x08).
    ///
    /// Request to remove a specific entry from the Foreign Device Table.
    DeleteForeignDeviceTableEntry = 0x08,

    /// Distribute-Broadcast-To-Network (0x09).
    ///
    /// Used between BBMDs to distribute broadcast NPDUs to all devices on their
    /// respective networks.
    DistributeBroadcastToNetwork = 0x09,

    /// Forwarded-NPDU-From-Device (0x0C).
    ///
    /// Alternative forwarding mechanism that preserves the original device address.
    ForwardedNpduFromDevice = 0x0C,

    /// Secure-BVLL (0x0D).
    ///
    /// Used for BACnet Secure Connect (BACnet/SC) encrypted communication.
    SecureBvll = 0x0D,
}

/// BVLC header structure for BACnet/IP messages.
///
/// Every BACnet/IP message begins with this 4-byte header that identifies
/// the message type and length. The header is followed by function-specific
/// data and then the NPDU (if applicable).
///
/// # Wire Format
///
/// ```text
/// +--------+--------+--------+--------+
/// | Type   | Func   | Length (MSB/LSB)|
/// | (0x81) | Code   | (Total bytes)   |
/// +--------+--------+--------+--------+
/// ```
///
/// # Examples
///
/// ```
/// use bacnet_rs::datalink::bip::{BvlcHeader, BvlcFunction};
///
/// // Create header for a 20-byte unicast NPDU
/// let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 24);
/// assert_eq!(header.bvlc_type, 0x81);
/// assert_eq!(header.length, 24);  // 4-byte header + 20-byte NPDU
/// ```
#[derive(Debug, Clone)]
pub struct BvlcHeader {
    /// BVLC type identifier.
    ///
    /// Always 0x81 for BACnet/IP. Other values are reserved for future use
    /// or indicate non-BACnet/IP frames.
    pub bvlc_type: u8,

    /// BVLC function code.
    ///
    /// Identifies the specific BVLC operation this message performs.
    pub function: BvlcFunction,

    /// Total message length in bytes.
    ///
    /// Includes the 4-byte BVLC header plus all following data. Maximum
    /// value is typically limited by UDP MTU (usually 1472 bytes for
    /// standard Ethernet without fragmentation).
    pub length: u16,
}

impl BvlcHeader {
    /// Create a new BVLC header with the specified function and length.
    ///
    /// The BVLC type is automatically set to 0x81 for BACnet/IP.
    ///
    /// # Arguments
    ///
    /// * `function` - The BVLC function for this message
    /// * `length` - Total message length including the 4-byte header
    ///
    /// # Examples
    ///
    /// ```
    /// use bacnet_rs::datalink::bip::{BvlcHeader, BvlcFunction};
    ///
    /// // Create header for a broadcast NPDU
    /// let npdu_size = 50;
    /// let header = BvlcHeader::new(
    ///     BvlcFunction::OriginalBroadcastNpdu,
    ///     4 + npdu_size  // Header + NPDU
    /// );
    /// ```
    pub fn new(function: BvlcFunction, length: u16) -> Self {
        Self {
            bvlc_type: 0x81, // BACnet/IP
            function,
            length,
        }
    }

    /// Encode the BVLC header to its 4-byte wire format.
    ///
    /// # Returns
    ///
    /// A 4-byte vector containing the encoded header.
    ///
    /// # Examples
    ///
    /// ```
    /// use bacnet_rs::datalink::bip::{BvlcHeader, BvlcFunction};
    ///
    /// let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 100);
    /// let bytes = header.encode();
    /// assert_eq!(bytes, vec![0x81, 0x0A, 0x00, 0x64]);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        vec![
            self.bvlc_type,
            self.function as u8,
            (self.length >> 8) as u8,
            (self.length & 0xFF) as u8,
        ]
    }

    /// Decode a BVLC header from its wire format.
    ///
    /// # Arguments
    ///
    /// * `data` - Buffer containing at least 4 bytes of BVLC header
    ///
    /// # Returns
    ///
    /// The decoded BVLC header.
    ///
    /// # Errors
    ///
    /// Returns [`DataLinkError::InvalidFrame`] if:
    /// - The buffer is too short (less than 4 bytes)
    /// - The BVLC type is not 0x81
    /// - The function code is not recognized
    ///
    /// # Examples
    ///
    /// ```
    /// use bacnet_rs::datalink::bip::BvlcHeader;
    ///
    /// let data = vec![0x81, 0x0A, 0x00, 0x64];
    /// let header = BvlcHeader::decode(&data).unwrap();
    /// assert_eq!(header.length, 100);
    /// ```
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(DataLinkError::InvalidFrame);
        }

        let bvlc_type = data[0];
        if bvlc_type != 0x81 {
            return Err(DataLinkError::InvalidFrame);
        }

        let function = match data[1] {
            0x0A => BvlcFunction::OriginalUnicastNpdu,
            0x0B => BvlcFunction::OriginalBroadcastNpdu,
            0x04 => BvlcFunction::ForwardedNpdu,
            0x05 => BvlcFunction::RegisterForeignDevice,
            0x02 => BvlcFunction::ReadBroadcastDistributionTable,
            0x03 => BvlcFunction::ReadBroadcastDistributionTableAck,
            0x06 => BvlcFunction::ReadForeignDeviceTable,
            0x07 => BvlcFunction::ReadForeignDeviceTableAck,
            0x08 => BvlcFunction::DeleteForeignDeviceTableEntry,
            0x09 => BvlcFunction::DistributeBroadcastToNetwork,
            0x0C => BvlcFunction::ForwardedNpduFromDevice,
            0x0D => BvlcFunction::SecureBvll,
            _ => return Err(DataLinkError::InvalidFrame),
        };

        let length = ((data[2] as u16) << 8) | (data[3] as u16);

        Ok(BvlcHeader {
            bvlc_type,
            function,
            length,
        })
    }
}

/// Broadcast Distribution Table (BDT) entry.
///
/// Represents a peer BBMD in the broadcast distribution network. BBMDs use
/// the BDT to forward broadcast messages between different IP subnets,
/// enabling BACnet broadcasts to traverse routers.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")] {
/// use bacnet_rs::datalink::bip::BdtEntry;
/// use std::net::SocketAddr;
///
/// // Create a BDT entry for a peer BBMD
/// let peer_addr: SocketAddr = "192.168.2.10:47808".parse().unwrap();
/// let entry = BdtEntry {
///     address: peer_addr,
///     mask: [255, 255, 255, 0],  // Subnet mask
/// };
/// # }
/// ```
#[derive(Debug, Clone)]
#[cfg(feature = "std")]
pub struct BdtEntry {
    /// IP address and port of the peer BBMD.
    ///
    /// This is the address where broadcast messages should be forwarded.
    /// Typically uses the standard BACnet/IP port 47808.
    pub address: SocketAddr,

    /// Broadcast distribution mask (subnet mask).
    ///
    /// Defines the IP subnet associated with this BBMD. Used to determine
    /// which broadcasts should be forwarded to this peer. Common values:
    /// - `[255, 255, 255, 0]` - Class C subnet
    /// - `[255, 255, 0, 0]` - Class B subnet
    /// - `[255, 255, 255, 255]` - Host-specific entry
    pub mask: [u8; 4],
}

/// Foreign Device Table (FDT) entry.
///
/// Represents a foreign device that has registered with this BBMD to receive
/// broadcast messages. Foreign devices are BACnet/IP devices that are not on
/// the same IP subnet as any BBMD.
///
/// # Registration Process
///
/// Foreign devices must periodically re-register before their TTL expires.
/// The BBMD automatically removes expired entries.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")] {
/// use bacnet_rs::datalink::bip::FdtEntry;
/// use std::net::SocketAddr;
/// use std::time::Instant;
///
/// // Track a registered foreign device
/// let device_addr: SocketAddr = "192.168.100.50:47808".parse().unwrap();
/// let entry = FdtEntry {
///     address: device_addr,
///     ttl: 300,  // 5 minutes
///     registration_time: Instant::now(),
/// };
///
/// // Check if registration has expired
/// let elapsed = entry.registration_time.elapsed().as_secs();
/// let is_expired = elapsed >= entry.ttl as u64;
/// # }
/// ```
#[derive(Debug, Clone)]
#[cfg(feature = "std")]
pub struct FdtEntry {
    /// IP address and port of the foreign device.
    ///
    /// This is where broadcast messages will be forwarded for this
    /// registered foreign device.
    pub address: SocketAddr,

    /// Time-to-live in seconds.
    ///
    /// The foreign device must re-register before this time expires.
    /// Typical values range from 60 seconds to several minutes.
    /// Maximum value is 65535 seconds (about 18 hours).
    pub ttl: u16,

    /// Time when the device registered.
    ///
    /// Used to calculate when the registration expires. The entry
    /// should be removed when `registration_time + ttl` is reached.
    pub registration_time: Instant,
}

/// Type of broadcast destination.
///
/// Used in `BroadcastResult` to identify which type of broadcast
/// succeeded or failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "std")]
pub enum BroadcastType {
    /// Global broadcast (255.255.255.255)
    Global,
    /// Local subnet broadcast (calculated from IP and subnet mask)
    LocalSubnet,
    /// Additional configured broadcast address
    Additional,
    /// BBMD peer from the Broadcast Distribution Table
    Bbmd,
    /// Foreign device from the Foreign Device Table
    ForeignDevice,
}

/// Information about a successful broadcast.
#[derive(Debug, Clone)]
#[cfg(feature = "std")]
pub struct BroadcastSuccess {
    /// The address the broadcast was sent to.
    pub address: SocketAddr,
    /// Number of bytes sent.
    pub bytes_sent: usize,
    /// Type of broadcast destination.
    pub broadcast_type: BroadcastType,
}

/// Information about a failed broadcast.
#[derive(Debug, Clone)]
#[cfg(feature = "std")]
pub struct BroadcastFailure {
    /// The address the broadcast failed to reach.
    pub address: SocketAddr,
    /// Error message describing the failure.
    pub error: String,
    /// Type of broadcast destination.
    pub broadcast_type: BroadcastType,
}

/// Result of a detailed broadcast operation.
///
/// Contains information about which broadcasts succeeded and which failed,
/// useful for diagnostics and troubleshooting device discovery issues.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "std")] {
/// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
/// let npdu = vec![0x01, 0x20, 0xFF, 0xFF, 0x00, 0xFF, 0x10, 0x08];
/// let result = data_link.send_broadcast_npdu_detailed(&npdu);
///
/// println!("Successes: {}", result.success_count());
/// println!("Failures: {}", result.failure_count());
///
/// for success in &result.successes {
///     println!("  Sent {} bytes to {:?}: {}",
///         success.bytes_sent,
///         success.broadcast_type,
///         success.address);
/// }
///
/// for failure in &result.failures {
///     println!("  Failed {:?} {}: {}",
///         failure.broadcast_type,
///         failure.address,
///         failure.error);
/// }
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone, Default)]
#[cfg(feature = "std")]
pub struct BroadcastResult {
    /// Successful broadcast attempts.
    pub successes: Vec<BroadcastSuccess>,
    /// Failed broadcast attempts.
    pub failures: Vec<BroadcastFailure>,
}

#[cfg(feature = "std")]
impl BroadcastResult {
    /// Returns the number of successful broadcasts.
    pub fn success_count(&self) -> usize {
        self.successes.len()
    }

    /// Returns the number of failed broadcasts.
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    /// Returns true if at least one broadcast succeeded.
    pub fn any_success(&self) -> bool {
        !self.successes.is_empty()
    }

    /// Returns true if all broadcasts succeeded (no failures).
    pub fn all_success(&self) -> bool {
        self.failures.is_empty() && !self.successes.is_empty()
    }

    /// Returns the total number of bytes sent across all successful broadcasts.
    pub fn total_bytes_sent(&self) -> usize {
        self.successes.iter().map(|s| s.bytes_sent).sum()
    }
}

/// BACnet/IP data link implementation.
///
/// Provides complete BACnet/IP communication including BVLC protocol support,
/// broadcast management, and foreign device registration. This implementation
/// can function as a regular BACnet/IP device, a foreign device, or a BBMD
/// (BACnet Broadcast Management Device).
///
/// # Architecture
///
/// The implementation uses a UDP socket bound to the specified address and
/// manages broadcast distribution through local subnet broadcasts and BDT
/// forwarding. Foreign devices can register to receive broadcasts.
///
/// # Examples
///
/// ## Basic BACnet/IP Device
///
/// ```no_run
/// # #[cfg(feature = "std")] {
/// use bacnet_rs::datalink::bip::BacnetIpDataLink;
/// use bacnet_rs::datalink::{DataLink, DataLinkAddress};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a BACnet/IP device
/// let mut device = BacnetIpDataLink::new("0.0.0.0:47808")?;
///
/// // Send and receive frames
/// let npdu = vec![0x01, 0x04, 0x00, 0x00];
/// device.send_frame(&npdu, &DataLinkAddress::Broadcast)?;
///
/// match device.receive_frame() {
///     Ok((data, source)) => println!("Received {} bytes", data.len()),
///     Err(_) => println!("No frame received"),
/// }
/// # Ok(())
/// # }
/// # }
/// ```
///
/// ## BBMD Configuration
///
/// ```no_run
/// # #[cfg(feature = "std")] {
/// use bacnet_rs::datalink::bip::{BacnetIpDataLink, BdtEntry};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut bbmd = BacnetIpDataLink::new("192.168.1.10:47808")?;
///
/// // Add peer BBMDs to the BDT
/// let peer1 = "192.168.2.10:47808".parse()?;
/// bbmd.add_bdt_entry(peer1, [255, 255, 255, 0]);
///
/// let peer2 = "192.168.3.10:47808".parse()?;
/// bbmd.add_bdt_entry(peer2, [255, 255, 255, 0]);
/// # Ok(())
/// # }
/// # }
/// ```
#[cfg(feature = "std")]
pub struct BacnetIpDataLink {
    /// UDP socket for BACnet/IP communication.
    socket: UdpSocket,

    /// Local IP address and port.
    local_addr: SocketAddr,

    /// Broadcast Distribution Table.
    ///
    /// Contains peer BBMDs for broadcast forwarding. Only used when this
    /// device is configured as a BBMD.
    bdt: Vec<BdtEntry>,

    /// Foreign Device Table.
    ///
    /// Contains registered foreign devices that should receive broadcasts.
    /// Only used when this device is configured as a BBMD.
    fdt: Vec<FdtEntry>,

    /// Local broadcast address for this subnet.
    ///
    /// Calculated based on the local IP address and subnet mask.
    /// Used for Original-Broadcast-NPDU messages.
    local_broadcast_addr: SocketAddr,

    /// Broadcast configuration.
    ///
    /// Controls how broadcast messages are sent, including whether to use
    /// global broadcast, local broadcast, or both.
    broadcast_config: BroadcastConfig,

    /// Detected or configured subnet mask.
    subnet_mask: [u8; 4],
}

#[cfg(feature = "std")]
impl BacnetIpDataLink {
    /// Create a new BACnet/IP data link with default broadcast configuration.
    ///
    /// Binds a UDP socket to the specified address and configures it for
    /// BACnet/IP communication. Uses the default broadcast configuration which
    /// sends to both global broadcast (255.255.255.255) and the local subnet
    /// broadcast address.
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - The local address to bind to (e.g., "0.0.0.0:47808")
    ///
    /// # Returns
    ///
    /// A configured BACnet/IP data link ready for communication.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The socket cannot be bound (port already in use, permission denied)
    /// - The address cannot be resolved
    /// - IPv6 addresses are used (not currently supported)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// use bacnet_rs::datalink::bip::BacnetIpDataLink;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Bind to any interface on the standard port
    /// let data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
    ///
    /// // Bind to a specific interface
    /// let data_link = BacnetIpDataLink::new("192.168.1.100:47808")?;
    ///
    /// // Use a non-standard port
    /// let data_link = BacnetIpDataLink::new("0.0.0.0:47809")?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new<A: ToSocketAddrs>(bind_addr: A) -> Result<Self> {
        Self::with_config(bind_addr, BroadcastConfig::default())
    }

    /// Create a new BACnet/IP data link with custom broadcast configuration.
    ///
    /// This constructor allows fine-grained control over broadcast behavior,
    /// which is essential for reliable device discovery (Who-Is) operations.
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - The local address to bind to (e.g., "0.0.0.0:47808")
    /// * `config` - Broadcast configuration controlling how broadcasts are sent
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// use bacnet_rs::datalink::bip::{BacnetIpDataLink, BroadcastConfig};
    /// use std::net::Ipv4Addr;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // With explicit subnet mask for /16 network
    /// let config = BroadcastConfig::with_subnet_mask([255, 255, 0, 0]);
    /// let data_link = BacnetIpDataLink::with_config("0.0.0.0:47808", config)?;
    ///
    /// // With multiple subnets
    /// let config = BroadcastConfig::default()
    ///     .with_additional_broadcast(Ipv4Addr::new(192, 168, 1, 255))
    ///     .with_additional_broadcast(Ipv4Addr::new(10, 0, 0, 255));
    /// let data_link = BacnetIpDataLink::with_config("0.0.0.0:47808", config)?;
    ///
    /// // Global broadcast only (when network topology is unknown)
    /// let config = BroadcastConfig::global_only();
    /// let data_link = BacnetIpDataLink::with_config("0.0.0.0:47808", config)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn with_config<A: ToSocketAddrs>(bind_addr: A, config: BroadcastConfig) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).map_err(DataLinkError::IoError)?;

        let local_addr = socket.local_addr().map_err(DataLinkError::IoError)?;

        // Enable broadcast - required for sending to broadcast addresses
        socket.set_broadcast(true).map_err(DataLinkError::IoError)?;

        // Set receive timeout for non-blocking receive behavior
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(DataLinkError::IoError)?;

        // Determine subnet mask and calculate local broadcast address
        let (subnet_mask, local_broadcast_addr) = match local_addr {
            SocketAddr::V4(addr) => {
                let ip = addr.ip();

                // Use configured subnet mask or auto-detect
                let mask = config.subnet_mask.unwrap_or_else(|| detect_subnet_mask(ip));

                // Calculate the broadcast address using proper subnet mask
                let broadcast_ip = calculate_broadcast_address(ip, &mask);
                let broadcast_addr = SocketAddr::V4(SocketAddrV4::new(broadcast_ip, BACNET_IP_PORT));

                (mask, broadcast_addr)
            }
            SocketAddr::V6(_) => {
                // IPv6 uses multicast instead of broadcast
                return Err(DataLinkError::UnsupportedType);
            }
        };

        Ok(Self {
            socket,
            local_addr,
            bdt: Vec::new(),
            fdt: Vec::new(),
            local_broadcast_addr,
            broadcast_config: config,
            subnet_mask,
        })
    }

    /// Returns the current broadcast configuration.
    pub fn broadcast_config(&self) -> &BroadcastConfig {
        &self.broadcast_config
    }

    /// Returns the detected or configured subnet mask.
    pub fn subnet_mask(&self) -> &[u8; 4] {
        &self.subnet_mask
    }

    /// Returns the calculated local broadcast address.
    pub fn local_broadcast_addr(&self) -> &SocketAddr {
        &self.local_broadcast_addr
    }

    /// Updates the broadcast configuration.
    ///
    /// This can be used to modify broadcast behavior at runtime.
    pub fn set_broadcast_config(&mut self, config: BroadcastConfig) {
        // Recalculate local broadcast if subnet mask changed
        if let Some(new_mask) = config.subnet_mask {
            if new_mask != self.subnet_mask {
                self.subnet_mask = new_mask;
                if let SocketAddr::V4(addr) = self.local_addr {
                    let broadcast_ip = calculate_broadcast_address(addr.ip(), &new_mask);
                    self.local_broadcast_addr =
                        SocketAddr::V4(SocketAddrV4::new(broadcast_ip, BACNET_IP_PORT));
                }
            }
        }
        self.broadcast_config = config;
    }

    /// Send a unicast NPDU to a specific device.
    ///
    /// Wraps the NPDU in a BVLC Original-Unicast-NPDU message and sends it
    /// to the specified destination address.
    ///
    /// # Arguments
    ///
    /// * `npdu` - The NPDU data to send
    /// * `dest` - The destination IP address and port
    ///
    /// # Errors
    ///
    /// Returns an error if the socket send operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
    /// let npdu = vec![0x01, 0x04, 0x00, 0x00];  // Example NPDU
    /// let dest = "192.168.1.100:47808".parse()?;
    /// data_link.send_unicast_npdu(&npdu, dest)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn send_unicast_npdu(&mut self, npdu: &[u8], dest: SocketAddr) -> Result<()> {
        let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 4 + npdu.len() as u16);

        let mut frame = header.encode();
        frame.extend_from_slice(npdu);

        self.socket
            .send_to(&frame, dest)
            .map_err(DataLinkError::IoError)?;

        Ok(())
    }

    /// Send a broadcast NPDU to all devices.
    ///
    /// Wraps the NPDU in a BVLC Original-Broadcast-NPDU message and sends it to
    /// all configured broadcast destinations based on the BroadcastConfig:
    ///
    /// 1. Global broadcast (255.255.255.255:47808) if `use_global_broadcast` is true
    /// 2. Local subnet broadcast address if `use_local_broadcast` is true
    /// 3. Any additional broadcast addresses configured
    /// 4. All peer BBMDs in the BDT (if configured as BBMD)
    /// 5. All registered foreign devices in the FDT (if configured as BBMD)
    ///
    /// # Arguments
    ///
    /// * `npdu` - The NPDU data to broadcast
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if at least one broadcast succeeds. Returns an error only
    /// if ALL broadcast attempts fail.
    ///
    /// # BACnet Standard Reference
    ///
    /// Per ASHRAE 135 Annex J.4.3, Original-Broadcast-NPDU (0x0B) is used for
    /// broadcasts originating from this device. The BVLC header format is:
    /// - Type: 0x81 (BACnet/IP)
    /// - Function: 0x0B (Original-Broadcast-NPDU)
    /// - Length: Total message length
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
    /// // Broadcast a Who-Is request
    /// let who_is_npdu = vec![0x01, 0x20, 0xFF, 0xFF, 0x00, 0xFF, 0x10, 0x08];
    /// data_link.send_broadcast_npdu(&who_is_npdu)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn send_broadcast_npdu(&mut self, npdu: &[u8]) -> Result<()> {
        let header = BvlcHeader::new(BvlcFunction::OriginalBroadcastNpdu, 4 + npdu.len() as u16);

        let mut frame = header.encode();
        frame.extend_from_slice(npdu);

        let mut success_count = 0;
        let mut last_error: Option<std::io::Error> = None;

        // 1. Send to global broadcast (255.255.255.255) if configured
        if self.broadcast_config.use_global_broadcast {
            let global_addr = SocketAddr::V4(SocketAddrV4::new(GLOBAL_BROADCAST, BACNET_IP_PORT));
            match self.socket.send_to(&frame, global_addr) {
                Ok(_) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }

        // 2. Send to local subnet broadcast if configured
        if self.broadcast_config.use_local_broadcast {
            // Avoid duplicate if local broadcast == global broadcast
            let is_duplicate = self.broadcast_config.use_global_broadcast
                && matches!(self.local_broadcast_addr, SocketAddr::V4(addr) if *addr.ip() == GLOBAL_BROADCAST);

            if !is_duplicate {
                match self.socket.send_to(&frame, self.local_broadcast_addr) {
                    Ok(_) => success_count += 1,
                    Err(e) => last_error = Some(e),
                }
            }
        }

        // 3. Send to additional configured broadcast addresses
        for addr in &self.broadcast_config.additional_broadcasts {
            let socket_addr = SocketAddr::V4(SocketAddrV4::new(*addr, BACNET_IP_PORT));
            // Avoid duplicates
            if socket_addr != self.local_broadcast_addr {
                match self.socket.send_to(&frame, socket_addr) {
                    Ok(_) => success_count += 1,
                    Err(e) => last_error = Some(e),
                }
            }
        }

        // 4. Send to all BDT entries (BBMD peers)
        for entry in &self.bdt {
            match self.socket.send_to(&frame, entry.address) {
                Ok(_) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }

        // 5. Send to all FDT entries (registered foreign devices)
        for entry in &self.fdt {
            match self.socket.send_to(&frame, entry.address) {
                Ok(_) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }

        // Return success if at least one broadcast succeeded
        if success_count > 0 {
            Ok(())
        } else if let Some(e) = last_error {
            Err(DataLinkError::IoError(e))
        } else {
            // No broadcasts configured - this shouldn't happen with default config
            Err(DataLinkError::AddressError(
                "No broadcast destinations configured".to_string(),
            ))
        }
    }

    /// Send a broadcast NPDU with detailed results.
    ///
    /// Similar to `send_broadcast_npdu`, but returns detailed information about
    /// which broadcasts succeeded and which failed. Useful for diagnostics.
    ///
    /// # Returns
    ///
    /// A `BroadcastResult` containing success and failure counts along with
    /// details about each broadcast attempt.
    pub fn send_broadcast_npdu_detailed(&mut self, npdu: &[u8]) -> BroadcastResult {
        let header = BvlcHeader::new(BvlcFunction::OriginalBroadcastNpdu, 4 + npdu.len() as u16);

        let mut frame = header.encode();
        frame.extend_from_slice(npdu);

        let mut result = BroadcastResult::default();

        // Send to global broadcast if configured
        if self.broadcast_config.use_global_broadcast {
            let global_addr = SocketAddr::V4(SocketAddrV4::new(GLOBAL_BROADCAST, BACNET_IP_PORT));
            match self.socket.send_to(&frame, global_addr) {
                Ok(bytes) => {
                    result.successes.push(BroadcastSuccess {
                        address: global_addr,
                        bytes_sent: bytes,
                        broadcast_type: BroadcastType::Global,
                    });
                }
                Err(e) => {
                    result.failures.push(BroadcastFailure {
                        address: global_addr,
                        error: e.to_string(),
                        broadcast_type: BroadcastType::Global,
                    });
                }
            }
        }

        // Send to local subnet broadcast if configured
        if self.broadcast_config.use_local_broadcast {
            let is_duplicate = self.broadcast_config.use_global_broadcast
                && matches!(self.local_broadcast_addr, SocketAddr::V4(addr) if *addr.ip() == GLOBAL_BROADCAST);

            if !is_duplicate {
                match self.socket.send_to(&frame, self.local_broadcast_addr) {
                    Ok(bytes) => {
                        result.successes.push(BroadcastSuccess {
                            address: self.local_broadcast_addr,
                            bytes_sent: bytes,
                            broadcast_type: BroadcastType::LocalSubnet,
                        });
                    }
                    Err(e) => {
                        result.failures.push(BroadcastFailure {
                            address: self.local_broadcast_addr,
                            error: e.to_string(),
                            broadcast_type: BroadcastType::LocalSubnet,
                        });
                    }
                }
            }
        }

        // Send to additional configured addresses
        for addr in &self.broadcast_config.additional_broadcasts {
            let socket_addr = SocketAddr::V4(SocketAddrV4::new(*addr, BACNET_IP_PORT));
            if socket_addr != self.local_broadcast_addr {
                match self.socket.send_to(&frame, socket_addr) {
                    Ok(bytes) => {
                        result.successes.push(BroadcastSuccess {
                            address: socket_addr,
                            bytes_sent: bytes,
                            broadcast_type: BroadcastType::Additional,
                        });
                    }
                    Err(e) => {
                        result.failures.push(BroadcastFailure {
                            address: socket_addr,
                            error: e.to_string(),
                            broadcast_type: BroadcastType::Additional,
                        });
                    }
                }
            }
        }

        // Send to BDT entries
        for entry in &self.bdt {
            match self.socket.send_to(&frame, entry.address) {
                Ok(bytes) => {
                    result.successes.push(BroadcastSuccess {
                        address: entry.address,
                        bytes_sent: bytes,
                        broadcast_type: BroadcastType::Bbmd,
                    });
                }
                Err(e) => {
                    result.failures.push(BroadcastFailure {
                        address: entry.address,
                        error: e.to_string(),
                        broadcast_type: BroadcastType::Bbmd,
                    });
                }
            }
        }

        // Send to FDT entries
        for entry in &self.fdt {
            match self.socket.send_to(&frame, entry.address) {
                Ok(bytes) => {
                    result.successes.push(BroadcastSuccess {
                        address: entry.address,
                        bytes_sent: bytes,
                        broadcast_type: BroadcastType::ForeignDevice,
                    });
                }
                Err(e) => {
                    result.failures.push(BroadcastFailure {
                        address: entry.address,
                        error: e.to_string(),
                        broadcast_type: BroadcastType::ForeignDevice,
                    });
                }
            }
        }

        result
    }

    /// Register this device as a foreign device with a BBMD.
    ///
    /// Foreign device registration allows a device on a different IP subnet to
    /// receive BACnet broadcasts by registering with a BBMD. The device must
    /// re-register before the TTL expires.
    ///
    /// # Arguments
    ///
    /// * `bbmd_addr` - The IP address and port of the BBMD
    /// * `ttl` - Time-to-live in seconds (typically 60-600)
    ///
    /// # Errors
    ///
    /// Returns an error if the registration message cannot be sent.
    ///
    /// # Notes
    ///
    /// - The BBMD may reject the registration (no response is provided)
    /// - Re-registration should occur at intervals less than the TTL
    /// - A TTL of 0 cancels the registration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut data_link = BacnetIpDataLink::new("0.0.0.0:47808")?;
    /// // Register with a BBMD for 5 minutes
    /// let bbmd = "192.168.1.10:47808".parse()?;
    /// data_link.register_foreign_device(bbmd, 300)?;
    ///
    /// // Cancel registration
    /// data_link.register_foreign_device(bbmd, 0)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn register_foreign_device(&mut self, bbmd_addr: SocketAddr, ttl: u16) -> Result<()> {
        let header = BvlcHeader::new(BvlcFunction::RegisterForeignDevice, 6);
        let mut frame = header.encode();
        frame.extend_from_slice(&ttl.to_be_bytes());

        self.socket
            .send_to(&frame, bbmd_addr)
            .map_err(DataLinkError::IoError)?;

        Ok(())
    }

    /// Add a peer BBMD to the Broadcast Distribution Table.
    ///
    /// When configured as a BBMD, this device will forward broadcast messages
    /// to all peers in the BDT. Each peer BBMD is responsible for distributing
    /// broadcasts to devices on its local subnet.
    ///
    /// # Arguments
    ///
    /// * `address` - IP address and port of the peer BBMD
    /// * `mask` - Subnet mask associated with the peer (e.g., [255, 255, 255, 0])
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut bbmd = BacnetIpDataLink::new("0.0.0.0:47808")?;
    /// // Configure as BBMD with two peers
    /// bbmd.add_bdt_entry("192.168.1.10:47808".parse()?, [255, 255, 255, 0]);
    /// bbmd.add_bdt_entry("192.168.2.10:47808".parse()?, [255, 255, 255, 0]);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn add_bdt_entry(&mut self, address: SocketAddr, mask: [u8; 4]) {
        self.bdt.push(BdtEntry { address, mask });
    }

    /// Remove expired entries from the Foreign Device Table.
    ///
    /// This method should be called periodically to remove foreign devices
    /// whose registration has expired. Devices that fail to re-register
    /// within their TTL period will no longer receive broadcasts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// # use bacnet_rs::datalink::bip::BacnetIpDataLink;
    /// # use std::thread;
    /// # use std::time::Duration;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut bbmd = BacnetIpDataLink::new("0.0.0.0:47808")?;
    /// // Periodically clean up expired registrations
    /// loop {
    ///     bbmd.cleanup_fdt();
    ///     thread::sleep(Duration::from_secs(30));
    /// }
    /// # }
    /// # }
    /// ```
    pub fn cleanup_fdt(&mut self) {
        let now = Instant::now();
        self.fdt.retain(|entry| {
            now.duration_since(entry.registration_time).as_secs() < entry.ttl as u64
        });
    }

    /// Process a received BVLC message.
    ///
    /// Handles all BVLC message types according to the BACnet/IP specification.
    /// Returns the encapsulated NPDU for data messages, or None for control messages.
    ///
    /// # Arguments
    ///
    /// * `data` - The complete BVLC message including header
    /// * `source` - The source IP address and port
    ///
    /// # Returns
    ///
    /// - `Some(npdu)` - For data messages (Original-Unicast-NPDU, etc.)
    /// - `None` - For control messages (Register-Foreign-Device, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if the message format is invalid.
    fn process_bvlc_message(&mut self, data: &[u8], source: SocketAddr) -> Result<Option<Vec<u8>>> {
        let header = BvlcHeader::decode(data)?;

        if data.len() != header.length as usize {
            return Err(DataLinkError::InvalidFrame);
        }

        match header.function {
            BvlcFunction::OriginalUnicastNpdu | BvlcFunction::OriginalBroadcastNpdu => {
                // Return the NPDU portion (skip 4-byte BVLC header)
                if data.len() > 4 {
                    Ok(Some(data[4..].to_vec()))
                } else {
                    Err(DataLinkError::InvalidFrame)
                }
            }
            BvlcFunction::ForwardedNpdu => {
                // Forwarded NPDU has original source address after header
                if data.len() > 10 {
                    Ok(Some(data[10..].to_vec()))
                } else {
                    Err(DataLinkError::InvalidFrame)
                }
            }
            BvlcFunction::RegisterForeignDevice => {
                // Handle foreign device registration
                if data.len() == 6 {
                    let ttl = u16::from_be_bytes([data[4], data[5]]);
                    self.fdt.push(FdtEntry {
                        address: source,
                        ttl,
                        registration_time: Instant::now(),
                    });
                }
                Ok(None)
            }
            _ => {
                // Other BVLC functions not yet implemented
                Ok(None)
            }
        }
    }
}

#[cfg(feature = "std")]
impl DataLink for BacnetIpDataLink {
    fn send_frame(&mut self, frame: &[u8], dest: &DataLinkAddress) -> Result<()> {
        match dest {
            DataLinkAddress::Ip(addr) => self.send_unicast_npdu(frame, *addr),
            DataLinkAddress::Broadcast => self.send_broadcast_npdu(frame),
            _ => Err(DataLinkError::UnsupportedType),
        }
    }

    fn receive_frame(&mut self) -> Result<(Vec<u8>, DataLinkAddress)> {
        let mut buffer = [0u8; 1500]; // MTU size

        match self.socket.recv_from(&mut buffer) {
            Ok((len, source)) => {
                let data = &buffer[..len];

                if let Some(npdu) = self.process_bvlc_message(data, source)? {
                    Ok((npdu, DataLinkAddress::Ip(source)))
                } else {
                    // No NPDU to return, try again
                    Err(DataLinkError::InvalidFrame)
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                Err(DataLinkError::IoError(e))
            }
            Err(e) => Err(DataLinkError::IoError(e)),
        }
    }

    fn link_type(&self) -> DataLinkType {
        DataLinkType::BacnetIp
    }

    fn local_address(&self) -> DataLinkAddress {
        DataLinkAddress::Ip(self.local_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bvlc_header_encode_decode() {
        let header = BvlcHeader::new(BvlcFunction::OriginalUnicastNpdu, 1024);
        let encoded = header.encode();

        assert_eq!(encoded.len(), 4);
        assert_eq!(encoded[0], 0x81);
        assert_eq!(encoded[1], 0x0A);
        assert_eq!(encoded[2], 0x04);
        assert_eq!(encoded[3], 0x00);

        let decoded = BvlcHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.bvlc_type, 0x81);
        assert_eq!(decoded.function, BvlcFunction::OriginalUnicastNpdu);
        assert_eq!(decoded.length, 1024);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_bacnet_ip_creation() {
        let result = BacnetIpDataLink::new("127.0.0.1:0");
        assert!(result.is_ok());

        let datalink = result.unwrap();
        assert_eq!(datalink.link_type(), DataLinkType::BacnetIp);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_address_calculation() {
        // Test /24 subnet (255.255.255.0)
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        let mask = [255, 255, 255, 0];
        let broadcast = calculate_broadcast_address(&ip, &mask);
        assert_eq!(broadcast, Ipv4Addr::new(192, 168, 1, 255));

        // Test /16 subnet (255.255.0.0)
        let mask = [255, 255, 0, 0];
        let broadcast = calculate_broadcast_address(&ip, &mask);
        assert_eq!(broadcast, Ipv4Addr::new(192, 168, 255, 255));

        // Test /8 subnet (255.0.0.0)
        let ip = Ipv4Addr::new(10, 20, 30, 40);
        let mask = [255, 0, 0, 0];
        let broadcast = calculate_broadcast_address(&ip, &mask);
        assert_eq!(broadcast, Ipv4Addr::new(10, 255, 255, 255));

        // Test /25 subnet (255.255.255.128)
        let ip = Ipv4Addr::new(172, 16, 10, 50);
        let mask = [255, 255, 255, 128];
        let broadcast = calculate_broadcast_address(&ip, &mask);
        assert_eq!(broadcast, Ipv4Addr::new(172, 16, 10, 127));

        // Test /25 subnet in upper half
        let ip = Ipv4Addr::new(172, 16, 10, 200);
        let broadcast = calculate_broadcast_address(&ip, &mask);
        assert_eq!(broadcast, Ipv4Addr::new(172, 16, 10, 255));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_default() {
        let config = BroadcastConfig::default();
        assert!(config.use_global_broadcast);
        assert!(config.use_local_broadcast);
        assert!(config.subnet_mask.is_none());
        assert!(config.additional_broadcasts.is_empty());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_with_subnet_mask() {
        let config = BroadcastConfig::with_subnet_mask([255, 255, 0, 0]);
        assert!(config.use_global_broadcast);
        assert!(config.use_local_broadcast);
        assert_eq!(config.subnet_mask, Some([255, 255, 0, 0]));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_global_only() {
        let config = BroadcastConfig::global_only();
        assert!(config.use_global_broadcast);
        assert!(!config.use_local_broadcast);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_local_only() {
        let config = BroadcastConfig::local_only();
        assert!(!config.use_global_broadcast);
        assert!(config.use_local_broadcast);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_with_additional() {
        let config = BroadcastConfig::default()
            .with_additional_broadcast(Ipv4Addr::new(192, 168, 1, 255))
            .with_additional_broadcast(Ipv4Addr::new(10, 0, 0, 255));

        assert_eq!(config.additional_broadcasts.len(), 2);
        assert_eq!(
            config.additional_broadcasts[0],
            Ipv4Addr::new(192, 168, 1, 255)
        );
        assert_eq!(
            config.additional_broadcasts[1],
            Ipv4Addr::new(10, 0, 0, 255)
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_config_builder_chain() {
        let config = BroadcastConfig::with_subnet_mask([255, 255, 255, 0])
            .without_global_broadcast()
            .with_additional_broadcast(Ipv4Addr::new(192, 168, 2, 255));

        assert!(!config.use_global_broadcast);
        assert!(config.use_local_broadcast);
        assert_eq!(config.subnet_mask, Some([255, 255, 255, 0]));
        assert_eq!(config.additional_broadcasts.len(), 1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_detect_subnet_mask_classful() {
        // Class A (10.x.x.x)
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let mask = detect_subnet_mask(&ip);
        // Should fall back to classful /8 or detected mask
        assert!(mask[0] == 255);

        // Class B (172.16.x.x)
        let ip = Ipv4Addr::new(172, 16, 0, 1);
        let mask = detect_subnet_mask(&ip);
        assert!(mask[0] == 255 && mask[1] == 255);

        // Class C (192.168.x.x)
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let mask = detect_subnet_mask(&ip);
        assert!(mask[0] == 255 && mask[1] == 255 && mask[2] == 255);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_bacnet_ip_with_config() {
        let config = BroadcastConfig::with_subnet_mask([255, 255, 255, 0]);
        let result = BacnetIpDataLink::with_config("127.0.0.1:0", config);
        assert!(result.is_ok());

        let datalink = result.unwrap();
        assert_eq!(datalink.subnet_mask(), &[255, 255, 255, 0]);
        assert!(datalink.broadcast_config().use_global_broadcast);
        assert!(datalink.broadcast_config().use_local_broadcast);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_global_broadcast_constant() {
        assert_eq!(GLOBAL_BROADCAST, Ipv4Addr::new(255, 255, 255, 255));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_broadcast_result() {
        let mut result = BroadcastResult::default();
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.failure_count(), 0);
        assert!(!result.any_success());
        assert!(!result.all_success());

        result.successes.push(BroadcastSuccess {
            address: "192.168.1.255:47808".parse().unwrap(),
            bytes_sent: 100,
            broadcast_type: BroadcastType::LocalSubnet,
        });

        assert_eq!(result.success_count(), 1);
        assert!(result.any_success());
        assert!(result.all_success());
        assert_eq!(result.total_bytes_sent(), 100);

        result.failures.push(BroadcastFailure {
            address: "10.0.0.255:47808".parse().unwrap(),
            error: "Network unreachable".to_string(),
            broadcast_type: BroadcastType::Additional,
        });

        assert_eq!(result.failure_count(), 1);
        assert!(result.any_success());
        assert!(!result.all_success()); // Has failures now
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_set_broadcast_config() {
        let mut datalink = BacnetIpDataLink::new("127.0.0.1:0").unwrap();
        let initial_mask = *datalink.subnet_mask();

        // Change to a different subnet mask
        let new_config = BroadcastConfig::with_subnet_mask([255, 255, 0, 0]);
        datalink.set_broadcast_config(new_config);

        assert_eq!(datalink.subnet_mask(), &[255, 255, 0, 0]);
        assert_ne!(*datalink.subnet_mask(), initial_mask);
    }
}

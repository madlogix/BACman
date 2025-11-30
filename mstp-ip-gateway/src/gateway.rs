//! BACnet Gateway - Routes messages between MS/TP and BACnet/IP networks
//!
//! This module implements a BACnet router between MS/TP and BACnet/IP networks,
//! following ASHRAE 135-2024 requirements for network layer routing.

use log::{debug, info, trace, warn};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bacnet_rs::app::{Apdu, SegmentationManager};
use bacnet_rs::service::{AbortReason, ConfirmedServiceChoice};
use crate::config::{BdtEntryConfig, NetworkTablePersistence, RoutingTableEntryConfig};
use crate::transaction::{PendingTransaction, TransactionTable, TransactionStats};
use esp_idf_svc::nvs::{EspNvsPartition, NvsDefault};

/// BACnet/IP BVLC function codes (ASHRAE 135 Annex J)
const BVLC_RESULT: u8 = 0x00;
const BVLC_WRITE_BDT: u8 = 0x01;
const BVLC_READ_BDT: u8 = 0x02;
const BVLC_READ_BDT_ACK: u8 = 0x03;
const BVLC_FORWARDED_NPDU: u8 = 0x04;
const BVLC_REGISTER_FOREIGN_DEVICE: u8 = 0x05;
const BVLC_READ_FDT: u8 = 0x06;
const BVLC_READ_FDT_ACK: u8 = 0x07;
const BVLC_DELETE_FDT_ENTRY: u8 = 0x08;
const BVLC_DISTRIBUTE_BROADCAST: u8 = 0x09;
const BVLC_ORIGINAL_UNICAST: u8 = 0x0A;
const BVLC_ORIGINAL_BROADCAST: u8 = 0x0B;

/// Network layer message types (ASHRAE 135 Clause 6)
const NL_WHO_IS_ROUTER_TO_NETWORK: u8 = 0x00;
const NL_I_AM_ROUTER_TO_NETWORK: u8 = 0x01;
const NL_REJECT_MESSAGE_TO_NETWORK: u8 = 0x03;
const NL_INITIALIZE_ROUTING_TABLE: u8 = 0x06;
const NL_INITIALIZE_ROUTING_TABLE_ACK: u8 = 0x07;

/// Reject-Message-To-Network reason codes (ASHRAE 135 Annex R)
/// All codes are defined per the BACnet standard, though not all are currently used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum RejectReason {
    /// Other error
    Other = 0,
    /// The router is not directly connected to DNET and cannot find a router to DNET
    NotRouterToDnet = 1,
    /// The router is busy and unable to process the message
    RouterBusy = 2,
    /// Unknown network layer message type
    UnknownNetworkMessage = 3,
    /// The message is too long to be routed
    MessageTooLong = 4,
    /// Security error
    SecurityError = 5,
    /// Addressing error (e.g., invalid DADR)
    AddressingError = 6,
}

/// BVLC Result codes
const BVLC_RESULT_SUCCESS: u16 = 0x0000;
const BVLC_RESULT_WRITE_BDT_NAK: u16 = 0x0010;
const BVLC_RESULT_READ_BDT_NAK: u16 = 0x0020;
const BVLC_RESULT_REGISTER_FD_NAK: u16 = 0x0030;
const BVLC_RESULT_READ_FDT_NAK: u16 = 0x0040;
const BVLC_RESULT_DELETE_FDT_NAK: u16 = 0x0050;
const BVLC_RESULT_DISTRIBUTE_NAK: u16 = 0x0060;

/// Default address table entry age (1 hour)
const DEFAULT_ADDRESS_AGE: Duration = Duration::from_secs(3600);

/// Default foreign device TTL (30 seconds per ASHRAE 135 Annex J)
const DEFAULT_FD_TTL: Duration = Duration::from_secs(30);

/// Minimum hop count for routing (ASHRAE 135)
const MIN_HOP_COUNT: u8 = 1;

/// Address table entry with timestamp for aging
#[derive(Debug, Clone)]
struct AddressEntry<T> {
    address: T,
    last_seen: Instant,
}

/// Foreign Device Table entry (ASHRAE 135 Annex J.5)
#[derive(Debug, Clone)]
struct ForeignDeviceEntry {
    /// IP address of the foreign device
    address: SocketAddr,
    /// Time-to-live remaining (in seconds)
    ttl_seconds: u16,
    /// Time when entry was registered/refreshed
    registered_at: Instant,
}

/// Broadcast Distribution Table entry (ASHRAE 135 Annex J.3)
/// Represents a peer BBMD for broadcast distribution across subnets
#[derive(Debug, Clone)]
struct BdtEntry {
    /// IP address and port of the peer BBMD
    address: SocketAddr,
    /// Broadcast distribution mask (subnet mask)
    /// Common values: [255,255,255,0] for /24, [255,255,255,255] for host-specific
    mask: Ipv4Addr,
}

/// Routing table entry for Initialize-Routing-Table (ASHRAE 135 Clause 6.4)
#[derive(Debug, Clone)]
struct RoutingTableEntry {
    /// Destination network number
    network: u16,
    /// Port ID (0 if directly connected)
    port_id: u8,
    /// Port information (MAC address length + MAC address bytes)
    port_info: Vec<u8>,
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

impl ForeignDeviceEntry {
    fn new(address: SocketAddr, ttl_seconds: u16) -> Self {
        Self {
            address,
            ttl_seconds,
            registered_at: Instant::now(),
        }
    }

    /// Refresh registration with new TTL
    fn refresh(&mut self, ttl_seconds: u16) {
        self.ttl_seconds = ttl_seconds;
        self.registered_at = Instant::now();
    }

    /// Check if entry has expired based on TTL
    fn is_expired(&self) -> bool {
        self.registered_at.elapsed() > Duration::from_secs(self.ttl_seconds as u64)
    }

    /// Get remaining TTL in seconds
    fn remaining_ttl(&self) -> u16 {
        let elapsed = self.registered_at.elapsed().as_secs() as u16;
        self.ttl_seconds.saturating_sub(elapsed)
    }
}

/// Information stored from first segment for APDU reconstruction
#[derive(Debug, Clone)]
struct SegmentedRequestInfo {
    /// Service choice from first segment
    service_choice: u8,
    /// Max APDU length accepted (from first segment header)
    max_apdu_accepted: u8,
    /// Whether segmented response is accepted
    segmented_response_accepted: bool,
    /// Original NPDU data for routing
    npdu_data: Vec<u8>,
    /// Source IP address
    source_addr: SocketAddr,
    /// Timestamp when first segment was received
    created_at: Instant,
}

/// Segment transmission tracking for retransmission
#[derive(Debug, Clone)]
struct SegmentTransmission {
    /// Invoke ID
    invoke_id: u8,
    /// Sequence number of this segment
    sequence_number: u8,
    /// Segment data (full APDU segment)
    segment_data: Vec<u8>,
    /// Destination address
    dest_addr: SocketAddr,
    /// Timestamp when segment was sent
    sent_at: Instant,
    /// Number of retransmission attempts
    retry_count: u8,
    /// Whether ACK has been received for this segment
    acked: bool,
}

/// BACnet Gateway
pub struct BacnetGateway {
    // Network configuration
    mstp_network: u16,
    ip_network: u16,

    // Local IP address for Forwarded-NPDU
    local_ip: Ipv4Addr,
    local_port: u16,

    // Subnet mask for directed broadcast calculation
    subnet_mask: Ipv4Addr,

    // Address translation tables with aging
    mstp_to_ip: HashMap<u8, AddressEntry<SocketAddr>>,
    ip_to_mstp: HashMap<SocketAddr, AddressEntry<u8>>,

    // Foreign Device Table (ASHRAE 135 Annex J.5)
    // Key is IP address to prevent duplicates on re-registration
    foreign_device_table: HashMap<SocketAddr, ForeignDeviceEntry>,

    // Broadcast Distribution Table (ASHRAE 135 Annex J.3)
    // List of peer BBMDs for broadcast distribution across subnets
    broadcast_distribution_table: Vec<BdtEntry>,

    // Routing table for Initialize-Routing-Table (ASHRAE 135 Clause 6.4)
    // Key is destination network number
    routing_table: HashMap<u16, RoutingTableEntry>,

    // Address aging configuration
    address_max_age: Duration,

    // Pending transmissions for IP side
    ip_send_queue: Vec<(Vec<u8>, SocketAddr)>,

    // Pending transmissions for MS/TP side (used for retries)
    // Each entry: (npdu_data, dest_mac)
    mstp_send_queue: Vec<(Vec<u8>, u8)>,

    // Statistics
    stats: GatewayStats,

    // NVS partition for BDT and routing table persistence
    nvs_partition: Option<EspNvsPartition<NvsDefault>>,

    // UDP socket for sending (shared with receive thread via Arc)
    ip_socket: Option<Arc<UdpSocket>>,

    // Router announcement sent flag
    router_announced: bool,

    // Transaction tracking for confirmed services
    transactions: TransactionTable,

    // Segmentation manager for reassembling large messages
    segmentation: SegmentationManager,

    // Segmented request header info (keyed by invoke_id)
    // Used to reconstruct APDU after reassembly
    segmented_request_info: HashMap<u8, SegmentedRequestInfo>,

    // Segment transmission tracking for retransmission
    // Key is (invoke_id, sequence_number)
    segment_transmissions: HashMap<(u8, u8), SegmentTransmission>,
}

/// Gateway statistics
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct GatewayStats {
    // Traffic counters
    pub mstp_to_ip_packets: u64,
    pub ip_to_mstp_packets: u64,
    pub routing_errors: u64,
    pub transaction_timeouts: u64,

    // Byte counters
    pub mstp_to_ip_bytes: u64,
    pub ip_to_mstp_bytes: u64,

    // Activity timestamps
    pub last_activity: Option<Instant>,
    pub last_mstp_activity: Option<Instant>,
    pub last_ip_activity: Option<Instant>,

    // Network health status
    pub mstp_network_up: bool,
    pub ip_network_up: bool,
}

#[allow(dead_code)]
impl BacnetGateway {
    /// Create a new gateway with local IP configuration and subnet mask
    pub fn new(
        mstp_network: u16,
        ip_network: u16,
        local_ip: Ipv4Addr,
        local_port: u16,
        subnet_mask: Ipv4Addr,
    ) -> Self {
        let broadcast = Self::calculate_broadcast_address(local_ip, subnet_mask);
        info!(
            "Creating BACnet gateway: MS/TP network {} <-> IP network {} (local {}:{}, broadcast {})",
            mstp_network, ip_network, local_ip, local_port, broadcast
        );

        Self {
            mstp_network,
            ip_network,
            local_ip,
            local_port,
            subnet_mask,
            mstp_to_ip: HashMap::new(),
            ip_to_mstp: HashMap::new(),
            foreign_device_table: HashMap::new(),
            broadcast_distribution_table: Vec::new(),
            routing_table: HashMap::new(),
            address_max_age: DEFAULT_ADDRESS_AGE,
            ip_send_queue: Vec::new(),
            mstp_send_queue: Vec::new(),
            stats: GatewayStats::default(),
            nvs_partition: None,
            ip_socket: None,
            router_announced: false,
            transactions: TransactionTable::new(),
            segmentation: SegmentationManager::new(),
            segmented_request_info: HashMap::new(),
            segment_transmissions: HashMap::new(),
        }
    }

    /// Create a new gateway with default port (47808) and default /24 subnet
    pub fn new_default(mstp_network: u16, ip_network: u16, local_ip: Ipv4Addr) -> Self {
        Self::new(
            mstp_network,
            ip_network,
            local_ip,
            47808,
            Ipv4Addr::new(255, 255, 255, 0), // Default /24 subnet
        )
    }

    /// Calculate directed broadcast address from IP and subnet mask
    fn calculate_broadcast_address(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
        let ip_octets = ip.octets();
        let mask_octets = mask.octets();

        // Broadcast = IP OR (NOT mask)
        Ipv4Addr::new(
            ip_octets[0] | !mask_octets[0],
            ip_octets[1] | !mask_octets[1],
            ip_octets[2] | !mask_octets[2],
            ip_octets[3] | !mask_octets[3],
        )
    }

    /// Set the subnet mask and recalculate broadcast address
    pub fn set_subnet_mask(&mut self, mask: Ipv4Addr) {
        self.subnet_mask = mask;
        let broadcast = Self::calculate_broadcast_address(self.local_ip, mask);
        info!("Updated subnet mask to {}, broadcast: {}", mask, broadcast);
    }

    /// Update the local IP address (used when switching between station and AP mode)
    pub fn set_local_ip(&mut self, ip: Ipv4Addr, mask: Ipv4Addr) {
        self.local_ip = ip;
        self.subnet_mask = mask;
        let broadcast = Self::calculate_broadcast_address(ip, mask);
        info!(
            "Updated gateway local IP to {}, subnet {}, broadcast {}",
            ip, mask, broadcast
        );
    }

    /// Set custom address aging timeout
    pub fn set_address_max_age(&mut self, max_age: Duration) {
        self.address_max_age = max_age;
    }

    /// Set NVS partition for BDT and routing table persistence
    /// Loads existing BDT and routing table from NVS if available
    pub fn set_nvs_partition(&mut self, partition: EspNvsPartition<NvsDefault>) {
        // Load existing BDT from NVS
        if let Ok(bdt_entries) = NetworkTablePersistence::load_bdt(partition.clone()) {
            if !bdt_entries.is_empty() {
                self.broadcast_distribution_table = bdt_entries
                    .into_iter()
                    .map(|e| BdtEntry {
                        address: e.address,
                        mask: Self::u32_to_ipv4(e.broadcast_mask),
                    })
                    .collect();
                info!("Loaded {} BDT entries from NVS", self.broadcast_distribution_table.len());
            }
        }

        // Load existing routing table from NVS
        if let Ok(rt_entries) = NetworkTablePersistence::load_routing_table(partition.clone()) {
            if !rt_entries.is_empty() {
                self.routing_table.clear();
                for entry in rt_entries {
                    self.routing_table.insert(entry.network, RoutingTableEntry {
                        network: entry.network,
                        port_id: entry.port_id,
                        port_info: entry.port_info,
                    });
                }
                info!("Loaded {} routing table entries from NVS", self.routing_table.len());
            }
        }

        self.nvs_partition = Some(partition);
    }

    /// Save current BDT to NVS
    fn save_bdt_to_nvs(&self) {
        if let Some(ref partition) = self.nvs_partition {
            let entries: Vec<BdtEntryConfig> = self.broadcast_distribution_table
                .iter()
                .map(|e| BdtEntryConfig {
                    address: e.address,
                    broadcast_mask: Self::ipv4_to_u32(e.mask),
                })
                .collect();
            if let Err(e) = NetworkTablePersistence::save_bdt(partition.clone(), &entries) {
                warn!("Failed to save BDT to NVS: {}", e);
            }
        }
    }

    /// Save current routing table to NVS
    fn save_routing_table_to_nvs(&self) {
        if let Some(ref partition) = self.nvs_partition {
            let entries: Vec<RoutingTableEntryConfig> = self.routing_table
                .values()
                .map(|e| RoutingTableEntryConfig {
                    network: e.network,
                    port_id: e.port_id,
                    port_info: e.port_info.clone(),
                })
                .collect();
            if let Err(e) = NetworkTablePersistence::save_routing_table(partition.clone(), &entries) {
                warn!("Failed to save routing table to NVS: {}", e);
            }
        }
    }

    /// Convert Ipv4Addr to u32 (network byte order)
    fn ipv4_to_u32(ip: Ipv4Addr) -> u32 {
        let octets = ip.octets();
        ((octets[0] as u32) << 24) | ((octets[1] as u32) << 16) | ((octets[2] as u32) << 8) | (octets[3] as u32)
    }

    /// Convert u32 (network byte order) to Ipv4Addr
    fn u32_to_ipv4(val: u32) -> Ipv4Addr {
        Ipv4Addr::new(
            ((val >> 24) & 0xFF) as u8,
            ((val >> 16) & 0xFF) as u8,
            ((val >> 8) & 0xFF) as u8,
            (val & 0xFF) as u8,
        )
    }

    /// Get BDT entries for web UI
    pub fn get_bdt_entries(&self) -> Vec<(SocketAddr, Ipv4Addr)> {
        self.broadcast_distribution_table
            .iter()
            .map(|e| (e.address, e.mask))
            .collect()
    }

    /// Add a BDT entry (for web UI) and persist to NVS
    pub fn add_bdt_entry(&mut self, address: SocketAddr, mask: Ipv4Addr) {
        // Check if entry already exists
        if !self.broadcast_distribution_table.iter().any(|e| e.address == address) {
            self.broadcast_distribution_table.push(BdtEntry { address, mask });
            info!("Added BDT entry: {} mask {}", address, mask);
            self.save_bdt_to_nvs();
        }
    }

    /// Remove a BDT entry (for web UI) and persist to NVS
    pub fn remove_bdt_entry(&mut self, address: SocketAddr) {
        let before = self.broadcast_distribution_table.len();
        self.broadcast_distribution_table.retain(|e| e.address != address);
        if self.broadcast_distribution_table.len() < before {
            info!("Removed BDT entry: {}", address);
            self.save_bdt_to_nvs();
        }
    }

    /// Clear all BDT entries and persist to NVS
    pub fn clear_bdt(&mut self) {
        self.broadcast_distribution_table.clear();
        info!("Cleared all BDT entries");
        self.save_bdt_to_nvs();
    }

    /// Get routing table entries for web UI
    pub fn get_routing_table_entries(&self) -> Vec<(u16, u8, Vec<u8>)> {
        self.routing_table
            .values()
            .map(|e| (e.network, e.port_id, e.port_info.clone()))
            .collect()
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

    /// Set the IP socket for sending (shared with receive thread)
    pub fn set_ip_socket(&mut self, socket: Arc<UdpSocket>) {
        // Drain any queued packets that were waiting for the socket
        let queued: Vec<_> = self.ip_send_queue.drain(..).collect();
        if !queued.is_empty() {
            info!("Draining {} queued IP packets after socket set", queued.len());
            for (data, dest) in queued {
                if let Err(e) = socket.send_to(&data, dest) {
                    warn!("Failed to send queued packet to {}: {}", dest, e);
                }
            }
        }
        self.ip_socket = Some(socket);
    }

    /// Process transaction timeouts and retry or send Abort PDUs to clients
    ///
    /// This should be called periodically (e.g., every 1 second) from the main loop.
    /// Returns the number of transactions that timed out.
    ///
    /// Implements retry mechanism per Phase 5.4:
    /// - If retries remaining: retransmit NPDU to MS/TP and re-add transaction with backoff
    /// - If retries exhausted: send Abort to IP client
    pub fn process_transaction_timeouts(&mut self) -> usize {
        let timed_out = self.transactions.check_timeouts();
        let count = timed_out.len();

        for tx in timed_out {
            if tx.retries < tx.max_retries {
                // Retries remaining - retransmit to MS/TP
                info!(
                    "Transaction timeout, retrying: invoke_id={} service={:?} dest={}:{} retry={}/{} age={:.1}s",
                    tx.invoke_id,
                    tx.service,
                    tx.dest_network,
                    tx.dest_mac,
                    tx.retries + 1,
                    tx.max_retries,
                    tx.created_at.elapsed().as_secs_f32()
                );

                // Queue NPDU for retransmission to MS/TP
                // The original_npdu already has proper routing info (SNET/SADR)
                self.queue_mstp_retransmit(tx.original_npdu.clone(), tx.dest_mac);

                // Re-add transaction with incremented retry count and exponential backoff
                if let Err(e) = self.transactions.retry(tx) {
                    warn!(
                        "Failed to re-add transaction for retry: {}",
                        e
                    );
                }
            } else {
                // Retries exhausted - send Abort PDU to IP client
                warn!(
                    "Transaction retries exhausted: invoke_id={} service={:?} dest={}:{} total_age={:.1}s",
                    tx.invoke_id,
                    tx.service,
                    tx.dest_network,
                    tx.dest_mac,
                    tx.created_at.elapsed().as_secs_f32()
                );

                // Track timeout in statistics
                self.stats.transaction_timeouts += 1;

                if let Err(e) = self.send_abort_to_client(&tx, AbortReason::Other) {
                    warn!(
                        "Failed to send timeout abort to {}: {}",
                        tx.source_addr, e
                    );
                }
            }
        }

        if count > 0 {
            debug!("Processed {} transaction timeout(s)", count);
        }

        count
    }

    /// Queue an NPDU for retransmission to MS/TP
    ///
    /// This is used by the retry mechanism to re-send timed-out requests.
    fn queue_mstp_retransmit(&mut self, npdu: Vec<u8>, dest_mac: u8) {
        debug!(
            "Queuing MS/TP retransmit: {} bytes to MAC {} (queue_len={})",
            npdu.len(),
            dest_mac,
            self.mstp_send_queue.len() + 1
        );
        self.mstp_send_queue.push((npdu, dest_mac));
    }

    /// Drain the MS/TP send queue and return all pending transmissions
    ///
    /// The caller (main loop) should call this periodically and send the frames
    /// via the MS/TP driver.
    pub fn drain_mstp_send_queue(&mut self) -> Vec<(Vec<u8>, u8)> {
        self.mstp_send_queue.drain(..).collect()
    }

    /// Send an Abort PDU to the IP client for a timed-out transaction
    fn send_abort_to_client(
        &mut self,
        tx: &PendingTransaction,
        reason: AbortReason,
    ) -> Result<(), GatewayError> {
        // Build Abort APDU
        let abort_apdu = Apdu::Abort {
            server: true,  // Gateway is acting as server (forwarding abort)
            invoke_id: tx.invoke_id,
            abort_reason: reason as u8,
        };

        let apdu_bytes = abort_apdu.encode();

        // Build NPDU (simple local response, no routing info needed)
        let mut npdu = Vec::with_capacity(apdu_bytes.len() + 2);
        npdu.push(0x01); // NPDU version
        npdu.push(0x00); // Control: no routing info, expecting reply = false
        npdu.extend_from_slice(&apdu_bytes);

        // Build BVLC wrapper (Original-Unicast-NPDU)
        let bvlc = build_bvlc(&npdu, false);

        // Send to original client
        debug!(
            "Sending timeout Abort to {}: invoke_id={} reason={:?}",
            tx.source_addr, tx.invoke_id, reason
        );

        self.send_ip_packet(&bvlc, tx.source_addr)
    }

    /// Get transaction table statistics
    pub fn get_transaction_stats(&self) -> &TransactionStats {
        self.transactions.stats()
    }

    /// Get number of active transactions
    pub fn active_transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Process a segmented request from IP and reassemble
    ///
    /// Returns:
    /// - Ok(Some((complete_apdu, npdu_data))) if reassembly is complete
    /// - Ok(None) if more segments are needed (SegmentAck sent)
    /// - Err if there's a protocol error
    ///
    /// The `first_segment_info` should be provided only for sequence number 0 and contains
    /// the APDU header info needed to reconstruct the complete non-segmented APDU.
    fn process_segmented_request(
        &mut self,
        invoke_id: u8,
        sequence_number: u8,
        proposed_window_size: u8,
        segment_data: &[u8],
        more_follows: bool,
        source_addr: SocketAddr,
        first_segment_info: Option<(u8, u8, bool, Vec<u8>)>, // (service_choice, max_apdu, seg_resp_accepted, npdu_data)
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, GatewayError> {
        // Use default max APDU length (1476 for BACnet/IP)
        const MAX_APDU_LENGTH: u16 = 1476;

        // Store header info from first segment
        if let Some((service_choice, max_apdu_accepted, segmented_response_accepted, npdu_data)) = first_segment_info {
            self.segmented_request_info.insert(
                invoke_id,
                SegmentedRequestInfo {
                    service_choice,
                    max_apdu_accepted,
                    segmented_response_accepted,
                    npdu_data,
                    source_addr,
                    created_at: Instant::now(),
                },
            );
            debug!(
                "Stored segmented request info: invoke_id={} service={}",
                invoke_id, service_choice
            );
        }

        // Process the segment
        match self.segmentation.process_segment(
            invoke_id,
            sequence_number,
            segment_data.to_vec(),
            more_follows,
            MAX_APDU_LENGTH,
        ) {
            Ok(Some(complete_service_data)) => {
                // Reassembly complete - send final SegmentAck
                debug!(
                    "Segment reassembly complete: invoke_id={} total_size={}",
                    invoke_id,
                    complete_service_data.len()
                );
                self.send_segment_ack(
                    invoke_id,
                    sequence_number,
                    proposed_window_size,
                    false, // positive ack
                    source_addr,
                )?;

                // Retrieve stored header info and build complete APDU
                if let Some(info) = self.segmented_request_info.remove(&invoke_id) {
                    // Build non-segmented ConfirmedRequest APDU
                    // Format: type/flags(1) + max_apdu(1) + invoke_id(1) + service(1) + service_data
                    let mut complete_apdu = Vec::with_capacity(4 + complete_service_data.len());

                    // Type byte: PDU Type=0 (ConfirmedRequest), no segmentation
                    // Bit 1 (0x02) = segmented_response_accepted
                    let mut type_byte: u8 = 0x00; // ConfirmedRequest, not segmented
                    if info.segmented_response_accepted {
                        type_byte |= 0x02;
                    }
                    complete_apdu.push(type_byte);

                    // Max APDU length accepted
                    complete_apdu.push(info.max_apdu_accepted);

                    // Invoke ID
                    complete_apdu.push(invoke_id);

                    // Service choice
                    complete_apdu.push(info.service_choice);

                    // Service data (reassembled)
                    complete_apdu.extend_from_slice(&complete_service_data);

                    info!(
                        "Reassembled APDU: invoke_id={} service={} total_len={} (from {} segments)",
                        invoke_id,
                        info.service_choice,
                        complete_apdu.len(),
                        sequence_number + 1
                    );

                    Ok(Some((complete_apdu, info.npdu_data)))
                } else {
                    // No header info stored - shouldn't happen
                    warn!("No header info found for completed segmented request: invoke_id={}", invoke_id);
                    Err(GatewayError::NpduError("Missing segmented request info".to_string()))
                }
            }
            Ok(None) => {
                // More segments needed - send SegmentAck
                debug!(
                    "Segment received: invoke_id={} seq={} more_follows={}",
                    invoke_id, sequence_number, more_follows
                );
                self.send_segment_ack(
                    invoke_id,
                    sequence_number,
                    proposed_window_size,
                    false, // positive ack
                    source_addr,
                )?;
                Ok(None)
            }
            Err(e) => {
                warn!("Segment processing error: {:?}", e);
                // Clean up stored info on error
                self.segmented_request_info.remove(&invoke_id);
                // Send negative SegmentAck
                self.send_segment_ack(
                    invoke_id,
                    sequence_number,
                    proposed_window_size,
                    true, // negative ack
                    source_addr,
                )?;
                Err(GatewayError::NpduError(format!("Segmentation error: {:?}", e)))
            }
        }
    }

    /// Send a SegmentAck PDU to an IP client
    fn send_segment_ack(
        &mut self,
        invoke_id: u8,
        sequence_number: u8,
        window_size: u8,
        negative: bool,
        dest: SocketAddr,
    ) -> Result<(), GatewayError> {
        // Build SegmentAck APDU
        let segment_ack = Apdu::SegmentAck {
            negative,
            server: true, // Gateway is acting as server
            invoke_id,
            sequence_number,
            window_size: window_size.max(1), // Minimum window size is 1
        };

        let apdu_bytes = segment_ack.encode();

        // Build NPDU (simple local response)
        let mut npdu = Vec::with_capacity(apdu_bytes.len() + 2);
        npdu.push(0x01); // NPDU version
        npdu.push(0x00); // Control: no routing info
        npdu.extend_from_slice(&apdu_bytes);

        // Build BVLC wrapper
        let bvlc = build_bvlc(&npdu, false);

        trace!(
            "Sending SegmentAck to {}: invoke_id={} seq={} negative={}",
            dest, invoke_id, sequence_number, negative
        );

        self.send_ip_packet(&bvlc, dest)
    }

    /// Cleanup timed out segment reassembly buffers
    /// Call this periodically (e.g., every 10 seconds)
    pub fn cleanup_segment_buffers(&mut self) {
        self.segmentation.cleanup_timed_out_buffers();

        // Also clean up stale segmented request info (60 second timeout)
        const SEGMENT_INFO_TIMEOUT: Duration = Duration::from_secs(60);
        self.segmented_request_info.retain(|invoke_id, info| {
            let keep = info.created_at.elapsed() < SEGMENT_INFO_TIMEOUT;
            if !keep {
                debug!(
                    "Cleaned up stale segmented request info: invoke_id={}",
                    invoke_id
                );
            }
            keep
        });
    }

    /// Get number of active segment reassemblies
    pub fn active_reassemblies(&self) -> usize {
        self.segmentation.active_reassemblies()
    }

    /// Handle incoming Segment-ACK (marks segments as acknowledged)
    pub fn handle_segment_ack(&mut self, invoke_id: u8, sequence_number: u8, negative: bool) {
        if negative {
            // Segment-NAK: retransmit the requested segment
            if let Some(segment) = self.segment_transmissions.get_mut(&(invoke_id, sequence_number)) {
                debug!(
                    "Segment-NAK received: invoke_id={} seq={}, retransmitting",
                    invoke_id, sequence_number
                );
                segment.retry_count += 1;
                segment.sent_at = Instant::now();
                // Retransmit will happen in check_segment_timeouts
            } else {
                warn!(
                    "Segment-NAK for unknown segment: invoke_id={} seq={}",
                    invoke_id, sequence_number
                );
            }
        } else {
            // Positive ACK: mark segments up to sequence_number as acknowledged
            let mut to_remove = Vec::new();
            for (&(seg_invoke_id, seg_seq), segment) in &mut self.segment_transmissions {
                if seg_invoke_id == invoke_id && seg_seq <= sequence_number {
                    segment.acked = true;
                    to_remove.push((seg_invoke_id, seg_seq));
                }
            }
            // Remove acknowledged segments
            for key in to_remove {
                self.segment_transmissions.remove(&key);
                trace!("Segment acknowledged: invoke_id={} seq={}", key.0, key.1);
            }
        }
    }

    /// Check for segment transmission timeouts and retransmit if needed
    /// Call this periodically (e.g., every second)
    pub fn check_segment_timeouts(&mut self) -> Result<(), GatewayError> {
        const SEGMENT_TIMEOUT: Duration = Duration::from_secs(3);
        const MAX_RETRIES: u8 = 3;

        let mut to_retransmit = Vec::new();
        let mut to_remove = Vec::new();

        for (&key, segment) in &self.segment_transmissions {
            if segment.acked {
                continue;
            }

            if segment.sent_at.elapsed() > SEGMENT_TIMEOUT {
                if segment.retry_count >= MAX_RETRIES {
                    warn!(
                        "Segment transmission failed after {} retries: invoke_id={} seq={}",
                        MAX_RETRIES, segment.invoke_id, segment.sequence_number
                    );
                    to_remove.push(key);
                } else {
                    debug!(
                        "Segment timeout, retransmitting: invoke_id={} seq={} retry={}",
                        segment.invoke_id, segment.sequence_number, segment.retry_count + 1
                    );
                    to_retransmit.push((key, segment.segment_data.clone(), segment.dest_addr));
                }
            }
        }

        // Retransmit timed-out segments
        for ((invoke_id, seq), data, dest) in to_retransmit {
            if let Some(segment) = self.segment_transmissions.get_mut(&(invoke_id, seq)) {
                segment.retry_count += 1;
                segment.sent_at = Instant::now();
                self.send_ip_packet(&data, dest)?;
            }
        }

        // Remove failed segments
        for key in to_remove {
            self.segment_transmissions.remove(&key);
        }

        Ok(())
    }

    /// Track a transmitted segment for retransmission
    fn track_segment_transmission(
        &mut self,
        invoke_id: u8,
        sequence_number: u8,
        segment_data: Vec<u8>,
        dest_addr: SocketAddr,
    ) {
        self.segment_transmissions.insert(
            (invoke_id, sequence_number),
            SegmentTransmission {
                invoke_id,
                sequence_number,
                segment_data,
                dest_addr,
                sent_at: Instant::now(),
                retry_count: 0,
                acked: false,
            },
        );
    }

    /// Route a frame from MS/TP to IP
    ///
    /// Returns `Ok(None)` on success, or `Ok(Some((reject_npdu, dest_addr)))` if a reject
    /// message should be sent back to the MS/TP source.
    pub fn route_from_mstp(&mut self, data: &[u8], source_addr: u8) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        if data.len() < 2 {
            warn!(
                "Malformed packet from MS/TP {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 64)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Parse NPDU
        let (npdu, _npdu_len) = match parse_npdu(data) {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    "Failed to parse NPDU from MS/TP {}: {} - {}",
                    source_addr,
                    e,
                    hex_dump(data, 64)
                );
                self.stats.routing_errors += 1;
                return Err(e);
            }
        };

        // Validate hop count before routing (ASHRAE 135 Clause 6.2.2)
        // If hop count reaches 0, message must be discarded
        if let Some(hop_count) = npdu.hop_count {
            if hop_count < MIN_HOP_COUNT {
                warn!(
                    "Discarding message from MS/TP {}: hop count exhausted (was {}) - {}",
                    source_addr,
                    hop_count,
                    hex_dump(data, 32)
                );
                self.stats.routing_errors += 1;
                return Err(GatewayError::HopCountExhausted);
            }
        }

        info!(
            "MS/TP->IP route: src_mac={} network_msg={} dest_present={} hop_count={:?}",
            source_addr, npdu.network_message, npdu.destination_present, npdu.hop_count
        );

        // Handle network layer messages (Who-Is-Router-To-Network, etc.)
        if npdu.network_message {
            return self.handle_network_message_from_mstp(data, &npdu, source_addr)
                .map(|()| None);
        }

        // Parse APDU for transaction tracking and response routing
        let apdu_data = &data[_npdu_len..];
        let mut response_dest: Option<SocketAddr> = None;

        if !apdu_data.is_empty() {
            match parse_apdu(apdu_data) {
                Ok(apdu_info) => {
                    // Check if this is a response to a confirmed request
                    if apdu_info.is_response() {
                        if let Some(invoke_id) = apdu_info.invoke_id {
                            // For segmented responses, we need to keep the transaction alive
                            // until the final segment is received (more_follows=false)
                            let is_segmented_response = apdu_info.segmented
                                && apdu_info.apdu_type == ApduTypeClass::ComplexAck;
                            let is_final_segment = !apdu_info.more_follows;

                            if is_segmented_response && !is_final_segment {
                                // Segmented response with more segments coming - lookup but don't remove
                                if let Some(transaction) = self.transactions.get(invoke_id, source_addr) {
                                    debug!(
                                        "Segmented response segment matched transaction: invoke_id={} service={:?} more_follows={}",
                                        invoke_id,
                                        transaction.service,
                                        apdu_info.more_follows
                                    );
                                    response_dest = Some(transaction.source_addr);
                                }
                            } else {
                                // Non-segmented response OR final segment - remove transaction
                                if let Some(transaction) = self.transactions.remove(invoke_id, source_addr) {
                                    debug!(
                                        "Response matched transaction: invoke_id={} service={:?} age={:.2}s segmented={}",
                                        invoke_id,
                                        transaction.service,
                                        transaction.created_at.elapsed().as_secs_f32(),
                                        is_segmented_response
                                    );
                                    response_dest = Some(transaction.source_addr);
                                } else {
                                    // No matching transaction - will fall back to broadcast routing
                                    trace!(
                                        "No transaction found for response: invoke_id={} from MS/TP {}",
                                        invoke_id, source_addr
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log but don't fail - still route the packet
                    trace!("Could not parse APDU for transaction tracking: {:?}", e);
                }
            }
        }

        // Determine destination - use transaction-based routing if available
        let dest_addr = if let Some(unicast_dest) = response_dest {
            // Response routing: send directly to original requester
            unicast_dest
        } else if let Some(ref dest) = npdu.destination {
            if dest.network == self.ip_network {
                // Specific device on IP network
                self.resolve_ip_address(&dest.address)?
            } else if dest.network == 0xFFFF {
                // Global broadcast
                self.get_broadcast_address()
            } else {
                // Unknown network - send Reject-Message-To-Network back to source
                warn!(
                    "Network {} unreachable from MS/TP source {}: router only knows networks {} and {} - DNET={} DADR={} - {}",
                    dest.network,
                    source_addr,
                    self.mstp_network,
                    self.ip_network,
                    dest.network,
                    if dest.address.is_empty() { "broadcast".to_string() } else { format!("{:?}", dest.address) },
                    hex_dump(data, 32)
                );
                self.stats.routing_errors += 1;
                let reject_npdu = self.build_reject_message_to_network(
                    RejectReason::NotRouterToDnet,
                    dest.network,
                );
                return Ok(Some((reject_npdu, source_addr)));
            }
        } else {
            // Local network broadcast - forward to IP broadcast
            self.get_broadcast_address()
        };

        // Determine if this is a broadcast or unicast
        let is_broadcast = match dest_addr.ip() {
            IpAddr::V4(ipv4) => ipv4.is_broadcast() || ipv4.octets()[3] == 255,
            IpAddr::V6(ipv6) => ipv6.is_multicast(),
        };

        // Build NPDU with source network info
        // For unicast responses going directly to IP client: final_delivery = true
        // This strips DNET/DADR per ASHRAE 135 - the destination is the UDP endpoint itself
        // For broadcasts: final_delivery = false (may be re-routed by other routers)
        let final_delivery = !is_broadcast;
        let routed_npdu = build_routed_npdu(
            data,
            self.mstp_network,
            &[source_addr],
            &npdu,
            final_delivery,
        )?;
        let bvlc = self.build_original_npdu(&routed_npdu, is_broadcast);

        // Send via IP
        info!("MS/TP->IP SEND: {} bytes to {} (BVLC: {:02X?})",
              bvlc.len(), dest_addr, &bvlc[..bvlc.len().min(20)]);
        self.send_ip_packet(&bvlc, dest_addr)?;

        // Also forward to registered foreign devices and BDT entries if this is a broadcast
        let is_broadcast_or_multicast = match dest_addr.ip() {
            IpAddr::V4(ipv4) => ipv4.is_broadcast() || ipv4.is_multicast(),
            IpAddr::V6(ipv6) => ipv6.is_multicast(),
        };
        if is_broadcast_or_multicast {
            self.forward_to_foreign_devices(&bvlc)?;
            // Forward to BDT entries - use local IP as source for Forwarded-NPDU
            let local_addr = SocketAddr::new(IpAddr::V4(self.local_ip), self.local_port);
            self.forward_to_bdt_entries(&routed_npdu, local_addr)?;
        }

        self.stats.mstp_to_ip_packets += 1;
        self.stats.mstp_to_ip_bytes += bvlc.len() as u64;
        let now = Instant::now();
        self.stats.last_activity = Some(now);
        self.stats.last_mstp_activity = Some(now);

        Ok(None)
    }

    /// Get the broadcast address for the local subnet
    /// Uses directed broadcast (subnet broadcast) instead of limited broadcast (255.255.255.255)
    /// for better compatibility with routers and firewalls
    fn get_broadcast_address(&self) -> SocketAddr {
        let broadcast = Self::calculate_broadcast_address(self.local_ip, self.subnet_mask);
        SocketAddr::new(IpAddr::V4(broadcast), self.local_port)
    }

    /// Build a Forwarded-NPDU BVLC message (ASHRAE 135 Annex J.4.5)
    ///
    /// Per ASHRAE 135 Annex J.4.5, Forwarded-NPDU messages MUST contain the
    /// original source B/IP address, not the gateway's address.
    ///
    /// # Arguments
    /// * `npdu` - The NPDU data to forward
    /// * `source_addr` - Original source B/IP address (IP:port)
    fn build_forwarded_npdu(&self, npdu: &[u8], source_addr: SocketAddr) -> Vec<u8> {
        // Forwarded-NPDU format:
        // 0x81 (BVLC type)
        // 0x04 (Forwarded-NPDU function)
        // 2-byte length
        // 6-byte original source B/IP address (4 IP + 2 port)
        // NPDU
        let mut result = Vec::with_capacity(10 + npdu.len());

        result.push(0x81); // BVLC type
        result.push(BVLC_FORWARDED_NPDU);

        let length = 10 + npdu.len();
        result.push((length >> 8) as u8);
        result.push((length & 0xFF) as u8);

        // Original source address (from parameter, not gateway address)
        if let IpAddr::V4(ipv4) = source_addr.ip() {
            result.extend_from_slice(&ipv4.octets());
        } else {
            // Fallback for IPv6 (should not happen in BACnet/IP)
            result.extend_from_slice(&self.local_ip.octets());
        }
        let port = source_addr.port();
        result.push((port >> 8) as u8);
        result.push((port & 0xFF) as u8);

        // NPDU
        result.extend_from_slice(npdu);

        result
    }

    /// Build an Original-Unicast-NPDU or Original-Broadcast-NPDU BVLC message
    ///
    /// This format is simpler than Forwarded-NPDU and is more widely accepted by
    /// BACnet clients (like JCI CCT).
    ///
    /// # Arguments
    /// * `npdu` - The NPDU data to send
    /// * `is_broadcast` - If true, use Original-Broadcast-NPDU (0x0B), else Original-Unicast-NPDU (0x0A)
    fn build_original_npdu(&self, npdu: &[u8], is_broadcast: bool) -> Vec<u8> {
        // Original-Unicast/Broadcast-NPDU format:
        // 0x81 (BVLC type)
        // 0x0A (Original-Unicast) or 0x0B (Original-Broadcast)
        // 2-byte length
        // NPDU
        let mut result = Vec::with_capacity(4 + npdu.len());

        result.push(0x81); // BVLC type
        if is_broadcast {
            result.push(BVLC_ORIGINAL_BROADCAST);
        } else {
            result.push(BVLC_ORIGINAL_UNICAST);
        }

        let length = 4 + npdu.len();
        result.push((length >> 8) as u8);
        result.push((length & 0xFF) as u8);

        // NPDU
        result.extend_from_slice(npdu);

        result
    }

    /// Send a packet via IP socket
    fn send_ip_packet(&mut self, data: &[u8], dest: SocketAddr) -> Result<(), GatewayError> {
        if let Some(ref socket) = self.ip_socket {
            match socket.send_to(data, dest) {
                Ok(bytes_sent) => {
                    debug!("IP TX: sent {} bytes to {}", bytes_sent, dest);
                    Ok(())
                }
                Err(e) => {
                    warn!("IP TX failed to {}: {}", dest, e);
                    Err(GatewayError::IoError(e.to_string()))
                }
            }
        } else {
            // Queue for later - this shouldn't happen after set_ip_socket is called
            warn!("IP socket not set! Queuing packet for {} (queue_len={})", dest, self.ip_send_queue.len() + 1);
            self.ip_send_queue.push((data.to_vec(), dest));
            Ok(())
        }
    }

    /// Forward a broadcast message to all registered foreign devices
    fn forward_to_foreign_devices(&mut self, data: &[u8]) -> Result<(), GatewayError> {
        // Remove expired entries first
        self.foreign_device_table.retain(|addr, entry| {
            let keep = !entry.is_expired();
            if !keep {
                debug!("Removing expired foreign device: {}", addr);
            }
            keep
        });

        // Forward to each foreign device
        for entry in self.foreign_device_table.values() {
            if let Some(ref socket) = self.ip_socket {
                if let Err(e) = socket.send_to(data, entry.address) {
                    warn!("Failed to forward to foreign device {}: {}", entry.address, e);
                }
            }
        }
        Ok(())
    }

    /// Forward broadcast to BDT entries (ASHRAE 135 Annex J.3)
    /// Sends Forwarded-NPDU messages to peer BBMDs in the Broadcast Distribution Table
    fn forward_to_bdt_entries(&mut self, npdu_data: &[u8], source_addr: SocketAddr) -> Result<(), GatewayError> {
        if self.broadcast_distribution_table.is_empty() {
            return Ok(());
        }

        // Build Forwarded-NPDU with original source address
        let forwarded = self.build_forwarded_npdu(npdu_data, source_addr);

        // Forward to each BDT entry
        for entry in &self.broadcast_distribution_table {
            if let Some(ref socket) = self.ip_socket {
                if let Err(e) = socket.send_to(&forwarded, entry.address) {
                    warn!("Failed to forward to BDT entry {}: {}", entry.address, e);
                } else {
                    trace!("Forwarded broadcast to BDT entry: {}", entry.address);
                }
            }
        }
        Ok(())
    }

    /// Handle network layer messages from MS/TP side
    fn handle_network_message_from_mstp(
        &mut self,
        data: &[u8],
        npdu: &NpduInfo,
        _source_addr: u8,
    ) -> Result<(), GatewayError> {
        let (_, npdu_len) = parse_npdu(data)?;
        if npdu_len >= data.len() {
            return Err(GatewayError::InvalidFrame);
        }

        let msg_type = data[npdu_len];

        match msg_type {
            NL_WHO_IS_ROUTER_TO_NETWORK => {
                debug!("Received Who-Is-Router-To-Network from MS/TP (source: {})", _source_addr);
                // Check if they're asking about a specific network
                let requested_network = if npdu_len + 2 < data.len() {
                    Some(((data[npdu_len + 1] as u16) << 8) | (data[npdu_len + 2] as u16))
                } else {
                    None // Query for all networks
                };

                debug!("  Requested network: {:?}, our IP network: {}", requested_network, self.ip_network);

                let is_our_network = requested_network.is_none()
                    || requested_network == Some(self.ip_network)
                    || requested_network == Some(self.mstp_network)
                    || requested_network == Some(0xFFFF);

                if is_our_network {
                    // Respond with I-Am-Router-To-Network for both our networks
                    // Response is broadcast on IP to reach the original requester
                    let response = self.build_i_am_router_to_network(&[self.ip_network, self.mstp_network]);
                    let bvlc = build_bvlc(&response, true);
                    let broadcast = self.get_broadcast_address();
                    self.send_ip_packet(&bvlc, broadcast)?;
                    debug!("  Sent I-Am-Router-To-Network: networks {:?}", [self.ip_network, self.mstp_network]);
                }

                // Forward to IP network for other routers to respond (6.5.3)
                // This allows routers on the IP side to respond if they know the network
                if requested_network.is_none() || !is_our_network {
                    debug!("  Forwarding Who-Is-Router-To-Network to IP for other routers");
                    let routed_npdu = build_routed_npdu(data, self.mstp_network, &[_source_addr], npdu, false)?;
                    let gateway_addr = SocketAddr::new(IpAddr::V4(self.local_ip), self.local_port);
                    let bvlc = self.build_forwarded_npdu(&routed_npdu, gateway_addr);
                    let dest = self.get_broadcast_address();
                    self.send_ip_packet(&bvlc, dest)?;
                }
            }
            _ => {
                // Forward other network messages to IP side
                let routed_npdu = build_routed_npdu(data, self.mstp_network, &[_source_addr], npdu, false)?;
                // For MS/TP->IP routing, use gateway's IP as source (MS/TP devices have no IP)
                let gateway_addr = SocketAddr::new(IpAddr::V4(self.local_ip), self.local_port);
                let bvlc = self.build_forwarded_npdu(&routed_npdu, gateway_addr);
                let dest = self.get_broadcast_address();
                self.send_ip_packet(&bvlc, dest)?;
            }
        }
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
            warn!(
                "Malformed BVLC packet from {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 64)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Parse BVLC header
        if data[0] != 0x81 {
            warn!(
                "Invalid BVLC type from {}: expected 0x81, got 0x{:02X} - {}",
                source_addr,
                data[0],
                hex_dump(data, 64)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        let bvlc_function = data[1];
        let bvlc_length = ((data[2] as usize) << 8) | (data[3] as usize);

        if data.len() != bvlc_length {
            warn!(
                "BVLC length mismatch from {}: packet {} bytes, header says {} - {}",
                source_addr,
                data.len(),
                bvlc_length,
                hex_dump(data, 64)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Handle BVLC control messages first
        match bvlc_function {
            BVLC_REGISTER_FOREIGN_DEVICE => {
                return self.handle_register_foreign_device(data, source_addr);
            }
            BVLC_READ_FDT => {
                return self.handle_read_fdt(source_addr);
            }
            BVLC_DELETE_FDT_ENTRY => {
                return self.handle_delete_fdt_entry(data, source_addr);
            }
            BVLC_READ_BDT => {
                return self.handle_read_bdt(source_addr);
            }
            BVLC_WRITE_BDT => {
                return self.handle_write_bdt(data, source_addr);
            }
            BVLC_DISTRIBUTE_BROADCAST => {
                return self.handle_distribute_broadcast(data, source_addr);
            }
            _ => {}
        }

        // Extract NPDU based on BVLC function
        let npdu_data = match bvlc_function {
            BVLC_ORIGINAL_UNICAST | BVLC_ORIGINAL_BROADCAST => &data[4..],
            BVLC_FORWARDED_NPDU => {
                if data.len() < 10 {
                    warn!(
                        "Malformed Forwarded-NPDU from {}: too short ({} bytes) - {}",
                        source_addr,
                        data.len(),
                        hex_dump(data, 64)
                    );
                    self.stats.routing_errors += 1;
                    return Err(GatewayError::InvalidFrame);
                }
                &data[10..] // Skip original source address
            }
            _ => {
                // Unknown BVLC functions
                debug!("Ignoring unknown BVLC function 0x{:02X} from {}", bvlc_function, source_addr);
                return Ok(None);
            }
        };

        if npdu_data.len() < 2 {
            warn!(
                "NPDU too short from {}: {} bytes after BVLC - {}",
                source_addr,
                npdu_data.len(),
                hex_dump(data, 64)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Parse NPDU
        let (npdu, _npdu_len) = match parse_npdu(npdu_data) {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    "Failed to parse NPDU from {}: {} - {}",
                    source_addr,
                    e,
                    hex_dump(npdu_data, 64)
                );
                self.stats.routing_errors += 1;
                return Err(e);
            }
        };

        // Validate hop count before routing (ASHRAE 135 Clause 6.2.2)
        if let Some(hop_count) = npdu.hop_count {
            if hop_count < MIN_HOP_COUNT {
                warn!(
                    "Discarding message from {}: hop count exhausted (was {}) - {}",
                    source_addr,
                    hop_count,
                    hex_dump(npdu_data, 32)
                );
                self.stats.routing_errors += 1;
                return Err(GatewayError::HopCountExhausted);
            }
        }

        debug!(
            "Routing IP->MS/TP: src={} network_msg={} dest_present={} hop_count={:?}",
            source_addr, npdu.network_message, npdu.destination_present, npdu.hop_count
        );

        // Handle network layer messages
        if npdu.network_message {
            return self.handle_network_message_from_ip(npdu_data, &npdu, source_addr);
        }

        // Parse APDU for transaction tracking (after NPDU header)
        let (_npdu_parsed, npdu_len) = parse_npdu(npdu_data)?;
        let apdu_data = &npdu_data[npdu_len..];

        // Try to parse APDU and handle segmentation
        if !apdu_data.is_empty() {
            match parse_apdu(apdu_data) {
                Ok(apdu_info) => {
                    // Handle segmented requests - buffer and reassemble
                    if apdu_info.segmented && apdu_info.apdu_type == ApduTypeClass::ConfirmedRequest {
                        if let Some(invoke_id) = apdu_info.invoke_id {
                            // Extract segment data (service data portion after APDU header)
                            // APDU header for segmented: type(1) + max_info(1) + invoke_id(1) + seq(1) + window(1) + service(1) = 6 bytes
                            let segment_header_len = 6;
                            if apdu_data.len() > segment_header_len {
                                let max_apdu_accepted = apdu_data[1];
                                let sequence_number = apdu_data[3];
                                let proposed_window_size = apdu_data[4];
                                let service_choice = apdu_data[5];
                                let segment_payload = &apdu_data[segment_header_len..];

                                info!(
                                    "Segmented request: invoke_id={} seq={} service={} more_follows={} payload_len={}",
                                    invoke_id, sequence_number, service_choice, apdu_info.more_follows, segment_payload.len()
                                );

                                // For first segment (seq 0), store header info for APDU reconstruction
                                let first_segment_info = if sequence_number == 0 {
                                    Some((
                                        service_choice,
                                        max_apdu_accepted,
                                        apdu_info.segmented_response_accepted,
                                        npdu_data.to_vec(),
                                    ))
                                } else {
                                    None
                                };

                                // Process segment
                                match self.process_segmented_request(
                                    invoke_id,
                                    sequence_number,
                                    proposed_window_size,
                                    segment_payload,
                                    apdu_info.more_follows,
                                    source_addr,
                                    first_segment_info,
                                ) {
                                    Ok(Some((complete_apdu, original_npdu))) => {
                                        // Reassembly complete - forward to MS/TP
                                        // Parse original NPDU to get routing info
                                        let (orig_npdu_info, orig_npdu_len) = parse_npdu(&original_npdu)?;

                                        // Determine MS/TP destination
                                        let mstp_dest = if let Some(ref dest) = orig_npdu_info.destination {
                                            if dest.network == self.mstp_network {
                                                if dest.address.is_empty() { 255 } else { dest.address[0] }
                                            } else if dest.network == 0xFFFF {
                                                255
                                            } else {
                                                255
                                            }
                                        } else {
                                            255
                                        };

                                        // Build new NPDU with reassembled APDU
                                        // Create a synthetic "original data" with our complete APDU
                                        let mut synthetic_npdu = original_npdu[..orig_npdu_len].to_vec();
                                        synthetic_npdu.extend_from_slice(&complete_apdu);

                                        let final_delivery = orig_npdu_info.destination
                                            .as_ref()
                                            .map(|d| d.network == self.mstp_network || d.network == 0xFFFF)
                                            .unwrap_or(true);

                                        let routed_npdu = build_routed_npdu(
                                            &synthetic_npdu,
                                            self.ip_network,
                                            &ip_to_mac(&source_addr),
                                            &orig_npdu_info,
                                            final_delivery,
                                        )?;

                                        // Create transaction for the reassembled request
                                        if let Ok(service) = ConfirmedServiceChoice::try_from(complete_apdu[3]) {
                                            let transaction = PendingTransaction::new(
                                                invoke_id,
                                                source_addr,
                                                orig_npdu_info.source.as_ref().map(|s| s.network),
                                                orig_npdu_info.source.as_ref().map(|s| s.address.clone()).unwrap_or_default(),
                                                self.mstp_network,
                                                mstp_dest,
                                                service,
                                                true, // Segmented request
                                                routed_npdu.clone(), // Original NPDU for retry
                                            );
                                            if let Err(e) = self.transactions.add(transaction) {
                                                debug!("Failed to create transaction for reassembled request: {}", e);
                                            }
                                        }

                                        self.stats.ip_to_mstp_packets += 1;
                                        self.stats.ip_to_mstp_bytes += routed_npdu.len() as u64;
                                        let now = Instant::now();
                                        self.stats.last_activity = Some(now);
                                        self.stats.last_ip_activity = Some(now);

                                        info!(
                                            "Forwarding reassembled APDU to MS/TP: invoke_id={} dest={} len={}",
                                            invoke_id, mstp_dest, routed_npdu.len()
                                        );

                                        return Ok(Some((routed_npdu, mstp_dest)));
                                    }
                                    Ok(None) => {
                                        // More segments needed - SegmentAck was sent
                                        return Ok(None);
                                    }
                                    Err(e) => {
                                        warn!("Segment processing failed: {:?}", e);
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }

                    // Create transaction for confirmed requests (non-segmented)
                    // We need to create the transaction BEFORE routing, so we can capture the routed NPDU
                    if apdu_info.apdu_type == ApduTypeClass::ConfirmedRequest && !apdu_info.segmented {
                        if let (Some(invoke_id), Some(service_raw)) = (apdu_info.invoke_id, apdu_info.service) {
                            // Determine destination MS/TP address early (needed for transaction key)
                            let dest_mac = if let Some(ref dest) = npdu.destination {
                                if dest.network == self.mstp_network {
                                    if dest.address.is_empty() { 255 } else { dest.address[0] }
                                } else if dest.network == 0xFFFF {
                                    255 // Global broadcast
                                } else {
                                    255 // Unknown network - will be rejected later
                                }
                            } else {
                                255 // No destination - local broadcast
                            };

                            // Convert service code to ConfirmedServiceChoice
                            if let Ok(service) = ConfirmedServiceChoice::try_from(service_raw) {
                                // Build routed NPDU early so we can store it in the transaction
                                let (mstp_dest, final_delivery) = if let Some(ref dest) = npdu.destination {
                                    if dest.network == self.mstp_network {
                                        let addr = if dest.address.is_empty() { 255 } else { dest.address[0] };
                                        (addr, true)
                                    } else if dest.network == 0xFFFF {
                                        (255, true)
                                    } else if dest.network == self.ip_network {
                                        // Don't create transaction for messages to IP network
                                        (0, false)
                                    } else {
                                        (255, false)
                                    }
                                } else {
                                    (255, true)
                                };

                                // Only create transaction if message is for MS/TP network
                                if mstp_dest > 0 {
                                    // Build routed NPDU now so we can store it
                                    if let Ok(routed_npdu) = build_routed_npdu(
                                        npdu_data,
                                        self.ip_network,
                                        &ip_to_mac(&source_addr),
                                        &npdu,
                                        final_delivery,
                                    ) {
                                        let transaction = PendingTransaction::new(
                                            invoke_id,
                                            source_addr,
                                            npdu.source.as_ref().map(|s| s.network),
                                            npdu.source.as_ref().map(|s| s.address.clone()).unwrap_or_default(),
                                            self.mstp_network,
                                            dest_mac,
                                            service,
                                            false, // Non-segmented
                                            routed_npdu, // Original NPDU for retry
                                        );

                                        if let Err(e) = self.transactions.add(transaction) {
                                            debug!("Failed to create transaction for invoke_id={}: {}", invoke_id, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log but don't fail - still route the packet
                    trace!("Could not parse APDU for transaction tracking: {:?}", e);
                }
            }
        }

        // Determine MS/TP destination and whether this is final delivery
        // ASHRAE 135 Clause 6.2.2: Strip DNET/DADR when delivering to final destination network
        let (mstp_dest, final_delivery) = if let Some(ref dest) = npdu.destination {
            if dest.network == self.mstp_network {
                // Specific device on MS/TP network - THIS IS FINAL DELIVERY
                let addr = if dest.address.is_empty() {
                    255 // Broadcast on MS/TP network
                } else {
                    dest.address[0]
                };
                (addr, true) // Final delivery - strip DNET/DADR
            } else if dest.network == 0xFFFF {
                // Global broadcast - delivered locally, so final delivery
                (255, true) // Final delivery - strip DNET/DADR
            } else if dest.network == self.ip_network {
                // Message is for the IP network, not MS/TP - don't route
                return Ok(None);
            } else {
                // Unknown network - send Reject-Message-To-Network back to IP source
                warn!(
                    "Network {} unreachable from IP source {}: router only knows networks {} and {} - DNET={} DADR={} - {}",
                    dest.network,
                    source_addr,
                    self.mstp_network,
                    self.ip_network,
                    dest.network,
                    if dest.address.is_empty() { "broadcast".to_string() } else { format!("{:?}", dest.address) },
                    hex_dump(npdu_data, 32)
                );
                self.stats.routing_errors += 1;
                let reject_npdu = self.build_reject_message_to_network(
                    RejectReason::NotRouterToDnet,
                    dest.network,
                );
                let bvlc = build_bvlc(&reject_npdu, false);
                self.send_ip_packet(&bvlc, source_addr)?;
                return Ok(None);
            }
        } else {
            // No destination network - local delivery (final delivery)
            (255, true)
        };

        // Build NPDU with source network info
        // final_delivery=true strips DNET/DADR per ASHRAE 135 Clause 6.2.2
        let routed_npdu = build_routed_npdu(
            npdu_data,
            self.ip_network,
            &ip_to_mac(&source_addr),
            &npdu,
            final_delivery,
        )?;

        self.stats.ip_to_mstp_packets += 1;
        self.stats.ip_to_mstp_bytes += routed_npdu.len() as u64;
        let now = Instant::now();
        self.stats.last_activity = Some(now);
        self.stats.last_ip_activity = Some(now);

        // Update address translation table with aging
        if let Some(ref src) = npdu.source {
            if !src.address.is_empty() {
                self.learn_ip_address(source_addr, src.address[0]);
            }
        }

        Ok(Some((routed_npdu, mstp_dest)))
    }

    /// Handle Register-Foreign-Device BVLC message (ASHRAE 135 Annex J.5.2)
    fn handle_register_foreign_device(
        &mut self,
        data: &[u8],
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        if data.len() < 6 {
            warn!(
                "Malformed Register-Foreign-Device from {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 32)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Extract TTL (2 bytes at offset 4)
        let ttl_seconds = ((data[4] as u16) << 8) | (data[5] as u16);

        info!(
            "Foreign device registration from {} with TTL {} seconds",
            source_addr, ttl_seconds
        );

        // Update or insert entry - using HashMap keyed by address prevents duplicates
        if let Some(entry) = self.foreign_device_table.get_mut(&source_addr) {
            // Re-registration: refresh TTL (fixes duplicate entry bug)
            entry.refresh(ttl_seconds);
            debug!("Refreshed foreign device registration for {}", source_addr);
        } else {
            // Check FDT capacity limit (prevent DoS via excessive registrations)
            const MAX_FDT_ENTRIES: usize = 255;
            if self.foreign_device_table.len() >= MAX_FDT_ENTRIES {
                warn!("FDT full ({} entries), rejecting registration from {}", MAX_FDT_ENTRIES, source_addr);
                let result = self.build_bvlc_result(BVLC_RESULT_REGISTER_FD_NAK);
                self.send_ip_packet(&result, source_addr)?;
                return Ok(None);
            }
            // New registration
            self.foreign_device_table.insert(
                source_addr,
                ForeignDeviceEntry::new(source_addr, ttl_seconds),
            );
            debug!("Added new foreign device: {}", source_addr);
        }

        // Send BVLC-Result with success (ASHRAE 135 Annex J.5.2)
        let result = self.build_bvlc_result(BVLC_RESULT_SUCCESS);
        self.send_ip_packet(&result, source_addr)?;

        Ok(None) // No NPDU to route to MS/TP
    }

    /// Handle Read-Foreign-Device-Table BVLC message
    fn handle_read_fdt(&mut self, source_addr: SocketAddr) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        debug!("Read-FDT request from {}", source_addr);

        // Build FDT response
        let response = self.build_read_fdt_ack();
        self.send_ip_packet(&response, source_addr)?;

        Ok(None)
    }

    /// Handle Delete-Foreign-Device-Table-Entry BVLC message
    fn handle_delete_fdt_entry(
        &mut self,
        data: &[u8],
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        if data.len() < 10 {
            warn!(
                "Malformed Delete-FDT-Entry from {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 32)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        // Extract address to delete (6 bytes at offset 4)
        let ip = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
        let port = ((data[8] as u16) << 8) | (data[9] as u16);
        let addr_to_delete = SocketAddr::new(IpAddr::V4(ip), port);

        info!("Delete-FDT-Entry request for {} from {}", addr_to_delete, source_addr);

        let result_code = if self.foreign_device_table.remove(&addr_to_delete).is_some() {
            debug!("Deleted foreign device entry: {}", addr_to_delete);
            BVLC_RESULT_SUCCESS
        } else {
            warn!("Foreign device entry not found: {}", addr_to_delete);
            BVLC_RESULT_DELETE_FDT_NAK
        };

        let result = self.build_bvlc_result(result_code);
        self.send_ip_packet(&result, source_addr)?;

        Ok(None)
    }

    /// Handle Read-Broadcast-Distribution-Table BVLC message (ASHRAE 135 Annex J.3)
    fn handle_read_bdt(&mut self, source_addr: SocketAddr) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        debug!("Read-BDT request from {}", source_addr);

        // Build BDT response
        let response = self.build_read_bdt_ack();
        self.send_ip_packet(&response, source_addr)?;

        Ok(None)
    }

    /// Handle Write-Broadcast-Distribution-Table BVLC message (ASHRAE 135 Annex J.3)
    fn handle_write_bdt(
        &mut self,
        data: &[u8],
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        if data.len() < 4 {
            warn!(
                "Malformed Write-BDT from {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 32)
            );
            let result = self.build_bvlc_result(BVLC_RESULT_WRITE_BDT_NAK);
            self.send_ip_packet(&result, source_addr)?;
            return Ok(None);
        }

        // Each BDT entry is 10 bytes: 4 IP + 2 port + 4 mask
        let entry_data = &data[4..];
        if entry_data.len() % 10 != 0 {
            warn!(
                "Invalid Write-BDT from {}: payload not multiple of 10 bytes ({} bytes) - {}",
                source_addr,
                entry_data.len(),
                hex_dump(data, 32)
            );
            let result = self.build_bvlc_result(BVLC_RESULT_WRITE_BDT_NAK);
            self.send_ip_packet(&result, source_addr)?;
            return Ok(None);
        }

        let num_entries = entry_data.len() / 10;
        let mut new_bdt = Vec::new();

        for i in 0..num_entries {
            let offset = i * 10;
            let ip = Ipv4Addr::new(
                entry_data[offset],
                entry_data[offset + 1],
                entry_data[offset + 2],
                entry_data[offset + 3],
            );
            let port = ((entry_data[offset + 4] as u16) << 8) | (entry_data[offset + 5] as u16);
            let mask = Ipv4Addr::new(
                entry_data[offset + 6],
                entry_data[offset + 7],
                entry_data[offset + 8],
                entry_data[offset + 9],
            );

            new_bdt.push(BdtEntry {
                address: SocketAddr::new(IpAddr::V4(ip), port),
                mask,
            });
        }

        info!(
            "Write-BDT from {}: {} entries updated",
            source_addr,
            new_bdt.len()
        );
        for (i, entry) in new_bdt.iter().enumerate() {
            debug!("  BDT[{}]: {} mask {}", i, entry.address, entry.mask);
        }

        self.broadcast_distribution_table = new_bdt;

        // Persist BDT to NVS
        self.save_bdt_to_nvs();

        // Send success response
        let result = self.build_bvlc_result(BVLC_RESULT_SUCCESS);
        self.send_ip_packet(&result, source_addr)?;

        Ok(None)
    }

    /// Handle Distribute-Broadcast-To-Network BVLC message (ASHRAE 135 Annex J.5.4)
    fn handle_distribute_broadcast(
        &mut self,
        data: &[u8],
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        // Verify sender is a registered foreign device
        if !self.foreign_device_table.contains_key(&source_addr) {
            warn!("Distribute-Broadcast from unregistered device: {}", source_addr);
            let result = self.build_bvlc_result(BVLC_RESULT_DISTRIBUTE_NAK);
            self.send_ip_packet(&result, source_addr)?;
            return Ok(None);
        }

        if data.len() < 5 {
            warn!(
                "Malformed Distribute-Broadcast from {}: too short ({} bytes) - {}",
                source_addr,
                data.len(),
                hex_dump(data, 32)
            );
            self.stats.routing_errors += 1;
            return Err(GatewayError::InvalidFrame);
        }

        let npdu_data = &data[4..];

        // Forward as Forwarded-NPDU to local broadcast and other foreign devices
        // CRITICAL: Use original sender's address per ASHRAE 135 Annex J.4.5
        let forwarded = self.build_forwarded_npdu(npdu_data, source_addr);
        let broadcast_addr = self.get_broadcast_address();
        self.send_ip_packet(&forwarded, broadcast_addr)?;

        // Forward to other foreign devices (excluding sender)
        // Collect addresses first to avoid borrow issues
        let fd_addresses: Vec<_> = self.foreign_device_table.values()
            .filter(|entry| entry.address != source_addr)
            .map(|entry| entry.address)
            .collect();
        for addr in fd_addresses {
            if let Err(e) = self.send_ip_packet(&forwarded, addr) {
                warn!("Failed to forward to foreign device {}: {}", addr, e);
            }
        }

        // Also route to MS/TP network
        let (npdu, _) = parse_npdu(npdu_data)?;

        // Validate hop count
        if let Some(hop_count) = npdu.hop_count {
            if hop_count < MIN_HOP_COUNT {
                return Err(GatewayError::HopCountExhausted);
            }
        }

        // Delivering to local MS/TP network = final delivery
        let routed_npdu = build_routed_npdu(
            npdu_data,
            self.ip_network,
            &ip_to_mac(&source_addr),
            &npdu,
            true, // Final delivery - strip DNET/DADR
        )?;

        Ok(Some((routed_npdu, 255))) // Broadcast to MS/TP
    }

    /// Handle network layer messages from IP side
    fn handle_network_message_from_ip(
        &mut self,
        data: &[u8],
        npdu: &NpduInfo,
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        let (_, npdu_len) = parse_npdu(data)?;
        if npdu_len >= data.len() {
            return Err(GatewayError::InvalidFrame);
        }

        let msg_type = data[npdu_len];

        match msg_type {
            NL_WHO_IS_ROUTER_TO_NETWORK => {
                debug!("Received Who-Is-Router-To-Network from IP (source: {})", source_addr);
                // Check if asking about our MS/TP network
                let requested_network = if npdu_len + 2 < data.len() {
                    Some(((data[npdu_len + 1] as u16) << 8) | (data[npdu_len + 2] as u16))
                } else {
                    None // Query for all networks
                };

                debug!("  Requested network: {:?}, our MS/TP network: {}", requested_network, self.mstp_network);

                let is_our_network = requested_network.is_none()
                    || requested_network == Some(self.mstp_network)
                    || requested_network == Some(self.ip_network)
                    || requested_network == Some(0xFFFF);

                if is_our_network {
                    // Respond with I-Am-Router-To-Network
                    // Include both networks we route to
                    let response = self.build_i_am_router_to_network(&[self.mstp_network, self.ip_network]);
                    let bvlc = build_bvlc(&response, true);

                    // Send to broadcast for network discovery
                    let broadcast = self.get_broadcast_address();
                    self.send_ip_packet(&bvlc, broadcast)?;

                    // Also send directly to the requester (common BACnet practice)
                    // This ensures they receive our response even if broadcast fails
                    debug!("  Sending I-Am-Router-To-Network: networks {:?}", [self.mstp_network, self.ip_network]);
                    self.send_ip_packet(&bvlc, source_addr)?;
                }

                // Forward to MS/TP network for other routers to respond (6.5.3)
                // This allows routers on the MS/TP side to respond if they know the network
                if requested_network.is_none() || !is_our_network {
                    debug!("  Forwarding Who-Is-Router-To-Network to MS/TP for other routers");
                    // Build NPDU with source info to route responses back
                    let forwarded = build_routed_npdu(data, self.ip_network, &ip_to_mac(&source_addr), npdu, true)?;
                    return Ok(Some((forwarded, 255))); // Broadcast on MS/TP
                }
            }
            NL_INITIALIZE_ROUTING_TABLE => {
                debug!("Received Initialize-Routing-Table from IP (source: {})", source_addr);
                return self.handle_initialize_routing_table(data, npdu_len, source_addr);
            }
            _ => {
                // Forward to MS/TP network - final delivery
                let routed_npdu = build_routed_npdu(data, self.ip_network, &ip_to_mac(&source_addr), npdu, true)?;
                return Ok(Some((routed_npdu, 255)));
            }
        }
        Ok(None)
    }

    /// Handle Initialize-Routing-Table network layer message (ASHRAE 135 Clause 6.4)
    fn handle_initialize_routing_table(
        &mut self,
        data: &[u8],
        npdu_len: usize,
        source_addr: SocketAddr,
    ) -> Result<Option<(Vec<u8>, u8)>, GatewayError> {
        // Skip message type byte
        let mut offset = npdu_len + 1;

        // Parse number of ports
        if offset >= data.len() {
            warn!("Malformed Initialize-Routing-Table: missing port count");
            return Err(GatewayError::InvalidFrame);
        }
        let num_ports = data[offset];
        offset += 1;

        info!(
            "Initialize-Routing-Table from {}: {} ports",
            source_addr, num_ports
        );

        // Clear existing routing table
        self.routing_table.clear();

        // Parse routing table entries
        for port_idx in 0..num_ports {
            if offset >= data.len() {
                warn!("Malformed Initialize-Routing-Table: truncated port data");
                return Err(GatewayError::InvalidFrame);
            }

            // Network count for this port
            let net_count = data[offset];
            offset += 1;

            // Networks reachable via this port
            for _ in 0..net_count {
                if offset + 1 >= data.len() {
                    warn!("Malformed Initialize-Routing-Table: truncated network data");
                    return Err(GatewayError::InvalidFrame);
                }
                let network = ((data[offset] as u16) << 8) | (data[offset + 1] as u16);
                offset += 2;

                // Port info length
                if offset >= data.len() {
                    warn!("Malformed Initialize-Routing-Table: missing port info length");
                    return Err(GatewayError::InvalidFrame);
                }
                let port_info_len = data[offset] as usize;
                offset += 1;

                // Port info data (MAC address)
                if offset + port_info_len > data.len() {
                    warn!("Malformed Initialize-Routing-Table: truncated port info");
                    return Err(GatewayError::InvalidFrame);
                }
                let port_info = data[offset..offset + port_info_len].to_vec();
                offset += port_info_len;

                debug!(
                    "  Port {}: network {} via {:?}",
                    port_idx, network, port_info
                );

                // Store routing entry
                self.routing_table.insert(
                    network,
                    RoutingTableEntry {
                        network,
                        port_id: port_idx,
                        port_info,
                    },
                );
            }
        }

        // Persist routing table to NVS
        self.save_routing_table_to_nvs();

        // Send Initialize-Routing-Table-Ack
        let ack = self.build_initialize_routing_table_ack();
        let bvlc = build_bvlc(&ack, false);
        self.send_ip_packet(&bvlc, source_addr)?;

        Ok(None)
    }

    /// Build Initialize-Routing-Table-Ack message (ASHRAE 135 Clause 6.4)
    fn build_initialize_routing_table_ack(&self) -> Vec<u8> {
        vec![
            0x01, // NPDU version
            0x80, // Control: network layer message, no DNET/SNET
            NL_INITIALIZE_ROUTING_TABLE_ACK,
        ]
    }

    /// Build a BVLC-Result message (ASHRAE 135 Annex J.2.1)
    fn build_bvlc_result(&self, result_code: u16) -> Vec<u8> {
        vec![
            0x81, // BVLC type
            BVLC_RESULT,
            0x00, 0x06, // Length: 6 bytes
            (result_code >> 8) as u8,
            (result_code & 0xFF) as u8,
        ]
    }

    /// Build a Read-Foreign-Device-Table-Ack message
    fn build_read_fdt_ack(&self) -> Vec<u8> {
        // Each FDT entry is 10 bytes: 6-byte address + 2-byte TTL + 2-byte remaining TTL
        let entry_count = self.foreign_device_table.len();
        let length = 4 + (entry_count * 10);

        let mut result = Vec::with_capacity(length);
        result.push(0x81);
        result.push(BVLC_READ_FDT_ACK);
        result.push((length >> 8) as u8);
        result.push((length & 0xFF) as u8);

        for entry in self.foreign_device_table.values() {
            if let SocketAddr::V4(v4) = entry.address {
                result.extend_from_slice(&v4.ip().octets());
                result.push((v4.port() >> 8) as u8);
                result.push((v4.port() & 0xFF) as u8);
                result.push((entry.ttl_seconds >> 8) as u8);
                result.push((entry.ttl_seconds & 0xFF) as u8);
                let remaining = entry.remaining_ttl();
                result.push((remaining >> 8) as u8);
                result.push((remaining & 0xFF) as u8);
            }
        }

        result
    }

    /// Build a Read-Broadcast-Distribution-Table-Ack message (ASHRAE 135 Annex J.3)
    fn build_read_bdt_ack(&self) -> Vec<u8> {
        // Each BDT entry is 10 bytes: 4-byte IP + 2-byte port + 4-byte mask
        let entry_count = self.broadcast_distribution_table.len();
        let length = 4 + (entry_count * 10);

        let mut result = Vec::with_capacity(length);
        result.push(0x81);
        result.push(BVLC_READ_BDT_ACK);
        result.push((length >> 8) as u8);
        result.push((length & 0xFF) as u8);

        for entry in &self.broadcast_distribution_table {
            if let SocketAddr::V4(v4) = entry.address {
                result.extend_from_slice(&v4.ip().octets());
                result.push((v4.port() >> 8) as u8);
                result.push((v4.port() & 0xFF) as u8);
                result.extend_from_slice(&entry.mask.octets());
            }
        }

        result
    }

    /// Build an I-Am-Router-To-Network message (ASHRAE 135 Clause 6.4.2)
    fn build_i_am_router_to_network(&self, networks: &[u16]) -> Vec<u8> {
        let mut result = Vec::new();

        // NPDU header
        result.push(0x01); // Version
        result.push(0x80); // Control: network layer message, no DNET/SNET

        // Network layer message type
        result.push(NL_I_AM_ROUTER_TO_NETWORK);

        // List of reachable networks
        for &network in networks {
            result.push((network >> 8) as u8);
            result.push((network & 0xFF) as u8);
        }

        result
    }

    /// Build a Reject-Message-To-Network message (ASHRAE 135 Clause 6.4.4)
    ///
    /// This message is sent when a router cannot forward a message to a destination network.
    /// The message is sent back toward the source of the original message.
    ///
    /// Format:
    /// - NPDU header (version, control)
    /// - Message type (0x03)
    /// - Reject reason (1 byte)
    /// - DNET (2 bytes) - the network that could not be reached
    fn build_reject_message_to_network(&self, reason: RejectReason, dnet: u16) -> Vec<u8> {
        let mut result = Vec::new();

        // NPDU header
        result.push(0x01); // Version
        result.push(0x80); // Control: network layer message, no DNET/SNET

        // Network layer message type
        result.push(NL_REJECT_MESSAGE_TO_NETWORK);

        // Reject reason
        result.push(reason as u8);

        // DNET that was unreachable
        result.push((dnet >> 8) as u8);
        result.push((dnet & 0xFF) as u8);

        result
    }

    /// Send a Reject-Message-To-Network back to the source
    fn send_reject_to_source(
        &mut self,
        reason: RejectReason,
        dnet: u16,
        source: &NpduInfo,
        received_from_ip: bool,
        ip_source: Option<SocketAddr>,
    ) -> Result<(), GatewayError> {
        let reject_npdu = self.build_reject_message_to_network(reason, dnet);

        if received_from_ip {
            // Send back to IP source
            if let Some(addr) = ip_source {
                let bvlc = build_bvlc(&reject_npdu, false);
                self.send_ip_packet(&bvlc, addr)?;
                info!(
                    "Sent Reject-Message-To-Network to {}: reason={:?}, dnet={}",
                    addr, reason, dnet
                );
            }
        } else {
            // Send back to MS/TP source - queue for transmission
            // The reject will be returned via the IP send queue mechanism
            // since we need to return it to the caller for MS/TP transmission
            if let Some(ref src) = source.source {
                if !src.address.is_empty() {
                    // Log for now - actual MS/TP transmission handled by caller
                    info!(
                        "Reject-Message-To-Network for MS/TP source network={}, addr={:?}: reason={:?}, dnet={}",
                        src.network, src.address, reason, dnet
                    );
                }
            }
        }

        Ok(())
    }

    /// Announce this router's presence on startup
    pub fn announce_router(&mut self) -> Result<(), GatewayError> {
        if self.router_announced {
            return Ok(());
        }

        info!("Announcing router presence for networks {} and {}",
              self.mstp_network, self.ip_network);

        // Send I-Am-Router-To-Network for MS/TP network on IP side
        let response = self.build_i_am_router_to_network(&[self.mstp_network]);
        let bvlc = build_bvlc(&response, true);
        let broadcast = self.get_broadcast_address();
        self.send_ip_packet(&bvlc, broadcast)?;

        self.router_announced = true;
        Ok(())
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
        let fdt_before = self.foreign_device_table.len();

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

        // Remove expired foreign device entries (ASHRAE 135 Annex J.5.3)
        self.foreign_device_table.retain(|addr, entry| {
            let keep = !entry.is_expired();
            if !keep {
                info!("Foreign device registration expired: {}", addr);
            }
            keep
        });

        // Log if any entries were removed
        let mstp_removed = mstp_before - self.mstp_to_ip.len();
        let ip_removed = ip_before - self.ip_to_mstp.len();
        let fdt_removed = fdt_before - self.foreign_device_table.len();
        if mstp_removed > 0 || ip_removed > 0 || fdt_removed > 0 {
            info!(
                "Housekeeping: removed {} MS/TP, {} IP, {} FDT entries",
                mstp_removed, ip_removed, fdt_removed
            );
        }
    }

    /// Get number of registered foreign devices
    pub fn foreign_device_count(&self) -> usize {
        self.foreign_device_table.len()
    }

    /// Get gateway statistics
    pub fn get_stats(&self) -> &GatewayStats {
        &self.stats
    }

    /// Check network health based on recent activity
    /// A network is considered "healthy" if activity occurred within the last 60 seconds
    pub fn check_network_health(&mut self) {
        const HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

        // Check MS/TP network health
        let mstp_healthy = self.stats.last_mstp_activity
            .map(|t| t.elapsed() < HEALTH_TIMEOUT)
            .unwrap_or(false);

        // Detect MS/TP network up/down transitions
        if mstp_healthy != self.stats.mstp_network_up {
            if mstp_healthy {
                info!("MS/TP network is now UP (activity detected)");
            } else {
                warn!("MS/TP network is now DOWN (no activity for {} seconds)", HEALTH_TIMEOUT.as_secs());
            }
            self.stats.mstp_network_up = mstp_healthy;
        }

        // Check IP network health
        let ip_healthy = self.stats.last_ip_activity
            .map(|t| t.elapsed() < HEALTH_TIMEOUT)
            .unwrap_or(false);

        // Detect IP network up/down transitions
        if ip_healthy != self.stats.ip_network_up {
            if ip_healthy {
                info!("IP network is now UP (activity detected)");
            } else {
                warn!("IP network is now DOWN (no activity for {} seconds)", HEALTH_TIMEOUT.as_secs());
            }
            self.stats.ip_network_up = ip_healthy;
        }
    }

    /// Check if a specific network is healthy (has recent activity)
    pub fn is_network_healthy(&self, network_type: NetworkType) -> bool {
        match network_type {
            NetworkType::Mstp => self.stats.mstp_network_up,
            NetworkType::Ip => self.stats.ip_network_up,
        }
    }

    /// Get a formatted statistics summary for logging
    pub fn get_stats_summary(&self) -> String {
        let mstp_status = if self.stats.mstp_network_up { "UP" } else { "DOWN" };
        let ip_status = if self.stats.ip_network_up { "UP" } else { "DOWN" };

        let mstp_activity = self.stats.last_mstp_activity
            .map(|t| format!("{:.1}s ago", t.elapsed().as_secs_f32()))
            .unwrap_or_else(|| "never".to_string());

        let ip_activity = self.stats.last_ip_activity
            .map(|t| format!("{:.1}s ago", t.elapsed().as_secs_f32()))
            .unwrap_or_else(|| "never".to_string());

        format!(
            "Gateway Stats:\n  \
            MS/TP->IP: {} pkts ({} bytes), last: {}, status: {}\n  \
            IP->MS/TP: {} pkts ({} bytes), last: {}, status: {}\n  \
            Errors: {} routing, {} timeouts\n  \
            Active transactions: {}, Foreign devices: {}",
            self.stats.mstp_to_ip_packets,
            self.stats.mstp_to_ip_bytes,
            mstp_activity,
            mstp_status,
            self.stats.ip_to_mstp_packets,
            self.stats.ip_to_mstp_bytes,
            ip_activity,
            ip_status,
            self.stats.routing_errors,
            self.stats.transaction_timeouts,
            self.transactions.len(),
            self.foreign_device_table.len()
        )
    }
}

/// Network type for health checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    Mstp,
    Ip,
}

/// Gateway error types
#[derive(Debug)]
pub enum GatewayError {
    InvalidFrame,
    InvalidAddress,
    NetworkUnreachable(u16),
    IoError(String),
    NpduError(String),
    HopCountExhausted,
    BvlcError(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::InvalidFrame => write!(f, "Invalid frame"),
            GatewayError::InvalidAddress => write!(f, "Invalid address"),
            GatewayError::NetworkUnreachable(n) => write!(f, "Network {} unreachable", n),
            GatewayError::IoError(s) => write!(f, "I/O error: {}", s),
            GatewayError::NpduError(s) => write!(f, "NPDU error: {}", s),
            GatewayError::HopCountExhausted => write!(f, "Hop count exhausted"),
            GatewayError::BvlcError(s) => write!(f, "BVLC error: {}", s),
        }
    }
}

impl std::error::Error for GatewayError {}

/// APDU type classification for transaction tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApduTypeClass {
    ConfirmedRequest,
    UnconfirmedRequest,
    SimpleAck,
    ComplexAck,
    SegmentAck,
    Error,
    Reject,
    Abort,
}

/// Parsed APDU information for transaction tracking
///
/// Extracts key fields needed to track confirmed service transactions:
/// - Invoke ID for request/response correlation
/// - Service type for timeout configuration
/// - Segmentation flags for buffer management
#[derive(Debug, Clone)]
pub struct ApduInfo {
    pub apdu_type: ApduTypeClass,
    pub invoke_id: Option<u8>,
    pub service: Option<u8>,
    pub segmented: bool,
    pub more_follows: bool,
    pub segmented_response_accepted: bool,
}

impl ApduInfo {
    /// Check if this APDU is a response type (SimpleAck, ComplexAck, Error, Reject, Abort)
    pub fn is_response(&self) -> bool {
        matches!(
            self.apdu_type,
            ApduTypeClass::SimpleAck
                | ApduTypeClass::ComplexAck
                | ApduTypeClass::SegmentAck
                | ApduTypeClass::Error
                | ApduTypeClass::Reject
                | ApduTypeClass::Abort
        )
    }

    /// Check if this APDU requires transaction tracking (confirmed request or response)
    pub fn needs_tracking(&self) -> bool {
        matches!(
            self.apdu_type,
            ApduTypeClass::ConfirmedRequest
                | ApduTypeClass::SimpleAck
                | ApduTypeClass::ComplexAck
                | ApduTypeClass::Error
                | ApduTypeClass::Reject
                | ApduTypeClass::Abort
        )
    }
}

/// Parse APDU header from data (after NPDU header)
///
/// Returns ApduInfo with invoke_id, service type, and segmentation flags.
/// The data should start at the APDU (after NPDU header).
fn parse_apdu(data: &[u8]) -> Result<ApduInfo, GatewayError> {
    if data.is_empty() {
        return Err(GatewayError::InvalidFrame);
    }

    let pdu_type_byte = data[0];
    let pdu_type_raw = (pdu_type_byte >> 4) & 0x0F;

    let apdu_type = match pdu_type_raw {
        0 => ApduTypeClass::ConfirmedRequest,
        1 => ApduTypeClass::UnconfirmedRequest,
        2 => ApduTypeClass::SimpleAck,
        3 => ApduTypeClass::ComplexAck,
        4 => ApduTypeClass::SegmentAck,
        5 => ApduTypeClass::Error,
        6 => ApduTypeClass::Reject,
        7 => ApduTypeClass::Abort,
        _ => return Err(GatewayError::InvalidFrame),
    };

    match apdu_type {
        ApduTypeClass::ConfirmedRequest => {
            if data.len() < 4 {
                return Err(GatewayError::InvalidFrame);
            }

            let segmented = (pdu_type_byte & 0x08) != 0;
            let more_follows = (pdu_type_byte & 0x04) != 0;
            let segmented_response_accepted = (pdu_type_byte & 0x02) != 0;

            let invoke_id = data[2];
            let service_pos = if segmented { 5 } else { 3 };

            let service = if data.len() > service_pos {
                Some(data[service_pos])
            } else {
                None
            };

            Ok(ApduInfo {
                apdu_type,
                invoke_id: Some(invoke_id),
                service,
                segmented,
                more_follows,
                segmented_response_accepted,
            })
        }

        ApduTypeClass::UnconfirmedRequest => Ok(ApduInfo {
            apdu_type,
            invoke_id: None,
            service: if data.len() > 1 { Some(data[1]) } else { None },
            segmented: false,
            more_follows: false,
            segmented_response_accepted: false,
        }),

        ApduTypeClass::SimpleAck => {
            if data.len() < 3 {
                return Err(GatewayError::InvalidFrame);
            }

            Ok(ApduInfo {
                apdu_type,
                invoke_id: Some(data[1]),
                service: Some(data[2]),
                segmented: false,
                more_follows: false,
                segmented_response_accepted: false,
            })
        }

        ApduTypeClass::ComplexAck => {
            if data.len() < 3 {
                return Err(GatewayError::InvalidFrame);
            }

            let segmented = (pdu_type_byte & 0x08) != 0;
            let more_follows = (pdu_type_byte & 0x04) != 0;

            let invoke_id = data[1];
            let service_pos = if segmented { 4 } else { 2 };

            let service = if data.len() > service_pos {
                Some(data[service_pos])
            } else {
                None
            };

            Ok(ApduInfo {
                apdu_type,
                invoke_id: Some(invoke_id),
                service,
                segmented,
                more_follows,
                segmented_response_accepted: false,
            })
        }

        ApduTypeClass::SegmentAck => {
            if data.len() < 2 {
                return Err(GatewayError::InvalidFrame);
            }

            Ok(ApduInfo {
                apdu_type,
                invoke_id: Some(data[1]),
                service: None,
                segmented: false,
                more_follows: false,
                segmented_response_accepted: false,
            })
        }

        ApduTypeClass::Error | ApduTypeClass::Reject | ApduTypeClass::Abort => {
            if data.len() < 2 {
                return Err(GatewayError::InvalidFrame);
            }

            let invoke_id = data[1];
            let service = if apdu_type == ApduTypeClass::Error && data.len() > 2 {
                Some(data[2])
            } else {
                None
            };

            Ok(ApduInfo {
                apdu_type,
                invoke_id: Some(invoke_id),
                service,
                segmented: false,
                more_follows: false,
                segmented_response_accepted: false,
            })
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

/// Create a hex dump string for error logging
///
/// Returns a formatted hex string showing up to `max_bytes` of data.
/// Format: "len=N [01 02 03 04...]" or "len=N [01 02 03...and M more]"
fn hex_dump(data: &[u8], max_bytes: usize) -> String {
    let show_bytes = data.len().min(max_bytes);
    let hex_str: Vec<String> = data[..show_bytes]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect();

    if data.len() > max_bytes {
        format!(
            "len={} [{} ...and {} more]",
            data.len(),
            hex_str.join(" "),
            data.len() - max_bytes
        )
    } else {
        format!("len={} [{}]", data.len(), hex_str.join(" "))
    }
}

/// Parse NPDU header
fn parse_npdu(data: &[u8]) -> Result<(NpduInfo, usize), GatewayError> {
    if data.len() < 2 {
        return Err(GatewayError::NpduError(format!(
            "NPDU too short: {} bytes (minimum 2)",
            data.len()
        )));
    }

    let version = data[0];
    if version != 1 {
        return Err(GatewayError::NpduError(format!(
            "Invalid NPDU version: expected 1, got {}",
            version
        )));
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
            return Err(GatewayError::NpduError(format!(
                "NPDU destination truncated: need {} bytes, have {}",
                pos + 3,
                data.len()
            )));
        }
        let network = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        let addr_len = data[pos + 2] as usize;
        pos += 3;

        if pos + addr_len > data.len() {
            return Err(GatewayError::NpduError(format!(
                "NPDU destination address truncated: need {} bytes, have {}",
                pos + addr_len,
                data.len()
            )));
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
            return Err(GatewayError::NpduError(format!(
                "NPDU source truncated: need {} bytes, have {}",
                pos + 3,
                data.len()
            )));
        }
        let network = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        let addr_len = data[pos + 2] as usize;
        pos += 3;

        if pos + addr_len > data.len() {
            return Err(GatewayError::NpduError(format!(
                "NPDU source address truncated: need {} bytes, have {}",
                pos + addr_len,
                data.len()
            )));
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
            return Err(GatewayError::NpduError(format!(
                "NPDU hop count missing: need {} bytes, have {}",
                pos + 1,
                data.len()
            )));
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
///
/// Per ASHRAE 135 Clause 6.2.2: When delivering to the final destination network,
/// the DNET/DADR fields must be stripped from the NPDU. Set `final_delivery` to true
/// when the destination network matches the local network being delivered to.
fn build_routed_npdu(
    original_data: &[u8],
    source_network: u16,
    source_address: &[u8],
    npdu: &NpduInfo,
    final_delivery: bool,
) -> Result<Vec<u8>, GatewayError> {
    let mut result = Vec::new();

    // Version
    result.push(1);

    // Build control byte
    let mut control = npdu.priority;
    if npdu.network_message {
        control |= 0x80;
    }
    // ASHRAE 135 Clause 6.2.2: Strip DNET/DADR for final delivery
    if npdu.destination.is_some() && !final_delivery {
        control |= 0x20;
    }
    // Always set source present since we're routing
    control |= 0x08;
    if npdu.expecting_reply {
        control |= 0x04;
    }
    result.push(control);

    // Destination (only if NOT final delivery per ASHRAE 135 Clause 6.2.2)
    if let Some(ref dest) = npdu.destination {
        if !final_delivery {
            result.push((dest.network >> 8) as u8);
            result.push((dest.network & 0xFF) as u8);
            result.push(dest.address.len() as u8);
            result.extend_from_slice(&dest.address);
        }
    }

    // Source (always add for routing)
    result.push((source_network >> 8) as u8);
    result.push((source_network & 0xFF) as u8);
    result.push(source_address.len() as u8);
    result.extend_from_slice(source_address);

    // Hop count (if destination present and NOT final delivery)
    if npdu.destination.is_some() && !final_delivery {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_dump_short() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = hex_dump(&data, 64);
        assert_eq!(result, "len=4 [01 02 03 04]");
    }

    #[test]
    fn test_hex_dump_long() {
        let data = vec![0xAA; 100]; // 100 bytes of 0xAA
        let result = hex_dump(&data, 8);
        assert!(result.contains("len=100"));
        assert!(result.contains("...and 92 more"));
        assert!(result.contains("AA AA AA AA AA AA AA AA"));
    }

    #[test]
    fn test_hex_dump_empty() {
        let data = vec![];
        let result = hex_dump(&data, 64);
        assert_eq!(result, "len=0 []");
    }

    #[test]
    fn test_parse_npdu_too_short() {
        let data = vec![0x01]; // Only 1 byte
        let result = parse_npdu(&data);
        assert!(result.is_err());
        if let Err(GatewayError::NpduError(msg)) = result {
            assert!(msg.contains("too short"));
            assert!(msg.contains("minimum 2"));
        }
    }

    #[test]
    fn test_parse_npdu_invalid_version() {
        let data = vec![0x02, 0x00]; // Version 2 (invalid)
        let result = parse_npdu(&data);
        assert!(result.is_err());
        if let Err(GatewayError::NpduError(msg)) = result {
            assert!(msg.contains("Invalid NPDU version"));
            assert!(msg.contains("expected 1, got 2"));
        }
    }

    #[test]
    fn test_parse_npdu_truncated_destination() {
        // NPDU with destination flag set but incomplete data
        let data = vec![
            0x01, // Version
            0x20, // Control: destination present
            0x00, 0x01, // DNET = 1
            0x05, // DADR length = 5 (but no address follows)
        ];
        let result = parse_npdu(&data);
        assert!(result.is_err());
        if let Err(GatewayError::NpduError(msg)) = result {
            assert!(msg.contains("destination address truncated"));
        }
    }

    #[test]
    fn test_parse_npdu_valid_simple() {
        // Simple NPDU with no destination or source
        let data = vec![
            0x01, // Version
            0x00, // Control: no flags
        ];
        let result = parse_npdu(&data);
        assert!(result.is_ok());
        let (npdu, len) = result.unwrap();
        assert_eq!(len, 2);
        assert!(!npdu.network_message);
        assert!(!npdu.destination_present);
        assert!(!npdu.source_present);
    }

    #[test]
    fn test_reject_reason_codes() {
        // Verify reject reason enum values match BACnet spec
        assert_eq!(RejectReason::Other as u8, 0);
        assert_eq!(RejectReason::NotRouterToDnet as u8, 1);
        assert_eq!(RejectReason::RouterBusy as u8, 2);
        assert_eq!(RejectReason::UnknownNetworkMessage as u8, 3);
        assert_eq!(RejectReason::MessageTooLong as u8, 4);
        assert_eq!(RejectReason::SecurityError as u8, 5);
        assert_eq!(RejectReason::AddressingError as u8, 6);
    }

    #[test]
    fn test_build_reject_message_to_network() {
        let gateway = BacnetGateway::new_default(1, 2, Ipv4Addr::new(192, 168, 1, 100));
        let reject = gateway.build_reject_message_to_network(
            RejectReason::NotRouterToDnet,
            999, // Unknown network
        );

        // Verify NPDU structure
        assert_eq!(reject[0], 0x01); // Version
        assert_eq!(reject[1], 0x80); // Control: network layer message
        assert_eq!(reject[2], NL_REJECT_MESSAGE_TO_NETWORK); // Message type
        assert_eq!(reject[3], RejectReason::NotRouterToDnet as u8); // Reject reason
        assert_eq!(reject[4], (999 >> 8) as u8); // DNET high byte
        assert_eq!(reject[5], (999 & 0xFF) as u8); // DNET low byte
    }
}

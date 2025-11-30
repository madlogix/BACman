//! Gateway configuration with NVS persistence
//!
//! Configuration is stored in ESP32 Non-Volatile Storage (NVS) for persistence
//! across reboots. First boot uses default values which can be updated via
//! runtime configuration.

use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use log::{info, warn};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// NVS namespace for gateway configuration
const NVS_NAMESPACE: &str = "bacman_cfg";

/// NVS keys for configuration values
mod nvs_keys {
    pub const WIFI_SSID: &str = "wifi_ssid";
    pub const WIFI_PASS: &str = "wifi_pass";
    pub const MSTP_ADDR: &str = "mstp_addr";
    pub const MSTP_MAX: &str = "mstp_max";
    pub const MSTP_BAUD: &str = "mstp_baud";
    pub const MSTP_NET: &str = "mstp_net";
    pub const IP_PORT: &str = "ip_port";
    pub const IP_NET: &str = "ip_net";
    pub const DEV_INST: &str = "dev_inst";
    pub const DEV_NAME: &str = "dev_name";
    pub const CONFIGURED: &str = "configured";
    // AP mode settings
    pub const AP_SSID: &str = "ap_ssid";
    pub const AP_PASS: &str = "ap_pass";
    // BDT persistence (stores as comma-separated IP:port list)
    pub const BDT_ENTRIES: &str = "bdt_entries";
    pub const BDT_COUNT: &str = "bdt_count";
    // Routing table persistence
    pub const RT_ENTRIES: &str = "rt_entries";
    pub const RT_COUNT: &str = "rt_count";
}

/// Gateway configuration settings
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    // WiFi Station mode settings
    pub wifi_ssid: String,
    pub wifi_password: String,

    // WiFi Access Point mode settings
    pub ap_ssid: String,
    pub ap_password: String,

    // MS/TP settings
    pub mstp_address: u8,
    pub mstp_max_master: u8,
    pub mstp_baud_rate: u32,
    pub mstp_network: u16,

    // BACnet/IP settings
    pub bacnet_ip_port: u16,
    pub ip_network: u16,

    // Gateway settings
    pub device_instance: u32,
    pub device_name: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            // WiFi Station mode - MUST be configured via web interface or NVS
            // Empty credentials will trigger AP mode for initial configuration
            wifi_ssid: String::new(),
            wifi_password: String::new(),

            // WiFi Access Point mode - creates "BACman-XXXX" network
            // Password must be 8+ characters for WPA2
            ap_ssid: "BACman-Gateway".to_string(),
            ap_password: "bacnet123".to_string(),

            // MS/TP settings
            mstp_address: 3,        // Gateway's MS/TP address (0-127 for master)
            mstp_max_master: 127,   // Maximum master address on network
            mstp_baud_rate: 38400,  // Standard MS/TP baud rate
            mstp_network: 65001,    // BACnet network number for MS/TP side

            // BACnet/IP settings
            bacnet_ip_port: 47808,  // Standard BACnet/IP port (0xBAC0)
            ip_network: 10001,      // BACnet network number for IP side

            // Gateway device settings
            device_instance: 1234,
            device_name: "BACman-Gateway".to_string(),
        }
    }
}

#[allow(dead_code)]
impl GatewayConfig {
    /// Load configuration from NVS, falling back to defaults if not configured
    pub fn load_from_nvs(nvs_partition: EspNvsPartition<NvsDefault>) -> Result<Self, anyhow::Error> {
        let nvs = match EspNvs::new(nvs_partition, NVS_NAMESPACE, true) {
            Ok(nvs) => nvs,
            Err(e) => {
                warn!("Failed to open NVS namespace, using defaults: {}", e);
                return Ok(Self::default());
            }
        };

        // Check if configuration has been saved before
        let configured: bool = nvs.get_u8(nvs_keys::CONFIGURED)
            .ok()
            .flatten()
            .map(|v| v != 0)
            .unwrap_or(false);

        if !configured {
            info!("No saved configuration found, using defaults");
            return Ok(Self::default());
        }

        info!("Loading configuration from NVS...");

        let mut config = Self::default();

        // Load WiFi Station mode settings
        if let Ok(Some(ssid)) = Self::get_string(&nvs, nvs_keys::WIFI_SSID) {
            config.wifi_ssid = ssid;
        }
        if let Ok(Some(pass)) = Self::get_string(&nvs, nvs_keys::WIFI_PASS) {
            config.wifi_password = pass;
        }

        // Load WiFi AP mode settings
        if let Ok(Some(ap_ssid)) = Self::get_string(&nvs, nvs_keys::AP_SSID) {
            config.ap_ssid = ap_ssid;
        }
        if let Ok(Some(ap_pass)) = Self::get_string(&nvs, nvs_keys::AP_PASS) {
            config.ap_password = ap_pass;
        }

        // Load MS/TP settings
        if let Ok(Some(addr)) = nvs.get_u8(nvs_keys::MSTP_ADDR) {
            config.mstp_address = addr;
        }
        if let Ok(Some(max)) = nvs.get_u8(nvs_keys::MSTP_MAX) {
            config.mstp_max_master = max;
        }
        if let Ok(Some(baud)) = nvs.get_u32(nvs_keys::MSTP_BAUD) {
            config.mstp_baud_rate = baud;
        }
        if let Ok(Some(net)) = nvs.get_u16(nvs_keys::MSTP_NET) {
            config.mstp_network = net;
        }

        // Load BACnet/IP settings
        if let Ok(Some(port)) = nvs.get_u16(nvs_keys::IP_PORT) {
            config.bacnet_ip_port = port;
        }
        if let Ok(Some(net)) = nvs.get_u16(nvs_keys::IP_NET) {
            config.ip_network = net;
        }

        // Load device settings
        if let Ok(Some(inst)) = nvs.get_u32(nvs_keys::DEV_INST) {
            config.device_instance = inst;
        }
        if let Ok(Some(name)) = Self::get_string(&nvs, nvs_keys::DEV_NAME) {
            config.device_name = name;
        }

        info!("Configuration loaded from NVS");
        Ok(config)
    }

    /// Save configuration to NVS
    pub fn save_to_nvs(&self, nvs_partition: EspNvsPartition<NvsDefault>) -> Result<(), anyhow::Error> {
        let mut nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;

        info!("Saving configuration to NVS...");

        // Save WiFi Station mode settings
        Self::set_string(&mut nvs, nvs_keys::WIFI_SSID, &self.wifi_ssid)?;
        Self::set_string(&mut nvs, nvs_keys::WIFI_PASS, &self.wifi_password)?;

        // Save WiFi AP mode settings
        Self::set_string(&mut nvs, nvs_keys::AP_SSID, &self.ap_ssid)?;
        Self::set_string(&mut nvs, nvs_keys::AP_PASS, &self.ap_password)?;

        // Save MS/TP settings
        nvs.set_u8(nvs_keys::MSTP_ADDR, self.mstp_address)?;
        nvs.set_u8(nvs_keys::MSTP_MAX, self.mstp_max_master)?;
        nvs.set_u32(nvs_keys::MSTP_BAUD, self.mstp_baud_rate)?;
        nvs.set_u16(nvs_keys::MSTP_NET, self.mstp_network)?;

        // Save BACnet/IP settings
        nvs.set_u16(nvs_keys::IP_PORT, self.bacnet_ip_port)?;
        nvs.set_u16(nvs_keys::IP_NET, self.ip_network)?;

        // Save device settings
        nvs.set_u32(nvs_keys::DEV_INST, self.device_instance)?;
        Self::set_string(&mut nvs, nvs_keys::DEV_NAME, &self.device_name)?;

        // Mark as configured
        nvs.set_u8(nvs_keys::CONFIGURED, 1)?;

        info!("Configuration saved to NVS");
        Ok(())
    }

    /// Helper to get string from NVS
    fn get_string(nvs: &EspNvs<NvsDefault>, key: &str) -> Result<Option<String>, anyhow::Error> {
        let mut buf = [0u8; 64];
        match nvs.get_str(key, &mut buf) {
            Ok(Some(s)) => Ok(Some(s.to_string())),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!("Failed to read NVS key {}: {}", key, e);
                Ok(None)
            }
        }
    }

    /// Helper to set string in NVS
    fn set_string(nvs: &mut EspNvs<NvsDefault>, key: &str, value: &str) -> Result<(), anyhow::Error> {
        nvs.set_str(key, value)?;
        Ok(())
    }

    /// Clear all saved configuration (reset to defaults on next boot)
    pub fn clear_nvs(nvs_partition: EspNvsPartition<NvsDefault>) -> Result<(), anyhow::Error> {
        let nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;
        nvs.set_u8(nvs_keys::CONFIGURED, 0)?;
        info!("Configuration cleared - will use defaults on next boot");
        Ok(())
    }
}

/// BDT entry for NVS persistence (matches gateway::BdtEntry)
#[derive(Debug, Clone)]
pub struct BdtEntryConfig {
    pub address: SocketAddr,
    pub broadcast_mask: u32,
}

/// Routing table entry for NVS persistence (matches gateway::RoutingTableEntry)
#[derive(Debug, Clone)]
pub struct RoutingTableEntryConfig {
    pub network: u16,
    pub port_id: u8,
    pub port_info: Vec<u8>,
}

/// BDT and Routing Table persistence functions
pub struct NetworkTablePersistence;

impl NetworkTablePersistence {
    /// Save BDT entries to NVS
    /// Format: count (u8), then for each entry: IP (4 bytes) + port (2 bytes BE) + mask (4 bytes BE)
    pub fn save_bdt(
        nvs_partition: EspNvsPartition<NvsDefault>,
        entries: &[BdtEntryConfig],
    ) -> Result<(), anyhow::Error> {
        let mut nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;

        let count = entries.len().min(255) as u8;
        nvs.set_u8(nvs_keys::BDT_COUNT, count)?;

        if count == 0 {
            info!("BDT cleared from NVS");
            return Ok(());
        }

        // Serialize entries: 10 bytes each (4 IP + 2 port + 4 mask)
        let mut buf = Vec::with_capacity(count as usize * 10);
        for entry in entries.iter().take(count as usize) {
            if let IpAddr::V4(ipv4) = entry.address.ip() {
                buf.extend_from_slice(&ipv4.octets());
                buf.extend_from_slice(&entry.address.port().to_be_bytes());
                buf.extend_from_slice(&entry.broadcast_mask.to_be_bytes());
            }
        }

        nvs.set_blob(nvs_keys::BDT_ENTRIES, &buf)?;
        info!("Saved {} BDT entries to NVS", count);
        Ok(())
    }

    /// Load BDT entries from NVS
    pub fn load_bdt(
        nvs_partition: EspNvsPartition<NvsDefault>,
    ) -> Result<Vec<BdtEntryConfig>, anyhow::Error> {
        let nvs = match EspNvs::new(nvs_partition, NVS_NAMESPACE, true) {
            Ok(nvs) => nvs,
            Err(e) => {
                warn!("Failed to open NVS for BDT load: {}", e);
                return Ok(Vec::new());
            }
        };

        let count = nvs.get_u8(nvs_keys::BDT_COUNT)?.unwrap_or(0);
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut buf = vec![0u8; count as usize * 10];
        match nvs.get_blob(nvs_keys::BDT_ENTRIES, &mut buf) {
            Ok(Some(data)) => {
                let mut entries = Vec::with_capacity(count as usize);
                for chunk in data.chunks_exact(10) {
                    let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                    let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                    let mask = u32::from_be_bytes([chunk[6], chunk[7], chunk[8], chunk[9]]);
                    entries.push(BdtEntryConfig {
                        address: SocketAddr::new(IpAddr::V4(ip), port),
                        broadcast_mask: mask,
                    });
                }
                info!("Loaded {} BDT entries from NVS", entries.len());
                Ok(entries)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => {
                warn!("Failed to read BDT from NVS: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Save routing table entries to NVS
    /// Format: count (u8), then for each entry: network (2 bytes BE) + port_id (1 byte) + info_len (1 byte) + info
    pub fn save_routing_table(
        nvs_partition: EspNvsPartition<NvsDefault>,
        entries: &[RoutingTableEntryConfig],
    ) -> Result<(), anyhow::Error> {
        let mut nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;

        let count = entries.len().min(255) as u8;
        nvs.set_u8(nvs_keys::RT_COUNT, count)?;

        if count == 0 {
            info!("Routing table cleared from NVS");
            return Ok(());
        }

        // Calculate total size and serialize
        let mut buf = Vec::new();
        for entry in entries.iter().take(count as usize) {
            buf.extend_from_slice(&entry.network.to_be_bytes());
            buf.push(entry.port_id);
            let info_len = entry.port_info.len().min(255) as u8;
            buf.push(info_len);
            buf.extend_from_slice(&entry.port_info[..info_len as usize]);
        }

        nvs.set_blob(nvs_keys::RT_ENTRIES, &buf)?;
        info!("Saved {} routing table entries to NVS", count);
        Ok(())
    }

    /// Load routing table entries from NVS
    pub fn load_routing_table(
        nvs_partition: EspNvsPartition<NvsDefault>,
    ) -> Result<Vec<RoutingTableEntryConfig>, anyhow::Error> {
        let nvs = match EspNvs::new(nvs_partition, NVS_NAMESPACE, true) {
            Ok(nvs) => nvs,
            Err(e) => {
                warn!("Failed to open NVS for routing table load: {}", e);
                return Ok(Vec::new());
            }
        };

        let count = nvs.get_u8(nvs_keys::RT_COUNT)?.unwrap_or(0);
        if count == 0 {
            return Ok(Vec::new());
        }

        // Max size: count * (2 + 1 + 1 + 255) = count * 259
        let mut buf = vec![0u8; count as usize * 259];
        match nvs.get_blob(nvs_keys::RT_ENTRIES, &mut buf) {
            Ok(Some(data)) => {
                let mut entries = Vec::with_capacity(count as usize);
                let mut offset = 0;
                while offset + 4 <= data.len() && entries.len() < count as usize {
                    let network = u16::from_be_bytes([data[offset], data[offset + 1]]);
                    let port_id = data[offset + 2];
                    let info_len = data[offset + 3] as usize;
                    offset += 4;

                    let port_info = if offset + info_len <= data.len() {
                        data[offset..offset + info_len].to_vec()
                    } else {
                        Vec::new()
                    };
                    offset += info_len;

                    entries.push(RoutingTableEntryConfig {
                        network,
                        port_id,
                        port_info,
                    });
                }
                info!("Loaded {} routing table entries from NVS", entries.len());
                Ok(entries)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => {
                warn!("Failed to read routing table from NVS: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Clear BDT and routing table from NVS
    pub fn clear_tables(nvs_partition: EspNvsPartition<NvsDefault>) -> Result<(), anyhow::Error> {
        let nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;
        nvs.set_u8(nvs_keys::BDT_COUNT, 0)?;
        nvs.set_u8(nvs_keys::RT_COUNT, 0)?;
        info!("BDT and routing table cleared from NVS");
        Ok(())
    }
}

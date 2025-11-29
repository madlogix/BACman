//! Gateway configuration with NVS persistence
//!
//! Configuration is stored in ESP32 Non-Volatile Storage (NVS) for persistence
//! across reboots. First boot uses default values which can be updated via
//! runtime configuration.

use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use log::{info, warn};

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

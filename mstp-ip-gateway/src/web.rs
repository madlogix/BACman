//! Web portal for configuration and diagnostics
//!
//! Provides a simple HTTP server with:
//! - Status dashboard with real-time stats
//! - Configuration page for all settings
//! - Save/reset configuration to NVS
//! - Reboot functionality

use embedded_svc::io::Write;
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use esp_idf_svc::nvs::{EspNvsPartition, NvsDefault};
use log::{error, info};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use crate::config::GatewayConfig;
use crate::local_device::DiscoveredDevice;
use crate::mstp_driver::MstpStats;

/// Web server port
const WEB_PORT: u16 = 80;

/// Shared state for web handlers
pub struct WebState {
    pub config: GatewayConfig,
    pub nvs_partition: Option<EspNvsPartition<NvsDefault>>,
    pub mstp_stats: MstpStats,
    pub gateway_stats: GatewayStats,
    pub wifi_connected: bool,
    pub ip_address: String,
    pub reset_stats_requested: bool,
    pub scan_requested: bool,
    pub discovered_devices: Vec<DiscoveredDevice>,
    pub scan_in_progress: bool,
    pub start_time: std::time::Instant,
    /// Last few received BACnet data frames for debugging (source_mac, hex_data)
    pub last_rx_frames: std::collections::VecDeque<(u8, String)>,
    /// BDT entries for display and management (synced from gateway)
    pub bdt_entries: Vec<(SocketAddr, Ipv4Addr)>,
    /// Request to add BDT entry (IP:port, mask)
    pub bdt_add_request: Option<(SocketAddr, Ipv4Addr)>,
    /// Request to remove BDT entry by address
    pub bdt_remove_request: Option<SocketAddr>,
    /// Request to clear all BDT entries
    pub bdt_clear_request: bool,
}

/// Gateway stats snapshot for web display
#[derive(Default, Clone)]
pub struct GatewayStats {
    pub mstp_to_ip_packets: u64,
    pub ip_to_mstp_packets: u64,
    pub mstp_to_ip_bytes: u64,
    pub ip_to_mstp_bytes: u64,
    pub routing_errors: u64,
    pub transaction_timeouts: u64,
}

impl WebState {
    pub fn new(config: GatewayConfig, nvs_partition: Option<EspNvsPartition<NvsDefault>>) -> Self {
        Self {
            config,
            nvs_partition,
            mstp_stats: MstpStats::default(),
            gateway_stats: GatewayStats::default(),
            wifi_connected: false,
            ip_address: String::new(),
            reset_stats_requested: false,
            scan_requested: false,
            discovered_devices: Vec::new(),
            scan_in_progress: false,
            start_time: std::time::Instant::now(),
            last_rx_frames: std::collections::VecDeque::new(),
            bdt_entries: Vec::new(),
            bdt_add_request: None,
            bdt_remove_request: None,
            bdt_clear_request: false,
        }
    }

    /// Add a received frame to the debug buffer (keeps last 10)
    pub fn add_rx_frame(&mut self, source_mac: u8, data: &[u8]) {
        let hex = data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        self.last_rx_frames.push_back((source_mac, hex));
        while self.last_rx_frames.len() > 10 {
            self.last_rx_frames.pop_front();
        }
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get formatted uptime string (e.g., "2d 5h 30m")
    pub fn uptime_formatted(&self) -> String {
        let secs = self.uptime_secs();
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, mins)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

/// Start the web server
pub fn start_web_server(
    state: Arc<Mutex<WebState>>,
) -> anyhow::Result<EspHttpServer<'static>> {
    let http_config = HttpConfig {
        http_port: WEB_PORT,
        ..Default::default()
    };

    let mut server = EspHttpServer::new(&http_config)?;
    info!("Web server starting on port {}", WEB_PORT);

    // Clone state for each handler
    let state_status = Arc::clone(&state);
    let state_config = Arc::clone(&state);
    let state_config_post = Arc::clone(&state);
    let state_save = Arc::clone(&state);
    let state_reset = Arc::clone(&state);
    let state_api_status = Arc::clone(&state);
    let state_reset_stats = Arc::clone(&state);
    let state_export = Arc::clone(&state);
    let state_scan = Arc::clone(&state);
    let state_devices = Arc::clone(&state);

    // Index page - redirect to status
    server.fn_handler("/", embedded_svc::http::Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write_all(HTML_REDIRECT_STATUS.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Status page
    server.fn_handler("/status", embedded_svc::http::Method::Get, move |req| {
        let state = state_status.lock().unwrap();
        let html = generate_status_page(&state);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Configuration page (GET)
    server.fn_handler("/config", embedded_svc::http::Method::Get, move |req| {
        let state = state_config.lock().unwrap();
        let html = generate_config_page(&state);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Configuration form submit (POST)
    server.fn_handler("/config", embedded_svc::http::Method::Post, move |mut req| {
        // Read POST body
        let mut body = [0u8; 1024];
        let len = req.read(&mut body).unwrap_or(0);
        let body_str = std::str::from_utf8(&body[..len]).unwrap_or("");

        // Parse form data
        let mut state = state_config_post.lock().unwrap();
        parse_config_form(body_str, &mut state.config);

        // Redirect back to config page with success message
        let html = generate_config_page_with_message(&state, "Configuration updated. Click 'Save to NVS' to persist changes.");
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Save configuration to NVS
    server.fn_handler("/save", embedded_svc::http::Method::Post, move |req| {
        let state = state_save.lock().unwrap();
        let message = if let Some(ref nvs) = state.nvs_partition {
            match state.config.save_to_nvs(nvs.clone()) {
                Ok(_) => {
                    info!("Configuration saved to NVS via web portal");
                    "Configuration saved successfully! Reboot to apply changes."
                }
                Err(e) => {
                    error!("Failed to save config: {}", e);
                    "Error saving configuration!"
                }
            }
        } else {
            "NVS not available"
        };

        let html = generate_config_page_with_message(&state, message);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Reset configuration to defaults
    server.fn_handler("/reset", embedded_svc::http::Method::Post, move |req| {
        let mut state = state_reset.lock().unwrap();
        if let Some(ref nvs) = state.nvs_partition {
            let _ = GatewayConfig::clear_nvs(nvs.clone());
        }
        state.config = GatewayConfig::default();
        info!("Configuration reset to defaults via web portal");

        let html = generate_config_page_with_message(&state, "Configuration reset to defaults.");
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Reboot device
    server.fn_handler("/reboot", embedded_svc::http::Method::Post, |req| {
        info!("Reboot requested via web portal");
        let html = HTML_REBOOT_PAGE;
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;

        // Schedule reboot after response is sent
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(2));
            // SAFETY: esp_restart() is always safe to call on ESP32 - it performs a
            // software reset. The 2-second delay ensures the HTTP response is sent.
            unsafe { esp_idf_svc::sys::esp_restart(); }
        });

        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint for status JSON (for AJAX updates)
    server.fn_handler("/api/status", embedded_svc::http::Method::Get, move |req| {
        let state = state_api_status.lock().unwrap();
        let json = generate_status_json(&state);
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to reset statistics
    server.fn_handler("/api/reset-stats", embedded_svc::http::Method::Post, move |req| {
        let mut state = state_reset_stats.lock().unwrap();
        state.reset_stats_requested = true;
        info!("Statistics reset requested via web portal");
        let json = r#"{"status":"ok","message":"Statistics reset requested"}"#;
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to export all data as JSON
    server.fn_handler("/api/export", embedded_svc::http::Method::Get, move |req| {
        let state = state_export.lock().unwrap();
        let json = generate_export_json(&state);
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Content-Disposition", "attachment; filename=\"bacman-export.json\""),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to start a Who-Is scan
    server.fn_handler("/api/scan", embedded_svc::http::Method::Post, move |req| {
        let mut state = state_scan.lock().unwrap();
        if state.scan_in_progress {
            let json = r#"{"status":"busy","message":"Scan already in progress"}"#;
            let mut resp = req.into_response(200, Some("OK"), &[
                ("Content-Type", "application/json"),
                ("Access-Control-Allow-Origin", "*"),
            ])?;
            resp.write_all(json.as_bytes())?;
        } else {
            state.scan_requested = true;
            state.scan_in_progress = true;
            state.discovered_devices.clear();
            info!("Who-Is scan requested via web portal");
            let json = r#"{"status":"ok","message":"Scan started"}"#;
            let mut resp = req.into_response(200, Some("OK"), &[
                ("Content-Type", "application/json"),
                ("Access-Control-Allow-Origin", "*"),
            ])?;
            resp.write_all(json.as_bytes())?;
        }
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to get discovered devices
    server.fn_handler("/api/devices", embedded_svc::http::Method::Get, move |req| {
        let state = state_devices.lock().unwrap();
        let json = generate_devices_json(&state);
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to stop scan
    let state_stop_scan = Arc::clone(&state);
    server.fn_handler("/api/stop-scan", embedded_svc::http::Method::Post, move |req| {
        let mut state = state_stop_scan.lock().unwrap();
        state.scan_in_progress = false;
        info!("Scan stopped via web portal");
        let json = r#"{"status":"ok","message":"Scan stopped"}"#;
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to get last received frames (debug)
    let state_debug = Arc::clone(&state);
    server.fn_handler("/api/debug/frames", embedded_svc::http::Method::Get, move |req| {
        let state = state_debug.lock().unwrap();
        let frames: Vec<String> = state.last_rx_frames.iter()
            .map(|(mac, hex)| format!("{{\"mac\":{},\"data\":\"{}\"}}", mac, hex))
            .collect();
        let json = format!("{{\"frames\":[{}]}}", frames.join(","));
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // BDT page (GET)
    let state_bdt = Arc::clone(&state);
    server.fn_handler("/bdt", embedded_svc::http::Method::Get, move |req| {
        let state = state_bdt.lock().unwrap();
        let html = generate_bdt_page(&state);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // BDT add entry (POST)
    let state_bdt_add = Arc::clone(&state);
    server.fn_handler("/bdt/add", embedded_svc::http::Method::Post, move |mut req| {
        let mut body = [0u8; 256];
        let len = req.read(&mut body).unwrap_or(0);
        let body_str = std::str::from_utf8(&body[..len]).unwrap_or("");

        let mut state = state_bdt_add.lock().unwrap();
        let message = parse_bdt_add_form(body_str, &mut state);

        let html = generate_bdt_page_with_message(&state, message);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // BDT remove entry (POST)
    let state_bdt_remove = Arc::clone(&state);
    server.fn_handler("/bdt/remove", embedded_svc::http::Method::Post, move |mut req| {
        let mut body = [0u8; 128];
        let len = req.read(&mut body).unwrap_or(0);
        let body_str = std::str::from_utf8(&body[..len]).unwrap_or("");

        let mut state = state_bdt_remove.lock().unwrap();
        let message = parse_bdt_remove_form(body_str, &mut state);

        let html = generate_bdt_page_with_message(&state, message);
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // BDT clear all (POST)
    let state_bdt_clear = Arc::clone(&state);
    server.fn_handler("/bdt/clear", embedded_svc::http::Method::Post, move |req| {
        let mut state = state_bdt_clear.lock().unwrap();
        state.bdt_clear_request = true;
        info!("BDT clear requested via web portal");

        let html = generate_bdt_page_with_message(&state, "BDT clear requested. Entries will be removed.");
        let mut resp = req.into_ok_response()?;
        resp.write_all(html.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    // API endpoint to get BDT entries as JSON
    let state_bdt_api = Arc::clone(&state);
    server.fn_handler("/api/bdt", embedded_svc::http::Method::Get, move |req| {
        let state = state_bdt_api.lock().unwrap();
        let json = generate_bdt_json(&state);
        let mut resp = req.into_response(200, Some("OK"), &[
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ])?;
        resp.write_all(json.as_bytes())?;
        Ok::<(), anyhow::Error>(())
    })?;

    info!("Web server started successfully");
    Ok(server)
}

/// Valid MS/TP baud rates per ASHRAE 135
const VALID_MSTP_BAUD_RATES: [u32; 5] = [9600, 19200, 38400, 76800, 115200];

/// Maximum BACnet device instance (2^22 - 2)
const MAX_DEVICE_INSTANCE: u32 = 4194302;

/// Parse URL-encoded form data with validation
fn parse_config_form(body: &str, config: &mut GatewayConfig) {
    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let value = urlencoding::decode(value).unwrap_or_default();

        match key {
            "wifi_ssid" => {
                // SSID max 32 characters
                if value.len() <= 32 {
                    config.wifi_ssid = value.to_string();
                }
            }
            "wifi_pass" => {
                // Only update if not empty (allows keeping existing password)
                // WPA2 requires 8-63 characters
                if !value.is_empty() && value.len() >= 8 && value.len() <= 63 {
                    config.wifi_password = value.to_string();
                }
            }
            "ap_ssid" => {
                // SSID max 32 characters
                if value.len() <= 32 && !value.is_empty() {
                    config.ap_ssid = value.to_string();
                }
            }
            "ap_pass" => {
                // Only update if not empty (allows keeping existing password)
                // WPA2 requires 8-63 characters
                if !value.is_empty() && value.len() >= 8 && value.len() <= 63 {
                    config.ap_password = value.to_string();
                }
            }
            "mstp_addr" => {
                // MS/TP master address: 0-127
                if let Ok(v) = value.parse::<u8>() {
                    if v <= 127 {
                        config.mstp_address = v;
                    }
                }
            }
            "mstp_max" => {
                // MS/TP max master: 0-127, must be >= mstp_address
                if let Ok(v) = value.parse::<u8>() {
                    if v <= 127 && v >= config.mstp_address {
                        config.mstp_max_master = v;
                    }
                }
            }
            "mstp_baud" => {
                // Only accept valid MS/TP baud rates
                if let Ok(v) = value.parse::<u32>() {
                    if VALID_MSTP_BAUD_RATES.contains(&v) {
                        config.mstp_baud_rate = v;
                    }
                }
            }
            "mstp_net" => {
                // BACnet network number: 1-65534 (0 and 65535 reserved)
                if let Ok(v) = value.parse::<u16>() {
                    if v >= 1 && v <= 65534 {
                        config.mstp_network = v;
                    }
                }
            }
            "ip_port" => {
                // Port must be > 0
                if let Ok(v) = value.parse::<u16>() {
                    if v > 0 {
                        config.bacnet_ip_port = v;
                    }
                }
            }
            "ip_net" => {
                // BACnet network number: 1-65534 (0 and 65535 reserved)
                if let Ok(v) = value.parse::<u16>() {
                    if v >= 1 && v <= 65534 {
                        config.ip_network = v;
                    }
                }
            }
            "dev_inst" => {
                // Device instance: 0-4194302 (max per ASHRAE 135)
                if let Ok(v) = value.parse::<u32>() {
                    if v <= MAX_DEVICE_INSTANCE {
                        config.device_instance = v;
                    }
                }
            }
            "dev_name" => {
                // Device name max 64 characters
                if value.len() <= 64 && !value.is_empty() {
                    config.device_name = value.to_string();
                }
            }
            _ => {}
        }
    }
}

/// Generate status page HTML
fn generate_status_page(state: &WebState) -> String {
    // Convert discovered_masters bitmap to hex string
    let masters_hex = format!("{:032x}", state.mstp_stats.discovered_masters);

    format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>BACman Gateway - Status</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>{}</style>
    <script>
        const STATE_NAMES = ['Init', 'Idle', 'UseToken', 'WaitReply', 'PassToken', 'NoToken', 'PollMaster', 'AnswerReq', 'DoneToken'];

        function updateDeviceGrid(hexStr, stationAddr) {{
            const grid = document.getElementById('device-grid');
            if (!grid) return;

            // Parse hex string to BigInt
            let bitmap = BigInt('0x' + hexStr);

            for (let i = 0; i < 128; i++) {{
                const cell = document.getElementById('dev-' + i);
                if (cell) {{
                    const isPresent = (bitmap >> BigInt(i)) & BigInt(1);
                    cell.className = 'grid-cell';
                    if (i === stationAddr) {{
                        cell.className += ' self';
                    }} else if (isPresent) {{
                        cell.className += ' active';
                    }}
                }}
            }}
        }}

        function updateStatus() {{
            fetch('/api/status')
                .then(r => r.json())
                .then(data => {{
                    // Frame counters
                    document.getElementById('rx_frames').textContent = data.rx_frames;
                    document.getElementById('tx_frames').textContent = data.tx_frames;
                    document.getElementById('tokens_received').textContent = data.tokens_received;

                    // Error counters with highlighting
                    const crcEl = document.getElementById('crc_errors');
                    crcEl.textContent = data.crc_errors;
                    crcEl.className = data.crc_errors > 0 ? 'value error' : 'value';

                    const frameErrEl = document.getElementById('frame_errors');
                    frameErrEl.textContent = data.frame_errors;
                    frameErrEl.className = data.frame_errors > 0 ? 'value error' : 'value';

                    const replyTOEl = document.getElementById('reply_timeouts');
                    replyTOEl.textContent = data.reply_timeouts;
                    replyTOEl.className = data.reply_timeouts > 0 ? 'value error' : 'value';

                    const passFailEl = document.getElementById('token_pass_failures');
                    passFailEl.textContent = data.token_pass_failures;
                    passFailEl.className = data.token_pass_failures > 0 ? 'value error' : 'value';

                    // Token loop timing
                    document.getElementById('token_loop').textContent = data.token_loop_ms + ' ms';
                    document.getElementById('token_loop_min').textContent = data.token_loop_min_ms + ' ms';
                    document.getElementById('token_loop_max').textContent = data.token_loop_max_ms + ' ms';
                    document.getElementById('token_loop_avg').textContent = data.token_loop_avg_ms + ' ms';

                    // State machine
                    document.getElementById('masters').textContent = data.master_count;
                    document.getElementById('state').textContent = STATE_NAMES[data.current_state] || 'Unknown';
                    document.getElementById('next_station').textContent = data.next_station;
                    document.getElementById('poll_station').textContent = data.poll_station;

                    const silenceEl = document.getElementById('silence');
                    silenceEl.textContent = data.silence_ms + ' ms';
                    silenceEl.className = data.silence_ms > 500 ? 'value warning' : 'value';

                    const soleMasterEl = document.getElementById('sole_master');
                    soleMasterEl.textContent = data.sole_master ? 'Yes' : 'No';
                    soleMasterEl.className = data.sole_master ? 'value warning' : 'value';

                    // Queue depths
                    document.getElementById('send_queue').textContent = data.send_queue_len;
                    document.getElementById('receive_queue').textContent = data.receive_queue_len;

                    // Gateway stats
                    document.getElementById('mstp_to_ip').textContent = data.mstp_to_ip;
                    document.getElementById('ip_to_mstp').textContent = data.ip_to_mstp;

                    // Uptime
                    document.getElementById('uptime').textContent = data.uptime;

                    // Device count chip
                    document.getElementById('device-count').textContent = data.master_count + ' found';

                    updateDeviceGrid(data.discovered_masters, data.station_address);
                }})
                .catch(e => console.error('Update failed:', e));
        }}
        function resetStats() {{
            fetch('/api/reset-stats', {{ method: 'POST' }})
                .then(r => r.json())
                .then(data => {{ if(data.status === 'ok') updateStatus(); }})
                .catch(e => console.error('Reset failed:', e));
        }}
        function exportData() {{
            window.location.href = '/api/export';
        }}
        let scanPollInterval = null;
        function startScan() {{
            document.getElementById('scanBtn').disabled = true;
            document.getElementById('scanBtn').textContent = 'Scanning...';
            document.getElementById('scan-results').style.display = 'block';
            document.getElementById('scan-status').textContent = 'Sending Who-Is broadcast...';
            document.getElementById('device-list').innerHTML = '';

            fetch('/api/scan', {{ method: 'POST' }})
                .then(r => r.json())
                .then(data => {{
                    if (data.status === 'ok') {{
                        scanPollInterval = setInterval(pollScanResults, 1000);
                        setTimeout(stopScan, 5000);
                    }} else {{
                        document.getElementById('scan-status').textContent = data.message;
                        document.getElementById('scanBtn').disabled = false;
                        document.getElementById('scanBtn').textContent = 'Scan Devices (Who-Is)';
                    }}
                }});
        }}
        function pollScanResults() {{
            fetch('/api/devices')
                .then(r => r.json())
                .then(data => {{
                    const list = document.getElementById('device-list');
                    list.innerHTML = '';
                    if (data.devices.length === 0) {{
                        document.getElementById('scan-status').textContent = 'Waiting for I-Am responses...';
                    }} else {{
                        document.getElementById('scan-status').textContent = 'Found ' + data.devices.length + ' device(s):';
                        data.devices.forEach(dev => {{
                            const div = document.createElement('div');
                            div.className = 'device-row';
                            div.innerHTML = '<span>MAC ' + dev.mac + '</span><span>Instance ' + dev.instance + '</span><span>Vendor ' + dev.vendor + '</span>';
                            div.onclick = () => showDeviceInfo(dev);
                            list.appendChild(div);
                        }});
                    }}
                }});
        }}
        function stopScan() {{
            if (scanPollInterval) clearInterval(scanPollInterval);
            scanPollInterval = null;
            document.getElementById('scanBtn').disabled = false;
            document.getElementById('scanBtn').textContent = 'Scan Devices (Who-Is)';
            fetch('/api/stop-scan', {{ method: 'POST' }});
            pollScanResults();
        }}
        function showDeviceInfo(dev) {{
            const modal = document.getElementById('device-modal');
            const body = document.getElementById('modal-body');
            body.innerHTML = '<p><b>MAC Address:</b> ' + dev.mac + '</p>' +
                '<p><b>Device Instance:</b> ' + dev.instance + '</p>' +
                '<p><b>Vendor ID:</b> ' + dev.vendor + '</p>' +
                '<p><b>Max APDU:</b> ' + dev.max_apdu + '</p>' +
                '<p><b>Segmentation:</b> ' + ['Both', 'Transmit', 'Receive', 'None'][dev.segmentation] + '</p>';
            modal.style.display = 'flex';
        }}
        function closeModal(e) {{
            if (!e || e.target.id === 'device-modal') {{
                document.getElementById('device-modal').style.display = 'none';
            }}
        }}
        function showGridDeviceInfo(mac) {{
            fetch('/api/devices')
                .then(r => r.json())
                .then(data => {{
                    const dev = data.devices.find(d => d.mac === mac);
                    if (dev) {{
                        showDeviceInfo(dev);
                    }} else {{
                        const modal = document.getElementById('device-modal');
                        const body = document.getElementById('modal-body');
                        body.innerHTML = '<p><b>MAC Address:</b> ' + mac + '</p><p>No I-Am received. Run a scan first.</p>';
                        modal.style.display = 'flex';
                    }}
                }});
        }}
        setInterval(updateStatus, 2000);
        document.addEventListener('DOMContentLoaded', () => updateDeviceGrid('{}', {}));
    </script>
</head>
<body>
    <div class="container">
        <h1>BACman Gateway</h1>
        <nav>
            <a href="/status" class="active">Status</a>
            <a href="/config">Configuration</a>
        </nav>

        <div class="card">
            <div class="card-header">
                <h2>MS/TP Device Map <span class="chip" id="device-count">{} found</span></h2>
                <button class="btn btn-sm" id="scanBtn" onclick="startScan()">Scan (Who-Is)</button>
            </div>
            <div class="device-grid" id="device-grid">{}</div>
            <div class="grid-legend">
                <span><span class="legend-box self"></span> This Device</span>
                <span><span class="legend-box active"></span> Active Master</span>
                <span><span class="legend-box"></span> Not Found</span>
            </div>
            <div id="scan-results" style="margin-top:12px;display:none;">
                <div class="scan-status" id="scan-status"></div>
                <div id="device-list"></div>
            </div>
        </div>

        <div class="card">
            <h2>State Machine</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">State</span>
                    <span class="value" id="state">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Sole Master</span>
                    <span class="value {}" id="sole_master">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Next Station</span>
                    <span class="value" id="next_station">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Poll Station</span>
                    <span class="value" id="poll_station">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Silence</span>
                    <span class="value" id="silence">{} ms</span>
                </div>
                <div class="status-item">
                    <span class="label">Masters Found</span>
                    <span class="value" id="masters">{}</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>MS/TP Statistics</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">RX Frames</span>
                    <span class="value" id="rx_frames">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">TX Frames</span>
                    <span class="value" id="tx_frames">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Tokens Received</span>
                    <span class="value" id="tokens_received">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Send Queue</span>
                    <span class="value" id="send_queue">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Receive Queue</span>
                    <span class="value" id="receive_queue">{}</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>Token Loop Timing</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">Current</span>
                    <span class="value" id="token_loop">{} ms</span>
                </div>
                <div class="status-item">
                    <span class="label">Min</span>
                    <span class="value" id="token_loop_min">{} ms</span>
                </div>
                <div class="status-item">
                    <span class="label">Max</span>
                    <span class="value" id="token_loop_max">{} ms</span>
                </div>
                <div class="status-item">
                    <span class="label">Average</span>
                    <span class="value" id="token_loop_avg">{} ms</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>Errors</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">CRC Errors</span>
                    <span class="value {}" id="crc_errors">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Frame Errors</span>
                    <span class="value {}" id="frame_errors">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Reply Timeouts</span>
                    <span class="value {}" id="reply_timeouts">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Token Pass Fail</span>
                    <span class="value {}" id="token_pass_failures">{}</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>Gateway Routing</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">WiFi</span>
                    <span class="value {}">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">IP Address</span>
                    <span class="value auto-size">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">MS/TP to IP</span>
                    <span class="value" id="mstp_to_ip">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">IP to MS/TP</span>
                    <span class="value" id="ip_to_mstp">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Uptime</span>
                    <span class="value" id="uptime">{}</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>Network Configuration</h2>
            <div class="status-grid">
                <div class="status-item">
                    <span class="label">MS/TP Network</span>
                    <span class="value">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">IP Network</span>
                    <span class="value">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Station Address</span>
                    <span class="value">{}</span>
                </div>
                <div class="status-item">
                    <span class="label">Device Instance</span>
                    <span class="value">{}</span>
                </div>
            </div>
        </div>

        <div class="card">
            <h2>Tools</h2>
            <div class="button-row">
                <button class="btn" onclick="resetStats()">Reset Statistics</button>
                <button class="btn" onclick="exportData()">Export JSON</button>
            </div>
        </div>

        <div id="device-modal" class="modal" onclick="closeModal(event)">
            <div class="modal-content" onclick="event.stopPropagation()">
                <h3>Device Info</h3>
                <div id="modal-body"></div>
                <button class="btn" onclick="closeModal()">Close</button>
            </div>
        </div>

        <p class="footer">BACman v0.1.0</p>
    </div>
</body>
</html>"#,
        CSS_STYLES,
        masters_hex,
        state.mstp_stats.station_address,
        // Device Map card
        state.mstp_stats.master_count,
        generate_device_grid_html(state.mstp_stats.discovered_masters, state.mstp_stats.station_address),
        // State Machine card
        get_state_name(state.mstp_stats.current_state),
        if state.mstp_stats.sole_master { "warning" } else { "" },
        if state.mstp_stats.sole_master { "Yes" } else { "No" },
        state.mstp_stats.next_station,
        state.mstp_stats.poll_station,
        state.mstp_stats.silence_ms,
        state.mstp_stats.master_count,
        // MS/TP Statistics card
        state.mstp_stats.rx_frames,
        state.mstp_stats.tx_frames,
        state.mstp_stats.tokens_received,
        state.mstp_stats.send_queue_len,
        state.mstp_stats.receive_queue_len,
        // Token Loop Timing card
        state.mstp_stats.token_loop_time_ms,
        state.mstp_stats.token_loop_min_ms,
        state.mstp_stats.token_loop_max_ms,
        state.mstp_stats.token_loop_avg_ms,
        // Errors card
        if state.mstp_stats.crc_errors > 0 { "error" } else { "" },
        state.mstp_stats.crc_errors,
        if state.mstp_stats.frame_errors > 0 { "error" } else { "" },
        state.mstp_stats.frame_errors,
        if state.mstp_stats.reply_timeouts > 0 { "error" } else { "" },
        state.mstp_stats.reply_timeouts,
        if state.mstp_stats.token_pass_failures > 0 { "error" } else { "" },
        state.mstp_stats.token_pass_failures,
        // Gateway Routing card
        if state.wifi_connected { "ok" } else { "error" },
        if state.wifi_connected { "Connected" } else { "Disconnected" },
        state.ip_address,
        state.gateway_stats.mstp_to_ip_packets,
        state.gateway_stats.ip_to_mstp_packets,
        state.uptime_formatted(),
        // Network Configuration card
        state.config.mstp_network,
        state.config.ip_network,
        state.config.mstp_address,
        state.config.device_instance,
    )
}

/// Generate HTML for the device grid (128 cells for addresses 0-127)
fn generate_device_grid_html(discovered_masters: u128, station_address: u8) -> String {
    let mut html = String::with_capacity(8192);
    for i in 0..128u8 {
        let is_present = (discovered_masters >> i) & 1 == 1;
        let is_self = i == station_address;
        let class = if is_self {
            "grid-cell self"
        } else if is_present {
            "grid-cell active"
        } else {
            "grid-cell"
        };
        // Make active and self cells clickable to show device info
        if is_present || is_self {
            html.push_str(&format!(r#"<div class="{}" id="dev-{}" title="Address {}" onclick="showGridDeviceInfo({})">{}</div>"#, class, i, i, i, i));
        } else {
            html.push_str(&format!(r#"<div class="{}" id="dev-{}" title="Address {}">{}</div>"#, class, i, i, i));
        }
    }
    html
}

/// Get state name from state number
fn get_state_name(state: u8) -> &'static str {
    match state {
        0 => "Initialize",
        1 => "Idle",
        2 => "UseToken",
        3 => "WaitForReply",
        4 => "PassToken",
        5 => "NoToken",
        6 => "PollForMaster",
        7 => "AnswerDataRequest",
        8 => "DoneWithToken",
        _ => "Unknown",
    }
}

/// Generate configuration page HTML
fn generate_config_page(state: &WebState) -> String {
    generate_config_page_with_message(state, "")
}

/// Generate configuration page with message
fn generate_config_page_with_message(state: &WebState, message: &str) -> String {
    let message_html = if message.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="message">{}</div>"#, message)
    };

    format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>BACman Gateway - Configuration</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>{}</style>
</head>
<body>
    <div class="container">
        <h1>BACman Gateway</h1>
        <nav>
            <a href="/status">Status</a>
            <a href="/config" class="active">Configuration</a>
        </nav>

        {}

        <form method="POST" action="/config">
            <div class="card">
                <h2>WiFi Station Mode</h2>
                <p class="hint">Connect to an existing WiFi network</p>
                <div class="form-group">
                    <label for="wifi_ssid">SSID</label>
                    <input type="text" id="wifi_ssid" name="wifi_ssid" value="{}" maxlength="32">
                </div>
                <div class="form-group">
                    <label for="wifi_pass">Password</label>
                    <input type="password" id="wifi_pass" name="wifi_pass" placeholder="(leave blank to keep current)" maxlength="64">
                </div>
            </div>

            <div class="card">
                <h2>WiFi Access Point Mode</h2>
                <p class="hint">Create a WiFi hotspot (activate via long-press on APConfig screen)</p>
                <div class="form-group">
                    <label for="ap_ssid">AP SSID</label>
                    <input type="text" id="ap_ssid" name="ap_ssid" value="{}" maxlength="32">
                </div>
                <div class="form-group">
                    <label for="ap_pass">AP Password (min 8 chars)</label>
                    <input type="password" id="ap_pass" name="ap_pass" placeholder="(leave blank to keep current)" maxlength="64" minlength="8">
                </div>
            </div>

            <div class="card">
                <h2>MS/TP Settings</h2>
                <div class="form-group">
                    <label for="mstp_addr">Station Address (0-127)</label>
                    <input type="number" id="mstp_addr" name="mstp_addr" value="{}" min="0" max="127">
                </div>
                <div class="form-group">
                    <label for="mstp_max">Max Master (0-127)</label>
                    <input type="number" id="mstp_max" name="mstp_max" value="{}" min="0" max="127">
                </div>
                <div class="form-group">
                    <label for="mstp_baud">Baud Rate</label>
                    <select id="mstp_baud" name="mstp_baud">
                        <option value="9600" {}>9600</option>
                        <option value="19200" {}>19200</option>
                        <option value="38400" {}>38400</option>
                        <option value="57600" {}>57600</option>
                        <option value="76800" {}>76800</option>
                        <option value="115200" {}>115200</option>
                    </select>
                </div>
                <div class="form-group">
                    <label for="mstp_net">MS/TP Network Number</label>
                    <input type="number" id="mstp_net" name="mstp_net" value="{}" min="1" max="65534">
                </div>
            </div>

            <div class="card">
                <h2>BACnet/IP Settings</h2>
                <div class="form-group">
                    <label for="ip_port">UDP Port</label>
                    <input type="number" id="ip_port" name="ip_port" value="{}" min="1" max="65535">
                </div>
                <div class="form-group">
                    <label for="ip_net">IP Network Number</label>
                    <input type="number" id="ip_net" name="ip_net" value="{}" min="1" max="65534">
                </div>
            </div>

            <div class="card">
                <h2>Device Settings</h2>
                <div class="form-group">
                    <label for="dev_inst">Device Instance (0-4194303)</label>
                    <input type="number" id="dev_inst" name="dev_inst" value="{}" min="0" max="4194303">
                </div>
                <div class="form-group">
                    <label for="dev_name">Device Name</label>
                    <input type="text" id="dev_name" name="dev_name" value="{}" maxlength="64">
                </div>
            </div>

            <div class="button-row">
                <button type="submit" class="btn btn-primary">Apply Changes</button>
            </div>
        </form>

        <div class="card">
            <h2>Persist Settings</h2>
            <p>Save configuration to flash memory (NVS) for persistence across reboots.</p>
            <div class="button-row">
                <form method="POST" action="/save" style="display:inline">
                    <button type="submit" class="btn btn-success">Save to NVS</button>
                </form>
                <form method="POST" action="/reset" style="display:inline" onsubmit="return confirm('Reset all settings to defaults?')">
                    <button type="submit" class="btn btn-warning">Reset Defaults</button>
                </form>
                <form method="POST" action="/reboot" style="display:inline" onsubmit="return confirm('Reboot the gateway?')">
                    <button type="submit" class="btn btn-danger">Reboot</button>
                </form>
            </div>
        </div>

        <p class="footer">BACman v0.1.0 | Changes take effect after reboot</p>
    </div>
</body>
</html>"#,
        CSS_STYLES,
        message_html,
        state.config.wifi_ssid,
        state.config.ap_ssid,
        state.config.mstp_address,
        state.config.mstp_max_master,
        if state.config.mstp_baud_rate == 9600 { "selected" } else { "" },
        if state.config.mstp_baud_rate == 19200 { "selected" } else { "" },
        if state.config.mstp_baud_rate == 38400 { "selected" } else { "" },
        if state.config.mstp_baud_rate == 57600 { "selected" } else { "" },
        if state.config.mstp_baud_rate == 76800 { "selected" } else { "" },
        if state.config.mstp_baud_rate == 115200 { "selected" } else { "" },
        state.config.mstp_network,
        state.config.bacnet_ip_port,
        state.config.ip_network,
        state.config.device_instance,
        state.config.device_name,
    )
}

/// Generate status JSON for API endpoint
fn generate_status_json(state: &WebState) -> String {
    // Convert discovered_masters bitmap to hex string for the device grid
    let masters_hex = format!("{:032x}", state.mstp_stats.discovered_masters);

    format!(r#"{{"rx_frames":{},"tx_frames":{},"crc_errors":{},"frame_errors":{},"reply_timeouts":{},"tokens_received":{},"token_pass_failures":{},"token_loop_ms":{},"token_loop_min_ms":{},"token_loop_max_ms":{},"token_loop_avg_ms":{},"master_count":{},"mstp_to_ip":{},"ip_to_mstp":{},"wifi_connected":{},"discovered_masters":"{}","current_state":{},"next_station":{},"poll_station":{},"silence_ms":{},"station_address":{},"sole_master":{},"send_queue_len":{},"receive_queue_len":{},"uptime_secs":{},"uptime":"{}"}}"#,
        state.mstp_stats.rx_frames,
        state.mstp_stats.tx_frames,
        state.mstp_stats.crc_errors,
        state.mstp_stats.frame_errors,
        state.mstp_stats.reply_timeouts,
        state.mstp_stats.tokens_received,
        state.mstp_stats.token_pass_failures,
        state.mstp_stats.token_loop_time_ms,
        state.mstp_stats.token_loop_min_ms,
        state.mstp_stats.token_loop_max_ms,
        state.mstp_stats.token_loop_avg_ms,
        state.mstp_stats.master_count,
        state.gateway_stats.mstp_to_ip_packets,
        state.gateway_stats.ip_to_mstp_packets,
        state.wifi_connected,
        masters_hex,
        state.mstp_stats.current_state,
        state.mstp_stats.next_station,
        state.mstp_stats.poll_station,
        state.mstp_stats.silence_ms,
        state.mstp_stats.station_address,
        state.mstp_stats.sole_master,
        state.mstp_stats.send_queue_len,
        state.mstp_stats.receive_queue_len,
        state.uptime_secs(),
        state.uptime_formatted(),
    )
}

/// Generate export JSON with all diagnostic data
fn generate_export_json(state: &WebState) -> String {
    let masters_hex = format!("{:032x}", state.mstp_stats.discovered_masters);

    // Build list of discovered device addresses
    let mut devices = Vec::new();
    for i in 0..128u8 {
        if (state.mstp_stats.discovered_masters >> i) & 1 == 1 {
            devices.push(i);
        }
    }
    let devices_str: Vec<String> = devices.iter().map(|d| d.to_string()).collect();

    format!(r#"{{
  "export_time": "{}",
  "uptime_secs": {},
  "uptime": "{}",
  "device": {{
    "name": "{}",
    "instance": {},
    "station_address": {},
    "ip_address": "{}"
  }},
  "networks": {{
    "mstp_network": {},
    "ip_network": {},
    "baud_rate": {}
  }},
  "mstp_stats": {{
    "rx_frames": {},
    "tx_frames": {},
    "tokens_received": {},
    "crc_errors": {},
    "frame_errors": {},
    "reply_timeouts": {},
    "token_pass_failures": {},
    "master_count": {},
    "discovered_masters_hex": "{}",
    "discovered_addresses": [{}]
  }},
  "token_loop_timing": {{
    "current_ms": {},
    "min_ms": {},
    "max_ms": {},
    "avg_ms": {}
  }},
  "queues": {{
    "send_queue_len": {},
    "receive_queue_len": {}
  }},
  "state_machine": {{
    "current_state": "{}",
    "sole_master": {},
    "next_station": {},
    "poll_station": {},
    "silence_ms": {}
  }},
  "gateway_stats": {{
    "mstp_to_ip_packets": {},
    "ip_to_mstp_packets": {}
  }},
  "wifi": {{
    "connected": {},
    "ssid": "{}"
  }}
}}"#,
        chrono_lite_timestamp(),
        state.uptime_secs(),
        state.uptime_formatted(),
        state.config.device_name,
        state.config.device_instance,
        state.mstp_stats.station_address,
        state.ip_address,
        state.config.mstp_network,
        state.config.ip_network,
        state.config.mstp_baud_rate,
        state.mstp_stats.rx_frames,
        state.mstp_stats.tx_frames,
        state.mstp_stats.tokens_received,
        state.mstp_stats.crc_errors,
        state.mstp_stats.frame_errors,
        state.mstp_stats.reply_timeouts,
        state.mstp_stats.token_pass_failures,
        state.mstp_stats.master_count,
        masters_hex,
        devices_str.join(","),
        state.mstp_stats.token_loop_time_ms,
        state.mstp_stats.token_loop_min_ms,
        state.mstp_stats.token_loop_max_ms,
        state.mstp_stats.token_loop_avg_ms,
        state.mstp_stats.send_queue_len,
        state.mstp_stats.receive_queue_len,
        get_state_name(state.mstp_stats.current_state),
        state.mstp_stats.sole_master,
        state.mstp_stats.next_station,
        state.mstp_stats.poll_station,
        state.mstp_stats.silence_ms,
        state.gateway_stats.mstp_to_ip_packets,
        state.gateway_stats.ip_to_mstp_packets,
        state.wifi_connected,
        state.config.wifi_ssid,
    )
}

/// Simple timestamp (uptime in seconds since no RTC)
fn chrono_lite_timestamp() -> String {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("uptime_{}s", uptime)
}

/// Generate JSON for discovered devices
fn generate_devices_json(state: &WebState) -> String {
    let mut json = String::from(r#"{"scan_in_progress":"#);
    json.push_str(if state.scan_in_progress { "true" } else { "false" });
    json.push_str(r#","devices":["#);

    for (i, device) in state.discovered_devices.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"mac":{},"instance":{},"vendor":{},"max_apdu":{},"segmentation":{}}}"#,
            device.mac_address,
            device.device_instance,
            device.vendor_id,
            device.max_apdu_length,
            device.segmentation
        ));
    }

    json.push_str("]}");
    json
}

/// CSS styles - Modern monochrome design
const CSS_STYLES: &str = r#"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace; background: #0a0a0a; color: #e0e0e0; line-height: 1.6; }
.container { max-width: 800px; margin: 0 auto; padding: 24px; }
h1 { color: #fff; text-align: center; margin-bottom: 24px; font-size: 1.5em; font-weight: 600; letter-spacing: 2px; text-transform: uppercase; }
h2 { color: #fff; margin-bottom: 10px; font-size: 0.8em; font-weight: 500; letter-spacing: 1px; text-transform: uppercase; border-bottom: 1px solid #2a2a2a; padding-bottom: 6px; }
nav { display: flex; justify-content: center; gap: 4px; margin-bottom: 24px; }
nav a { color: #666; text-decoration: none; padding: 10px 24px; font-size: 0.85em; letter-spacing: 1px; text-transform: uppercase; border: 1px solid #222; transition: all 0.2s; }
nav a:hover { color: #fff; border-color: #444; }
nav a.active { color: #fff; background: #1a1a1a; border-color: #333; }
.card { background: #111; border: 1px solid #222; padding: 16px; margin-bottom: 12px; }
.card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; border-bottom: 1px solid #2a2a2a; padding-bottom: 6px; }
.card-header h2 { margin-bottom: 0; border-bottom: none; padding-bottom: 0; }
.status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 6px; }
.status-item { background: #0a0a0a; border: 1px solid #1a1a1a; padding: 8px 10px; text-align: center; }
.status-item .label { display: block; color: #555; font-size: 0.65em; letter-spacing: 1px; text-transform: uppercase; margin-bottom: 2px; }
.status-item .value { display: block; font-size: 1.1em; font-weight: 600; color: #fff; font-variant-numeric: tabular-nums; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.status-item .value.auto-size { font-size: clamp(0.7em, 2.5vw, 1.1em); }
.chip { display: inline-block; background: #333; color: #fff; padding: 2px 8px; font-size: 0.7em; font-weight: 400; margin-left: 8px; vertical-align: middle; }
.status-item .value.ok { color: #888; }
.status-item .value.error { color: #fff; background: #333; padding: 2px 8px; }
.status-item .value.warning { color: #000; background: #fff; padding: 2px 8px; animation: blink 1s infinite; }
@keyframes blink { 50% { opacity: 0.5; } }
.device-grid { display: grid; grid-template-columns: repeat(16, 1fr); gap: 2px; margin-bottom: 12px; }
.grid-cell { aspect-ratio: 1; background: #1a1a1a; border: 1px solid #222; display: flex; align-items: center; justify-content: center; font-size: 0.55em; color: #333; transition: all 0.2s; cursor: default; }
.grid-cell.active { background: #333; color: #fff; border-color: #444; }
.grid-cell.self { background: #fff; color: #000; border-color: #fff; font-weight: bold; }
.grid-legend { display: flex; gap: 16px; justify-content: center; font-size: 0.75em; color: #666; }
.legend-box { display: inline-block; width: 12px; height: 12px; border: 1px solid #333; margin-right: 4px; vertical-align: middle; }
.legend-box.active { background: #333; }
.legend-box.self { background: #fff; }
.form-group { margin-bottom: 16px; }
.form-group label { display: block; margin-bottom: 6px; color: #666; font-size: 0.75em; letter-spacing: 1px; text-transform: uppercase; }
.hint { color: #555; font-size: 0.8em; margin: -8px 0 12px 0; font-style: italic; }
.form-group input, .form-group select { width: 100%; padding: 12px; border: 1px solid #222; background: #0a0a0a; color: #fff; font-size: 0.95em; font-family: inherit; transition: border-color 0.2s; }
.form-group input:focus, .form-group select:focus { outline: none; border-color: #444; }
.form-group input::placeholder { color: #444; }
.button-row { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 12px; }
.btn { padding: 8px 16px; border: 1px solid #333; background: transparent; color: #fff; cursor: pointer; font-size: 0.75em; font-family: inherit; letter-spacing: 1px; text-transform: uppercase; transition: all 0.2s; }
.btn:hover { background: #1a1a1a; border-color: #444; }
.btn-sm { padding: 4px 10px; font-size: 0.65em; }
.btn-primary { background: #fff; color: #000; border-color: #fff; }
.btn-primary:hover { background: #ccc; border-color: #ccc; }
.btn-success { background: #333; border-color: #444; }
.btn-success:hover { background: #444; }
.btn-warning { background: #222; border-color: #333; }
.btn-warning:hover { background: #333; }
.btn-danger { background: #1a1a1a; border-color: #333; color: #888; }
.btn-danger:hover { background: #2a2a2a; color: #fff; }
.message { background: #111; border-left: 2px solid #444; padding: 16px; margin-bottom: 20px; font-size: 0.9em; }
.footer { text-align: center; color: #333; margin-top: 32px; font-size: 0.75em; letter-spacing: 1px; }
.footer a { color: #555; text-decoration: none; }
.footer a:hover { color: #888; }
.modal { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.8); justify-content: center; align-items: center; z-index: 1000; }
.modal-content { background: #111; border: 1px solid #333; padding: 24px; max-width: 400px; width: 90%; }
.modal-content h3 { margin-bottom: 16px; font-size: 1em; letter-spacing: 1px; text-transform: uppercase; border-bottom: 1px solid #222; padding-bottom: 8px; }
.modal-content p { margin: 8px 0; font-size: 0.9em; }
.modal-content p b { color: #888; }
.device-row { display: flex; justify-content: space-between; padding: 12px; margin: 4px 0; background: #0a0a0a; border: 1px solid #1a1a1a; cursor: pointer; font-size: 0.85em; transition: all 0.2s; }
.device-row:hover { background: #1a1a1a; border-color: #333; }
.device-row span { color: #888; }
.scan-status { color: #666; font-size: 0.85em; margin-bottom: 8px; }
.grid-cell.active { cursor: pointer; }
.grid-cell.active:hover { background: #444; transform: scale(1.1); }
@media (max-width: 600px) { .container { padding: 16px; } .card { padding: 16px; } .btn { padding: 10px 16px; } .device-grid { grid-template-columns: repeat(8, 1fr); } .grid-cell { font-size: 0.5em; } }
"#;

/// HTML redirect to status page
const HTML_REDIRECT_STATUS: &str = r#"<!DOCTYPE html>
<html><head><meta http-equiv="refresh" content="0;url=/status"></head>
<body>Redirecting to <a href="/status">status page</a>...</body></html>"#;

/// HTML reboot page
const HTML_REBOOT_PAGE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>BACman Gateway - Rebooting</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace; background: #0a0a0a; color: #e0e0e0; display: flex; justify-content: center; align-items: center; min-height: 100vh; }
        .message { text-align: center; }
        h1 { color: #fff; font-size: 1.2em; font-weight: 500; letter-spacing: 2px; text-transform: uppercase; }
        .spinner { width: 40px; height: 40px; border: 2px solid #222; border-top: 2px solid #fff; border-radius: 50%; animation: spin 1s linear infinite; margin: 24px auto; }
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
        p { color: #555; font-size: 0.85em; letter-spacing: 1px; }
    </style>
    <script>setTimeout(() => location.href = '/status', 10000);</script>
</head>
<body>
    <div class="message">
        <h1>Rebooting</h1>
        <div class="spinner"></div>
        <p>The gateway is restarting. You will be redirected automatically.</p>
    </div>
</body>
</html>"#;

/// Parse BDT add form data
fn parse_bdt_add_form(body: &str, state: &mut WebState) -> &'static str {
    let mut ip_str = String::new();
    let mut port: u16 = 47808;
    let mut mask_str = String::new();

    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let value = urlencoding::decode(value).unwrap_or_default();

        match key {
            "ip" => ip_str = value.to_string(),
            "port" => {
                if let Ok(p) = value.parse::<u16>() {
                    port = p;
                }
            }
            "mask" => mask_str = value.to_string(),
            _ => {}
        }
    }

    // Parse IP address
    let ip: Ipv4Addr = match ip_str.parse() {
        Ok(ip) => ip,
        Err(_) => return "Invalid IP address format",
    };

    // Parse subnet mask (default to 255.255.255.255 for host-specific)
    let mask: Ipv4Addr = if mask_str.is_empty() {
        Ipv4Addr::new(255, 255, 255, 255)
    } else {
        match mask_str.parse() {
            Ok(m) => m,
            Err(_) => return "Invalid subnet mask format",
        }
    };

    // Create socket address
    let addr = SocketAddr::new(std::net::IpAddr::V4(ip), port);

    // Set request for main loop to process
    state.bdt_add_request = Some((addr, mask));
    info!("BDT add requested via web portal: {} mask {}", addr, mask);

    "BDT entry add requested. Entry will be added."
}

/// Parse BDT remove form data
fn parse_bdt_remove_form(body: &str, state: &mut WebState) -> &'static str {
    let mut addr_str = String::new();

    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let value = urlencoding::decode(value).unwrap_or_default();

        if key == "addr" {
            addr_str = value.to_string();
        }
    }

    // Parse socket address (format: "IP:port")
    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(_) => return "Invalid address format (expected IP:port)",
    };

    state.bdt_remove_request = Some(addr);
    info!("BDT remove requested via web portal: {}", addr);

    "BDT entry remove requested. Entry will be removed."
}

/// Generate BDT JSON
fn generate_bdt_json(state: &WebState) -> String {
    let entries: Vec<String> = state.bdt_entries
        .iter()
        .map(|(addr, mask)| {
            format!(
                r#"{{"address":"{}","mask":"{}"}}"#,
                addr, mask
            )
        })
        .collect();

    format!(r#"{{"entries":[{}]}}"#, entries.join(","))
}

/// Generate BDT page HTML
fn generate_bdt_page(state: &WebState) -> String {
    generate_bdt_page_with_message(state, "")
}

/// Generate BDT page HTML with optional message
fn generate_bdt_page_with_message(state: &WebState, message: &str) -> String {
    let msg_html = if message.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="message">{}</div>"#, message)
    };

    let entries_html: String = if state.bdt_entries.is_empty() {
        r#"<p style="color: #555; text-align: center;">No BDT entries configured</p>"#.to_string()
    } else {
        state.bdt_entries
            .iter()
            .map(|(addr, mask)| {
                format!(
                    r#"<div class="bdt-entry">
                        <span class="addr">{}</span>
                        <span class="mask">mask: {}</span>
                        <form method="POST" action="/bdt/remove" style="display:inline">
                            <input type="hidden" name="addr" value="{}">
                            <button type="submit" class="btn btn-small btn-danger">Remove</button>
                        </form>
                    </div>"#,
                    addr, mask, addr
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>BACman Gateway - BDT Configuration</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>{}</style>
    <style>
        .bdt-entry {{ display: flex; align-items: center; gap: 16px; padding: 12px; background: #111; border: 1px solid #222; margin-bottom: 8px; }}
        .bdt-entry .addr {{ color: #fff; font-weight: 500; min-width: 180px; }}
        .bdt-entry .mask {{ color: #666; flex: 1; }}
        .btn-small {{ padding: 4px 12px; font-size: 0.7em; }}
        .btn-danger {{ border-color: #633; }}
        .btn-danger:hover {{ background: #633; border-color: #844; }}
        .add-form {{ background: #111; border: 1px solid #222; padding: 16px; margin-top: 16px; }}
        .add-form h3 {{ margin-bottom: 16px; font-size: 0.9em; }}
        .form-row {{ display: flex; gap: 12px; align-items: end; flex-wrap: wrap; }}
        .form-row .form-group {{ margin-bottom: 0; }}
        .form-group.small {{ max-width: 100px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>BACman Gateway</h1>
        <nav>
            <a href="/status">Status</a>
            <a href="/config">Config</a>
            <a href="/bdt" class="active">BDT</a>
        </nav>

        {}

        <div class="card">
            <h2>Broadcast Distribution Table</h2>
            <p style="color: #555; font-size: 0.8em; margin-bottom: 16px;">
                BDT entries define peer BBMDs for broadcast distribution across subnets.
            </p>
            {}
        </div>

        <div class="add-form">
            <h3>Add BDT Entry</h3>
            <form method="POST" action="/bdt/add">
                <div class="form-row">
                    <div class="form-group">
                        <label>IP Address</label>
                        <input type="text" name="ip" placeholder="192.168.1.100" required>
                    </div>
                    <div class="form-group small">
                        <label>Port</label>
                        <input type="number" name="port" value="47808" min="1" max="65535">
                    </div>
                    <div class="form-group">
                        <label>Subnet Mask</label>
                        <input type="text" name="mask" placeholder="255.255.255.255">
                    </div>
                    <button type="submit" class="btn">Add Entry</button>
                </div>
            </form>
        </div>

        <div style="margin-top: 16px; display: flex; gap: 8px;">
            <form method="POST" action="/bdt/clear" onsubmit="return confirm('Clear all BDT entries?')">
                <button type="submit" class="btn btn-danger">Clear All Entries</button>
            </form>
        </div>
    </div>
</body>
</html>"#,
        CSS_STYLES,
        msg_html,
        entries_html
    )
}

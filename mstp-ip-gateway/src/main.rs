//! BACnet MS/TP to IP Gateway for M5StickC Plus2
//!
//! This firmware creates a BACnet router that bridges MS/TP (RS-485) and BACnet/IP networks.
//!
//! ## Production Features
//! - NVS-based configuration persistence
//! - WiFi auto-reconnection
//! - Watchdog timer for automatic recovery
//! - Panic handler with automatic restart
//! - Serial console for runtime configuration

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::PinDriver,
        peripheral::Peripheral,
        prelude::*,
        spi::{SpiDeviceDriver, SpiDriver, SpiDriverConfig, config::Config as SpiConfig},
        uart::{config::Config as UartConfig, UartDriver},
        units::Hertz,
        task::watchdog::{TWDTConfig, TWDTDriver},
    },
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{error, info, warn};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod config;
mod display;
mod gateway;
mod local_device;
mod mstp_driver;
mod web;

use config::GatewayConfig;
use display::{Display, DisplayScreen, GatewayStatus};
use gateway::BacnetGateway;
use local_device::LocalDevice;
use mstp_driver::MstpDriver;
use web::{WebState, start_web_server};

/// Global flag for WiFi connection status (used by reconnection logic)
static WIFI_CONNECTED: AtomicBool = AtomicBool::new(false);

/// WiFi reconnection interval in seconds
const WIFI_RECONNECT_INTERVAL_SECS: u64 = 10;

/// Watchdog timeout in seconds
const WATCHDOG_TIMEOUT_SECS: u64 = 30;

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Set up panic handler for automatic restart
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
        error!("Restarting in 3 seconds...");
        thread::sleep(Duration::from_secs(3));
        unsafe { esp_idf_svc::sys::esp_restart(); }
    }));

    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║           BACman - BACnet MS/TP to IP Gateway                ║");
    info!("║              Hardware: M5StickC Plus2 + RS-485 HAT           ║");
    info!("╚══════════════════════════════════════════════════════════════╝");

    // Get peripherals
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Clone NVS partition for config loading and console
    let nvs_for_config = nvs.clone();
    let nvs_for_console = nvs.clone();

    // Initialize Task Watchdog Timer (TWDT)
    info!("Initializing watchdog timer...");
    let twdt_config = TWDTConfig {
        duration: Duration::from_secs(WATCHDOG_TIMEOUT_SECS),
        panic_on_trigger: true,
        subscribed_idle_tasks: enumset::EnumSet::empty(),
    };
    let mut twdt_driver = TWDTDriver::new(peripherals.twdt, &twdt_config)?;
    let mut watchdog = twdt_driver.watch_current_task()?;
    info!("Watchdog timer initialized with {}s timeout", WATCHDOG_TIMEOUT_SECS);

    // Initialize LCD Display
    // M5StickC Plus2 ST7789V2: MOSI=15, SCK=13, CS=5, DC=14, RST=12, BL=27
    info!("Initializing LCD display...");
    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio13, // SCK
        peripherals.pins.gpio15, // MOSI
        None::<esp_idf_svc::hal::gpio::Gpio12>, // MISO not used
        &SpiDriverConfig::new(),
    )?;

    let spi_config = SpiConfig::new()
        .baudrate(Hertz(26_000_000))  // Max supported without IOMUX pins
        .data_mode(esp_idf_svc::hal::spi::config::MODE_0);

    let spi_device = SpiDeviceDriver::new(
        spi_driver,
        Some(peripherals.pins.gpio5), // CS
        &spi_config,
    )?;

    let dc = PinDriver::output(peripherals.pins.gpio14)?;
    let rst = PinDriver::output(peripherals.pins.gpio12)?;
    let backlight = PinDriver::output(peripherals.pins.gpio27)?;

    let mut lcd = Display::new(spi_device, dc, rst, backlight)?;
    lcd.show_splash_screen()?;
    info!("LCD display initialized");

    // Show splash screen for 2 seconds
    thread::sleep(Duration::from_secs(2));

    // Initialize buttons (active low)
    // Button A (front): GPIO37 - big button on front
    // Button B (side): GPIO39 - small button on side
    // Button C (power): GPIO35 - power/menu button
    // Note: These are input-only pins on ESP32 with external pull-ups on M5StickC Plus2
    let btn_a = PinDriver::input(peripherals.pins.gpio37)?;
    let btn_b = PinDriver::input(peripherals.pins.gpio39)?;
    let btn_c = PinDriver::input(peripherals.pins.gpio35)?;
    info!("Buttons initialized (A=GPIO37, B=GPIO39, C=GPIO35)");

    // Load configuration from NVS (falls back to defaults if not configured)
    let config = match GatewayConfig::load_from_nvs(nvs_for_config) {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!("Failed to load config from NVS: {}, using defaults", e);
            GatewayConfig::default()
        }
    };
    info!("Configuration loaded:");
    info!("  MS/TP Station Address: {}", config.mstp_address);
    info!("  MS/TP Network Number: {}", config.mstp_network);
    info!("  IP Network Number: {}", config.ip_network);
    info!("  Device Instance: {}", config.device_instance);

    // Initialize WiFi with retry logic
    info!("Initializing WiFi...");
    lcd.show_wifi_connecting(&config.wifi_ssid)?;

    let wifi = init_wifi_with_retry(
        peripherals.modem,
        sys_loop.clone(),
        nvs,
        &config.wifi_ssid,
        &config.wifi_password,
        3, // max retries
    ).unwrap_or_else(|e| {
        error!("WiFi initialization failed after retries: {}", e);
        error!("Restarting...");
        thread::sleep(Duration::from_secs(3));
        unsafe { esp_idf_svc::sys::esp_restart(); }
        // This loop satisfies the type checker - esp_restart() doesn't return
        #[allow(unreachable_code)]
        loop { thread::sleep(Duration::from_secs(1)); }
    });

    WIFI_CONNECTED.store(true, Ordering::SeqCst);
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("WiFi connected!");
    info!("  IP Address: {}", ip_info.ip);
    info!("  Subnet: {}", ip_info.subnet.mask);
    info!("  Gateway: {}", ip_info.subnet.gateway);

    // Initialize RS-485 UART for MS/TP
    // M5StickC Plus2 RS-485 HAT pinout:
    //   HAT UART_RX connects to ESP32 G0 (so ESP32 TX -> G0)
    //   HAT UART_TX connects to ESP32 G26 (so ESP32 RX <- G26)
    info!("Initializing RS-485 UART...");
    let uart_config = UartConfig::default()
        .baudrate(Hertz(config.mstp_baud_rate))
        .data_bits(esp_idf_svc::hal::uart::config::DataBits::DataBits8)
        .parity_none()
        .stop_bits(esp_idf_svc::hal::uart::config::StopBits::STOP1);

    let uart = UartDriver::new(
        peripherals.uart1,
        peripherals.pins.gpio0,  // TX - per M5Stack RS-485 HAT standard
        peripherals.pins.gpio26, // RX - per M5Stack RS-485 HAT standard
        Option::<esp_idf_svc::hal::gpio::Gpio27>::None, // CTS (not used)
        Option::<esp_idf_svc::hal::gpio::Gpio27>::None, // RTS (not used)
        &uart_config,
    )?;

    info!("RS-485 UART initialized at {} baud", config.mstp_baud_rate);
    info!("Note: M5Stack RS-485 HAT has automatic direction control (SP485EEN)");

    // Create MS/TP driver
    // Note: No GPIO direction pin needed - HAT has automatic TX/RX switching
    let mstp_driver = Arc::new(Mutex::new(MstpDriver::new(
        uart,
        config.mstp_address,
        config.mstp_max_master,
    )));

    // Create BACnet/IP UDP socket
    info!("Creating BACnet/IP socket...");
    let bind_addr = format!("0.0.0.0:{}", config.bacnet_ip_port);
    let socket = UdpSocket::bind(&bind_addr)?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    info!("BACnet/IP socket bound to {}", bind_addr);

    // Create gateway
    let gateway = Arc::new(Mutex::new(BacnetGateway::new(
        config.mstp_network,
        config.ip_network,
    )));

    // Create local BACnet device for gateway discoverability
    let local_device = Arc::new(LocalDevice::new_with_mstp(
        config.device_instance,
        config.mstp_max_master,
        1, // max_info_frames
    ));
    info!("Local BACnet device created: instance {}", config.device_instance);

    // Wrap WiFi in Arc<Mutex> for sharing with main loop (for reconnection)
    let wifi = Arc::new(Mutex::new(wifi));

    // Wrap socket in Arc for sharing between threads
    // (try_clone() doesn't work on ESP-IDF)
    let socket = Arc::new(socket);

    // Create web server state early so it can be shared with receive tasks
    let web_state = Arc::new(Mutex::new(WebState::new(config.clone(), Some(nvs_for_console))));

    // Spawn MS/TP receive thread
    let mstp_driver_clone = Arc::clone(&mstp_driver);
    let gateway_clone = Arc::clone(&gateway);
    let local_device_clone = Arc::clone(&local_device);
    let web_state_mstp = Arc::clone(&web_state);
    let _mstp_thread = thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            mstp_receive_task(mstp_driver_clone, gateway_clone, local_device_clone, web_state_mstp);
        })?;

    // Spawn BACnet/IP receive thread
    let socket_clone = Arc::clone(&socket);
    let gateway_clone = Arc::clone(&gateway);
    let mstp_driver_clone = Arc::clone(&mstp_driver);
    let local_device_clone = Arc::clone(&local_device);
    let _ip_thread = thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            ip_receive_task(socket_clone, gateway_clone, mstp_driver_clone, local_device_clone);
        })?;

    info!("Gateway running!");
    info!("  MS/TP Network {} <-> IP Network {}", config.mstp_network, config.ip_network);

    // Status tracking for display
    let mut status = GatewayStatus {
        wifi_connected: true,
        ip_address: ip_info.ip.to_string(),
        mstp_network: config.mstp_network,
        ip_network: config.ip_network,
        rx_frames: 0,
        tx_frames: 0,
        crc_errors: 0,
        token_loop_ms: 0,
        master_count: 0,
        // Connection screen fields
        mstp_address: config.mstp_address,
        mstp_max_master: config.mstp_max_master,
        mstp_baud_rate: config.mstp_baud_rate,
        mstp_state: "Initialize".to_string(),
        has_token: false,
    };

    // Display screen cycling with Button A
    let mut current_screen = DisplayScreen::Status;
    let mut btn_a_was_pressed = false;
    let mut btn_b_was_pressed = false;
    let mut btn_c_was_pressed = false;

    // WiFi reconnection tracking
    let mut wifi_check_counter: u32 = 0;
    const WIFI_CHECK_INTERVAL: u32 = 50; // Check every 5 seconds (50 * 100ms)

    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║                    Gateway Running!                          ║");
    info!("╚══════════════════════════════════════════════════════════════╝");

    // Update initial web state (web_state was created earlier for thread sharing)
    {
        let mut state = web_state.lock().unwrap();
        state.wifi_connected = true;
        state.ip_address = ip_info.ip.to_string();
    }

    // Start web server for configuration portal
    let web_state_clone = Arc::clone(&web_state);
    let _web_server = match start_web_server(web_state_clone) {
        Ok(server) => {
            info!("Web portal available at http://{}/", ip_info.ip);
            Some(server)
        }
        Err(e) => {
            error!("Failed to start web server: {}", e);
            None
        }
    };

    loop {
        // Feed the watchdog to prevent reset
        watchdog.feed()?;

        // Process any pending gateway tasks
        if let Ok(mut gw) = gateway.lock() {
            gw.process_housekeeping();
        }

        // Get MS/TP driver stats
        if let Ok(mut driver) = mstp_driver.lock() {
            let mstp_stats = driver.get_stats();
            status.rx_frames = mstp_stats.rx_frames;
            status.tx_frames = mstp_stats.tx_frames;
            status.crc_errors = mstp_stats.crc_errors;
            status.token_loop_ms = mstp_stats.token_loop_time_ms;
            status.master_count = mstp_stats.master_count;
            // Connection screen fields
            status.mstp_state = driver.get_state_name().to_string();
            status.has_token = driver.has_token();

            // Update web state with MS/TP stats
            if let Ok(mut web) = web_state.lock() {
                web.mstp_stats = mstp_stats;

                // Check if stats reset was requested from web portal
                if web.reset_stats_requested {
                    driver.reset_stats();
                    web.reset_stats_requested = false;
                    info!("Statistics reset completed");
                }

                // Check if Who-Is scan was requested from web portal
                if web.scan_requested {
                    web.scan_requested = false;
                    info!("Who-Is scan requested - sending broadcast");

                    // Build Who-Is APDU
                    let who_is_apdu = LocalDevice::build_who_is();

                    // Wrap in NPDU for LOCAL broadcast (no network layer addressing)
                    // For local MS/TP broadcast, we use simple NPDU without DNET
                    // Control byte 0x00 = no destination network, no source network, APDU follows
                    let mut npdu = Vec::with_capacity(who_is_apdu.len() + 2);
                    npdu.push(0x01); // NPDU version
                    npdu.push(0x00); // Control: no network layer info, expecting reply
                    npdu.extend_from_slice(&who_is_apdu);

                    // Send as broadcast via MS/TP (destination MAC = 0xFF for broadcast)
                    if let Err(e) = driver.send_frame(&npdu, 0xFF, false) {
                        warn!("Failed to send Who-Is broadcast: {}", e);
                    } else {
                        info!("Who-Is broadcast sent ({} bytes)", npdu.len());
                    }

                    // Set a timeout for scan to complete (handled by JavaScript)
                }
            }
        }

        // Get gateway stats for web portal
        if let Ok(gw) = gateway.lock() {
            let gw_stats = gw.get_stats();
            if let Ok(mut web) = web_state.lock() {
                web.gateway_stats.mstp_to_ip_packets = gw_stats.mstp_to_ip_packets;
                web.gateway_stats.ip_to_mstp_packets = gw_stats.ip_to_mstp_packets;
            }
        }

        // Periodically check WiFi connection and attempt reconnection if needed
        wifi_check_counter += 1;
        if wifi_check_counter >= WIFI_CHECK_INTERVAL {
            wifi_check_counter = 0;

            if let Ok(mut wifi_guard) = wifi.lock() {
                let connected = check_wifi_connection(&mut wifi_guard);
                if status.wifi_connected != connected {
                    status.wifi_connected = connected;
                    // Force display update when WiFi status changes
                    if current_screen != DisplayScreen::Splash {
                        lcd.clear_and_reset().ok();
                    }
                    // Update web state
                    if let Ok(mut web) = web_state.lock() {
                        web.wifi_connected = connected;
                    }
                }
            }
        }

        // Handle button A (front big button) - cycle through screens
        let btn_a_pressed = btn_a.is_low();
        if btn_a_pressed && !btn_a_was_pressed {
            current_screen = current_screen.next();
            info!("Button A pressed - screen: {:?}", current_screen);
            // Force full redraw when switching screens
            lcd.clear_and_reset().ok();
            if current_screen == DisplayScreen::Splash {
                lcd.show_splash_screen().ok();
            }
        }
        btn_a_was_pressed = btn_a_pressed;

        // Handle button B (side) - force current screen refresh
        let btn_b_pressed = btn_b.is_low();
        if btn_b_pressed && !btn_b_was_pressed {
            info!("Button B pressed - refresh display");
            lcd.clear_and_reset().ok();
            if current_screen == DisplayScreen::Splash {
                lcd.show_splash_screen().ok();
            }
        }
        btn_b_was_pressed = btn_b_pressed;

        // Handle button C (power) - jump to Status screen
        let btn_c_pressed = btn_c.is_low();
        if btn_c_pressed && !btn_c_was_pressed {
            info!("Button C pressed - go to Status screen");
            current_screen = DisplayScreen::Status;
            lcd.clear_and_reset().ok();
        }
        btn_c_was_pressed = btn_c_pressed;

        // Update display based on current screen
        match current_screen {
            DisplayScreen::Status => {
                if let Err(e) = lcd.update_status(&status) {
                    warn!("Failed to update status display: {}", e);
                }
            }
            DisplayScreen::Connection => {
                if let Err(e) = lcd.update_connection(&status) {
                    warn!("Failed to update connection display: {}", e);
                }
            }
            DisplayScreen::Splash => {
                // Splash screen is static, no updates needed
            }
        }

        // Small delay to prevent busy-waiting
        thread::sleep(Duration::from_millis(100));
    }
}

/// Initialize WiFi with retry logic
fn init_wifi_with_retry(
    modem: impl Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    password: &str,
    max_retries: u32,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: password.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;

    // Try to connect with retries
    let mut last_error = None;
    for attempt in 1..=max_retries {
        info!("WiFi connection attempt {}/{} to '{}'...", attempt, max_retries, ssid);

        match wifi.connect() {
            Ok(_) => {
                info!("WiFi connected, waiting for DHCP...");
                match wifi.wait_netif_up() {
                    Ok(_) => {
                        info!("WiFi fully connected!");
                        return Ok(wifi);
                    }
                    Err(e) => {
                        warn!("DHCP failed: {}", e);
                        last_error = Some(e.into());
                    }
                }
            }
            Err(e) => {
                warn!("WiFi connection failed: {}", e);
                last_error = Some(e.into());
            }
        }

        if attempt < max_retries {
            info!("Retrying in {} seconds...", WIFI_RECONNECT_INTERVAL_SECS);
            thread::sleep(Duration::from_secs(WIFI_RECONNECT_INTERVAL_SECS));
            // Disconnect before retry
            let _ = wifi.disconnect();
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("WiFi connection failed")))
}

/// Check WiFi connection and attempt reconnection if needed
fn check_wifi_connection(wifi: &mut BlockingWifi<EspWifi<'static>>) -> bool {
    if wifi.is_connected().unwrap_or(false) {
        if !WIFI_CONNECTED.load(Ordering::SeqCst) {
            info!("WiFi reconnected!");
            WIFI_CONNECTED.store(true, Ordering::SeqCst);
        }
        return true;
    }

    // WiFi disconnected
    if WIFI_CONNECTED.load(Ordering::SeqCst) {
        warn!("WiFi connection lost!");
        WIFI_CONNECTED.store(false, Ordering::SeqCst);
    }

    // Attempt reconnection
    info!("Attempting WiFi reconnection...");
    match wifi.connect() {
        Ok(_) => {
            if wifi.wait_netif_up().is_ok() {
                info!("WiFi reconnected successfully!");
                WIFI_CONNECTED.store(true, Ordering::SeqCst);
                return true;
            }
        }
        Err(e) => {
            warn!("WiFi reconnection failed: {}", e);
        }
    }

    false
}

/// MS/TP receive task - reads frames from RS-485 and routes to IP
fn mstp_receive_task(
    mstp_driver: Arc<Mutex<MstpDriver<'static>>>,
    gateway: Arc<Mutex<BacnetGateway>>,
    local_device: Arc<LocalDevice>,
    web_state: Arc<Mutex<web::WebState>>,
) {
    use local_device::DiscoveredDevice;

    info!("MS/TP receive task started");

    loop {
        // Try to receive an MS/TP frame
        let frame = {
            let mut driver = match mstp_driver.lock() {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to lock MS/TP driver: {}", e);
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };
            driver.receive_frame()
        };

        match frame {
            Ok(Some((data, source_addr))) => {
                // Check if this is an I-Am response (for device discovery)
                if let Some(apdu) = extract_apdu_from_npdu(&data) {
                    // Check for I-Am (Unconfirmed Request, Service 0)
                    if apdu.len() >= 2 && apdu[0] == 0x10 && apdu[1] == 0x00 {
                        if let Some(device) = DiscoveredDevice::from_i_am(apdu, source_addr) {
                            info!("Discovered device: instance {} at MAC {}, vendor {}",
                                device.device_instance, device.mac_address, device.vendor_id);

                            // Add to discovered devices list (avoid duplicates)
                            // Always capture I-Am responses - they can arrive anytime
                            if let Ok(mut web) = web_state.lock() {
                                // Check if device already exists (by instance or MAC)
                                let exists = web.discovered_devices.iter()
                                    .any(|d| d.device_instance == device.device_instance || d.mac_address == device.mac_address);
                                if !exists {
                                    web.discovered_devices.push(device);
                                    info!("Added device to discovered list (total: {})", web.discovered_devices.len());
                                }
                            }
                        }
                    }
                }

                // First, check if this is a message for our local device
                // Parse NPDU to get to APDU
                if let Some(response) = try_process_local_device(&data, &local_device) {
                    // Send the response back via MS/TP
                    if let Ok(mut driver) = mstp_driver.lock() {
                        // Build NPDU wrapper for the response
                        let (response_npdu, is_broadcast) = response;
                        let dest = if is_broadcast { 0xFF } else { source_addr };
                        if let Err(e) = driver.send_frame(&response_npdu, dest, false) {
                            warn!("Failed to send local device response: {}", e);
                        }
                    }
                } else {
                    // Route the frame through the gateway
                    if let Ok(mut gw) = gateway.lock() {
                        if let Err(e) = gw.route_from_mstp(&data, source_addr) {
                            warn!("Failed to route MS/TP frame: {}", e);
                        }
                    }
                }
            }
            Ok(None) => {
                // No frame available, small delay
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                warn!("MS/TP receive error: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Extract APDU from NPDU data
fn extract_apdu_from_npdu(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 2 {
        return None;
    }

    let version = data[0];
    if version != 0x01 {
        return None;
    }

    let control = data[1];
    let mut pos = 2;

    // Check for destination network (bit 5)
    if (control & 0x20) != 0 {
        if pos + 3 > data.len() {
            return None;
        }
        pos += 2; // DNET
        let dlen = data[pos] as usize;
        pos += 1 + dlen;
    }

    // Check for source network (bit 3)
    if (control & 0x08) != 0 {
        if pos + 3 > data.len() {
            return None;
        }
        pos += 2; // SNET
        let slen = data[pos] as usize;
        pos += 1 + slen;
    }

    // Skip hop count if destination was present
    if (control & 0x20) != 0 {
        pos += 1;
    }

    // If network layer message, no APDU
    if (control & 0x80) != 0 {
        return None;
    }

    if pos < data.len() {
        Some(&data[pos..])
    } else {
        None
    }
}

/// Try to process a message with the local device, returns response if applicable
fn try_process_local_device(data: &[u8], local_device: &LocalDevice) -> Option<(Vec<u8>, bool)> {
    // The data should be NPDU (network layer)
    // NPDU format: version (1) + control (1) + [optional dest/source] + APDU
    if data.len() < 2 {
        return None;
    }

    let version = data[0];
    if version != 0x01 {
        return None; // Not BACnet NPDU
    }

    let control = data[1];
    let mut pos = 2;

    // Check for destination network (bit 5)
    let has_dest = (control & 0x20) != 0;
    // Check for source network (bit 3)
    let has_source = (control & 0x08) != 0;
    // Network layer message (bit 7)
    let is_network_msg = (control & 0x80) != 0;

    // Skip destination if present
    if has_dest {
        if pos + 3 > data.len() {
            return None;
        }
        let dnet = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let dlen = data[pos] as usize;
        pos += 1;

        // If DNET is not 0xFFFF (broadcast) and not our network, skip
        // For now, we accept broadcast (0xFFFF) or messages with no destination
        if dnet != 0xFFFF {
            // This is targeted at a specific network, might not be for us
            // But we should still check - could be directed to our network
        }

        pos += dlen;
    }

    // Skip source if present
    if has_source {
        if pos + 3 > data.len() {
            return None;
        }
        pos += 2; // SNET
        let slen = data[pos] as usize;
        pos += 1;
        pos += slen;
    }

    // Skip hop count if destination was present
    if has_dest {
        if pos >= data.len() {
            return None;
        }
        pos += 1;
    }

    // If this is a network layer message, don't process with local device
    if is_network_msg {
        return None;
    }

    // Now we have APDU at data[pos..]
    if pos >= data.len() {
        return None;
    }

    let apdu = &data[pos..];

    // Process with local device
    if let Some((response_apdu, is_broadcast)) = local_device.process_apdu(apdu) {
        // Build NPDU wrapper for response
        // For I-Am (broadcast), use global broadcast
        // For ReadProperty response (unicast), use source routing if available
        let mut npdu = Vec::with_capacity(response_apdu.len() + 10);

        // NPDU Version
        npdu.push(0x01);

        if is_broadcast {
            // Broadcast response (I-Am)
            // Control: no destination/source network info, APDU present
            npdu.push(0x00);
        } else {
            // Unicast response - no network layer addressing needed for local response
            npdu.push(0x00);
        }

        // Append APDU
        npdu.extend_from_slice(&response_apdu);

        return Some((npdu, is_broadcast));
    }

    None
}

/// BACnet/IP receive task - reads UDP packets and routes to MS/TP
fn ip_receive_task(
    socket: Arc<UdpSocket>,
    gateway: Arc<Mutex<BacnetGateway>>,
    mstp_driver: Arc<Mutex<MstpDriver<'static>>>,
    local_device: Arc<LocalDevice>,
) {
    info!("BACnet/IP receive task started");

    let mut buffer = [0u8; 1500];

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((len, source_addr)) => {
                let data = &buffer[..len];

                // Try to process with local device first (for Who-Is from IP side)
                if let Some((response_npdu, is_broadcast)) = try_process_ip_local_device(data, &local_device) {
                    // Wrap in BVLC and send back
                    let mut bvlc = Vec::with_capacity(response_npdu.len() + 4);
                    bvlc.push(0x81); // BVLC type
                    if is_broadcast {
                        bvlc.push(0x0B); // Original-Broadcast-NPDU
                    } else {
                        bvlc.push(0x0A); // Original-Unicast-NPDU
                    }
                    let total_len = (response_npdu.len() + 4) as u16;
                    bvlc.extend_from_slice(&total_len.to_be_bytes());
                    bvlc.extend_from_slice(&response_npdu);

                    // Send response
                    if is_broadcast {
                        // Send to broadcast address
                        let broadcast_addr = "255.255.255.255:47808";
                        if let Err(e) = socket.send_to(&bvlc, broadcast_addr) {
                            warn!("Failed to send I-Am broadcast: {}", e);
                        }
                    } else {
                        if let Err(e) = socket.send_to(&bvlc, source_addr) {
                            warn!("Failed to send response to {}: {}", source_addr, e);
                        }
                    }
                }

                // Route the frame through the gateway
                if let Ok(mut gw) = gateway.lock() {
                    match gw.route_from_ip(data, source_addr) {
                        Ok(Some((mstp_data, mstp_dest))) => {
                            // Send to MS/TP
                            // TODO: Determine if this is an expecting-reply frame from NPDU
                            if let Ok(mut driver) = mstp_driver.lock() {
                                if let Err(e) = driver.send_frame(&mstp_data, mstp_dest, false) {
                                    warn!("Failed to send to MS/TP: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            // Frame handled internally (e.g., BVLC control)
                        }
                        Err(e) => {
                            warn!("Failed to route IP frame: {}", e);
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Timeout, no data available
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                warn!("UDP receive error: {}", e);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Try to process an IP message with the local device
fn try_process_ip_local_device(data: &[u8], local_device: &LocalDevice) -> Option<(Vec<u8>, bool)> {
    // BACnet/IP format: BVLC (4 bytes) + NPDU + APDU
    if data.len() < 4 {
        return None;
    }

    // Check BVLC header
    if data[0] != 0x81 {
        return None; // Not BACnet/IP
    }

    let bvlc_function = data[1];
    // Only process Original-Unicast-NPDU (0x0A) and Original-Broadcast-NPDU (0x0B)
    if bvlc_function != 0x0A && bvlc_function != 0x0B {
        return None;
    }

    // Skip BVLC header (4 bytes) to get NPDU
    let npdu_data = &data[4..];

    // Use the same NPDU parsing as MS/TP side
    try_process_local_device(npdu_data, local_device)
}

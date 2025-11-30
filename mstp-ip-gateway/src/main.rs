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
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, AccessPointConfiguration},
};
use log::{error, info, trace, warn};
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
mod transaction;
mod web;

use config::GatewayConfig;
use display::{Display, DisplayScreen, GatewayStatus};
use gateway::BacnetGateway;
use local_device::LocalDevice;
use mstp_driver::MstpDriver;
use web::{WebState, start_web_server};

/// Global flag for WiFi connection status (used by reconnection logic)
static WIFI_CONNECTED: AtomicBool = AtomicBool::new(false);

/// Global flag for AP mode status
static AP_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// WiFi reconnection interval in seconds
const WIFI_RECONNECT_INTERVAL_SECS: u64 = 10;

/// Watchdog timeout in seconds
const WATCHDOG_TIMEOUT_SECS: u64 = 30;

/// Router announcement interval in loop iterations (30 seconds = 3000 iterations at 10ms)
const ROUTER_ANNOUNCE_INTERVAL: u64 = 3000;

/// Default AP mode IP address
const AP_IP_ADDRESS: &str = "192.168.4.1";

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Set up panic handler for automatic restart
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
        error!("Restarting in 3 seconds...");
        thread::sleep(Duration::from_secs(3));
        // SAFETY: esp_restart() is always safe to call on ESP32 - it performs a
        // software reset. Used here to recover from panics automatically.
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

    // Initialize WiFi - check if credentials are configured
    info!("Initializing WiFi...");

    // Check if WiFi credentials are empty - if so, start in AP mode automatically
    let (wifi, ip_info_str, start_in_ap_mode) = if config.wifi_ssid.is_empty() {
        info!("No WiFi credentials configured - starting in AP mode");
        lcd.show_status_message("AP Mode", &format!("SSID: {}", config.ap_ssid))?;

        // Initialize WiFi in AP mode
        let mut wifi = BlockingWifi::wrap(
            EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
            sys_loop.clone(),
        )?;

        let ap_ip = switch_to_ap_mode(&mut wifi, &config.ap_ssid, &config.ap_password)?;
        AP_MODE_ACTIVE.store(true, Ordering::SeqCst);

        (wifi, ap_ip, true)
    } else {
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
            // SAFETY: esp_restart() is always safe to call on ESP32 - it performs a
            // software reset. Used here to retry WiFi initialization after failure.
            unsafe { esp_idf_svc::sys::esp_restart(); }
            // This loop satisfies the type checker - esp_restart() doesn't return
            #[allow(unreachable_code)]
            loop { thread::sleep(Duration::from_secs(1)); }
        });

        WIFI_CONNECTED.store(true, Ordering::SeqCst);
        let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
        let ip_str = ip_info.ip.to_string();

        info!("WiFi connected!");
        info!("  IP Address: {}", ip_info.ip);
        info!("  Subnet: {}", ip_info.subnet.mask);
        info!("  Gateway: {}", ip_info.subnet.gateway);

        (wifi, ip_str, false)
    };

    let ip_info = if start_in_ap_mode {
        // In AP mode, use AP netif for IP info
        wifi.wifi().ap_netif().get_ip_info()?
    } else {
        wifi.wifi().sta_netif().get_ip_info()?
    };

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

    // Create gateway - use local IP and subnet mask for routing
    let local_ip: std::net::Ipv4Addr = ip_info.ip.octets().into();
    // Convert CIDR prefix to subnet mask (e.g., 24 -> 255.255.255.0)
    let prefix: u8 = ip_info.subnet.mask.0;
    let mask_bits: u32 = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
    let subnet_mask: std::net::Ipv4Addr = mask_bits.to_be_bytes().into();
    let gateway = Arc::new(Mutex::new(BacnetGateway::new(
        config.mstp_network,
        config.ip_network,
        local_ip,
        config.bacnet_ip_port,
        subnet_mask,
    )));

    // Create local BACnet device for gateway discoverability
    let mut local_device = LocalDevice::new_with_mstp(
        config.device_instance,
        config.mstp_max_master,
        1, // max_info_frames
    );
    info!("Local BACnet device created: instance {}", config.device_instance);

    // Initialize Network Port objects for both interfaces
    // Get MAC address from WiFi interface (or use a dummy for now)
    let mac_address = if start_in_ap_mode {
        wifi.wifi().ap_netif().get_mac().unwrap_or([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
    } else {
        wifi.wifi().sta_netif().get_mac().unwrap_or([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
    };

    local_device.initialize_network_ports(
        config.mstp_network,
        config.mstp_address,
        config.mstp_baud_rate,
        config.ip_network,
        local_ip.octets(),
        subnet_mask.octets(),
        mac_address,
    );

    let local_device = Arc::new(local_device);

    // Wrap WiFi in Arc<Mutex> for sharing with main loop (for reconnection)
    let wifi = Arc::new(Mutex::new(wifi));

    // Wrap socket in Arc for sharing between threads
    // (try_clone() doesn't work on ESP-IDF)
    let socket = Arc::new(socket);

    // Set the IP socket on the gateway so it can send MS/TP->IP traffic
    // This is critical - without this, all MS/TP to IP packets are queued but never sent!
    if let Ok(mut gw) = gateway.lock() {
        gw.set_ip_socket(Arc::clone(&socket));
        info!("IP socket set on gateway for MS/TP->IP routing");
    }

    // Create web server state early so it can be shared with receive tasks
    let web_state = Arc::new(Mutex::new(WebState::new(config.clone(), Some(nvs_for_console))));

    // Spawn MS/TP receive thread
    info!(">>> [MAIN] About to spawn MS/TP receive thread...");
    let mstp_driver_clone = Arc::clone(&mstp_driver);
    let gateway_clone = Arc::clone(&gateway);
    let local_device_clone = Arc::clone(&local_device);
    let web_state_mstp = Arc::clone(&web_state);
    // Stack size increased from 8KB to 16KB to handle BACnet protocol processing
    // which may require significant stack space for NPDU parsing, routing tables,
    // and complex service handling (ASHRAE 135-2024)
    let mstp_network_for_thread = config.mstp_network;
    let _mstp_thread = thread::Builder::new()
        .stack_size(16384)
        .spawn(move || {
            mstp_receive_task(mstp_driver_clone, gateway_clone, local_device_clone, web_state_mstp, mstp_network_for_thread);
        })?;
    info!(">>> [MAIN] MS/TP thread spawned successfully!");

    // Spawn BACnet/IP receive thread
    let socket_clone = Arc::clone(&socket);
    let gateway_clone = Arc::clone(&gateway);
    let mstp_driver_clone = Arc::clone(&mstp_driver);
    let local_device_clone = Arc::clone(&local_device);
    let ip_network_for_thread = config.ip_network;
    let mstp_network_for_ip_thread = config.mstp_network;
    let gateway_mac_for_thread = config.mstp_address;
    // Stack size reduced from 16KB to 8KB to conserve memory for main loop
    info!(">>> [MAIN] About to spawn IP receive thread...");
    match thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            ip_receive_task(socket_clone, gateway_clone, mstp_driver_clone, local_device_clone,
                           ip_network_for_thread, mstp_network_for_ip_thread, gateway_mac_for_thread);
        }) {
        Ok(_thread) => {
            info!(">>> [MAIN] IP thread spawned successfully!");
        }
        Err(e) => {
            error!(">>> [MAIN] FAILED to spawn IP thread: {:?}", e);
            error!(">>> [MAIN] Continuing without IP receive thread - MS/TP only mode");
        }
    }

    info!(">>> [MAIN] Gateway running!");
    info!(">>> [MAIN] DEBUG: Line 306 - about to print network numbers");
    info!("  MS/TP Network {} <-> IP Network {}", config.mstp_network, config.ip_network);
    info!(">>> [MAIN] DEBUG: Line 308 - about to create GatewayStatus");

    // Status tracking for display
    let mut status = GatewayStatus {
        wifi_connected: !start_in_ap_mode,  // Only connected in Station mode
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
        // AP mode fields
        ap_mode_active: start_in_ap_mode,
        ap_ssid: config.ap_ssid.clone(),
        ap_ip: if start_in_ap_mode { ip_info_str.clone() } else { "192.168.4.1".to_string() },
        ap_clients: 0,
    };
    info!(">>> [MAIN] DEBUG: GatewayStatus created successfully");

    // Display screen cycling with Button A
    let mut current_screen = DisplayScreen::Status;
    let mut btn_a_was_pressed = false;
    let mut btn_b_was_pressed = false;
    let mut btn_c_was_pressed = false;

    // WiFi reconnection tracking
    let mut wifi_check_counter: u32 = 0;
    const WIFI_CHECK_INTERVAL: u32 = 50; // Check every 5 seconds (50 * 100ms)

    // Router announcement tracking (I-Am and I-Am-Router-To-Network)
    // Start at max to trigger immediate announcement on first loop
    let mut router_announce_counter: u64 = ROUTER_ANNOUNCE_INTERVAL;

    info!("╔══════════════════════════════════════════════════════════════╗");
    info!("║                    Gateway Running!                          ║");
    info!("╚══════════════════════════════════════════════════════════════╝");

    info!(">>> [MAIN] About to update web_state...");
    // Update initial web state (web_state was created earlier for thread sharing)
    {
        let mut state = web_state.lock().unwrap();
        state.wifi_connected = !start_in_ap_mode;  // Only connected in Station mode
        state.ip_address = ip_info.ip.to_string();
    }
    info!(">>> [MAIN] web_state updated");

    // Start web server for configuration portal
    info!(">>> [MAIN] About to start web server...");
    let web_state_clone = Arc::clone(&web_state);
    let _web_server = match start_web_server(web_state_clone) {
        Ok(server) => {
            info!(">>> [MAIN] Web server started! Portal at http://{}/", ip_info.ip);
            Some(server)
        }
        Err(e) => {
            error!(">>> [MAIN] Failed to start web server: {}", e);
            None
        }
    };
    info!(">>> [MAIN] Web server setup complete, about to enter main loop...");

    let mut loop_count: u64 = 0;
    info!(">>> [MAIN] ENTERING MAIN LOOP <<<");
    loop {
        loop_count += 1;

        // Log first iteration and then every 1000 iterations (~10 seconds at 10ms sleep)
        if loop_count == 1 || loop_count % 1000 == 0 {
            info!(">>> Main loop iteration {} <<<", loop_count);
        }

        // Feed the watchdog to prevent reset - don't use ? to avoid silent exit
        if let Err(e) = watchdog.feed() {
            warn!("Watchdog feed error (continuing anyway): {:?}", e);
        }

        // Process any pending gateway tasks (non-blocking)
        if let Ok(mut gw) = gateway.try_lock() {
            gw.process_housekeeping();

            // Check transaction timeouts every 100 iterations (1 second at 10ms/iteration)
            if loop_count % 100 == 0 {
                let timeout_count = gw.process_transaction_timeouts();
                if timeout_count > 0 {
                    info!(
                        "Transaction timeouts: {} aborted, {} active",
                        timeout_count,
                        gw.active_transaction_count()
                    );
                }
            }
        }

        // Check if Who-Is scan was requested from web portal (non-blocking)
        let scan_requested = {
            match web_state.try_lock() {
                Ok(mut web) => {
                    if web.scan_requested {
                        info!("Main loop: scan_requested=true, processing...");
                        web.scan_requested = false;
                        true
                    } else {
                        false
                    }
                }
                Err(_) => false,  // Skip this iteration if locked
            }
        };

        // Process scan request with driver lock
        if scan_requested {
            info!("Who-Is scan requested - sending broadcasts");

            // Build Who-Is APDU
            let who_is_apdu = LocalDevice::build_who_is();
            info!("Who-Is APDU: {:02X?}", who_is_apdu);

            // Send LOCAL broadcast first (simple NPDU, no network layer)
            // This reaches devices on the local MS/TP segment
            let mut local_npdu = Vec::with_capacity(who_is_apdu.len() + 2);
            local_npdu.push(0x01); // NPDU version
            local_npdu.push(0x00); // Control: no network layer info
            local_npdu.extend_from_slice(&who_is_apdu);
            info!("Who-Is NPDU (local): {:02X?}", local_npdu);

            // Also send GLOBAL broadcast (DNET=0xFFFF) for routers
            // Per Clause 6.2.2, when DNET is present we must include SNET/SADR so routers
            // know where to return replies. We include our configured MS/TP network and MAC.
            let mut global_npdu = Vec::with_capacity(who_is_apdu.len() + 12);
            global_npdu.push(0x01); // NPDU version
            // Control: destination present + source present (required when DNET is present)
            global_npdu.push(0x28);
            global_npdu.push(0xFF); // DNET high byte (0xFFFF = global broadcast)
            global_npdu.push(0xFF); // DNET low byte
            global_npdu.push(0x00); // DLEN = 0 (broadcast)
            // Source specifier (SNET/SADR) so I-Am can be routed back
            global_npdu.push((config.mstp_network >> 8) as u8); // SNET high
            global_npdu.push((config.mstp_network & 0xFF) as u8); // SNET low
            global_npdu.push(0x01); // SLEN = 1 (our MS/TP MAC length)
            global_npdu.push(config.mstp_address); // SADR = our MAC
            global_npdu.push(0xFF); // Hop count
            global_npdu.extend_from_slice(&who_is_apdu);
            info!("Who-Is NPDU (global): {:02X?}", global_npdu);

            // Now lock driver and queue frames
            if let Ok(mut driver) = mstp_driver.lock() {
                match driver.send_frame(&local_npdu, 0xFF, false) {
                    Ok(_) => info!("Local Who-Is broadcast queued"),
                    Err(e) => warn!("Failed to queue local Who-Is: {}", e),
                }
                match driver.send_frame(&global_npdu, 0xFF, false) {
                    Ok(_) => info!("Global Who-Is broadcast queued"),
                    Err(e) => warn!("Failed to queue global Who-Is: {}", e),
                }
            } else {
                warn!("Could not lock MS/TP driver to send Who-Is");
            }
        }

        // Periodic router announcements (I-Am and I-Am-Router-To-Network)
        // This announces the router's presence on the MS/TP network so devices know we exist
        router_announce_counter += 1;
        // Debug: log every 1000 iterations to verify counter is incrementing
        if router_announce_counter % 1000 == 0 {
            info!("Announcement counter: {} (threshold: {})", router_announce_counter, ROUTER_ANNOUNCE_INTERVAL);
        }
        if router_announce_counter >= ROUTER_ANNOUNCE_INTERVAL {
            router_announce_counter = 0;

            info!("Sending periodic router announcements...");

            // Build I-Am APDU for the gateway device
            let iam_apdu = local_device.build_i_am();

            // Wrap I-Am in NPDU (local broadcast, no network layer info)
            let mut iam_npdu = Vec::with_capacity(iam_apdu.len() + 2);
            iam_npdu.push(0x01); // NPDU version
            iam_npdu.push(0x00); // Control: no network layer info
            iam_npdu.extend_from_slice(&iam_apdu);

            // Build I-Am-Router-To-Network announcing the IP network
            // This tells MS/TP devices that we can route to the IP network
            let iartn_npdu = LocalDevice::build_i_am_router_to_network(&[config.ip_network]);

            // Queue both announcements
            if let Ok(mut driver) = mstp_driver.lock() {
                match driver.send_frame(&iam_npdu, 0xFF, false) {
                    Ok(_) => info!("I-Am broadcast queued"),
                    Err(e) => warn!("Failed to queue I-Am: {}", e),
                }
                match driver.send_frame(&iartn_npdu, 0xFF, false) {
                    Ok(_) => info!("I-Am-Router-To-Network broadcast queued (announcing network {})", config.ip_network),
                    Err(e) => warn!("Failed to queue I-Am-Router-To-Network: {}", e),
                }
            } else {
                warn!("Could not lock MS/TP driver for router announcements");
            }
        }

        // Get MS/TP driver stats (non-blocking to avoid starvation)
        if let Ok(mut driver) = mstp_driver.try_lock() {
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
            if let Ok(mut web) = web_state.try_lock() {
                web.mstp_stats = mstp_stats;

                // Check if stats reset was requested from web portal
                if web.reset_stats_requested {
                    driver.reset_stats();
                    web.reset_stats_requested = false;
                    info!("Statistics reset completed");
                }
            }
        }

        // Get gateway stats for web portal (non-blocking)
        if let Ok(gw) = gateway.try_lock() {
            let gw_stats = gw.get_stats();
            if let Ok(mut web) = web_state.try_lock() {
                web.gateway_stats.mstp_to_ip_packets = gw_stats.mstp_to_ip_packets;
                web.gateway_stats.ip_to_mstp_packets = gw_stats.ip_to_mstp_packets;
            }
        }

        // Periodically check WiFi connection and attempt reconnection if needed
        wifi_check_counter += 1;
        if wifi_check_counter >= WIFI_CHECK_INTERVAL {
            wifi_check_counter = 0;

            // In AP mode, update client count; in STA mode, check connection
            if AP_MODE_ACTIVE.load(Ordering::SeqCst) {
                // Query AP client count from ESP-IDF using sta_list
                // SAFETY: wifi_sta_list_t is a simple C struct with no pointers or
                // invariants that zeroed memory would violate. All fields are integers.
                let mut sta_list: esp_idf_sys::wifi_sta_list_t = unsafe { std::mem::zeroed() };
                // SAFETY: esp_wifi_ap_get_sta_list() fills the provided sta_list struct
                // with current AP client information. We pass a valid mutable reference
                // and the struct has been properly initialized above.
                unsafe {
                    esp_idf_sys::esp_wifi_ap_get_sta_list(&mut sta_list);
                }
                status.ap_clients = sta_list.num as u8;
            } else {
                if let Ok(mut wifi_guard) = wifi.lock() {
                    let connected = check_wifi_connection(&mut wifi_guard);
                    if status.wifi_connected != connected {
                        status.wifi_connected = connected;
                        // Force display update when WiFi status changes
                        if current_screen != DisplayScreen::Splash {
                            lcd.clear_and_reset().ok();
                        }
                        // Update web state (non-blocking)
                        if let Ok(mut web) = web_state.try_lock() {
                            web.wifi_connected = connected;
                        }
                    }
                }
            }
        }

        // Handle button A (front big button) - cycle through screens
        let btn_a_pressed = btn_a.is_low();
        if !btn_a_pressed && btn_a_was_pressed {
            // Button released - cycle to next screen
            current_screen = current_screen.next();
            info!("Button A - screen: {:?}", current_screen);
            lcd.clear_and_reset().ok();
            if current_screen == DisplayScreen::Splash {
                lcd.show_splash_screen().ok();
            }
        }
        btn_a_was_pressed = btn_a_pressed;

        // Handle button B (side) - toggle AP/Station mode
        let btn_b_pressed = btn_b.is_low();
        if btn_b_pressed && !btn_b_was_pressed {
            info!("Button B pressed - toggling WiFi mode");

            // Toggle AP mode
            let new_ap_mode = !AP_MODE_ACTIVE.load(Ordering::SeqCst);

            if new_ap_mode {
                // Switch to AP mode
                info!("Switching to AP mode...");
                if let Ok(mut wifi_guard) = wifi.lock() {
                    match switch_to_ap_mode(&mut wifi_guard, &config.ap_ssid, &config.ap_password) {
                        Ok(ap_ip_str) => {
                            AP_MODE_ACTIVE.store(true, Ordering::SeqCst);
                            WIFI_CONNECTED.store(false, Ordering::SeqCst);
                            status.ap_mode_active = true;
                            status.wifi_connected = false;
                            status.ip_address = ap_ip_str.clone();
                            status.ap_ip = ap_ip_str.clone();

                            // Update gateway's local IP for AP mode
                            if let Ok(mut gw) = gateway.lock() {
                                if let Ok(ap_ip) = ap_ip_str.parse::<std::net::Ipv4Addr>() {
                                    let ap_mask = std::net::Ipv4Addr::new(255, 255, 255, 0);
                                    gw.set_local_ip(ap_ip, ap_mask);
                                }
                            }

                            info!("AP mode activated: SSID={}, IP={}", config.ap_ssid, ap_ip_str);
                        }
                        Err(e) => {
                            error!("Failed to switch to AP mode: {}", e);
                        }
                    }
                }
            } else {
                // Switch back to Station mode
                info!("Switching back to Station mode...");
                if let Ok(mut wifi_guard) = wifi.lock() {
                    match switch_to_sta_mode(&mut wifi_guard, &config.wifi_ssid, &config.wifi_password) {
                        Ok(ip) => {
                            AP_MODE_ACTIVE.store(false, Ordering::SeqCst);
                            WIFI_CONNECTED.store(true, Ordering::SeqCst);
                            status.ap_mode_active = false;
                            status.wifi_connected = true;
                            status.ip_address = ip.clone();

                            // Update gateway's local IP for station mode
                            if let Ok(mut gw) = gateway.lock() {
                                if let Ok(sta_ip) = ip.parse::<std::net::Ipv4Addr>() {
                                    let sta_mask = std::net::Ipv4Addr::new(255, 255, 255, 0);
                                    gw.set_local_ip(sta_ip, sta_mask);
                                }
                            }

                            info!("Station mode activated");
                        }
                        Err(e) => {
                            error!("Failed to switch to Station mode: {}", e);
                            // Stay in AP mode if switching fails
                        }
                    }
                }
            }

            // Force display update
            lcd.clear_and_reset().ok();
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
            DisplayScreen::APConfig => {
                if let Err(e) = lcd.update_ap_config(&status) {
                    warn!("Failed to update AP config display: {}", e);
                }
            }
            DisplayScreen::Splash => {
                // Splash screen is static, no updates needed
            }
        }

        // Small delay to prevent busy-waiting
        // Reduced from 100ms to 10ms to be more responsive to scan requests
        // while still preventing excessive CPU usage
        thread::sleep(Duration::from_millis(10));
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
        ssid: ssid.try_into()
            .map_err(|_| anyhow::anyhow!("WiFi SSID exceeds maximum length (32 characters)"))?,
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: password.try_into()
            .map_err(|_| anyhow::anyhow!("WiFi password exceeds maximum length (64 characters)"))?,
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

/// Switch WiFi to Access Point mode
/// Returns the AP's IP address string on success
fn switch_to_ap_mode(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ap_ssid: &str,
    ap_password: &str,
) -> anyhow::Result<String> {
    info!("Configuring WiFi Access Point mode...");

    // Stop current WiFi operation
    let _ = wifi.disconnect();
    let _ = wifi.stop();

    // Configure as Access Point
    let ap_config = AccessPointConfiguration {
        ssid: ap_ssid.try_into().map_err(|_| anyhow::anyhow!("Invalid AP SSID"))?,
        ssid_hidden: false,
        auth_method: AuthMethod::WPA2Personal,
        password: ap_password.try_into().map_err(|_| anyhow::anyhow!("Invalid AP password"))?,
        channel: 6,  // Use channel 6 (common, less interference)
        max_connections: 4,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::AccessPoint(ap_config))?;
    wifi.start()?;

    // Wait for AP interface to be fully initialized
    // The AP netif needs time to start the DHCP server and configure the interface
    info!("Waiting for AP interface to initialize...");
    thread::sleep(Duration::from_millis(500));

    // Get AP netif reference
    let ap_netif = wifi.wifi().ap_netif();

    // Wait for netif to be up (with timeout)
    let mut netif_up = false;
    for i in 0..10 {
        match ap_netif.is_up() {
            Ok(true) => {
                netif_up = true;
                break;
            }
            Ok(false) => {}
            Err(e) => {
                warn!("Error checking AP netif status: {}", e);
            }
        }
        if i == 9 {
            warn!("AP netif not fully up after timeout, continuing anyway");
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Get the actual AP IP address from netif
    let ip_info = ap_netif.get_ip_info()?;
    let ip_str = format!("{}", ip_info.ip);

    info!("WiFi AP started: SSID='{}', IP={}, netif_up={}", ap_ssid, ip_str, netif_up);
    Ok(ip_str)
}

/// Switch WiFi back to Station (client) mode
fn switch_to_sta_mode(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ssid: &str,
    password: &str,
) -> anyhow::Result<String> {
    info!("Configuring WiFi Station mode...");

    // Stop current WiFi operation
    let _ = wifi.stop();

    // Configure as Station (client)
    let sta_config = ClientConfiguration {
        ssid: ssid.try_into().map_err(|_| anyhow::anyhow!("Invalid WiFi SSID"))?,
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: password.try_into().map_err(|_| anyhow::anyhow!("Invalid WiFi password"))?,
        channel: None,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::Client(sta_config))?;
    wifi.start()?;

    // Connect to the network
    info!("Connecting to WiFi network '{}'...", ssid);
    wifi.connect()?;
    wifi.wait_netif_up()?;

    // Get assigned IP address
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    let ip_str = ip_info.ip.to_string();

    info!("WiFi Station mode connected: IP={}", ip_str);
    Ok(ip_str)
}

/// MS/TP receive task - reads frames from RS-485 and routes to IP
fn mstp_receive_task(
    mstp_driver: Arc<Mutex<MstpDriver<'static>>>,
    gateway: Arc<Mutex<BacnetGateway>>,
    local_device: Arc<LocalDevice>,
    web_state: Arc<Mutex<web::WebState>>,
    mstp_network: u16,
) {
    use local_device::DiscoveredDevice;

    info!("MS/TP receive task started");

    // Counter for brief yields to prevent mutex starvation
    let mut iteration_counter: u32 = 0;

    loop {
        iteration_counter += 1;

        // Try to receive an MS/TP frame using try_lock()
        // This allows main loop to acquire the lock when needed
        let frame = {
            match mstp_driver.try_lock() {
                Ok(mut driver) => {
                    driver.receive_frame()
                }
                Err(_) => {
                    // Lock contention - yield to let main loop run
                    // This is critical for preventing mutex starvation!
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
            }
        };

        match frame {
            Ok(Some((data, source_addr))) => {
                info!("MS/TP RX queue: {} bytes from MAC {}, NPDU: {:02X?}",
                       data.len(), source_addr, &data[..data.len().min(30)]);

                // Store frame for debug viewing
                if let Ok(mut web) = web_state.lock() {
                    web.add_rx_frame(source_addr, &data);
                }

                // Check if this is an I-Am response (for device discovery)
                if let Some(apdu) = extract_apdu_from_npdu(&data) {
                    info!("  -> APDU extracted: {:02X?}", &apdu[..apdu.len().min(20)]);
                    // Check for I-Am (Unconfirmed Request, Service 0)
                    if apdu.len() >= 2 && apdu[0] == 0x10 && apdu[1] == 0x00 {
                        info!("  -> I-Am detected from MAC {}", source_addr);
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
                if let Some((response_npdu, is_broadcast, source_info)) = try_process_local_device(&data, &local_device, mstp_network) {
                    // CRITICAL FIX: Always send responses on MS/TP, not directly to IP!
                    // When the request came from a remote network (e.g., IP via router at station 2),
                    // we need to send the response on MS/TP TO THE ROUTER, which will forward it.
                    // This is how other devices (like JCI controllers) respond.

                    if let Some(ref src) = source_info {
                        // Request came from a remote network - build NPDU with routing info
                        // and send on MS/TP to the router that forwarded the request
                        info!("Local device response for remote request from SNET={}, SADR={:02X?}",
                              src.source_network, src.source_address);

                        // Build NPDU with destination network info (the original source becomes destination)
                        let mut routed_npdu = Vec::with_capacity(response_npdu.len() + 12);
                        routed_npdu.push(0x01); // Version

                        // Control: DNET present (0x20)
                        routed_npdu.push(0x20);

                        // DNET - original source network (where the request came from)
                        routed_npdu.extend_from_slice(&src.source_network.to_be_bytes());

                        // DLEN and DADR - original source address
                        routed_npdu.push(src.source_address.len() as u8);
                        routed_npdu.extend_from_slice(&src.source_address);

                        // Hop count
                        routed_npdu.push(0xFF);

                        // Append original APDU (skip version and control from response_npdu)
                        if response_npdu.len() > 2 {
                            routed_npdu.extend_from_slice(&response_npdu[2..]);
                        }

                        // Send on MS/TP to the router (source_addr is the MAC of the router that sent us the request)
                        // The router will see DNET in the NPDU and forward it to the appropriate network
                        if let Ok(mut driver) = mstp_driver.lock() {
                            trace!("Sending I-Am on MS/TP to router MAC {}: {} bytes, NPDU: {:02X?}",
                                  source_addr, routed_npdu.len(), &routed_npdu[..routed_npdu.len().min(30)]);
                            if let Err(e) = driver.send_frame(&routed_npdu, source_addr, false) {
                                warn!("Failed to send I-Am to MS/TP router: {}", e);
                            } else {
                                trace!("I-Am queued for MS/TP transmission to router MAC {}", source_addr);
                            }
                        }
                    } else {
                        // No source network info - send locally on MS/TP (broadcast for I-Am)
                        if let Ok(mut driver) = mstp_driver.lock() {
                            let dest = if is_broadcast { 0xFF } else { source_addr };
                            info!("Sending local device response: {} bytes to MAC {} (broadcast={})",
                                  response_npdu.len(), dest, is_broadcast);
                            if let Err(e) = driver.send_frame(&response_npdu, dest, false) {
                                warn!("Failed to send local device response: {}", e);
                            }
                        }
                    }
                } else {
                    // Route the frame through the gateway
                    if let Ok(mut gw) = gateway.lock() {
                        match gw.route_from_mstp(&data, source_addr) {
                            Ok(Some((reject_npdu, reject_dest))) => {
                                // Send reject message back to MS/TP source
                                drop(gw); // Release gateway lock before acquiring driver lock
                                if let Ok(mut driver) = mstp_driver.lock() {
                                    if let Err(e) = driver.send_frame(&reject_npdu, reject_dest, false) {
                                        warn!("Failed to send reject to MS/TP: {}", e);
                                    }
                                }
                            }
                            Ok(None) => {
                                // Successfully routed, nothing more to do
                            }
                            Err(e) => {
                                warn!("Failed to route MS/TP frame: {}", e);
                            }
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

/// Source routing information parsed from NPDU
#[derive(Debug, Clone)]
struct SourceRouteInfo {
    /// Source network number (SNET)
    pub source_network: u16,
    /// Source address (SADR)
    pub source_address: Vec<u8>,
}

/// Try to process a message with the local device, returns response if applicable
/// Returns: (response_npdu, is_broadcast, optional_source_route)
/// `local_network` is the network number where this local device resides (IP network for IP side, MS/TP network for MS/TP side)
fn try_process_local_device(data: &[u8], local_device: &LocalDevice, local_network: u16) -> Option<(Vec<u8>, bool, Option<SourceRouteInfo>)> {
    // The data should be NPDU (network layer)
    // NPDU format: version (1) + control (1) + [optional dest/source] + APDU
    info!(">>> try_process_local_device: {} bytes, NPDU: {:02X?}", data.len(), &data[..data.len().min(20)]);

    if data.len() < 2 {
        info!(">>> NPDU too short");
        return None;
    }

    let version = data[0];
    if version != 0x01 {
        info!(">>> Not BACnet NPDU (version=0x{:02X})", version);
        return None; // Not BACnet NPDU
    }

    let control = data[1];
    let mut pos = 2;
    info!(">>> NPDU: version=0x{:02X}, control=0x{:02X}", version, control);

    // Check for destination network (bit 5)
    let has_dest = (control & 0x20) != 0;
    // Check for source network (bit 3)
    let has_source = (control & 0x08) != 0;
    // Network layer message (bit 7)
    let is_network_msg = (control & 0x80) != 0;

    // Skip destination if present
    if has_dest {
        if pos + 3 > data.len() {
            info!(">>> DNET parse: pos+3 > len ({} > {})", pos + 3, data.len());
            return None;
        }
        let dnet = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let dlen = data[pos] as usize;
        pos += 1;
        info!(">>> DNET=0x{:04X}, DLEN={}, local_network={}", dnet, dlen, local_network);

        // If DNET is not 0xFFFF (global broadcast) and not our local network,
        // this message should be routed, not processed locally
        if dnet != 0xFFFF && dnet != local_network {
            // This is targeted at a different network - let routing handle it
            info!(">>> DNET not for us (not 0xFFFF and not local network {})", local_network);
            return None;
        }

        pos += dlen;
    }

    // Extract source network info if present
    let source_info = if has_source {
        if pos + 3 > data.len() {
            return None;
        }
        let snet = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let slen = data[pos] as usize;
        pos += 1;
        if pos + slen > data.len() {
            return None;
        }
        let sadr = data[pos..pos + slen].to_vec();
        pos += slen;
        Some(SourceRouteInfo {
            source_network: snet,
            source_address: sadr,
        })
    } else {
        None
    };

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
        info!(">>> No APDU: pos={} >= len={}", pos, data.len());
        return None;
    }

    let apdu = &data[pos..];
    info!(">>> APDU at pos={}: {:02X?}", pos, &apdu[..apdu.len().min(20)]);

    // Process with local device
    info!(">>> Calling local_device.process_apdu()...");
    if let Some((response_apdu, is_broadcast)) = local_device.process_apdu(apdu) {
        info!(">>> Got response from local_device: {} bytes, is_broadcast={}", response_apdu.len(), is_broadcast);
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

        return Some((npdu, is_broadcast, source_info));
    }

    None
}

/// BACnet/IP receive task - reads UDP packets and routes to MS/TP
fn ip_receive_task(
    socket: Arc<UdpSocket>,
    gateway: Arc<Mutex<BacnetGateway>>,
    mstp_driver: Arc<Mutex<MstpDriver<'static>>>,
    local_device: Arc<LocalDevice>,
    ip_network: u16,
    mstp_network: u16,
    gateway_mac: u8,
) {
    info!("BACnet/IP receive task started (gateway MAC {} on networks {} and {})",
          gateway_mac, ip_network, mstp_network);

    let mut buffer = [0u8; 1500];
    let mut poll_count: u32 = 0;

    loop {
        poll_count += 1;
        // Log heartbeat every 1000 polls (~10 seconds at 100ms timeout)
        if poll_count % 1000 == 0 {
            info!("BIP thread alive: {} polls, waiting for UDP on port 47808", poll_count);
        }

        match socket.recv_from(&mut buffer) {
            Ok((len, source_addr)) => {
                let data = &buffer[..len];

                // Log ALL received IP packets for debugging
                info!("BIP RX: {} bytes from {} BVLC: {:02X?}",
                      len, source_addr, &data[..data.len().min(20)]);

                // Debug: Log NPDU destination for routing decisions
                if len > 8 {
                    let npdu_start = if data[1] == 0x04 { 10 } else { 4 };  // Forwarded or Original
                    if len > npdu_start + 4 {
                        let control = data[npdu_start + 1];
                        if (control & 0x20) != 0 {  // DNET present
                            let dnet = ((data[npdu_start + 2] as u16) << 8) | (data[npdu_start + 3] as u16);
                            info!("BIP RX DNET: {} (mstp_network={})", dnet, mstp_network);
                        }
                    }
                }

                // Try to process with local device first (for Who-Is from IP side)
                // Also check for requests addressed to gateway via MS/TP routing (DNET=mstp_network, DADR=gateway_mac)
                if let Some((response_npdu, is_broadcast)) = try_process_ip_local_device(data, &local_device, ip_network, mstp_network, gateway_mac) {
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
                        // Send to broadcast address for network discovery
                        let broadcast_addr = "255.255.255.255:47808";
                        if let Err(e) = socket.send_to(&bvlc, broadcast_addr) {
                            warn!("Failed to send I-Am broadcast: {}", e);
                        }
                        // Also send directly to the requester (common BACnet practice)
                        // This ensures the requester gets our I-Am even if broadcast fails
                        if let Err(e) = socket.send_to(&bvlc, source_addr) {
                            warn!("Failed to send I-Am unicast to {}: {}", source_addr, e);
                        }
                    } else {
                        if let Err(e) = socket.send_to(&bvlc, source_addr) {
                            warn!("Failed to send response to {}: {}", source_addr, e);
                        }
                    }
                }

                // Route the frame through the gateway
                info!("BIP->routing: calling gateway.lock()...");
                if let Ok(mut gw) = gateway.lock() {
                    info!("BIP->routing: calling route_from_ip...");
                    match gw.route_from_ip(data, source_addr) {
                        Ok(Some((mstp_data, mstp_dest))) => {
                            // Check NPDU control byte for expecting-reply bit (bit 2 = 0x04)
                            // NPDU format: [version, control, ...]
                            // Control bit 2 indicates "data expecting reply"
                            let expecting_reply = if mstp_data.len() >= 2 {
                                (mstp_data[1] & 0x04) != 0
                            } else {
                                false
                            };

                            // Send to MS/TP
                            info!("IP->MS/TP routing: {} bytes to MS/TP dest={} expecting_reply={} NPDU: {:02X?}",
                                  mstp_data.len(), mstp_dest, expecting_reply, &mstp_data[..mstp_data.len().min(20)]);
                            if let Ok(mut driver) = mstp_driver.lock() {
                                match driver.send_frame(&mstp_data, mstp_dest, expecting_reply) {
                                    Ok(_) => trace!("IP->MS/TP frame queued successfully"),
                                    Err(e) => warn!("Failed to send to MS/TP: {}", e),
                                }
                            }
                        }
                        Ok(None) => {
                            // Frame handled internally (e.g., BVLC control) or not for MS/TP
                            info!("BIP->routing: route_from_ip returned None (BVLC control or not for MS/TP)");
                        }
                        Err(e) => {
                            warn!("BIP->routing: route_from_ip error: {}", e);
                        }
                    }
                } else {
                    warn!("BIP->routing: gateway.lock() failed!");
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
/// Returns (response_npdu, is_broadcast) - source info is ignored for IP side since
/// the response is sent directly via IP socket to the source_addr
///
/// This function handles requests for the gateway's local device from IP side, including:
/// - Direct requests (no DNET or DNET=ip_network)
/// - Routed requests to gateway's MS/TP address (DNET=mstp_network, DADR=gateway_mac)
fn try_process_ip_local_device(
    data: &[u8],
    local_device: &LocalDevice,
    ip_network: u16,
    mstp_network: u16,
    gateway_mac: u8,
) -> Option<(Vec<u8>, bool)> {
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

    // Check if this is addressed to gateway's MS/TP address (routed request)
    // NPDU: version (1) + control (1) + [DNET (2) + DLEN (1) + DADR (DLEN) + hop_count (1)] + ...
    if npdu_data.len() >= 6 {
        let control = npdu_data[1];
        let has_dest = (control & 0x20) != 0;

        if has_dest {
            let dnet = u16::from_be_bytes([npdu_data[2], npdu_data[3]]);
            let dlen = npdu_data[4] as usize;

            // Check if addressed to gateway's MS/TP address
            if dnet == mstp_network && dlen == 1 && npdu_data.len() > 5 {
                let dadr = npdu_data[5];
                if dadr == gateway_mac {
                    info!(">>> Routed request to gateway's MS/TP address (DNET={}, DADR={})",
                          dnet, dadr);
                    // Process as local device request, using mstp_network as local_network
                    // so the DNET check passes
                    return try_process_local_device(npdu_data, local_device, mstp_network)
                        .map(|(npdu, is_broadcast, _source_info)| (npdu, is_broadcast));
                }
            }
        }
    }

    // Standard processing - check for direct requests (no DNET or DNET=ip_network)
    try_process_local_device(npdu_data, local_device, ip_network)
        .map(|(npdu, is_broadcast, _source_info)| (npdu, is_broadcast))
}

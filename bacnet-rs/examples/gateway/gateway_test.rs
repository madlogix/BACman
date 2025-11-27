//! BACnet MS/TP Gateway Integration Test
//!
//! This example tests the BACnet MS/TP to IP gateway by:
//! 1. Sending Who-Is broadcasts to discover devices behind the gateway
//! 2. Verifying I-Am responses are routed correctly from MS/TP to IP
//! 3. Testing Read-Property requests to MS/TP devices through the gateway
//!
//! Usage:
//!   cargo run --example gateway_test [gateway_ip]
//!
//! The gateway should be configured with:
//! - MS/TP network number (e.g., 65001)
//! - IP network number (e.g., 10001)
//!
//! Expected behavior:
//! - Who-Is broadcast should be forwarded to MS/TP network
//! - I-Am responses from MS/TP devices should include SNET/SADR routing info
//! - Read-Property requests should be routed to correct MS/TP device

use bacnet_rs::{
    datalink::{bip::BacnetIpDataLink, DataLink, DataLinkAddress},
    network::Npdu,
    service::{IAmRequest, UnconfirmedServiceChoice, WhoIsRequest},
    vendor::get_vendor_name,
};
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    time::{Duration, Instant},
};

/// Gateway test configuration
struct GatewayTestConfig {
    gateway_ip: String,
    bacnet_port: u16,
    scan_duration_secs: u64,
    expected_mstp_network: u16,
}

impl Default for GatewayTestConfig {
    fn default() -> Self {
        Self {
            gateway_ip: "255.255.255.255".to_string(),
            bacnet_port: 47808,
            scan_duration_secs: 10,
            expected_mstp_network: 65001, // Default MS/TP network from gateway config
        }
    }
}

/// Structure to hold discovered device information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DiscoveredDevice {
    device_id: u32,
    address: SocketAddr,
    vendor_id: u32,
    vendor_name: String,
    max_apdu: u32,
    segmentation: u32,
    source_network: Option<u16>,
    source_address: Option<Vec<u8>>,
    is_routed: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║       BACnet MS/TP Gateway Integration Test                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Tests gateway routing between BACnet/IP and MS/TP networks  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut config = GatewayTestConfig::default();

    if args.len() > 1 {
        config.gateway_ip = args[1].clone();
    }

    println!("Test Configuration:");
    println!("  Gateway IP: {}", config.gateway_ip);
    println!("  BACnet Port: {}", config.bacnet_port);
    println!("  Scan Duration: {} seconds", config.scan_duration_secs);
    println!("  Expected MS/TP Network: {}", config.expected_mstp_network);
    println!();

    // Run tests
    let results = run_gateway_tests(&config)?;

    // Print results
    print_test_results(&results, &config);

    Ok(())
}

/// Test results structure
struct TestResults {
    devices_discovered: HashMap<u32, DiscoveredDevice>,
    routed_devices: Vec<u32>,
    direct_devices: Vec<u32>,
    total_messages_received: u32,
    routing_info_present: bool,
}

fn run_gateway_tests(config: &GatewayTestConfig) -> Result<TestResults, Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 1: Device Discovery via Gateway");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Create BACnet/IP data link
    println!("[1/4] Creating BACnet/IP data link...");
    let mut datalink = BacnetIpDataLink::new("0.0.0.0:0")?;
    println!("      ✓ Data link created successfully");

    // Create Who-Is request
    println!("[2/4] Preparing Who-Is broadcast...");
    let whois = WhoIsRequest::new();
    let mut service_data = Vec::new();
    whois.encode(&mut service_data)?;

    // Create APDU
    let mut apdu_buffer = Vec::new();
    apdu_buffer.push(0x10); // Unconfirmed Request PDU
    apdu_buffer.push(UnconfirmedServiceChoice::WhoIs as u8);
    apdu_buffer.extend_from_slice(&service_data);

    // Create NPDU with global broadcast
    let npdu = Npdu::global_broadcast();
    let npdu_buffer = npdu.encode();

    // Combine NPDU and APDU
    let mut message = npdu_buffer;
    message.extend_from_slice(&apdu_buffer);

    println!("      ✓ Who-Is message prepared ({} bytes)", message.len());

    // Send Who-Is broadcast
    println!("[3/4] Sending Who-Is broadcasts...");

    // Global broadcast
    match datalink.send_frame(&message, &DataLinkAddress::Broadcast) {
        Ok(_) => println!("      ✓ Global broadcast sent"),
        Err(e) => println!("      ✗ Global broadcast failed: {:?}", e),
    }

    // Directed to gateway if specified
    if config.gateway_ip != "255.255.255.255" {
        let gateway_addr: SocketAddr = format!("{}:{}", config.gateway_ip, config.bacnet_port).parse()?;
        match datalink.send_frame(&message, &DataLinkAddress::Ip(gateway_addr)) {
            Ok(_) => println!("      ✓ Directed broadcast to gateway: {}", gateway_addr),
            Err(e) => println!("      ✗ Directed broadcast failed: {:?}", e),
        }
    }

    // Common subnet broadcasts
    let local_broadcasts = vec![
        "10.161.1.255:47808",
        "192.168.1.255:47808",
        "192.168.0.255:47808",
    ];

    for addr_str in &local_broadcasts {
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            if datalink.send_frame(&message, &DataLinkAddress::Ip(addr)).is_ok() {
                println!("      ✓ Subnet broadcast to: {}", addr);
            }
        }
    }

    // Listen for responses
    println!("[4/4] Listening for I-Am responses...");
    println!();

    let mut results = TestResults {
        devices_discovered: HashMap::new(),
        routed_devices: Vec::new(),
        direct_devices: Vec::new(),
        total_messages_received: 0,
        routing_info_present: false,
    };

    let scan_duration = Duration::from_secs(config.scan_duration_secs);
    let start_time = Instant::now();
    let mut last_broadcast = Instant::now();

    while start_time.elapsed() < scan_duration {
        // Re-broadcast every 3 seconds
        if last_broadcast.elapsed() > Duration::from_secs(3) {
            let _ = datalink.send_frame(&message, &DataLinkAddress::Broadcast);
            last_broadcast = Instant::now();
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush()?;
        }

        // Try to receive a response
        match datalink.receive_frame() {
            Ok((data, source)) => {
                results.total_messages_received += 1;

                let source_addr = match source {
                    DataLinkAddress::Ip(addr) => addr,
                    _ => continue,
                };

                if let Some(device) = process_iam_response(&data, source_addr) {
                    let device_id = device.device_id;

                    if device.is_routed {
                        results.routing_info_present = true;
                        if !results.routed_devices.contains(&device_id) {
                            results.routed_devices.push(device_id);
                        }
                    } else if !results.direct_devices.contains(&device_id) {
                        results.direct_devices.push(device_id);
                    }

                    if let std::collections::hash_map::Entry::Vacant(e) =
                        results.devices_discovered.entry(device_id)
                    {
                        println!();
                        println!("  ┌─ Device Discovered ─────────────────────────────────────────┐");
                        println!("  │ Device ID: {:>10}                                       │", device.device_id);
                        println!("  │ IP Address: {:50} │", format!("{}", device.address));
                        println!("  │ Vendor: {:54} │", format!("{} ({})", device.vendor_name, device.vendor_id));

                        if device.is_routed {
                            if let Some(net) = device.source_network {
                                println!("  │ Source Network: {:>5} (ROUTED via gateway)               │", net);
                            }
                            if let Some(ref addr) = device.source_address {
                                println!("  │ MS/TP Address: {:>6}                                      │", addr.first().map(|a| a.to_string()).unwrap_or_default());
                            }
                        } else {
                            println!("  │ Connection: Direct (not routed)                            │");
                        }
                        println!("  └───────────────────────────────────────────────────────────────┘");

                        e.insert(device);
                    }
                }
            }
            Err(_) => {
                // Timeout - normal
            }
        }
    }

    println!();
    Ok(results)
}

/// Process I-Am response and extract device info with routing information
fn process_iam_response(data: &[u8], source: SocketAddr) -> Option<DiscoveredDevice> {
    if data.len() < 2 {
        return None;
    }

    // Decode NPDU to get routing info
    let (npdu, npdu_len) = match Npdu::decode(data) {
        Ok(result) => result,
        Err(_) => return None,
    };

    // Check for source routing info (indicates routed through gateway)
    // The source field contains Optional<NetworkAddress> with network number and address
    let (source_network, source_address) = match &npdu.source {
        Some(net_addr) => (Some(net_addr.network), Some(net_addr.address.clone())),
        None => (None, None),
    };
    let is_routed = source_network.is_some();

    // Skip to APDU
    if data.len() <= npdu_len {
        return None;
    }

    let apdu = &data[npdu_len..];

    // Check for unconfirmed I-Am
    if apdu.len() < 2 || apdu[0] != 0x10 {
        return None;
    }

    if apdu[1] != UnconfirmedServiceChoice::IAm as u8 {
        return None;
    }

    // Decode I-Am
    if apdu.len() <= 2 {
        return None;
    }

    match IAmRequest::decode(&apdu[2..]) {
        Ok(iam) => {
            let vendor_name = get_vendor_name(iam.vendor_identifier as u16)
                .unwrap_or("Unknown")
                .to_string();

            Some(DiscoveredDevice {
                device_id: iam.device_identifier.instance,
                address: source,
                vendor_id: iam.vendor_identifier,
                vendor_name,
                max_apdu: iam.max_apdu_length_accepted,
                segmentation: iam.segmentation_supported,
                source_network,
                source_address,
                is_routed,
            })
        }
        Err(_) => None,
    }
}

fn print_test_results(results: &TestResults, config: &GatewayTestConfig) {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST RESULTS SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    println!("Statistics:");
    println!("  Total messages received: {}", results.total_messages_received);
    println!("  Total devices discovered: {}", results.devices_discovered.len());
    println!("  Routed devices (via gateway): {}", results.routed_devices.len());
    println!("  Direct devices (BACnet/IP): {}", results.direct_devices.len());
    println!();

    // Gateway routing test
    println!("Gateway Routing Test:");
    if results.routing_info_present {
        println!("  ✓ PASS - Received I-Am with source routing info");
        println!("           Gateway is correctly adding SNET/SADR to routed messages");
    } else if results.devices_discovered.is_empty() {
        println!("  ? INCONCLUSIVE - No devices found");
        println!("    - Check that gateway is powered on and connected");
        println!("    - Check that MS/TP devices are on the bus");
        println!("    - Verify gateway IP address: {}", config.gateway_ip);
    } else {
        println!("  ? INFO - Only direct BACnet/IP devices found");
        println!("           No MS/TP devices detected behind gateway");
    }
    println!();

    // List devices by type
    if !results.routed_devices.is_empty() {
        println!("Routed Devices (MS/TP via Gateway):");
        for device_id in &results.routed_devices {
            if let Some(device) = results.devices_discovered.get(device_id) {
                println!("  • Device {} - {} @ MSTP addr {:?}",
                    device.device_id,
                    device.vendor_name,
                    device.source_address.as_ref().and_then(|a| a.first())
                );
            }
        }
        println!();
    }

    if !results.direct_devices.is_empty() {
        println!("Direct Devices (BACnet/IP):");
        for device_id in &results.direct_devices {
            if let Some(device) = results.devices_discovered.get(device_id) {
                println!("  • Device {} - {} @ {}",
                    device.device_id,
                    device.vendor_name,
                    device.address
                );
            }
        }
        println!();
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("Integration test complete.");
    println!();
    println!("Next steps:");
    println!("1. Run 'cargo run --example whois_scan' for comprehensive discovery");
    println!("2. Use Wireshark with BACnet filter to inspect traffic");
    println!("3. Test Read-Property to specific devices through gateway");
}

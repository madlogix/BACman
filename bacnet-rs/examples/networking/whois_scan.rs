//! BACnet Who-Is Scan Example
//!
//! This example demonstrates how to perform a Who-Is scan to discover
//! BACnet devices on the network using the production-grade broadcast implementation.
//!
//! # Broadcast Configuration
//!
//! By default, the scanner now sends to:
//! - Global broadcast (255.255.255.255:47808) for maximum reach
//! - Local subnet broadcast (calculated from IP and subnet mask)
//!
//! You can customize the broadcast behavior using `BroadcastConfig`.

use bacnet_rs::{
    datalink::{
        bip::{BacnetIpDataLink, BroadcastConfig},
        DataLink, DataLinkAddress,
    },
    network::Npdu,
    service::{IAmRequest, UnconfirmedServiceChoice, WhoIsRequest},
    vendor::get_vendor_name,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

// Note: Ipv4Addr is available via BroadcastConfig if you need to add custom broadcast addresses:
// use std::net::Ipv4Addr;
// let config = BroadcastConfig::default()
//     .with_additional_broadcast(Ipv4Addr::new(192, 168, 1, 255));

/// Structure to hold discovered device information
#[derive(Debug, Clone)]
struct DiscoveredDevice {
    device_id: u32,
    address: SocketAddr,
    vendor_id: u32,
    vendor_name: String,
    max_apdu: u32,
    segmentation: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BACnet Who-Is Scan Example (Production-Grade)");
    println!("=============================================\n");

    // Create broadcast configuration
    // Default config sends to both global (255.255.255.255) and local subnet broadcast
    // You can customize this for your network topology:
    //
    // Option 1: Default (recommended for unknown networks)
    let config = BroadcastConfig::default();
    //
    // Option 2: Explicit subnet mask (if you know your network)
    // let config = BroadcastConfig::with_subnet_mask([255, 255, 255, 0]);
    //
    // Option 3: Multiple subnets
    // let config = BroadcastConfig::default()
    //     .with_additional_broadcast(Ipv4Addr::new(192, 168, 1, 255))
    //     .with_additional_broadcast(Ipv4Addr::new(10, 0, 0, 255));
    //
    // Option 4: Global broadcast only
    // let config = BroadcastConfig::global_only();

    // Create BACnet/IP data link with broadcast config
    println!("Creating BACnet/IP data link...");
    let mut datalink = BacnetIpDataLink::with_config("0.0.0.0:0", config)?;

    println!("Data link created successfully");
    println!("  Local address: {:?}", datalink.local_address());
    println!("  Subnet mask: {:?}", datalink.subnet_mask());
    println!("  Local broadcast: {}", datalink.local_broadcast_addr());
    println!(
        "  Global broadcast enabled: {}",
        datalink.broadcast_config().use_global_broadcast
    );
    println!(
        "  Local broadcast enabled: {}",
        datalink.broadcast_config().use_local_broadcast
    );
    println!("\nStarting Who-Is scan...\n");

    // Create Who-Is request (broadcast to all devices)
    let whois = WhoIsRequest::new();

    // Encode the Who-Is service
    let mut service_data = Vec::new();
    whois.encode(&mut service_data)?;

    // Create APDU (Application Protocol Data Unit)
    let mut apdu_buffer = Vec::new();
    apdu_buffer.push(0x10); // Unconfirmed Request PDU
    apdu_buffer.push(UnconfirmedServiceChoice::WhoIs as u8);
    apdu_buffer.extend_from_slice(&service_data);

    // Create NPDU using our corrected global broadcast
    let npdu = Npdu::global_broadcast();

    // Encode NPDU
    let npdu_buffer = npdu.encode();

    // Combine NPDU and APDU
    let mut message = npdu_buffer;
    message.extend_from_slice(&apdu_buffer);

    // Send the Who-Is broadcast using the detailed API to see what happened
    println!("Sending Who-Is broadcast...");
    let broadcast_result = datalink.send_broadcast_npdu_detailed(&message);

    println!("Broadcast results:");
    println!(
        "  Successful: {} destinations",
        broadcast_result.success_count()
    );
    println!("  Failed: {} destinations", broadcast_result.failure_count());

    for success in &broadcast_result.successes {
        println!(
            "  ✓ {:?} -> {} ({} bytes)",
            success.broadcast_type, success.address, success.bytes_sent
        );
    }

    for failure in &broadcast_result.failures {
        println!(
            "  ✗ {:?} -> {}: {}",
            failure.broadcast_type, failure.address, failure.error
        );
    }

    if !broadcast_result.any_success() {
        println!("WARNING: No broadcasts succeeded! Check network configuration.");
    }

    println!("\nListening for I-Am responses...\n");

    // Listen for responses
    let mut discovered_devices: HashMap<u32, DiscoveredDevice> = HashMap::new();
    let scan_duration = Duration::from_secs(5);
    let start_time = Instant::now();

    // Send periodic Who-Is broadcasts
    let mut last_broadcast = Instant::now();

    while start_time.elapsed() < scan_duration {
        // Re-broadcast every 2 seconds
        if last_broadcast.elapsed() > Duration::from_secs(2) {
            println!("Sending periodic Who-Is broadcast...");
            let _ = datalink.send_frame(&message, &DataLinkAddress::Broadcast);
            last_broadcast = Instant::now();
        }

        // Try to receive a response
        match datalink.receive_frame() {
            Ok((data, source)) => {
                // Convert DataLinkAddress to SocketAddr for display
                let source_addr = match source {
                    DataLinkAddress::Ip(addr) => addr,
                    _ => continue, // Skip non-IP addresses
                };

                println!("Received {} bytes from {}", data.len(), source_addr);

                // Process the received message
                if let Some(device) = process_response(&data, source_addr) {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        discovered_devices.entry(device.device_id)
                    {
                        println!("Discovered new device:");
                        println!("  Device ID: {}", device.device_id);
                        println!("  Address: {}", device.address);
                        println!(
                            "  Vendor: {} (ID: {})",
                            device.vendor_name, device.vendor_id
                        );
                        println!("  Max APDU: {}", device.max_apdu);
                        println!(
                            "  Segmentation: {}",
                            match device.segmentation {
                                0 => "Both",
                                1 => "Transmit",
                                2 => "Receive",
                                3 => "None",
                                _ => "Unknown",
                            }
                        );
                        println!();

                        e.insert(device);
                    }
                }
            }
            Err(_) => {
                // Timeout or error - normal during scanning
            }
        }

        // Show progress
        let elapsed = start_time.elapsed().as_secs();
        print!(
            "\rScanning... {} seconds elapsed, {} devices found",
            elapsed,
            discovered_devices.len()
        );
        use std::io::{self, Write};
        io::stdout().flush()?;
    }

    println!("\n\nScan Complete!");
    println!("==============");
    println!("Total devices discovered: {}", discovered_devices.len());

    if !discovered_devices.is_empty() {
        println!("\nDevice Summary:");
        println!("---------------");

        // Sort by device ID
        let mut devices: Vec<_> = discovered_devices.values().collect();
        devices.sort_by_key(|d| d.device_id);

        for device in devices {
            println!(
                "Device {} @ {} - {}",
                device.device_id, device.address, device.vendor_name
            );
        }
    } else {
        println!("\nNo devices found. Possible reasons:");
        println!("- No BACnet devices on the network");
        println!("- Devices are on a different subnet");
        println!("- Firewall blocking BACnet traffic (UDP port 47808)");
        println!("- Devices configured for different port");
    }

    Ok(())
}

/// Process a received message and extract I-Am information
fn process_response(data: &[u8], source: SocketAddr) -> Option<DiscoveredDevice> {
    println!(
        "  Raw data: {:02X?}",
        &data[..std::cmp::min(32, data.len())]
    );

    // The BacnetIpDataLink already strips the BVLC header, so we start with NPDU
    if data.len() < 2 {
        println!("  Too short for NPDU");
        return None;
    }

    // Decode NPDU starting from the beginning of the data
    let (_npdu, npdu_len) = match Npdu::decode(data) {
        Ok(result) => result,
        Err(e) => {
            println!("  Failed to decode NPDU: {:?}", e);
            return None;
        }
    };

    // Skip to APDU
    let apdu_start = npdu_len;
    if data.len() <= apdu_start {
        println!("  Too short for APDU");
        return None;
    }

    let apdu = &data[apdu_start..];

    // Check if this is an unconfirmed I-Am service
    if apdu.len() < 2 || apdu[0] != 0x10 {
        // Unconfirmed Request PDU
        return None;
    }

    let service_choice = apdu[1];
    if service_choice != UnconfirmedServiceChoice::IAm as u8 {
        return None;
    }

    // Decode I-Am request
    if apdu.len() <= 2 {
        return None;
    }

    match IAmRequest::decode(&apdu[2..]) {
        Ok(iam) => {
            let vendor_name = get_vendor_name(iam.vendor_identifier as u16)
                .unwrap_or("Unknown Vendor")
                .to_string();

            Some(DiscoveredDevice {
                device_id: iam.device_identifier.instance,
                address: source,
                vendor_id: iam.vendor_identifier,
                vendor_name,
                max_apdu: iam.max_apdu_length_accepted,
                segmentation: iam.segmentation_supported,
            })
        }
        Err(_) => None,
    }
}

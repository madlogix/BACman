//! BACnet Transport Layer Demo
//!
//! This example demonstrates the comprehensive BACnet transport layer functionality
//! including BVLL encoding/decoding, foreign device registration, and broadcast management.

use bacnet_rs::transport::{
    constants::DEFAULT_FD_TTL, BacnetIpConfig, BacnetIpTransport, BdtEntry, BroadcastManager,
    BvllFunction, BvllMessage, ForeignDeviceRegistration, Transport,
};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BACnet Transport Layer Demo");
    println!("===========================\n");

    // Demo 1: BVLL Message Handling
    demo_bvll_messages()?;

    // Demo 2: BACnet/IP Transport
    demo_bacnet_ip_transport()?;

    // Demo 3: Foreign Device Registration
    demo_foreign_device_registration()?;

    // Demo 4: Broadcast Distribution Table Management
    demo_broadcast_management()?;

    println!("\nBACnet Transport Demo Complete!");
    Ok(())
}

/// Demonstrate BVLL message encoding and decoding
fn demo_bvll_messages() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. BVLL Message Handling");
    println!("========================");

    // Create test NPDU data
    let test_npdu = vec![
        0x01, 0x00, // NPDU header
        0x30, 0x00, 0x00, 0x00, // Sample APDU
    ];

    // Create Original-Unicast-NPDU message
    let unicast_msg = BvllMessage::new(BvllFunction::OriginalUnicastNpdu, test_npdu.clone());
    println!("Created Original-Unicast-NPDU message");
    println!("  Function: {:?}", unicast_msg.header.function);
    println!("  Length: {} bytes", unicast_msg.header.length);

    // Encode the message
    let encoded = unicast_msg.encode();
    println!("  Encoded size: {} bytes", encoded.len());
    println!("  Encoded data: {:02X?}", &encoded[..8]);

    // Decode the message back
    let decoded = BvllMessage::decode(&encoded)?;
    println!("  Decoded successfully");
    println!(
        "  Function matches: {}",
        decoded.header.function as u8 == unicast_msg.header.function as u8
    );
    println!("  Data matches: {}", decoded.data == unicast_msg.data);

    // Test broadcast message
    let broadcast_msg = BvllMessage::new(BvllFunction::OriginalBroadcastNpdu, test_npdu);
    let broadcast_encoded = broadcast_msg.encode();
    println!(
        "  Broadcast message encoded: {} bytes",
        broadcast_encoded.len()
    );

    println!();
    Ok(())
}

/// Demonstrate BACnet/IP transport configuration and usage
fn demo_bacnet_ip_transport() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. BACnet/IP Transport");
    println!("======================");

    // Create transport with default configuration
    let transport = BacnetIpTransport::new_default("0.0.0.0:0")?;
    println!("Created BACnet/IP transport");
    println!("  Local address: {}", transport.local_address()?);
    println!("  Is connected: {}", transport.is_connected());

    let config = transport.config();
    println!("  Broadcast enabled: {}", config.broadcast_enabled);
    println!("  Buffer size: {} bytes", config.buffer_size);
    println!("  BDT entries: {}", config.bdt.len());

    // Demonstrate configuration
    let custom_config = BacnetIpConfig {
        buffer_size: 2048,
        broadcast_enabled: true,
        ..Default::default()
    };

    println!("  Custom config buffer size: {}", custom_config.buffer_size);

    println!();
    Ok(())
}

/// Demonstrate foreign device registration
fn demo_foreign_device_registration() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Foreign Device Registration");
    println!("==============================");

    // Simulate BBMD address
    let bbmd_addr: SocketAddr = "192.168.1.100:47808".parse()?;

    println!("Foreign Device Registration Process:");
    println!("  BBMD Address: {}", bbmd_addr);
    println!("  TTL: {} seconds", DEFAULT_FD_TTL);

    // Create foreign device registration data
    let mut data = Vec::new();
    data.extend_from_slice(&DEFAULT_FD_TTL.to_be_bytes());

    let fd_message = BvllMessage::new(BvllFunction::RegisterForeignDevice, data);
    let encoded = fd_message.encode();

    println!("  Registration message size: {} bytes", encoded.len());
    println!("  Message header: {:02X?}", &encoded[..4]);
    println!("  TTL bytes: {:02X?}", &encoded[4..6]);

    // Simulate registration tracking
    let registration = ForeignDeviceRegistration {
        bbmd_address: bbmd_addr,
        ttl: DEFAULT_FD_TTL,
        last_registration: Instant::now(),
    };

    println!(
        "  Registration created at: {:?}",
        registration.last_registration
    );

    // Simulate time passing and check if re-registration is needed
    std::thread::sleep(Duration::from_millis(10));
    let elapsed = registration.last_registration.elapsed().as_secs() as u16;
    let needs_reregistration = elapsed >= registration.ttl / 2;

    println!("  Elapsed time: {} seconds", elapsed);
    println!("  Needs re-registration: {}", needs_reregistration);

    println!();
    Ok(())
}

/// Demonstrate broadcast distribution table management
fn demo_broadcast_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Broadcast Distribution Table Management");
    println!("==========================================");

    let mut manager = BroadcastManager::new();

    // Add some BDT entries
    let entries = [
        BdtEntry {
            address: "192.168.1.255".parse()?,
            port: 47808,
            mask: "255.255.255.0".parse()?,
        },
        BdtEntry {
            address: "10.0.0.255".parse()?,
            port: 47808,
            mask: "255.0.0.0".parse()?,
        },
        BdtEntry {
            address: "172.16.255.255".parse()?,
            port: 47808,
            mask: "255.255.0.0".parse()?,
        },
    ];

    for (i, entry) in entries.iter().enumerate() {
        manager.add_bdt_entry(entry.clone());
        println!(
            "  Added BDT entry {}: {} (mask: {})",
            i + 1,
            entry.address,
            entry.mask
        );
    }

    println!("  Total BDT entries: {}", manager.get_bdt_entries().len());

    // Encode BDT for transmission
    let encoded_bdt = manager.encode_bdt();
    println!("  Encoded BDT size: {} bytes", encoded_bdt.len());
    println!(
        "  Expected size: {} bytes (3 entries × 10 bytes each)",
        3 * 10
    );

    // Show encoded structure
    println!("  Encoded BDT structure:");
    for (i, chunk) in encoded_bdt.chunks(10).enumerate() {
        if chunk.len() == 10 {
            let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
            let mask = format!("{}.{}.{}.{}", chunk[6], chunk[7], chunk[8], chunk[9]);
            println!(
                "    Entry {}: IP={}, Port={}, Mask={}",
                i + 1,
                ip,
                port,
                mask
            );
        }
    }

    // Test decoding
    let mut decode_manager = BroadcastManager::new();
    decode_manager.decode_bdt(&encoded_bdt)?;

    let decoded_entries = decode_manager.get_bdt_entries();
    println!("  Decoded {} entries successfully", decoded_entries.len());

    // Verify data integrity
    for (i, (original, decoded)) in entries.iter().zip(decoded_entries.iter()).enumerate() {
        let matches = original.address == decoded.address && original.port == decoded.port;
        println!(
            "    Entry {} integrity check: {}",
            i + 1,
            if matches { "✓ PASS" } else { "✗ FAIL" }
        );
    }

    // Test BDT management operations
    println!("  Testing BDT management operations:");

    // Remove an entry
    let remove_ip: IpAddr = "10.0.0.255".parse()?;
    manager.remove_bdt_entry(remove_ip);
    println!("    Removed entry for {}", remove_ip);
    println!("    Remaining entries: {}", manager.get_bdt_entries().len());

    // Show remaining entries
    for entry in manager.get_bdt_entries() {
        println!(
            "      - {} (port: {}, mask: {})",
            entry.address, entry.port, entry.mask
        );
    }

    println!();
    Ok(())
}

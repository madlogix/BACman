//! BACnet Vendor ID Lookup Example
//!
//! This example demonstrates how to use the vendor lookup functionality
//! to identify BACnet devices by their vendor IDs and work with official
//! vendor information.

use bacnet_rs::{
    object::Device,
    vendor::{
        find_vendors_by_name, format_vendor_display, get_vendor_info, get_vendor_name,
        get_vendor_statistics, is_vendor_id_reserved,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BACnet Vendor ID Lookup Example");
    println!("===============================\n");

    // 1. Basic vendor lookups
    println!("1. Basic Vendor Lookups:");

    // Look up some common vendors
    let vendors_to_check = vec![0, 1, 2, 5, 105, 999, 9999];

    for vendor_id in vendors_to_check {
        match get_vendor_name(vendor_id) {
            Some(name) => println!("   ID {}: {}", vendor_id, name),
            None => println!("   ID {}: Unknown/Unassigned", vendor_id),
        }
    }

    println!();

    // 2. Detailed vendor information
    println!("2. Detailed Vendor Information:");

    if let Some(vendor_info) = get_vendor_info(5) {
        println!("   Vendor Info: {}", vendor_info);
        println!("   - ID: {}", vendor_info.id);
        println!("   - Name: {}", vendor_info.name);
    }

    println!();

    // 3. Search vendors by name
    println!("3. Vendor Search by Name:");

    let search_terms = vec!["Johnson", "Trane", "Schneider", "Honeywell"];

    for term in search_terms {
        let matching_vendors = find_vendors_by_name(term);
        println!("   '{}' matches:", term);
        for vendor in matching_vendors.iter().take(3) {
            // Show first 3 matches
            println!("     - {}", vendor);
        }
        if matching_vendors.len() > 3 {
            println!("     ... and {} more", matching_vendors.len() - 3);
        }
        println!();
    }

    // 4. Vendor statistics
    println!("4. Vendor Statistics:");
    let stats = get_vendor_statistics();
    println!("   {}", stats);
    println!();

    // 5. Working with Device objects
    println!("5. Device Vendor Information:");

    // Create devices with different vendor configurations
    let mut device1 = Device::new(1001, "Temperature Controller".to_string());
    println!(
        "   Default device vendor: {}",
        device1.format_vendor_display()
    );
    println!("   Is test vendor: {}", device1.is_vendor_id_test());
    println!();

    // Set to official vendor
    if device1.set_vendor_by_id(5).is_ok() {
        println!("   After setting to Johnson Controls (ID 5):");
        println!("   - Vendor display: {}", device1.format_vendor_display());
        println!("   - Official vendor: {}", device1.is_vendor_id_official());
        println!("   - Configured name: {}", device1.vendor_name);

        if let Some(official_name) = device1.get_official_vendor_name() {
            println!("   - Official name: {}", official_name);
        }
    }
    println!();

    // Create device with Trane vendor ID
    let mut device2 = Device::new(1002, "HVAC Unit".to_string());
    if device2.set_vendor_by_id(2).is_ok() {
        println!("   Trane device: {}", device2.format_vendor_display());
    }

    // Create device with custom vendor name but keep vendor ID
    device2.set_vendor_name("Trane - Custom Division".to_string());
    println!("   Custom name: {}", device2.vendor_name);
    println!(
        "   Official name: {}",
        device2.get_official_vendor_name().unwrap_or("Unknown")
    );
    println!();

    // 6. Reserved vendor IDs
    println!("6. Reserved/Test Vendor IDs:");
    let test_ids = vec![555, 666, 777, 888, 911, 999, 1111];

    for id in test_ids {
        let reserved = is_vendor_id_reserved(id);
        let display = format_vendor_display(id);
        println!("   ID {}: {} (Reserved: {})", id, display, reserved);
    }
    println!();

    // 7. Practical example: Identifying devices on a network
    println!("7. Network Device Identification Example:");

    // Simulate discovering devices with different vendor IDs
    let discovered_devices = vec![
        (2001, 5),    // Johnson Controls device
        (2002, 2),    // Trane device
        (2003, 105),  // Honeywell device
        (2004, 999),  // Test device
        (2005, 1234), // Unknown vendor
    ];

    for (device_instance, vendor_id) in discovered_devices {
        let vendor_display = format_vendor_display(vendor_id);
        let status = if is_vendor_id_reserved(vendor_id) {
            "TEST/RESERVED"
        } else if get_vendor_name(vendor_id).is_some() {
            "OFFICIAL"
        } else {
            "UNKNOWN"
        };

        println!(
            "   Device {}: {} [{}]",
            device_instance, vendor_display, status
        );
    }

    println!("\n8. Tips for Production Use:");
    println!("   - Vendor ID 999 is reserved for testing - don't use in production");
    println!("   - Get an official vendor ID from ASHRAE for commercial products");
    println!("   - Use vendor lookup to validate device compatibility");
    println!("   - Check if vendor IDs are official vs. test/reserved");

    Ok(())
}

//! Debug Formatting Utilities Example
//!
//! This example demonstrates the comprehensive debug formatting capabilities
//! for BACnet data structures and protocol analysis. These utilities are
//! essential for debugging communication issues, analyzing protocol packets,
//! and understanding BACnet data structures.

use bacnet_rs::util::debug;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BACnet Debug Formatting Utilities Demo ===\n");

    // 1. Property value formatting
    demo_property_value_formatting()?;

    // 2. Service choice formatting
    demo_service_choice_formatting()?;

    // 3. Error formatting
    demo_error_formatting()?;

    // 4. Protocol structure analysis
    demo_protocol_structure_analysis()?;

    // 5. Annotated hex dumps
    demo_annotated_hex_dumps()?;

    // 6. Complete packet analysis
    demo_complete_packet_analysis()?;

    println!("\n=== Debug Formatting Demo Complete ===");
    Ok(())
}

fn demo_property_value_formatting() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Property Value Formatting");
    println!("=============================");

    // Boolean values
    let boolean_true = &[0x11, 0x01];
    let boolean_false = &[0x11, 0x00];
    println!(
        "Boolean true:  {}",
        debug::format_property_value(boolean_true)
    );
    println!(
        "Boolean false: {}",
        debug::format_property_value(boolean_false)
    );

    // Real values (IEEE 754 floats)
    let real_42 = &[0x44, 0x42, 0x28, 0x00, 0x00]; // 42.0
    let real_temp = &[0x44, 0x41, 0xB6, 0x66, 0x66]; // 22.8°C
    println!("Real 42.0:     {}", debug::format_property_value(real_42));
    println!("Real 22.8:     {}", debug::format_property_value(real_temp));

    // Unsigned integers
    let uint_small = &[0x21, 0x05]; // 5
    let uint_large = &[0x24, 0x00, 0x01, 0x00, 0x00]; // 65536
    println!(
        "UInt 5:        {}",
        debug::format_property_value(uint_small)
    );
    println!(
        "UInt 65536:    {}",
        debug::format_property_value(uint_large)
    );

    // Character strings
    let ascii_string = &[0x75, 0x05, 0x00, b'H', b'e', b'l', b'l', b'o'];
    println!(
        "ASCII String:  {}",
        debug::format_property_value(ascii_string)
    );

    // UTF-16 string ("Test" in UTF-16 BE)
    let utf16_string = &[
        0x75, 0x08, 0x04, 0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74,
    ];
    println!(
        "UTF-16 String: {}",
        debug::format_property_value(utf16_string)
    );

    // Enumerated values
    let enum_units = &[0x91, 0x3E]; // Engineering units: degrees Celsius (62)
    println!(
        "Enumerated:    {}",
        debug::format_property_value(enum_units)
    );

    // Object identifiers
    let analog_input_1 = &[0xC4, 0x00, 0x00, 0x00, 0x01]; // analog-input,1
    let device_1234 = &[0xC4, 0x02, 0x00, 0x04, 0xD2]; // device,1234
    println!(
        "AI 1:          {}",
        debug::format_property_value(analog_input_1)
    );
    println!(
        "Device 1234:   {}",
        debug::format_property_value(device_1234)
    );

    // Date and time
    let date_data = &[0xA1, 0x7C, 0x03, 0x0F, 0x05]; // 2024/3/15 (Fri)
    let time_data = &[0xB1, 0x0E, 0x1E, 0x2D, 0x32]; // 14:30:45.50
    println!("Date:          {}", debug::format_property_value(date_data));
    println!("Time:          {}", debug::format_property_value(time_data));

    // Octet string
    let octet_data = &[0x83, 0xAA, 0xBB, 0xCC]; // 3 bytes: AA BB CC
    println!(
        "Octet String:  {}",
        debug::format_property_value(octet_data)
    );

    Ok(())
}

fn demo_service_choice_formatting() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Service Choice Formatting");
    println!("==============================");

    // Common confirmed services
    let services = [
        (12, "Read Property"),
        (14, "Read Property Multiple"),
        (15, "Write Property"),
        (16, "Write Property Multiple"),
        (10, "Create Object"),
        (11, "Delete Object"),
        (6, "Atomic Read File"),
        (7, "Atomic Write File"),
        (28, "Subscribe COV"),
        (20, "Reinitialize Device"),
    ];

    for (code, description) in services {
        println!(
            "{:2}: {} -> {}",
            code,
            description,
            debug::format_service_choice(code)
        );
    }

    // Unknown service
    println!(
        "99: Unknown Service -> {}",
        debug::format_service_choice(99)
    );

    Ok(())
}

fn demo_error_formatting() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Error Formatting");
    println!("====================");

    // Common BACnet error combinations
    let errors = [
        (0, 5, "Device busy"),
        (1, 2, "Object not found"),
        (2, 9, "Property not found"),
        (2, 10, "Property value out of range"),
        (2, 16, "Write access denied"),
        (3, 1, "Out of memory"),
        (4, 1, "Authentication required"),
        (5, 1, "Service not supported"),
        (6, 2, "VT session terminated"),
        (7, 1, "Communication timeout"),
    ];

    for (class, code, description) in errors {
        println!(
            "{} -> {}",
            description,
            debug::format_bacnet_error(class, code)
        );
    }

    Ok(())
}

fn demo_protocol_structure_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. Protocol Structure Analysis");
    println!("===============================");

    // BVLL structure analysis
    println!("BVLL (BACnet Virtual Link Layer):");
    let bvll_data = &[
        0x81, 0x0A, 0x00, 0x10, // BVLL header: BACnet/IP, Original-Unicast-NPDU, length 16
        0x01, 0x00, 0x30, 0x00, // NPDU start
        0x0C, 0x02, 0x00, 0x13, // APDU start
        0xB7, 0x19, 0x4D, 0x00, // Property data
    ];
    println!("{}", debug::format_bvll_structure(bvll_data));

    // NPDU structure analysis
    println!("NPDU (Network Protocol Data Unit):");
    let npdu_data = &bvll_data[4..]; // Skip BVLL header
    println!("{}", debug::format_npdu_structure(npdu_data));

    // APDU structure analysis
    println!("APDU (Application Protocol Data Unit):");

    // Confirmed Request: Read Property
    let confirmed_request = &[
        0x00, 0x05, 0x0C, // Confirmed request, invoke ID 5, read property
        0x0C, 0x02, 0x00, 0x00, 0x01, // Object ID: analog-input,1
        0x19, 0x55, // Property: present-value (85)
    ];
    println!("Confirmed Request (Read Property):");
    println!("{}", debug::format_apdu_structure(confirmed_request));

    // Complex ACK response
    let complex_ack = &[
        0x30, 0x05, 0x0C, // Complex ACK, invoke ID 5, read property
        0x0C, 0x02, 0x00, 0x00, 0x01, // Object ID: analog-input,1
        0x19, 0x55, // Property: present-value
        0x3E, 0x44, 0x42, 0x28, 0x00, 0x00, 0x3F, // Value: 42.0
    ];
    println!("\nComplex ACK (Read Property Response):");
    println!("{}", debug::format_apdu_structure(complex_ack));

    // Error response
    let error_response = &[
        0x50, 0x05, 0x0C, // Error PDU, invoke ID 5, read property
        0x91, 0x01, // Error class: object (1)
        0x91, 0x02, // Error code: unknown object (2)
    ];
    println!("\nError Response:");
    println!("{}", debug::format_apdu_structure(error_response));

    Ok(())
}

fn demo_annotated_hex_dumps() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5. Annotated Hex Dumps");
    println!("=======================");

    // Create a sample BACnet packet with annotations
    let packet_data = &[
        0x81, 0x0A, 0x00, 0x18, // BVLL header
        0x01, 0x00, 0x30, 0x00, // NPDU
        0x00, 0x05, 0x0C, // APDU header
        0x0C, 0x02, 0x00, 0x00, 0x01, // Object ID
        0x19, 0x55, // Property ID
        0x3E, 0x44, 0x42, 0x28, 0x00, 0x00, 0x3F, // Property value
    ];

    let annotations = vec![
        (0, "BVLL Type (0x81 = BACnet/IP)".to_string()),
        (
            1,
            "BVLL Function (0x0A = Original-Unicast-NPDU)".to_string(),
        ),
        (2, "BVLL Length (24 bytes)".to_string()),
        (4, "NPDU Version".to_string()),
        (5, "NPDU Control".to_string()),
        (8, "APDU Type (Confirmed Request)".to_string()),
        (9, "Max Segments & Max APDU".to_string()),
        (10, "Invoke ID & Service Choice".to_string()),
        (11, "Object ID Tag".to_string()),
        (16, "Property ID Tag".to_string()),
        (18, "Property Value Opening Tag".to_string()),
        (19, "Real Value Tag".to_string()),
        (24, "Property Value Closing Tag".to_string()),
    ];

    println!("Complete BACnet Packet with Annotations:");
    println!("{}", debug::annotated_hex_dump(packet_data, &annotations));

    Ok(())
}

fn demo_complete_packet_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n6. Complete Packet Analysis");
    println!("============================");

    // Simulate a real-world BACnet communication scenario
    println!("Scenario: Reading temperature from Analog Input 1 on Device 1234\n");

    // Request packet: Read Property (analog-input,1 present-value)
    let request_packet = &[
        0x81, 0x0A, 0x00, 0x11, // BVLL: BACnet/IP, Original-Unicast-NPDU, 17 bytes
        0x01, 0x00, 0x30, 0x00, // NPDU: Version 1, no routing
        0x00, 0x05, 0x0C, // APDU: Confirmed request, invoke ID 5, read property
        0x0C, 0x02, 0x00, 0x00, 0x01, // Object: analog-input,1
        0x19, 0x55, // Property: present-value (85)
    ];

    println!("REQUEST PACKET ANALYSIS:");
    println!("========================");
    analyze_complete_packet(request_packet);

    // Response packet: Complex ACK with temperature value
    let response_packet = &[
        0x81, 0x0A, 0x00, 0x18, // BVLL: BACnet/IP, Original-Unicast-NPDU, 24 bytes
        0x01, 0x00, 0x30, 0x00, // NPDU: Version 1, no routing
        0x30, 0x05, 0x0C, // APDU: Complex ACK, invoke ID 5, read property
        0x0C, 0x02, 0x00, 0x00, 0x01, // Object: analog-input,1
        0x19, 0x55, // Property: present-value
        0x3E, 0x44, 0x41, 0xB6, 0x66, 0x66, 0x3F, // Value: 22.8 (real)
    ];

    println!("\nRESPONSE PACKET ANALYSIS:");
    println!("=========================");
    analyze_complete_packet(response_packet);

    // Error packet: Object not found
    let error_packet = &[
        0x81, 0x0A, 0x00, 0x0B, // BVLL: BACnet/IP, Original-Unicast-NPDU, 11 bytes
        0x01, 0x00, 0x30, 0x00, // NPDU: Version 1, no routing
        0x50, 0x05, 0x0C, // APDU: Error, invoke ID 5, read property
        0x91, 0x01, // Error class: object (1)
        0x91, 0x02, // Error code: unknown object (2)
    ];

    println!("\nERROR PACKET ANALYSIS:");
    println!("======================");
    analyze_complete_packet(error_packet);

    Ok(())
}

fn analyze_complete_packet(packet: &[u8]) {
    println!("Raw Packet ({} bytes):", packet.len());
    println!("{}", bacnet_rs::util::hex_dump(packet, "  "));

    if packet.len() >= 4 {
        println!("{}", debug::format_bvll_structure(packet));

        if packet.len() > 4 {
            let npdu_data = &packet[4..];
            println!("{}", debug::format_npdu_structure(npdu_data));

            // Find APDU start (after NPDU header)
            let mut apdu_start = 2; // Skip version and control
            if npdu_data.len() > apdu_start {
                // Skip routing info if present
                let control = npdu_data[1];
                if (control & 0x20) != 0 {
                    // Destination present
                    apdu_start += 2; // Network number
                    if npdu_data.len() > apdu_start {
                        let addr_len = npdu_data[apdu_start] as usize;
                        apdu_start += 1 + addr_len;
                    }
                }
                if (control & 0x08) != 0 {
                    // Source present
                    apdu_start += 2; // Network number
                    if npdu_data.len() > apdu_start {
                        let addr_len = npdu_data[apdu_start] as usize;
                        apdu_start += 1 + addr_len;
                    }
                }
                if npdu_data.len() > apdu_start {
                    apdu_start += 1; // Hop count
                }

                if npdu_data.len() > apdu_start {
                    let apdu_data = &npdu_data[apdu_start..];
                    println!("{}", debug::format_apdu_structure(apdu_data));

                    // Analyze property values if present
                    analyze_property_data(apdu_data);
                }
            }
        }
    }

    println!("{}", "-".repeat(60));
}

fn analyze_property_data(apdu_data: &[u8]) {
    if apdu_data.is_empty() {
        return;
    }

    let pdu_type = (apdu_data[0] >> 4) & 0x0F;

    // Look for property values in Complex ACK responses
    if pdu_type == 3 && apdu_data.len() > 10 {
        println!("Property Data Analysis:");

        // Skip APDU header to find property values
        let mut pos = 3; // Skip PDU type, invoke ID, service choice

        // Skip object ID (typically 5 bytes: tag + 4 bytes object ID)
        if apdu_data.len() > pos + 5 && apdu_data[pos] == 0x0C {
            pos += 5;
        }

        // Skip property ID (typically 2-3 bytes)
        if apdu_data.len() > pos + 2 && apdu_data[pos] == 0x19 {
            pos += 2;
        }

        // Look for opening tag (property value)
        if apdu_data.len() > pos && apdu_data[pos] == 0x3E {
            pos += 1; // Skip opening tag

            // Find property value
            let mut value_end = pos;
            while value_end < apdu_data.len() && apdu_data[value_end] != 0x3F {
                value_end += 1;
            }

            if value_end > pos {
                let property_value = &apdu_data[pos..value_end];
                println!(
                    "  Property Value: {}",
                    debug::format_property_value(property_value)
                );
            }
        }
    }
}

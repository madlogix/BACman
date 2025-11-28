//! Standalone CRC Test Runner
//! Run with: rustc crc_test_runner.rs -o crc_test && ./crc_test

/// Calculate MS/TP header CRC-8 per ASHRAE 135 Annex G.1
fn calculate_header_crc(header: &[u8]) -> u8 {
    let mut crc = 0xFFu8;
    for &byte in header {
        let mut temp = (crc ^ byte) as u16;
        temp = temp
            ^ (temp << 1)
            ^ (temp << 2)
            ^ (temp << 3)
            ^ (temp << 4)
            ^ (temp << 5)
            ^ (temp << 6)
            ^ (temp << 7);
        crc = ((temp & 0xfe) ^ ((temp >> 8) & 1)) as u8;
    }
    !crc
}

/// Calculate header CRC register value (before ones complement)
fn calculate_header_crc_register(header: &[u8]) -> u8 {
    let mut crc = 0xFFu8;
    for &byte in header {
        let mut temp = (crc ^ byte) as u16;
        temp = temp
            ^ (temp << 1)
            ^ (temp << 2)
            ^ (temp << 3)
            ^ (temp << 4)
            ^ (temp << 5)
            ^ (temp << 6)
            ^ (temp << 7);
        crc = ((temp & 0xfe) ^ ((temp >> 8) & 1)) as u8;
    }
    crc
}

/// Calculate MS/TP data CRC-16 per ASHRAE 135 Annex G.2
fn calculate_data_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Calculate data CRC register value (before ones complement)
fn calculate_data_crc_register(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║      MS/TP CRC Test Suite - ASHRAE 135 Annex G Validation    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut passed = 0;
    let mut failed = 0;

    // =====================================================================
    // TEST 1.1: Header CRC-8 - Token Frame (ASHRAE 135 Annex G.1)
    // =====================================================================
    println!("━━━ TEST 1.1: Header CRC-8 (Token Frame) ━━━");
    let header = [0x00u8, 0x10, 0x05, 0x00, 0x00];
    let register = calculate_header_crc_register(&header);
    let crc = calculate_header_crc(&header);

    print!("  Register value after 5 bytes: 0x{:02X} ", register);
    if register == 0x73 {
        println!("✓ PASS (expected 0x73)");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0x73)");
        failed += 1;
    }

    print!("  Transmitted CRC (ones complement): 0x{:02X} ", crc);
    if crc == 0x8C {
        println!("✓ PASS (expected 0x8C)");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0x8C)");
        failed += 1;
    }

    // =====================================================================
    // TEST 1.1b: Header CRC-8 Receiver Validation
    // =====================================================================
    println!("\n━━━ TEST 1.1b: Header CRC Receiver Validation ━━━");
    let header_with_crc = [0x00u8, 0x10, 0x05, 0x00, 0x00, 0x8C];
    let receiver_check = calculate_header_crc_register(&header_with_crc);

    print!("  Receiver remainder: 0x{:02X} ", receiver_check);
    if receiver_check == 0x55 {
        println!("✓ PASS (expected 0x55 = valid frame)");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0x55)");
        failed += 1;
    }

    // =====================================================================
    // TEST 1.2: Data CRC-16 (ASHRAE 135 Annex G.2)
    // =====================================================================
    println!("\n━━━ TEST 1.2: Data CRC-16 (ASHRAE 135 Annex G.2 Vector) ━━━");
    let data = [0x01u8, 0x22, 0x30];

    // Test intermediate values step by step
    let mut crc = 0xFFFFu16;

    // After 0x01
    crc ^= 0x01u16;
    for _ in 0..8 {
        if crc & 0x0001 != 0 { crc = (crc >> 1) ^ 0x8408; } else { crc >>= 1; }
    }
    print!("  After 0x01: 0x{:04X} ", crc);
    if crc == 0x1E0E {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0x1E0E)");
        failed += 1;
    }

    // After 0x22
    crc ^= 0x22u16;
    for _ in 0..8 {
        if crc & 0x0001 != 0 { crc = (crc >> 1) ^ 0x8408; } else { crc >>= 1; }
    }
    print!("  After 0x22: 0x{:04X} ", crc);
    if crc == 0xEB70 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0xEB70)");
        failed += 1;
    }

    // After 0x30
    crc ^= 0x30u16;
    for _ in 0..8 {
        if crc & 0x0001 != 0 { crc = (crc >> 1) ^ 0x8408; } else { crc >>= 1; }
    }
    print!("  After 0x30: 0x{:04X} ", crc);
    if crc == 0x42EF {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0x42EF)");
        failed += 1;
    }

    // Final CRC
    let final_crc = calculate_data_crc(&data);
    print!("  Final CRC (ones complement): 0x{:04X} ", final_crc);
    if final_crc == 0xBD10 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0xBD10)");
        failed += 1;
    }

    println!("  Transmitted bytes (LSB first): [0x{:02X}, 0x{:02X}]",
        (final_crc & 0xFF) as u8, (final_crc >> 8) as u8);

    // =====================================================================
    // TEST 1.2b: Data CRC-16 Receiver Validation
    // =====================================================================
    println!("\n━━━ TEST 1.2b: Data CRC Receiver Validation ━━━");
    let data_with_crc = [0x01u8, 0x22, 0x30, 0x10, 0xBD];
    let receiver_check = calculate_data_crc_register(&data_with_crc);

    print!("  Receiver remainder: 0x{:04X} ", receiver_check);
    if receiver_check == 0xF0B8 {
        println!("✓ PASS (expected 0xF0B8 = valid frame)");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 0xF0B8)");
        failed += 1;
    }

    // =====================================================================
    // TEST 1.3: Frame Encoding - Token Frame
    // =====================================================================
    println!("\n━━━ TEST 1.3: Token Frame Encoding ━━━");
    let header = [0x00u8, 0x10, 0x05, 0x00, 0x00];
    let header_crc = calculate_header_crc(&header);
    let frame = [0x55, 0xFF, 0x00, 0x10, 0x05, 0x00, 0x00, header_crc];

    println!("  Frame bytes: {:02X?}", frame);
    print!("  Frame length: {} bytes ", frame.len());
    if frame.len() == 8 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 8)");
        failed += 1;
    }

    print!("  Preamble correct: ");
    if frame[0] == 0x55 && frame[1] == 0xFF {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 1.3b: Frame Encoding - Data Frame
    // =====================================================================
    println!("\n━━━ TEST 1.3b: Data Frame Encoding ━━━");
    let header = [0x06u8, 0x0A, 0x14, 0x00, 0x04]; // BACnetDataNotExpectingReply, len=4
    let data = [0x01u8, 0x02, 0x03, 0x04];
    let header_crc = calculate_header_crc(&header);
    let data_crc = calculate_data_crc(&data);

    let mut frame = vec![0x55, 0xFF];
    frame.extend_from_slice(&header);
    frame.push(header_crc);
    frame.extend_from_slice(&data);
    frame.push((data_crc & 0xFF) as u8);
    frame.push((data_crc >> 8) as u8);

    println!("  Frame bytes: {:02X?}", frame);
    print!("  Frame length: {} bytes ", frame.len());
    if frame.len() == 14 {
        println!("✓ PASS (2 preamble + 5 header + 1 hcrc + 4 data + 2 dcrc)");
        passed += 1;
    } else {
        println!("✗ FAIL (expected 14)");
        failed += 1;
    }

    // Verify header CRC
    let header_check = calculate_header_crc_register(&frame[2..8]);
    print!("  Header CRC valid: ");
    if header_check == 0x55 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (remainder 0x{:02X}, expected 0x55)", header_check);
        failed += 1;
    }

    // Verify data CRC
    let data_check = calculate_data_crc_register(&frame[8..14]);
    print!("  Data CRC valid: ");
    if data_check == 0xF0B8 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (remainder 0x{:04X}, expected 0xF0B8)", data_check);
        failed += 1;
    }

    // =====================================================================
    // TEST 1.4: Error Detection
    // =====================================================================
    println!("\n━━━ TEST 1.4: Error Detection ━━━");

    // Single bit error in header
    let mut corrupted = [0x00u8, 0x10, 0x05, 0x00, 0x00, 0x8C];
    corrupted[3] ^= 0x01;  // Flip bit in destination
    let check = calculate_header_crc_register(&corrupted);
    print!("  Header CRC detects 1-bit error: ");
    if check != 0x55 {
        println!("✓ PASS (remainder 0x{:02X} ≠ 0x55)", check);
        passed += 1;
    } else {
        println!("✗ FAIL (error not detected!)");
        failed += 1;
    }

    // Single bit error in data
    let mut corrupted_data = [0x01u8, 0x22, 0x30, 0x10, 0xBD];
    corrupted_data[1] ^= 0x01;
    let check = calculate_data_crc_register(&corrupted_data);
    print!("  Data CRC detects 1-bit error: ");
    if check != 0xF0B8 {
        println!("✓ PASS (remainder 0x{:04X} ≠ 0xF0B8)", check);
        passed += 1;
    } else {
        println!("✗ FAIL (error not detected!)");
        failed += 1;
    }

    // =====================================================================
    // SUMMARY
    // =====================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                        TEST SUMMARY                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Passed: {:2}                                                  ║", passed);
    println!("║  Failed: {:2}                                                  ║", failed);
    println!("║  Total:  {:2}                                                  ║", passed + failed);
    println!("╚══════════════════════════════════════════════════════════════╝");

    if failed == 0 {
        println!("\n🎉 ALL TESTS PASSED! CRC implementations match ASHRAE 135 spec.\n");
    } else {
        println!("\n❌ SOME TESTS FAILED! Review implementation.\n");
        std::process::exit(1);
    }
}

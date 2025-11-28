//! CRC Unit Tests - ASHRAE 135 Annex G Validation
//!
//! These tests validate our CRC implementations against the official
//! ASHRAE 135 standard test vectors.

/// Calculate MS/TP header CRC-8 per ASHRAE 135 Annex G.1
/// Uses polynomial X^8 + X^7 + 1 (parallel algorithm)
fn calculate_header_crc(header: &[u8]) -> u8 {
    let mut crc = 0xFFu8;

    for &byte in header {
        // XOR C7..C0 with D7..D0
        let mut temp = (crc ^ byte) as u16;

        // Exclusive OR the terms in the table (top down)
        // This implements the polynomial X^8 + X^7 + 1
        temp = temp
            ^ (temp << 1)
            ^ (temp << 2)
            ^ (temp << 3)
            ^ (temp << 4)
            ^ (temp << 5)
            ^ (temp << 6)
            ^ (temp << 7);

        // Combine bits shifted out left hand end
        crc = ((temp & 0xfe) ^ ((temp >> 8) & 1)) as u8;
    }

    !crc
}

/// Calculate MS/TP data CRC-16 per ASHRAE 135 Annex G.2
/// Uses CRC-CCITT polynomial: x^16 + x^12 + x^5 + 1 (reflected form: 0x8408)
fn calculate_data_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;

    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;  // CRC-CCITT reflected polynomial
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// Calculate header CRC and return the register value (before ones complement)
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

    crc  // Return register value, not ones complement
}

/// Calculate data CRC and return the register value (before ones complement)
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

    crc  // Return register value, not ones complement
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // PHASE 1.1: Header CRC-8 Tests (ASHRAE 135 Annex G.1)
    // =========================================================================

    #[test]
    fn test_header_crc_token_frame_ashrae_vector() {
        // From ASHRAE 135 Annex G.1 example:
        // Token frame from node 0x05 to node 0x10
        // Frame type = TOKEN = 0x00
        // Destination = 0x10
        // Source = 0x05
        // Length MSB = 0x00
        // Length LSB = 0x00
        // Expected CRC register after processing = 0x73
        // Expected transmitted CRC (ones complement) = 0x8C

        let header = [0x00u8, 0x10, 0x05, 0x00, 0x00];

        // Check register value before ones complement
        let register = calculate_header_crc_register(&header);
        assert_eq!(register, 0x73,
            "Header CRC register mismatch: expected 0x73, got 0x{:02X}", register);

        // Check final transmitted CRC (ones complement)
        let crc = calculate_header_crc(&header);
        assert_eq!(crc, 0x8C,
            "Header CRC mismatch: expected 0x8C, got 0x{:02X}", crc);

        println!("✓ Header CRC Token frame test PASSED");
        println!("  Input: {:02X?}", header);
        println!("  Register: 0x{:02X}", register);
        println!("  CRC (transmitted): 0x{:02X}", crc);
    }

    #[test]
    fn test_header_crc_receiver_validation() {
        // At the receiver, processing header + CRC should yield 0x55
        // This is the "valid frame" indicator per ASHRAE 135

        let header_with_crc = [0x00u8, 0x10, 0x05, 0x00, 0x00, 0x8C];
        let register = calculate_header_crc_register(&header_with_crc);

        assert_eq!(register, 0x55,
            "Receiver validation failed: expected 0x55, got 0x{:02X}", register);

        println!("✓ Header CRC receiver validation PASSED");
        println!("  Input (with CRC): {:02X?}", header_with_crc);
        println!("  Final register: 0x{:02X} (expected 0x55)", register);
    }

    #[test]
    fn test_header_crc_poll_for_master() {
        // PollForMaster frame: type=0x01, dest=0x7F, src=0x03, len=0x0000
        let header = [0x01u8, 0x7F, 0x03, 0x00, 0x00];
        let crc = calculate_header_crc(&header);

        // Verify receiver validation works
        let mut header_with_crc = header.to_vec();
        header_with_crc.push(crc);
        let receiver_check = calculate_header_crc_register(&header_with_crc);

        assert_eq!(receiver_check, 0x55,
            "PFM receiver validation failed: expected 0x55, got 0x{:02X}", receiver_check);

        println!("✓ Header CRC PollForMaster test PASSED");
        println!("  Header: {:02X?}", header);
        println!("  CRC: 0x{:02X}", crc);
    }

    #[test]
    fn test_header_crc_data_frame() {
        // BACnetDataNotExpectingReply: type=0x06, dest=0x0A, src=0x14, len=0x0004
        let header = [0x06u8, 0x0A, 0x14, 0x00, 0x04];
        let crc = calculate_header_crc(&header);

        // Verify receiver validation
        let mut header_with_crc = header.to_vec();
        header_with_crc.push(crc);
        let receiver_check = calculate_header_crc_register(&header_with_crc);

        assert_eq!(receiver_check, 0x55,
            "Data frame receiver validation failed: expected 0x55, got 0x{:02X}", receiver_check);

        println!("✓ Header CRC Data frame test PASSED");
        println!("  Header: {:02X?}", header);
        println!("  CRC: 0x{:02X}", crc);
    }

    // =========================================================================
    // PHASE 1.2: Data CRC-16 Tests (ASHRAE 135 Annex G.2)
    // =========================================================================

    #[test]
    fn test_data_crc_ashrae_vector() {
        // From ASHRAE 135 Annex G.2 example:
        // Data sequence: 0x01, 0x22, 0x30
        // After 0x01: CRC register = 0x1E0E
        // After 0x22: CRC register = 0xEB70
        // After 0x30: CRC register = 0x42EF
        // Ones complement = 0xBD10
        // Transmitted as: 0x10, 0xBD (LSB first)

        let data = [0x01u8, 0x22, 0x30];

        // Test intermediate values
        let mut crc = 0xFFFFu16;

        // After first byte (0x01)
        crc ^= 0x01u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
        assert_eq!(crc, 0x1E0E,
            "After 0x01: expected 0x1E0E, got 0x{:04X}", crc);

        // After second byte (0x22)
        crc ^= 0x22u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
        assert_eq!(crc, 0xEB70,
            "After 0x22: expected 0xEB70, got 0x{:04X}", crc);

        // After third byte (0x30)
        crc ^= 0x30u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
        assert_eq!(crc, 0x42EF,
            "After 0x30: expected 0x42EF, got 0x{:04X}", crc);

        // Check final CRC (ones complement)
        let final_crc = calculate_data_crc(&data);
        assert_eq!(final_crc, 0xBD10,
            "Final CRC mismatch: expected 0xBD10, got 0x{:04X}", final_crc);

        println!("✓ Data CRC ASHRAE vector test PASSED");
        println!("  Data: {:02X?}", data);
        println!("  Intermediate values: 0x1E0E → 0xEB70 → 0x42EF");
        println!("  Final CRC: 0x{:04X}", final_crc);
        println!("  Transmitted (LSB first): [0x{:02X}, 0x{:02X}]",
            (final_crc & 0xFF) as u8, (final_crc >> 8) as u8);
    }

    #[test]
    fn test_data_crc_receiver_validation() {
        // At the receiver, processing data + CRC should yield 0xF0B8
        // This is the "valid frame" indicator per ASHRAE 135 Annex G.2

        // Data with CRC appended (LSB first): 0x01, 0x22, 0x30, 0x10, 0xBD
        let data_with_crc = [0x01u8, 0x22, 0x30, 0x10, 0xBD];
        let register = calculate_data_crc_register(&data_with_crc);

        assert_eq!(register, 0xF0B8,
            "Receiver validation failed: expected 0xF0B8, got 0x{:04X}", register);

        println!("✓ Data CRC receiver validation PASSED");
        println!("  Input (with CRC): {:02X?}", data_with_crc);
        println!("  Final register: 0x{:04X} (expected 0xF0B8)", register);
    }

    #[test]
    fn test_data_crc_empty() {
        // Empty data should return 0x0000 (0xFFFF ones-complemented is 0x0000)
        // Wait, that's wrong. Let me check...
        // Actually empty data means CRC register stays at 0xFFFF
        // Ones complement of 0xFFFF is 0x0000
        let data: [u8; 0] = [];
        let crc = calculate_data_crc(&data);
        assert_eq!(crc, 0x0000,
            "Empty data CRC mismatch: expected 0x0000, got 0x{:04X}", crc);

        println!("✓ Data CRC empty test PASSED");
    }

    #[test]
    fn test_data_crc_single_byte() {
        // Test with single byte to verify algorithm
        let data = [0x00u8];
        let crc = calculate_data_crc(&data);

        // Verify receiver validation
        let mut data_with_crc = data.to_vec();
        data_with_crc.push((crc & 0xFF) as u8);
        data_with_crc.push((crc >> 8) as u8);
        let receiver_check = calculate_data_crc_register(&data_with_crc);

        assert_eq!(receiver_check, 0xF0B8,
            "Single byte receiver validation failed: expected 0xF0B8, got 0x{:04X}", receiver_check);

        println!("✓ Data CRC single byte test PASSED");
        println!("  Data: {:02X?}", data);
        println!("  CRC: 0x{:04X}", crc);
    }

    // =========================================================================
    // PHASE 1.3: Frame Encoding Tests
    // =========================================================================

    #[test]
    fn test_token_frame_encoding() {
        // Token frame: [preamble1, preamble2, type, dest, src, len_hi, len_lo, crc]
        // Preambles: 0x55, 0xFF
        // Type: 0x00 (Token)
        // No data, no data CRC

        let frame_type = 0x00u8;  // Token
        let dest = 0x10u8;
        let src = 0x05u8;
        let len_hi = 0x00u8;
        let len_lo = 0x00u8;

        let header = [frame_type, dest, src, len_hi, len_lo];
        let header_crc = calculate_header_crc(&header);

        let frame = vec![0x55, 0xFF, frame_type, dest, src, len_hi, len_lo, header_crc];

        assert_eq!(frame.len(), 8, "Token frame should be 8 bytes");
        assert_eq!(frame[0], 0x55, "Preamble 1 should be 0x55");
        assert_eq!(frame[1], 0xFF, "Preamble 2 should be 0xFF");
        assert_eq!(frame[7], 0x8C, "Header CRC should be 0x8C");

        println!("✓ Token frame encoding test PASSED");
        println!("  Frame: {:02X?}", frame);
    }

    #[test]
    fn test_poll_for_master_frame_encoding() {
        // PollForMaster: type=0x01, no data
        let frame_type = 0x01u8;
        let dest = 0x7Fu8;
        let src = 0x03u8;

        let header = [frame_type, dest, src, 0x00, 0x00];
        let header_crc = calculate_header_crc(&header);

        let frame = vec![0x55, 0xFF, frame_type, dest, src, 0x00, 0x00, header_crc];

        assert_eq!(frame.len(), 8, "PFM frame should be 8 bytes");

        // Verify CRC is valid
        let receiver_check = calculate_header_crc_register(&frame[2..8]);
        assert_eq!(receiver_check, 0x55, "PFM frame CRC validation failed");

        println!("✓ PollForMaster frame encoding test PASSED");
        println!("  Frame: {:02X?}", frame);
    }

    #[test]
    fn test_data_frame_encoding() {
        // BACnetDataNotExpectingReply with 4 bytes of data
        let frame_type = 0x06u8;  // BACnetDataNotExpectingReply
        let dest = 0x0Au8;
        let src = 0x14u8;
        let data = [0x01u8, 0x02, 0x03, 0x04];
        let len = data.len() as u16;

        let header = [frame_type, dest, src, (len >> 8) as u8, (len & 0xFF) as u8];
        let header_crc = calculate_header_crc(&header);
        let data_crc = calculate_data_crc(&data);

        let mut frame = vec![0x55, 0xFF];
        frame.extend_from_slice(&header);
        frame.push(header_crc);
        frame.extend_from_slice(&data);
        frame.push((data_crc & 0xFF) as u8);
        frame.push((data_crc >> 8) as u8);

        // Frame structure: 2 preamble + 5 header + 1 hcrc + 4 data + 2 dcrc = 14 bytes
        assert_eq!(frame.len(), 14, "Data frame should be 14 bytes");

        // Verify header CRC
        let header_check = calculate_header_crc_register(&frame[2..8]);
        assert_eq!(header_check, 0x55, "Data frame header CRC validation failed");

        // Verify data CRC
        let data_check = calculate_data_crc_register(&frame[8..14]);
        assert_eq!(data_check, 0xF0B8, "Data frame data CRC validation failed");

        println!("✓ Data frame encoding test PASSED");
        println!("  Frame: {:02X?}", frame);
        println!("  Header CRC: 0x{:02X}", header_crc);
        println!("  Data CRC: 0x{:04X} (bytes: [{:02X}, {:02X}])",
            data_crc, (data_crc & 0xFF) as u8, (data_crc >> 8) as u8);
    }

    // =========================================================================
    // PHASE 1.4: Preamble Recognition Tests
    // =========================================================================

    #[test]
    fn test_preamble_detection() {
        // Valid preamble sequence
        let valid_preamble = [0x55u8, 0xFF];
        assert_eq!(valid_preamble[0], 0x55);
        assert_eq!(valid_preamble[1], 0xFF);

        // Multiple 0x55 before 0xFF is valid (sync bytes)
        let extended_preamble = [0x55u8, 0x55, 0x55, 0xFF];
        let ff_pos = extended_preamble.iter().position(|&b| b == 0xFF);
        assert_eq!(ff_pos, Some(3), "Should find 0xFF at position 3");

        println!("✓ Preamble detection tests PASSED");
    }

    #[test]
    fn test_frame_parsing() {
        // Complete Token frame
        let frame = [0x55u8, 0xFF, 0x00, 0x10, 0x05, 0x00, 0x00, 0x8C];

        // Find preamble
        assert_eq!(frame[0], 0x55, "First byte should be 0x55");
        assert_eq!(frame[1], 0xFF, "Second byte should be 0xFF");

        // Extract header fields
        let frame_type = frame[2];
        let dest = frame[3];
        let src = frame[4];
        let len = ((frame[5] as u16) << 8) | (frame[6] as u16);
        let header_crc = frame[7];

        assert_eq!(frame_type, 0x00, "Frame type should be Token (0x00)");
        assert_eq!(dest, 0x10, "Destination should be 0x10");
        assert_eq!(src, 0x05, "Source should be 0x05");
        assert_eq!(len, 0, "Length should be 0");
        assert_eq!(header_crc, 0x8C, "Header CRC should be 0x8C");

        // Validate CRC
        let crc_check = calculate_header_crc_register(&frame[2..8]);
        assert_eq!(crc_check, 0x55, "CRC validation should yield 0x55");

        println!("✓ Frame parsing test PASSED");
        println!("  Frame type: 0x{:02X} (Token)", frame_type);
        println!("  Dest: 0x{:02X}, Src: 0x{:02X}", dest, src);
        println!("  Length: {}", len);
        println!("  Header CRC: 0x{:02X} (valid)", header_crc);
    }

    // =========================================================================
    // Error Detection Tests
    // =========================================================================

    #[test]
    fn test_header_crc_detects_single_bit_error() {
        let frame = [0x55u8, 0xFF, 0x00, 0x10, 0x05, 0x00, 0x00, 0x8C];

        // Corrupt one bit in destination
        let mut corrupted = frame.clone();
        corrupted[3] ^= 0x01;  // Flip LSB of destination

        let crc_check = calculate_header_crc_register(&corrupted[2..8]);
        assert_ne!(crc_check, 0x55, "CRC should detect single bit error");

        println!("✓ Single bit error detection test PASSED");
    }

    #[test]
    fn test_data_crc_detects_single_bit_error() {
        let data = [0x01u8, 0x22, 0x30];
        let crc = calculate_data_crc(&data);

        let mut data_with_crc = data.to_vec();
        data_with_crc.push((crc & 0xFF) as u8);
        data_with_crc.push((crc >> 8) as u8);

        // Corrupt one bit
        data_with_crc[1] ^= 0x01;

        let crc_check = calculate_data_crc_register(&data_with_crc);
        assert_ne!(crc_check, 0xF0B8, "Data CRC should detect single bit error");

        println!("✓ Data CRC single bit error detection test PASSED");
    }
}

// Run tests with: cargo test --lib crc_tests

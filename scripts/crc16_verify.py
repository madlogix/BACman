#!/usr/bin/env python3
"""Verify MS/TP CRC-16 calculation against known good frames."""

def calculate_data_crc_ccitt(data: bytes) -> int:
    """
    Calculate MS/TP data CRC-16 per ASHRAE 135 Annex G.2
    Uses CRC-CCITT polynomial: x^16 + x^12 + x^5 + 1 (reflected form: 0x8408)
    """
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return (~crc) & 0xFFFF


def calculate_data_crc_modbus(data: bytes) -> int:
    """
    Calculate CRC-16 MODBUS (polynomial 0xA001) for comparison.
    NOT the correct algorithm for MS/TP, but including for comparison.
    """
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc


def check_crc_ccitt_with_crc(data_with_crc: bytes) -> int:
    """
    When CRC-16 CCITT is calculated over data INCLUDING the transmitted CRC,
    the result should be a constant "good CRC" value.
    Per ASHRAE 135 Annex G: good_crc = 0xF0B8
    """
    crc = 0xFFFF
    for byte in data_with_crc:
        crc ^= byte
        for _ in range(8):
            if crc & 0x0001:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return crc  # Should be 0xF0B8 for valid frame (NOT inverted)


# Test with known good Who-Is frame from BACrouter (from capture)
# Frame: 55 ff 06 ff 02 00 11 d8 01 28 ff ff 00 27 11 06 c0 a8 8e 02 ba c0 0d 10 08 [CRC_LOW] [CRC_HIGH]
# - 55 ff = preamble
# - 06 = frame type (BacnetDataNotExpectingReply)
# - ff = dest (broadcast)
# - 02 = source (MAC 2)
# - 00 11 = length (17 bytes big-endian)
# - d8 = header CRC
# Data starts at byte 8: 01 28 ff ff 00 27 11 06 c0 a8 8e 02 ba c0 0d 10 08
# That's 17 bytes of data, then 2 bytes of CRC

# Data from first Who-Is frame (from hexdump)
# The data portion (17 bytes):
data = bytes([0x01, 0x28, 0xff, 0xff, 0x00, 0x27, 0x11, 0x06,
              0xc0, 0xa8, 0x8e, 0x02, 0xba, 0xc0, 0x0d, 0x10, 0x08])

print("MS/TP CRC-16 Verification")
print("=" * 50)
print(f"Data ({len(data)} bytes): {data.hex()}")

# Calculate CRC using both algorithms
crc_ccitt = calculate_data_crc_ccitt(data)
crc_modbus = calculate_data_crc_modbus(data)

print(f"\nCRC-CCITT (0x8408, ~final): 0x{crc_ccitt:04X}")
print(f"  - Low byte first: 0x{crc_ccitt & 0xFF:02X} 0x{(crc_ccitt >> 8) & 0xFF:02X}")
print(f"CRC-16 MODBUS (0xA001):    0x{crc_modbus:04X}")

# Now let's try to figure out what CRC bytes would be in the frame
# We need to extract the actual CRC from the capture

# From the hexdump, after the data there should be 2 CRC bytes
# Let me look at different possible CRC byte orderings

print("\n" + "=" * 50)
print("Testing what the frame CRC should be:")

# If the captured frame was valid, then:
# - CRC bytes are stored low-byte-first in the frame
# - So received_crc = (high_byte << 8) | low_byte when reading as (crc_low, crc_high)

# Looking at the hexdump more closely:
# 00003680: 1b00 0000 1b00 0000 55ff 06ff 0200 11d8  ........U.......
# 00003690: 0128 ffff 0027 1106 c0a8 8e02 bac0 0d10  .(...'..........
# 000036a0: 08a9 d00e a628 6908 5d01 0008 0000 0008  .....(i.].......

# The frame data continues at 0x3690: 01 28 ff ff 00 27 11 06 c0 a8 8e 02 ba c0 0d 10
# Then at 0x36a0: 08 a9 d0 ...
# So data ends at 08, and CRC should be: a9 d0

# Let's verify this
crc_from_frame_low = 0xa9
crc_from_frame_high = 0xd0
received_crc = (crc_from_frame_high << 8) | crc_from_frame_low
print(f"\nFrom capture file:")
print(f"  CRC bytes: low=0x{crc_from_frame_low:02X}, high=0x{crc_from_frame_high:02X}")
print(f"  Combined as (high<<8)|low: 0x{received_crc:04X}")
print(f"  Calculated CRC-CCITT:      0x{crc_ccitt:04X}")

if received_crc == crc_ccitt:
    print("  >> CRC MATCHES! Algorithm is correct.")
else:
    print("  >> CRC MISMATCH!")
    # Try other interpretations
    alt_received = (crc_from_frame_low << 8) | crc_from_frame_high
    print(f"  Alternative (low<<8)|high: 0x{alt_received:04X}")
    if alt_received == crc_ccitt:
        print("  >> Alternative byte order MATCHES!")

# Also check the "good CRC" constant approach
print("\n" + "=" * 50)
print("Checking 'good CRC' constant (0xF0B8 per ASHRAE 135):")
data_with_crc = data + bytes([crc_from_frame_low, crc_from_frame_high])
check_result = check_crc_ccitt_with_crc(data_with_crc)
print(f"  CRC over data+CRC: 0x{check_result:04X}")
if check_result == 0xF0B8:
    print("  >> GOOD CRC constant matches! Frame is valid.")
else:
    # Try swapped byte order
    data_with_crc_swap = data + bytes([crc_from_frame_high, crc_from_frame_low])
    check_result_swap = check_crc_ccitt_with_crc(data_with_crc_swap)
    print(f"  CRC over data+CRC (swapped): 0x{check_result_swap:04X}")
    if check_result_swap == 0xF0B8:
        print("  >> GOOD CRC with swapped bytes matches!")

# Compare to what the Rust code does
print("\n" + "=" * 50)
print("Rust code comparison:")
print(f"  Rust calculate_data_crc returns: 0x{crc_ccitt:04X}")
print(f"  Rust receives CRC as (high<<8)|low: 0x{received_crc:04X}")
print(f"  These are {'EQUAL' if crc_ccitt == received_crc else 'NOT EQUAL'}")

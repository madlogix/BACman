#!/usr/bin/env python3
"""Test CRC against REAL frames from the bus"""

# Real frames from UART_RX:
# [55, FF, 01, 03, 06, 00, 00, B1] - Poll-For-Master dest=3 src=6
# [55, FF, 01, 7C, 06, 00, 00, F2] - Poll-For-Master dest=124 src=6
# [55, FF, 01, 00, 06, 00, 00, 29] - Poll-For-Master dest=0 src=6

def crc8_ashrae(data):
    """Standard ASHRAE 135 CRC-8 with poly 0x8C"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return (~crc) & 0xFF

real_frames = [
    (bytes([0x01, 0x03, 0x06, 0x00, 0x00]), 0xB1, "Poll dest=3 src=6"),
    (bytes([0x01, 0x7C, 0x06, 0x00, 0x00]), 0xF2, "Poll dest=124 src=6"),
    (bytes([0x01, 0x00, 0x06, 0x00, 0x00]), 0x29, "Poll dest=0 src=6"),
    (bytes([0x01, 0x04, 0x06, 0x00, 0x00]), 0x0B, "Poll dest=4 src=6"),
    (bytes([0x01, 0x05, 0x06, 0x00, 0x00]), 0x82, "Poll dest=5 src=6"),
    (bytes([0x01, 0x07, 0x06, 0x00, 0x00]), 0x93, "Poll dest=7 src=6"),
    (bytes([0x01, 0x08, 0x06, 0x00, 0x00]), 0x6D, "Poll dest=8 src=6"),
    (bytes([0x01, 0x0B, 0x06, 0x00, 0x00]), 0xF5, "Poll dest=11 src=6"),
]

print("Testing CRC-8 against REAL bus frames:")
print("=" * 60)

all_match = True
for header, bus_crc, desc in real_frames:
    calc_crc = crc8_ashrae(header)
    match = "✓" if calc_crc == bus_crc else "✗"
    if calc_crc != bus_crc:
        all_match = False
    print(f"{desc}: header={header.hex()}")
    print(f"  Bus CRC: 0x{bus_crc:02X}, Calculated: 0x{calc_crc:02X} {match}")

print()
if all_match:
    print("✓ ALL CRCs MATCH! The standard algorithm is correct!")
else:
    print("✗ Some CRCs don't match - need to investigate further")

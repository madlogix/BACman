#!/usr/bin/env python3
"""Test the ORIGINAL parallel CRC algorithm from the Rust code"""

def crc8_parallel(data):
    """Original parallel algorithm from mstp_driver.rs"""
    crc = 0xFF
    for byte in data:
        # XOR C7..C0 with D7..D0
        temp = (crc ^ byte) & 0xFF
        temp16 = temp
        
        # Exclusive OR the terms in the table (top down)
        temp16 = (temp16 
            ^ (temp16 << 1)
            ^ (temp16 << 2)
            ^ (temp16 << 3)
            ^ (temp16 << 4)
            ^ (temp16 << 5)
            ^ (temp16 << 6)
            ^ (temp16 << 7))
        
        # Combine bits shifted out left hand end
        crc = ((temp16 & 0xfe) ^ ((temp16 >> 8) & 1)) & 0xFF
    
    return (~crc) & 0xFF

# Real frames from the bus
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

print("Testing ORIGINAL parallel CRC algorithm:")
print("=" * 60)

all_match = True
for header, bus_crc, desc in real_frames:
    calc_crc = crc8_parallel(header)
    match = "✓ MATCH!" if calc_crc == bus_crc else ""
    if calc_crc != bus_crc:
        all_match = False
    print(f"{desc}: header={header.hex()}")
    print(f"  Bus CRC: 0x{bus_crc:02X}, Parallel: 0x{calc_crc:02X} {match}")

print()
if all_match:
    print("✓ ORIGINAL PARALLEL ALGORITHM MATCHES!")
    print("The original code WAS correct! We should NOT have changed it!")
else:
    print("✗ Parallel algorithm also doesn't match")

#!/usr/bin/env python3
"""Test CRC implementations against known BACnet MS/TP test vectors"""

def crc8_header_python(data):
    """Calculate 8-bit CRC for MS/TP header - Python implementation from mstp_test_sender.py"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return (~crc) & 0xFF

def crc8_header_rust_style(data):
    """Simulate the Rust implementation from mstp_driver.rs"""
    crc = 0xFF
    for byte in data:
        # XOR C7..C0 with D7..D0
        temp = (crc ^ byte) & 0xFF
        temp16 = temp
        
        # Exclusive OR the terms in the table (top down)
        temp16 = temp16 ^ (temp16 << 1) ^ (temp16 << 2) ^ (temp16 << 3) ^ (temp16 << 4) ^ (temp16 << 5) ^ (temp16 << 6) ^ (temp16 << 7)
        
        # Combine bits shifted out left hand end
        crc = ((temp16 & 0xfe) ^ ((temp16 >> 8) & 1)) & 0xFF
    
    return (~crc) & 0xFF

def crc16_data_python(data):
    """Calculate 16-bit CRC for MS/TP data - CRC-CCITT (0x8408)"""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408
            else:
                crc >>= 1
    return (~crc) & 0xFFFF

# Test vectors for MS/TP
# Token frame: type=0, dest=6, src=3, len=0
# Header bytes: [type, dest, src, len_hi, len_lo] = [0x00, 0x06, 0x03, 0x00, 0x00]
token_header = bytes([0x00, 0x06, 0x03, 0x00, 0x00])

# Poll for Master: type=1, dest=6, src=3
poll_header = bytes([0x01, 0x06, 0x03, 0x00, 0x00])

# Reply to Poll for Master: type=2, dest=6, src=3
reply_header = bytes([0x02, 0x03, 0x06, 0x00, 0x00])

print("=" * 60)
print("MS/TP Header CRC-8 Test")
print("=" * 60)

test_headers = [
    ("Token (3->6)", token_header),
    ("Poll-For-Master (3->6)", poll_header),
    ("Reply-To-Poll (6->3)", reply_header),
]

all_match = True
for name, header in test_headers:
    py_crc = crc8_header_python(header)
    rust_crc = crc8_header_rust_style(header)
    match = "✓" if py_crc == rust_crc else "✗ MISMATCH!"
    if py_crc != rust_crc:
        all_match = False
    print(f"{name}:")
    print(f"  Header: {header.hex()}")
    print(f"  Python CRC:    0x{py_crc:02X}")
    print(f"  Rust-style:    0x{rust_crc:02X}  {match}")
    print()

# Full token frame: 55 FF 00 06 03 00 00 CRC
print("=" * 60)
print("Full Token Frame (3 -> 6):")
crc = crc8_header_python(token_header)
full_frame = bytes([0x55, 0xFF]) + token_header + bytes([crc])
print(f"  Frame: {full_frame.hex()}")
print(f"  Expected: 55 ff 00 06 03 00 00 {crc:02x}")
print()

# Test data CRC with some sample NPDU
sample_npdu = bytes([0x01, 0x24, 0xFD, 0xE9, 0x01, 0x06, 0xFF])
print("=" * 60)
print("Data CRC-16 Test")
print(f"  NPDU: {sample_npdu.hex()}")
crc16 = crc16_data_python(sample_npdu)
print(f"  CRC-16: 0x{crc16:04X}")
print(f"  Low byte: 0x{crc16 & 0xFF:02X}, High byte: 0x{(crc16 >> 8) & 0xFF:02X}")
print()

if all_match:
    print("✓ All CRC-8 calculations match!")
else:
    print("✗ CRC-8 MISMATCH DETECTED - This is the problem!")

# Verify against the CRCs we're seeing from the bus:
# Token (type=0, dest=2, src=6) - got CRC 0xBE from bus, we calculated 0xE0
# Token (type=0, dest=6, src=2) - got CRC 0xFA from bus, we calculated 0x70
print("=" * 60)
print("Verifying CRCs observed on the bus:")
print("=" * 60)

headers_from_bus = [
    ("Token (6->2)", bytes([0x00, 0x02, 0x06, 0x00, 0x00]), 0xBE),  # What we saw on bus
    ("Token (2->6)", bytes([0x00, 0x06, 0x02, 0x00, 0x00]), 0xFA),  # What we saw on bus
]

for name, header, bus_crc in headers_from_bus:
    py_crc = crc8_header_python(header)
    print(f"{name}:")
    print(f"  Header bytes: {header.hex()}")
    print(f"  Bus CRC:     0x{bus_crc:02X}")
    print(f"  Python CRC:  0x{py_crc:02X}")
    if py_crc == bus_crc:
        print(f"  -> MATCH! Python agrees with bus")
    else:
        print(f"  -> MISMATCH! Bus value differs from Python")
    print()

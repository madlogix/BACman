#!/usr/bin/env python3
"""Test different CRC-8 variants to find what matches the bus"""

# From bus:
# Token (6->2): header=[0x00, 0x02, 0x06, 0x00, 0x00], CRC=0xBE
# Token (2->6): header=[0x00, 0x06, 0x02, 0x00, 0x00], CRC=0xFA

test_headers = [
    (bytes([0x00, 0x02, 0x06, 0x00, 0x00]), 0xBE, "Token 6->2"),
    (bytes([0x00, 0x06, 0x02, 0x00, 0x00]), 0xFA, "Token 2->6"),
]

def crc8_variant1(data):
    """Standard reflected CRC-8 with poly 0x8C, init 0xFF, xor out 0xFF"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return (~crc) & 0xFF

def crc8_variant2(data):
    """Without final inversion"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return crc & 0xFF  # No inversion

def crc8_variant3(data):
    """Init 0x00, no xor out"""
    crc = 0x00
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return crc & 0xFF

def crc8_variant4(data):
    """MSB-first processing (non-reflected)"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x80:
                crc = ((crc << 1) ^ 0x81) & 0xFF
            else:
                crc = (crc << 1) & 0xFF
    return (~crc) & 0xFF

def crc8_variant5(data):
    """ASHRAE 135 Annex G.1 exact algorithm from spec"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        # Process 8 bits
        for _ in range(8):
            if crc & 0x01:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return crc ^ 0xFF  # One's complement

def crc8_variant6(data):
    """Using polynomial 0xE0 (another common representation)"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xE0
            else:
                crc >>= 1
    return (~crc) & 0xFF

variants = [
    ("Var1: poly=0x8C, init=FF, xor=FF", crc8_variant1),
    ("Var2: poly=0x8C, init=FF, xor=00", crc8_variant2),
    ("Var3: poly=0x8C, init=00, xor=00", crc8_variant3),
    ("Var4: MSB-first poly=0x81", crc8_variant4),
    ("Var5: ASHRAE exact (same as V1)", crc8_variant5),
    ("Var6: poly=0xE0, init=FF, xor=FF", crc8_variant6),
]

print("Testing CRC-8 variants against known bus values:")
print("=" * 70)

for name, header, expected, desc in [(v[2], v[0], v[1], v[2]) for v in test_headers]:
    print(f"\n{desc}: header={header.hex()}, expected=0x{expected:02X}")
    for vname, vfunc in variants:
        result = vfunc(header)
        match = "✓ MATCH" if result == expected else ""
        print(f"  {vname}: 0x{result:02X} {match}")

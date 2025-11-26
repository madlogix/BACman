#!/usr/bin/env python3
"""MS/TP CRC Verification Tool - Captures frames and verifies standard BACnet CRC"""
import serial
import sys

def calc_standard_crc(data):
    """Standard BACnet MS/TP CRC-8 per ASHRAE 135 Annex G.1
    Uses polynomial X^8 + X^7 + 1 (same as Wireshark implementation)"""
    crc = 0xFF
    for byte in data:
        # XOR C7..C0 with D7..D0
        temp = crc ^ byte
        # Exclusive OR the terms in the table (top down)
        temp = temp ^ (temp << 1) ^ (temp << 2) ^ (temp << 3) ^ (temp << 4) ^ (temp << 5) ^ (temp << 6) ^ (temp << 7)
        # Combine bits shifted out left hand end
        crc = (temp & 0xfe) ^ ((temp >> 8) & 1)
    return (~crc) & 0xFF

def calc_empirical_crc(data):
    """Empirical CRC that was matching JCI traffic"""
    crc = 0xFF
    for byte in data:
        reflected_byte = int('{:08b}'.format(byte)[::-1], 2)  # reverse bits
        crc ^= reflected_byte
        for _ in range(8):
            if crc & 0x80:
                crc = ((crc << 1) ^ 0x81) & 0xFF
            else:
                crc = (crc << 1) & 0xFF
    return int('{:08b}'.format(crc)[::-1], 2) ^ 0xFF  # reverse bits and XOR

FRAME_TYPES = {
    0: "Token",
    1: "PollForMaster",
    2: "ReplyToPoll",
    3: "TestRequest",
    4: "TestResponse",
    5: "DataExpectReply",
    6: "DataNoReply",
    7: "ReplyPostponed"
}

port = sys.argv[1] if len(sys.argv) > 1 else '/dev/ttyACM0'
baud = int(sys.argv[2]) if len(sys.argv) > 2 else 38400

try:
    ser = serial.Serial(port, baud, timeout=1)
except Exception as e:
    print(f"Error opening {port}: {e}")
    sys.exit(1)

print(f"MS/TP CRC Verification Tool")
print(f"Listening on {port} at {baud} baud...")
print(f"Press Ctrl+C to stop\n")
print(f"{'Frame':<6} {'Type':<15} {'Dst':<4} {'Src':<4} {'Len':<5} {'RX CRC':<8} {'Std CRC':<8} {'Emp CRC':<8} {'Result':<10} {'Rate'}")
print("-" * 100)

buffer = bytearray()
frame_count = 0
std_match = 0
emp_match = 0

try:
    while True:
        data = ser.read(256)
        if data:
            buffer.extend(data)

        while len(buffer) >= 8:
            try:
                idx = buffer.index(0x55)
                if idx > 0:
                    buffer = buffer[idx:]
                if len(buffer) < 2 or buffer[1] != 0xFF:
                    buffer = buffer[1:]
                    continue

                if len(buffer) < 8:
                    break

                frame_type = buffer[2]
                dest = buffer[3]
                src = buffer[4]
                length = (buffer[5] << 8) | buffer[6]
                rx_crc = buffer[7]

                header = bytes(buffer[2:7])
                std_crc = calc_standard_crc(header)
                emp_crc = calc_empirical_crc(header)

                frame_count += 1

                if rx_crc == std_crc:
                    std_match += 1
                    result = "STD-OK"
                elif rx_crc == emp_crc:
                    emp_match += 1
                    result = "EMP-OK"
                else:
                    result = "FAIL"

                ftype_name = FRAME_TYPES.get(frame_type, f"Unknown({frame_type})")
                rate = f"{100*std_match/frame_count:.1f}% std"

                print(f"{frame_count:<6} {ftype_name:<15} {dest:<4} {src:<4} {length:<5} 0x{rx_crc:02X}     0x{std_crc:02X}     0x{emp_crc:02X}     {result:<10} {rate}")

                frame_size = 8 + (length + 2 if length > 0 else 0)
                if len(buffer) >= frame_size:
                    buffer = buffer[frame_size:]
                else:
                    break

            except ValueError:
                buffer.clear()
                break

except KeyboardInterrupt:
    print(f"\n" + "=" * 100)
    print(f"SUMMARY:")
    print(f"  Total frames: {frame_count}")
    print(f"  Standard CRC matches: {std_match} ({100*std_match/frame_count:.1f}%)" if frame_count else "")
    print(f"  Empirical CRC matches: {emp_match} ({100*emp_match/frame_count:.1f}%)" if frame_count else "")
    print(f"  Failed: {frame_count - std_match - emp_match}")
finally:
    ser.close()

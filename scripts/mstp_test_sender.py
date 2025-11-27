#!/usr/bin/env python3
"""
Simple MS/TP frame sender for testing RS-485 connectivity.
Sends Poll-For-Master frames and listens for responses.
"""

import sys
import serial
import time
import struct

def crc8_header(data):
    """Calculate 8-bit CRC for MS/TP header."""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return (~crc) & 0xFF

def crc16_data(data):
    """Calculate 16-bit CRC for MS/TP data."""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408  # CRC-CCITT (NOT 0xA001 which is MODBUS)
            else:
                crc >>= 1
    return (~crc) & 0xFFFF

def build_mstp_frame(frame_type, dest_addr, src_addr, data=None):
    """Build an MS/TP frame."""
    # Frame types:
    # 0x00 = Token
    # 0x01 = Poll-For-Master
    # 0x02 = Reply-To-Poll-For-Master
    # 0x03 = Test Request
    # 0x04 = Test Response
    # 0x05 = BACnet Data Expecting Reply
    # 0x06 = BACnet Data Not Expecting Reply
    # 0x07 = Reply Postponed

    preamble = bytes([0x55, 0xFF])  # MS/TP preamble per ASHRAE 135

    if data is None:
        data_len = 0
        header = bytes([frame_type, dest_addr, src_addr, 0x00, 0x00])
        header_crc = crc8_header(header)
        return preamble + header + bytes([header_crc])
    else:
        data_len = len(data)
        header = bytes([frame_type, dest_addr, src_addr, (data_len >> 8) & 0xFF, data_len & 0xFF])
        header_crc = crc8_header(header)
        data_crc = crc16_data(data)
        return preamble + header + bytes([header_crc]) + data + struct.pack('<H', data_crc)

def decode_frame_type(ft):
    """Decode frame type to human-readable string."""
    types = {
        0x00: "Token",
        0x01: "Poll-For-Master",
        0x02: "Reply-To-Poll-For-Master",
        0x03: "Test Request",
        0x04: "Test Response",
        0x05: "BACnet Data Expecting Reply",
        0x06: "BACnet Data Not Expecting Reply",
        0x07: "Reply Postponed"
    }
    return types.get(ft, f"Unknown(0x{ft:02X})")

def main():
    if len(sys.argv) < 2:
        print("Usage: mstp_test_sender.py <serial_port> [baud_rate] [our_mac]")
        print("Example: mstp_test_sender.py /dev/ttyUSB0 38400 1")
        sys.exit(1)

    port = sys.argv[1]
    baud = int(sys.argv[2]) if len(sys.argv) > 2 else 38400
    our_mac = int(sys.argv[3]) if len(sys.argv) > 3 else 1

    print(f"Opening {port} at {baud} baud, our MAC address: {our_mac}")

    try:
        ser = serial.Serial(port, baud, timeout=0.5)
        ser.reset_input_buffer()
        print(f"Port opened successfully")
    except Exception as e:
        print(f"Failed to open port: {e}")
        sys.exit(1)

    print("\n=== MS/TP Test Sender ===")
    print("This will send Poll-For-Master frames and listen for responses.\n")

    # Poll for masters at addresses 0-31
    for dest in range(0, 32):
        if dest == our_mac:
            continue

        # Build Poll-For-Master frame
        frame = build_mstp_frame(0x01, dest, our_mac)

        print(f"Sending Poll-For-Master to address {dest}: {frame.hex()}")
        ser.write(frame)
        ser.flush()

        # Wait for response (Reply-To-Poll-For-Master should come within Treply_delay)
        time.sleep(0.050)  # 50ms wait for reply

        # Check for response
        response = ser.read(100)
        if response:
            print(f"  RESPONSE: {response.hex()}")
            # Try to parse it (MS/TP preamble is 55 FF)
            if len(response) >= 8 and response[0] == 0x55 and response[1] == 0xFF:
                ft = response[2]
                da = response[3]
                sa = response[4]
                print(f"  -> Frame Type: {decode_frame_type(ft)}, Dest: {da}, Src: {sa}")
        else:
            print(f"  (no response)")

        time.sleep(0.010)  # Small delay between polls

    print("\n=== Listening for traffic ===")
    print("Press Ctrl+C to exit\n")

    try:
        while True:
            data = ser.read(100)
            if data:
                print(f"Received: {data.hex()}")
                # Try to find frame sync (MS/TP preamble is 55 FF)
                i = 0
                while i < len(data) - 1:
                    if data[i] == 0x55 and data[i+1] == 0xFF:
                        if i + 7 < len(data):
                            ft = data[i+2]
                            da = data[i+3]
                            sa = data[i+4]
                            dlen = (data[i+5] << 8) | data[i+6]
                            print(f"  -> {decode_frame_type(ft)}: {sa} -> {da}, len={dlen}")
                        break
                    i += 1
    except KeyboardInterrupt:
        print("\nExiting...")
    finally:
        ser.close()

if __name__ == "__main__":
    main()

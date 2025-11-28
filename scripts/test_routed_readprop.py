#!/usr/bin/env python3
"""Send routed ReadProperty to MS/TP device via gateway"""
import socket
import struct
import sys

# Gateway at 192.168.86.141:47808
# Target: Device 206 on network 65001 (0xFDE9), MAC 6

GATEWAY_IP = "192.168.86.141"
GATEWAY_PORT = 47808

# Build ReadProperty for Device Object, Property = Object-Identifier (75)
# Device 206 = 0x008000CE (object instance in device type)
# Device object = type 8 (0x08) + instance 206 = 0x0200CE

# APDU: Confirmed Request, Service=ReadProperty(12)
# PDU Type 0 (confirmed request) + more segments bit + SA bit + invoke ID
invoke_id = 1
service_choice = 0x0C  # ReadProperty

# Encode Object-Identifier: Device:206 -> Context Tag 0
# Device type = 8, instance = 206
# Raw: (8 << 22) | 206 = 0x020000CE
obj_id = (8 << 22) | 206
obj_id_bytes = struct.pack('>I', obj_id)

# Property-Identifier: Object-Identifier (75) -> Context Tag 1
prop_id = 75  # Object-Identifier

# Build APDU
apdu = bytes([
    0x00,           # Confirmed request, no seg, no SegAcc
    0x05,           # max-seg=0, max-apdu-len=5 (1476 octets)
    invoke_id,
    service_choice,
    0x0C,           # Context tag 0, len=4 (object identifier)
]) + obj_id_bytes + bytes([
    0x19, prop_id,  # Context tag 1, len=1 (property identifier)
])

# Build NPDU with DNET=65001, DADR=[6]
# Control: destination present (0x20) | expecting reply (0x04)
control = 0x24  # destination present, expecting reply
dnet = 65001    # 0xFDE9
dadr = bytes([6])  # MAC address 6

npdu = bytes([
    0x01,           # Version
    control,
    (dnet >> 8) & 0xFF,
    dnet & 0xFF,
    len(dadr),
]) + dadr + bytes([0xFF]) + apdu  # hop count + APDU

# Build BVLC header (Original-Unicast-NPDU)
bvlc_type = 0x81
bvlc_func = 0x0A  # Original-Unicast-NPDU
bvlc_len = 4 + len(npdu)

packet = bytes([bvlc_type, bvlc_func, (bvlc_len >> 8) & 0xFF, bvlc_len & 0xFF]) + npdu

print(f"Sending routed ReadProperty to Device 206 on network {dnet}")
print(f"Target MAC: {dadr[0]}")
print(f"Packet: {packet.hex()}")

# Send packet and wait for response
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.settimeout(5.0)
sock.bind(('0.0.0.0', 0))

try:
    sock.sendto(packet, (GATEWAY_IP, GATEWAY_PORT))
    print(f"Sent {len(packet)} bytes to {GATEWAY_IP}:{GATEWAY_PORT}")

    # Wait for response
    try:
        data, addr = sock.recvfrom(1500)
        print(f"\nReceived {len(data)} bytes from {addr}")
        print(f"Response: {data.hex()}")

        # Parse BVLC
        if len(data) >= 4 and data[0] == 0x81:
            bvlc_func = data[1]
            bvlc_len = (data[2] << 8) | data[3]
            npdu_data = data[4:]
            print(f"BVLC function: 0x{bvlc_func:02X}, length: {bvlc_len}")

            # Parse NPDU
            if len(npdu_data) >= 2:
                npdu_version = npdu_data[0]
                npdu_ctrl = npdu_data[1]
                print(f"NPDU version: {npdu_version}, control: 0x{npdu_ctrl:02X}")

                # Find APDU start
                pos = 2
                if npdu_ctrl & 0x20:  # DNET present
                    dnet_resp = (npdu_data[pos] << 8) | npdu_data[pos+1]
                    dlen = npdu_data[pos+2]
                    print(f"  DNET: {dnet_resp}, DLEN: {dlen}")
                    pos += 3 + dlen
                if npdu_ctrl & 0x08:  # SNET present
                    snet = (npdu_data[pos] << 8) | npdu_data[pos+1]
                    slen = npdu_data[pos+2]
                    sadr = npdu_data[pos+3:pos+3+slen]
                    print(f"  SNET: {snet}, SADR: {sadr.hex()}")
                    pos += 3 + slen
                if npdu_ctrl & 0x20:  # hop count
                    pos += 1

                apdu = npdu_data[pos:]
                print(f"APDU: {apdu.hex()}")

                # Parse APDU type
                if len(apdu) >= 1:
                    pdu_type = (apdu[0] >> 4) & 0x0F
                    types = {0: "Confirmed-REQ", 1: "Unconfirmed-REQ", 2: "Simple-ACK",
                             3: "Complex-ACK", 4: "Segment-ACK", 5: "Error", 6: "Reject", 7: "Abort"}
                    print(f"PDU Type: {types.get(pdu_type, 'Unknown')} ({pdu_type})")

                    if pdu_type == 3:  # Complex-ACK
                        print("SUCCESS! Got Complex-ACK - ReadProperty was routed and responded!")
                    elif pdu_type == 5:  # Error
                        if len(apdu) >= 5:
                            error_class = apdu[4] if len(apdu) > 4 else 0
                            error_code = apdu[6] if len(apdu) > 6 else 0
                            print(f"Error class: {error_class}, code: {error_code}")
                    elif pdu_type == 6:  # Reject
                        print(f"Reject reason: {apdu[2] if len(apdu) > 2 else 'unknown'}")

    except socket.timeout:
        print("\nNo response within 5 seconds (timeout)")
        print("This could mean:")
        print("  - Packet was routed to MS/TP but no response came back")
        print("  - Device 206 is not responding")
        print("  - Routing failed silently")

finally:
    sock.close()

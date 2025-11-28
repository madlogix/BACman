#!/usr/bin/env python3
"""
Test BACnet Gateway Routing: IP <-> MS/TP

Tests:
1. Who-Is global broadcast from IP - should reach MS/TP devices
2. Who-Is to MS/TP network (65001) from IP
3. Listen for I-Am responses routed back from MS/TP
4. ReadProperty to specific MS/TP device
5. Verify NPDU headers in routed responses

Network Configuration:
- Gateway IP: auto-detect or 192.168.142.141
- IP Network: 10001
- MS/TP Network: 65001
- Gateway device: 1234 on MS/TP MAC 3
"""
import socket
import struct
import time
import argparse
import threading
from datetime import datetime

# Default configuration
DEFAULT_GATEWAY_IP = "192.168.86.141"
BACNET_PORT = 47808
IP_NETWORK = 10001
MSTP_NETWORK = 65001

# BVLC function codes
BVLC_ORIGINAL_UNICAST = 0x0A
BVLC_ORIGINAL_BROADCAST = 0x0B
BVLC_FORWARDED_NPDU = 0x04

# Service choices
SERVICE_WHO_IS = 8
SERVICE_I_AM = 0
SERVICE_READ_PROPERTY = 12


def build_who_is(low_limit=None, high_limit=None):
    """Build Who-Is APDU"""
    apdu = bytes([0x10, SERVICE_WHO_IS])  # Unconfirmed request, Who-Is

    if low_limit is not None and high_limit is not None:
        # Context tag 0: low limit
        if low_limit <= 255:
            apdu += bytes([0x09, low_limit])
        else:
            apdu += bytes([0x0A, (low_limit >> 8) & 0xFF, low_limit & 0xFF])

        # Context tag 1: high limit
        if high_limit <= 255:
            apdu += bytes([0x19, high_limit])
        else:
            apdu += bytes([0x1A, (high_limit >> 8) & 0xFF, high_limit & 0xFF])

    return apdu


def build_read_property(invoke_id, device_instance, property_id):
    """Build ReadProperty APDU for Device object"""
    obj_id = (8 << 22) | device_instance  # Device object type = 8
    obj_id_bytes = struct.pack('>I', obj_id)

    apdu = bytes([
        0x00,           # Confirmed request
        0x05,           # max-seg=0, max-apdu-len=5
        invoke_id,
        SERVICE_READ_PROPERTY,
        0x0C,           # Context tag 0, len=4 (object identifier)
    ]) + obj_id_bytes

    if property_id <= 255:
        apdu += bytes([0x19, property_id])  # Context tag 1, len=1
    else:
        apdu += bytes([0x1A, (property_id >> 8) & 0xFF, property_id & 0xFF])

    return apdu


def build_npdu(apdu, dnet=None, dadr=None, expecting_reply=False):
    """Build NPDU with optional routing"""
    control = 0x00
    if dnet is not None:
        control |= 0x20  # Destination present
    if expecting_reply:
        control |= 0x04  # Expecting reply

    npdu = bytes([0x01, control])  # Version 1

    if dnet is not None:
        npdu += struct.pack('>H', dnet)  # DNET
        if dadr:
            npdu += bytes([len(dadr)]) + dadr  # DLEN + DADR
        else:
            npdu += bytes([0])  # DLEN=0 (broadcast on that network)
        npdu += bytes([0xFF])  # Hop count

    npdu += apdu
    return npdu


def build_bvlc(npdu, is_broadcast=False):
    """Wrap NPDU in BVLC"""
    func = BVLC_ORIGINAL_BROADCAST if is_broadcast else BVLC_ORIGINAL_UNICAST
    length = 4 + len(npdu)
    return bytes([0x81, func, (length >> 8) & 0xFF, length & 0xFF]) + npdu


def parse_bvlc(data):
    """Parse BVLC header"""
    if len(data) < 4 or data[0] != 0x81:
        return None, None, None

    func = data[1]
    length = (data[2] << 8) | data[3]

    if func == BVLC_FORWARDED_NPDU:
        # Skip original source address (6 bytes)
        if len(data) < 10:
            return None, None, None
        orig_ip = f"{data[4]}.{data[5]}.{data[6]}.{data[7]}"
        orig_port = (data[8] << 8) | data[9]
        return func, f"{orig_ip}:{orig_port}", data[10:]
    else:
        return func, None, data[4:]


def parse_npdu(data):
    """Parse NPDU and extract APDU"""
    if len(data) < 2:
        return None

    version = data[0]
    control = data[1]
    pos = 2

    result = {
        'version': version,
        'control': control,
        'network_msg': bool(control & 0x80),
        'dnet': None,
        'dadr': None,
        'snet': None,
        'sadr': None,
        'hop_count': None,
        'apdu': None
    }

    # Parse destination
    if control & 0x20:
        if pos + 3 > len(data):
            return result
        result['dnet'] = (data[pos] << 8) | data[pos+1]
        dlen = data[pos+2]
        pos += 3
        if dlen > 0:
            if pos + dlen > len(data):
                return result
            result['dadr'] = data[pos:pos+dlen]
            pos += dlen

    # Parse source
    if control & 0x08:
        if pos + 3 > len(data):
            return result
        result['snet'] = (data[pos] << 8) | data[pos+1]
        slen = data[pos+2]
        pos += 3
        if slen > 0:
            if pos + slen > len(data):
                return result
            result['sadr'] = data[pos:pos+slen]
            pos += slen

    # Hop count
    if control & 0x20:
        if pos < len(data):
            result['hop_count'] = data[pos]
            pos += 1

    # APDU
    if not result['network_msg'] and pos < len(data):
        result['apdu'] = data[pos:]

    return result


def parse_i_am(apdu):
    """Parse I-Am APDU"""
    if len(apdu) < 12:
        return None

    if apdu[0] != 0x10 or apdu[1] != SERVICE_I_AM:
        return None

    pos = 2

    # Device Object Identifier (tag 0xC4)
    if apdu[pos] != 0xC4:
        return None
    pos += 1

    obj_id = struct.unpack('>I', apdu[pos:pos+4])[0]
    device_instance = obj_id & 0x3FFFFF
    pos += 4

    # Max APDU Length (tag 0x22 or 0x21)
    max_apdu = 0
    if apdu[pos] == 0x22:
        max_apdu = (apdu[pos+1] << 8) | apdu[pos+2]
        pos += 3
    elif apdu[pos] == 0x21:
        max_apdu = apdu[pos+1]
        pos += 2

    # Segmentation Supported (tag 0x91)
    segmentation = 0
    if pos < len(apdu) and apdu[pos] == 0x91:
        segmentation = apdu[pos+1]
        pos += 2

    # Vendor ID (tag 0x21 or 0x22)
    vendor_id = 0
    if pos < len(apdu):
        if apdu[pos] == 0x22:
            vendor_id = (apdu[pos+1] << 8) | apdu[pos+2]
        elif apdu[pos] == 0x21:
            vendor_id = apdu[pos+1]

    return {
        'device_instance': device_instance,
        'max_apdu': max_apdu,
        'segmentation': segmentation,
        'vendor_id': vendor_id
    }


def test_who_is_global(sock, gateway_addr):
    """Test 1: Global Who-Is broadcast"""
    print("\n" + "="*60)
    print("TEST 1: Who-Is Global Broadcast (DNET=0xFFFF)")
    print("="*60)

    apdu = build_who_is()
    npdu = build_npdu(apdu, dnet=0xFFFF)  # Global broadcast
    packet = build_bvlc(npdu, is_broadcast=True)

    print(f"Sending to {gateway_addr}")
    print(f"Packet ({len(packet)} bytes): {packet.hex()}")

    sock.sendto(packet, gateway_addr)
    return True


def test_who_is_mstp_network(sock, gateway_addr):
    """Test 2: Who-Is to MS/TP network specifically"""
    print("\n" + "="*60)
    print(f"TEST 2: Who-Is to MS/TP Network {MSTP_NETWORK}")
    print("="*60)

    apdu = build_who_is()
    npdu = build_npdu(apdu, dnet=MSTP_NETWORK)  # MS/TP network broadcast
    packet = build_bvlc(npdu, is_broadcast=True)

    print(f"Sending to {gateway_addr}")
    print(f"Packet ({len(packet)} bytes): {packet.hex()}")

    sock.sendto(packet, gateway_addr)
    return True


def test_read_property(sock, gateway_addr, device_instance, mstp_mac, property_id=75):
    """Test 3: ReadProperty to specific MS/TP device"""
    print("\n" + "="*60)
    print(f"TEST 3: ReadProperty to Device {device_instance} (MAC {mstp_mac}) on MS/TP")
    print("="*60)

    apdu = build_read_property(1, device_instance, property_id)
    npdu = build_npdu(apdu, dnet=MSTP_NETWORK, dadr=bytes([mstp_mac]), expecting_reply=True)
    packet = build_bvlc(npdu, is_broadcast=False)

    print(f"Property ID: {property_id} ({'Object-Identifier' if property_id == 75 else 'Object-Name' if property_id == 77 else property_id})")
    print(f"Sending to {gateway_addr}")
    print(f"Packet ({len(packet)} bytes): {packet.hex()}")

    sock.sendto(packet, gateway_addr)
    return True


def receive_responses(sock, timeout=10):
    """Receive and parse responses"""
    print("\n" + "="*60)
    print(f"LISTENING FOR RESPONSES (timeout: {timeout}s)")
    print("="*60)

    devices_found = []
    start_time = time.time()

    sock.settimeout(1.0)  # 1 second per recv call

    while time.time() - start_time < timeout:
        try:
            data, addr = sock.recvfrom(1500)
            ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]

            print(f"\n[{ts}] Received {len(data)} bytes from {addr}")
            print(f"  Raw: {data.hex()}")

            # Parse BVLC
            func, orig_addr, npdu_data = parse_bvlc(data)
            if func is None:
                print("  ERROR: Invalid BVLC header")
                continue

            func_names = {
                0x04: "Forwarded-NPDU",
                0x0A: "Original-Unicast",
                0x0B: "Original-Broadcast"
            }
            print(f"  BVLC: {func_names.get(func, f'0x{func:02X}')}", end="")
            if orig_addr:
                print(f" (original: {orig_addr})", end="")
            print()

            # Parse NPDU
            npdu = parse_npdu(npdu_data)
            if npdu is None:
                print("  ERROR: Invalid NPDU")
                continue

            print(f"  NPDU: control=0x{npdu['control']:02X}", end="")
            if npdu['snet']:
                sadr_str = npdu['sadr'].hex() if npdu['sadr'] else "broadcast"
                print(f", SNET={npdu['snet']}, SADR={sadr_str}", end="")
            if npdu['dnet']:
                dadr_str = npdu['dadr'].hex() if npdu['dadr'] else "broadcast"
                print(f", DNET={npdu['dnet']}, DADR={dadr_str}", end="")
            if npdu['hop_count'] is not None:
                print(f", hop={npdu['hop_count']}", end="")
            print()

            # Parse APDU
            if npdu['apdu']:
                apdu = npdu['apdu']
                pdu_type = (apdu[0] >> 4) & 0x0F
                types = {
                    0: "Confirmed-REQ", 1: "Unconfirmed-REQ", 2: "Simple-ACK",
                    3: "Complex-ACK", 4: "Segment-ACK", 5: "Error", 6: "Reject", 7: "Abort"
                }
                print(f"  APDU: {types.get(pdu_type, f'Unknown({pdu_type})')}")

                if pdu_type == 1:  # Unconfirmed
                    service = apdu[1]
                    if service == SERVICE_I_AM:
                        iam = parse_i_am(apdu)
                        if iam:
                            source_info = ""
                            if npdu['snet']:
                                mac = npdu['sadr'][0] if npdu['sadr'] else "?"
                                source_info = f" (Network {npdu['snet']}, MAC {mac})"
                            print(f"  >>> I-AM: Device {iam['device_instance']}{source_info}, Vendor {iam['vendor_id']}")
                            devices_found.append({
                                'instance': iam['device_instance'],
                                'network': npdu['snet'],
                                'mac': npdu['sadr'][0] if npdu['sadr'] else None,
                                'vendor': iam['vendor_id']
                            })

                elif pdu_type == 3:  # Complex-ACK
                    invoke_id = apdu[1]
                    service = apdu[2]
                    print(f"  >>> Complex-ACK: Invoke={invoke_id}, Service={service}")
                    if service == SERVICE_READ_PROPERTY:
                        print("  >>> ReadProperty SUCCESS - routing works!")

                elif pdu_type == 5:  # Error
                    invoke_id = apdu[1]
                    service = apdu[2]
                    print(f"  >>> Error: Invoke={invoke_id}, Service={service}")

                elif pdu_type == 6:  # Reject
                    invoke_id = apdu[1]
                    reason = apdu[2] if len(apdu) > 2 else 0
                    print(f"  >>> Reject: Invoke={invoke_id}, Reason={reason}")

        except socket.timeout:
            continue
        except Exception as e:
            print(f"Error receiving: {e}")

    return devices_found


def main():
    parser = argparse.ArgumentParser(description='Test BACnet Gateway Routing')
    parser.add_argument('--gateway', '-g', default=DEFAULT_GATEWAY_IP,
                        help=f'Gateway IP address (default: {DEFAULT_GATEWAY_IP})')
    parser.add_argument('--device', '-d', type=int, default=None,
                        help='Device instance for ReadProperty test')
    parser.add_argument('--mac', '-m', type=int, default=None,
                        help='MS/TP MAC address for ReadProperty test')
    parser.add_argument('--timeout', '-t', type=int, default=10,
                        help='Response timeout in seconds (default: 10)')
    parser.add_argument('--test', choices=['all', 'whois', 'readprop'], default='all',
                        help='Which test to run (default: all)')
    args = parser.parse_args()

    gateway_addr = (args.gateway, BACNET_PORT)

    print("="*60)
    print("BACNET GATEWAY ROUTING TEST")
    print("="*60)
    print(f"Gateway: {args.gateway}:{BACNET_PORT}")
    print(f"IP Network: {IP_NETWORK}")
    print(f"MS/TP Network: {MSTP_NETWORK}")
    print(f"Timeout: {args.timeout}s")

    # Create socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(('0.0.0.0', 0))

    local_port = sock.getsockname()[1]
    print(f"Local port: {local_port}")

    try:
        if args.test in ['all', 'whois']:
            # Test 1: Global Who-Is
            test_who_is_global(sock, gateway_addr)
            time.sleep(0.1)

            # Test 2: Who-Is to MS/TP network
            test_who_is_mstp_network(sock, gateway_addr)

        if args.test in ['all', 'readprop']:
            if args.device and args.mac:
                time.sleep(0.2)
                # Test 3: ReadProperty
                test_read_property(sock, gateway_addr, args.device, args.mac, 77)  # Object-Name

        # Listen for responses
        devices = receive_responses(sock, args.timeout)

        print("\n" + "="*60)
        print("SUMMARY")
        print("="*60)

        if devices:
            print(f"Discovered {len(devices)} device(s) via routing:")
            for dev in devices:
                net_info = f"Network {dev['network']}, MAC {dev['mac']}" if dev['network'] else "Local"
                print(f"  - Device {dev['instance']} ({net_info}), Vendor {dev['vendor']}")

            # Check if we got MS/TP devices
            mstp_devices = [d for d in devices if d['network'] == MSTP_NETWORK]
            if mstp_devices:
                print(f"\n SUCCESS: {len(mstp_devices)} device(s) discovered on MS/TP network {MSTP_NETWORK}")
                print("  This confirms IP -> MS/TP -> IP routing is working!")
            else:
                print(f"\n WARNING: No devices found on MS/TP network {MSTP_NETWORK}")
                print("  Check if MS/TP devices are responding")
        else:
            print("No devices discovered")
            print("\nPossible issues:")
            print("  - Gateway not receiving IP packets")
            print("  - Gateway not routing to MS/TP")
            print("  - MS/TP devices not responding")
            print("  - Responses not being routed back to IP")

    finally:
        sock.close()


if __name__ == '__main__':
    main()

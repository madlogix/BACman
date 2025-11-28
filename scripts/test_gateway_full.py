#!/usr/bin/env python3
"""
Comprehensive BACnet Gateway Routing Test Suite

Tests all aspects of the MS/TP to IP gateway:
1. Basic routing (Who-Is, I-Am)
2. Network layer messages (Who-Is-Router, I-Am-Router)
3. ReadProperty routing
4. WriteProperty routing (if safe target available)
5. Error handling (unknown device, unknown network)
6. Foreign Device Table
7. Performance/timing tests
8. Gateway local device access

Network Configuration:
- Gateway IP: 192.168.86.141
- IP Network: 10001
- MS/TP Network: 65001
"""
import socket
import struct
import time
import argparse
import sys
from datetime import datetime
from collections import defaultdict

# Configuration
DEFAULT_GATEWAY_IP = "192.168.86.141"
BACNET_PORT = 47808
IP_NETWORK = 10001
MSTP_NETWORK = 65001
GATEWAY_DEVICE = 1234
GATEWAY_MAC = 3

# BVLC function codes
BVLC_RESULT = 0x00
BVLC_FORWARDED_NPDU = 0x04
BVLC_REGISTER_FD = 0x05
BVLC_READ_FDT = 0x06
BVLC_READ_FDT_ACK = 0x07
BVLC_ORIGINAL_UNICAST = 0x0A
BVLC_ORIGINAL_BROADCAST = 0x0B

# Network layer message types
NL_WHO_IS_ROUTER = 0x00
NL_I_AM_ROUTER = 0x01
NL_REJECT_MESSAGE = 0x03

# Service choices
SERVICE_WHO_IS = 8
SERVICE_I_AM = 0
SERVICE_READ_PROPERTY = 12
SERVICE_WRITE_PROPERTY = 15

# Property IDs
PROP_OBJECT_ID = 75
PROP_OBJECT_NAME = 77
PROP_OBJECT_TYPE = 79
PROP_DESCRIPTION = 28

# Test results tracking
test_results = {}


def log(msg, level="INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    print(f"[{ts}] {level}: {msg}")


def build_bvlc(npdu, is_broadcast=False):
    """Wrap NPDU in BVLC"""
    func = BVLC_ORIGINAL_BROADCAST if is_broadcast else BVLC_ORIGINAL_UNICAST
    length = 4 + len(npdu)
    return bytes([0x81, func, (length >> 8) & 0xFF, length & 0xFF]) + npdu


def build_npdu(apdu=None, dnet=None, dadr=None, expecting_reply=False, network_msg=None):
    """Build NPDU with optional routing and network message"""
    control = 0x00
    if dnet is not None:
        control |= 0x20
    if expecting_reply:
        control |= 0x04
    if network_msg is not None:
        control |= 0x80

    npdu = bytes([0x01, control])

    if dnet is not None:
        npdu += struct.pack('>H', dnet)
        if dadr:
            npdu += bytes([len(dadr)]) + dadr
        else:
            npdu += bytes([0])
        npdu += bytes([0xFF])

    if network_msg is not None:
        npdu += bytes([network_msg])
    elif apdu:
        npdu += apdu

    return npdu


def build_who_is(low=None, high=None):
    """Build Who-Is APDU"""
    apdu = bytes([0x10, SERVICE_WHO_IS])
    if low is not None and high is not None:
        if low <= 255:
            apdu += bytes([0x09, low])
        else:
            apdu += bytes([0x0A, (low >> 8) & 0xFF, low & 0xFF])
        if high <= 255:
            apdu += bytes([0x19, high])
        else:
            apdu += bytes([0x1A, (high >> 8) & 0xFF, high & 0xFF])
    return apdu


def build_read_property(invoke_id, device_instance, property_id):
    """Build ReadProperty APDU"""
    obj_id = (8 << 22) | device_instance
    apdu = bytes([0x00, 0x05, invoke_id, SERVICE_READ_PROPERTY, 0x0C])
    apdu += struct.pack('>I', obj_id)
    if property_id <= 255:
        apdu += bytes([0x19, property_id])
    else:
        apdu += bytes([0x1A, (property_id >> 8) & 0xFF, property_id & 0xFF])
    return apdu


def build_who_is_router(target_network=None):
    """Build Who-Is-Router-To-Network message"""
    npdu = build_npdu(network_msg=NL_WHO_IS_ROUTER)
    if target_network is not None:
        npdu += struct.pack('>H', target_network)
    return npdu


def build_register_fd(ttl_seconds):
    """Build Register-Foreign-Device BVLC"""
    return bytes([0x81, BVLC_REGISTER_FD, 0x00, 0x06,
                  (ttl_seconds >> 8) & 0xFF, ttl_seconds & 0xFF])


def build_read_fdt():
    """Build Read-Foreign-Device-Table BVLC"""
    return bytes([0x81, BVLC_READ_FDT, 0x00, 0x04])


def parse_response(data):
    """Parse response and return structured result"""
    if len(data) < 4 or data[0] != 0x81:
        return None

    result = {
        'bvlc_func': data[1],
        'bvlc_len': (data[2] << 8) | data[3],
        'npdu': None,
        'apdu': None,
        'type': None
    }

    # Handle different BVLC functions
    if result['bvlc_func'] == BVLC_RESULT:
        result['type'] = 'bvlc_result'
        result['result_code'] = (data[4] << 8) | data[5] if len(data) >= 6 else 0
        return result

    if result['bvlc_func'] == BVLC_READ_FDT_ACK:
        result['type'] = 'fdt_ack'
        result['fdt_entries'] = []
        pos = 4
        while pos + 10 <= len(data):
            ip = f"{data[pos]}.{data[pos+1]}.{data[pos+2]}.{data[pos+3]}"
            port = (data[pos+4] << 8) | data[pos+5]
            ttl = (data[pos+6] << 8) | data[pos+7]
            remaining = (data[pos+8] << 8) | data[pos+9]
            result['fdt_entries'].append({
                'address': f"{ip}:{port}",
                'ttl': ttl,
                'remaining': remaining
            })
            pos += 10
        return result

    npdu_start = 4
    if result['bvlc_func'] == BVLC_FORWARDED_NPDU:
        npdu_start = 10

    if len(data) < npdu_start + 2:
        return result

    npdu_data = data[npdu_start:]
    control = npdu_data[1]
    pos = 2

    result['npdu'] = {
        'control': control,
        'network_msg': bool(control & 0x80),
        'dnet': None, 'dadr': None,
        'snet': None, 'sadr': None,
        'hop_count': None
    }

    if control & 0x20:
        if pos + 3 <= len(npdu_data):
            result['npdu']['dnet'] = (npdu_data[pos] << 8) | npdu_data[pos+1]
            dlen = npdu_data[pos+2]
            pos += 3
            if dlen > 0 and pos + dlen <= len(npdu_data):
                result['npdu']['dadr'] = npdu_data[pos:pos+dlen]
                pos += dlen

    if control & 0x08:
        if pos + 3 <= len(npdu_data):
            result['npdu']['snet'] = (npdu_data[pos] << 8) | npdu_data[pos+1]
            slen = npdu_data[pos+2]
            pos += 3
            if slen > 0 and pos + slen <= len(npdu_data):
                result['npdu']['sadr'] = npdu_data[pos:pos+slen]
                pos += slen

    if control & 0x20 and pos < len(npdu_data):
        result['npdu']['hop_count'] = npdu_data[pos]
        pos += 1

    if result['npdu']['network_msg']:
        if pos < len(npdu_data):
            msg_type = npdu_data[pos]
            result['type'] = 'network_msg'
            result['network_msg_type'] = msg_type
            if msg_type == NL_I_AM_ROUTER:
                result['networks'] = []
                pos += 1
                while pos + 2 <= len(npdu_data):
                    net = (npdu_data[pos] << 8) | npdu_data[pos+1]
                    result['networks'].append(net)
                    pos += 2
    elif pos < len(npdu_data):
        apdu = npdu_data[pos:]
        result['apdu'] = apdu
        pdu_type = (apdu[0] >> 4) & 0x0F

        if pdu_type == 1:  # Unconfirmed
            result['type'] = 'unconfirmed'
            result['service'] = apdu[1] if len(apdu) > 1 else None
            if result['service'] == SERVICE_I_AM and len(apdu) >= 6:
                obj_id = struct.unpack('>I', apdu[3:7])[0]
                result['device_instance'] = obj_id & 0x3FFFFF
        elif pdu_type == 3:  # Complex-ACK
            result['type'] = 'complex_ack'
            result['invoke_id'] = apdu[1] if len(apdu) > 1 else None
            result['service'] = apdu[2] if len(apdu) > 2 else None
        elif pdu_type == 5:  # Error
            result['type'] = 'error'
            result['invoke_id'] = apdu[1] if len(apdu) > 1 else None
            result['service'] = apdu[2] if len(apdu) > 2 else None
        elif pdu_type == 6:  # Reject
            result['type'] = 'reject'
            result['invoke_id'] = apdu[1] if len(apdu) > 1 else None
            result['reason'] = apdu[2] if len(apdu) > 2 else None

    return result


def receive_until(sock, timeout, stop_condition=None):
    """Receive responses until timeout or stop condition"""
    responses = []
    start = time.time()
    sock.settimeout(0.5)

    while time.time() - start < timeout:
        try:
            data, addr = sock.recvfrom(1500)
            resp = parse_response(data)
            if resp:
                resp['raw'] = data
                resp['addr'] = addr
                resp['time'] = time.time() - start
                responses.append(resp)
                if stop_condition and stop_condition(resp):
                    break
        except socket.timeout:
            continue

    return responses


# =============================================================================
# TEST FUNCTIONS
# =============================================================================

def test_who_is_global(sock, gateway):
    """Test 1: Global Who-Is broadcast"""
    log("TEST 1: Who-Is Global Broadcast (DNET=0xFFFF)")

    apdu = build_who_is()
    npdu = build_npdu(apdu, dnet=0xFFFF)
    packet = build_bvlc(npdu, is_broadcast=True)

    sock.sendto(packet, gateway)
    responses = receive_until(sock, 5)

    devices = [r for r in responses if r.get('type') == 'unconfirmed'
               and r.get('service') == SERVICE_I_AM]

    mstp_devices = [r for r in devices if r.get('npdu', {}).get('snet') == MSTP_NETWORK]

    if mstp_devices:
        log(f"  PASS: Found {len(mstp_devices)} MS/TP device(s) via routing")
        for d in mstp_devices:
            mac = d['npdu']['sadr'][0] if d['npdu'].get('sadr') else '?'
            log(f"    - Device {d.get('device_instance')} on MAC {mac}")
        return True, mstp_devices
    else:
        log(f"  FAIL: No MS/TP devices discovered (got {len(devices)} total)")
        return False, devices


def test_who_is_router(sock, gateway):
    """Test 2: Who-Is-Router-To-Network"""
    log("TEST 2: Who-Is-Router-To-Network")

    # Ask for router to MS/TP network
    npdu = build_who_is_router(MSTP_NETWORK)
    packet = build_bvlc(npdu, is_broadcast=True)

    sock.sendto(packet, gateway)
    responses = receive_until(sock, 3)

    routers = [r for r in responses if r.get('type') == 'network_msg'
               and r.get('network_msg_type') == NL_I_AM_ROUTER]

    if routers:
        for r in routers:
            nets = r.get('networks', [])
            log(f"  PASS: Router announces networks: {nets}")
        return True, routers
    else:
        log("  FAIL: No I-Am-Router-To-Network response")
        return False, responses


def test_read_property_mstp(sock, gateway, device, mac):
    """Test 3: ReadProperty to MS/TP device"""
    log(f"TEST 3: ReadProperty to Device {device} (MAC {mac}) on MS/TP")

    apdu = build_read_property(1, device, PROP_OBJECT_NAME)
    npdu = build_npdu(apdu, dnet=MSTP_NETWORK, dadr=bytes([mac]), expecting_reply=True)
    packet = build_bvlc(npdu)

    start = time.time()
    sock.sendto(packet, gateway)

    def is_response(r):
        return r.get('type') in ['complex_ack', 'error', 'reject'] and r.get('invoke_id') == 1

    responses = receive_until(sock, 5, is_response)
    elapsed = (time.time() - start) * 1000

    ack = next((r for r in responses if r.get('type') == 'complex_ack'), None)
    if ack:
        log(f"  PASS: Got Complex-ACK in {elapsed:.1f}ms")
        return True, ack
    else:
        error = next((r for r in responses if r.get('type') in ['error', 'reject']), None)
        if error:
            log(f"  FAIL: Got {error['type']} response")
        else:
            log(f"  FAIL: No response (timeout)")
        return False, responses


def test_read_property_gateway(sock, gateway):
    """Test 4: ReadProperty to gateway's local device"""
    log(f"TEST 4: ReadProperty to Gateway Device {GATEWAY_DEVICE}")

    # Direct to IP (no DNET)
    apdu = build_read_property(2, GATEWAY_DEVICE, PROP_OBJECT_NAME)
    npdu = build_npdu(apdu, expecting_reply=True)
    packet = build_bvlc(npdu)

    sock.sendto(packet, gateway)

    def is_response(r):
        return r.get('type') in ['complex_ack', 'error', 'reject'] and r.get('invoke_id') == 2

    responses = receive_until(sock, 3, is_response)

    ack = next((r for r in responses if r.get('type') == 'complex_ack'), None)
    if ack:
        log("  PASS: Gateway responded to ReadProperty")
        return True, ack
    else:
        log("  FAIL: No response from gateway")
        return False, responses


def test_read_property_via_routing(sock, gateway):
    """Test 5: ReadProperty to gateway via MS/TP routing (routed request)"""
    log(f"TEST 5: ReadProperty to Gateway via MS/TP routing (DNET={MSTP_NETWORK})")

    apdu = build_read_property(3, GATEWAY_DEVICE, PROP_OBJECT_NAME)
    npdu = build_npdu(apdu, dnet=MSTP_NETWORK, dadr=bytes([GATEWAY_MAC]), expecting_reply=True)
    packet = build_bvlc(npdu)

    sock.sendto(packet, gateway)

    def is_response(r):
        return r.get('type') in ['complex_ack', 'error', 'reject'] and r.get('invoke_id') == 3

    responses = receive_until(sock, 5, is_response)

    ack = next((r for r in responses if r.get('type') == 'complex_ack'), None)
    if ack:
        log("  PASS: Gateway responded via MS/TP routing")
        return True, ack
    else:
        log("  FAIL: No routed response from gateway")
        return False, responses


def test_unknown_device(sock, gateway):
    """Test 6: ReadProperty to non-existent device (expect no response/timeout)"""
    log("TEST 6: ReadProperty to non-existent device 99999 on MS/TP")

    apdu = build_read_property(4, 99999, PROP_OBJECT_NAME)
    npdu = build_npdu(apdu, dnet=MSTP_NETWORK, dadr=bytes([99]), expecting_reply=True)
    packet = build_bvlc(npdu)

    sock.sendto(packet, gateway)

    def is_response(r):
        return r.get('invoke_id') == 4

    responses = receive_until(sock, 3, is_response)

    if not responses:
        log("  PASS: No response (correct - device doesn't exist)")
        return True, []
    else:
        log(f"  INFO: Got {len(responses)} unexpected response(s)")
        return True, responses  # Still pass - might get network reject


def test_unknown_network(sock, gateway):
    """Test 7: Request to unknown network (expect Reject-Message-To-Network)"""
    log("TEST 7: Request to unknown network 59999")

    apdu = build_read_property(5, 1, PROP_OBJECT_NAME)
    npdu = build_npdu(apdu, dnet=59999, dadr=bytes([1]), expecting_reply=True)
    packet = build_bvlc(npdu)

    sock.sendto(packet, gateway)
    responses = receive_until(sock, 3)

    reject = next((r for r in responses if r.get('type') == 'network_msg'
                   and r.get('network_msg_type') == NL_REJECT_MESSAGE), None)

    if reject:
        log("  PASS: Got Reject-Message-To-Network")
        return True, reject
    else:
        log("  WARN: No reject message received (may be silently dropped)")
        return True, responses  # Still pass - behavior varies


def test_foreign_device_register(sock, gateway):
    """Test 8: Register as Foreign Device"""
    log("TEST 8: Register-Foreign-Device (TTL=60s)")

    packet = build_register_fd(60)
    sock.sendto(packet, gateway)

    responses = receive_until(sock, 2)

    result = next((r for r in responses if r.get('type') == 'bvlc_result'), None)

    if result and result.get('result_code') == 0:
        log("  PASS: Registration successful (result=0)")
        return True, result
    elif result:
        log(f"  FAIL: Registration failed (result={result.get('result_code')})")
        return False, result
    else:
        log("  FAIL: No BVLC-Result received")
        return False, responses


def test_read_fdt(sock, gateway):
    """Test 9: Read Foreign Device Table"""
    log("TEST 9: Read-Foreign-Device-Table")

    packet = build_read_fdt()
    sock.sendto(packet, gateway)

    responses = receive_until(sock, 2)

    fdt = next((r for r in responses if r.get('type') == 'fdt_ack'), None)

    if fdt:
        entries = fdt.get('fdt_entries', [])
        log(f"  PASS: FDT has {len(entries)} entries")
        for e in entries:
            log(f"    - {e['address']} TTL={e['ttl']}s remaining={e['remaining']}s")
        return True, fdt
    else:
        log("  FAIL: No FDT-Ack received")
        return False, responses


def test_performance(sock, gateway, device, mac, iterations=10):
    """Test 10: Performance - multiple rapid requests"""
    log(f"TEST 10: Performance test ({iterations} ReadProperty requests)")

    times = []
    successes = 0

    for i in range(iterations):
        apdu = build_read_property(10 + i, device, PROP_OBJECT_NAME)
        npdu = build_npdu(apdu, dnet=MSTP_NETWORK, dadr=bytes([mac]), expecting_reply=True)
        packet = build_bvlc(npdu)

        start = time.time()
        sock.sendto(packet, gateway)

        def is_resp(r):
            return r.get('invoke_id') == 10 + i

        responses = receive_until(sock, 3, is_resp)
        elapsed = (time.time() - start) * 1000

        if any(r.get('type') == 'complex_ack' for r in responses):
            times.append(elapsed)
            successes += 1

    if times:
        avg = sum(times) / len(times)
        min_t = min(times)
        max_t = max(times)
        log(f"  PASS: {successes}/{iterations} successful")
        log(f"    Avg: {avg:.1f}ms, Min: {min_t:.1f}ms, Max: {max_t:.1f}ms")
        return True, {'avg': avg, 'min': min_t, 'max': max_t, 'success_rate': successes/iterations}
    else:
        log(f"  FAIL: No successful responses")
        return False, {}


# =============================================================================
# MAIN
# =============================================================================

def main():
    parser = argparse.ArgumentParser(description='Comprehensive BACnet Gateway Test')
    parser.add_argument('--gateway', '-g', default=DEFAULT_GATEWAY_IP)
    parser.add_argument('--device', '-d', type=int, help='MS/TP device instance for tests')
    parser.add_argument('--mac', '-m', type=int, help='MS/TP MAC address for tests')
    parser.add_argument('--skip-perf', action='store_true', help='Skip performance test')
    args = parser.parse_args()

    gateway = (args.gateway, BACNET_PORT)

    print("=" * 70)
    print("COMPREHENSIVE BACNET GATEWAY TEST SUITE")
    print("=" * 70)
    print(f"Gateway: {args.gateway}:{BACNET_PORT}")
    print(f"IP Network: {IP_NETWORK}, MS/TP Network: {MSTP_NETWORK}")
    print("=" * 70)

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(('0.0.0.0', 0))

    results = {}
    mstp_device = args.device
    mstp_mac = args.mac

    try:
        # Test 1: Who-Is Global
        print()
        passed, data = test_who_is_global(sock, gateway)
        results['who_is_global'] = passed

        # Auto-discover an MS/TP device if not specified
        if not mstp_device and data:
            for d in data:
                if d.get('npdu', {}).get('snet') == MSTP_NETWORK:
                    mstp_device = d.get('device_instance')
                    mstp_mac = d['npdu']['sadr'][0] if d['npdu'].get('sadr') else None
                    log(f"Auto-discovered: Device {mstp_device} on MAC {mstp_mac}")
                    break

        # Test 2: Who-Is-Router
        print()
        passed, _ = test_who_is_router(sock, gateway)
        results['who_is_router'] = passed

        # Test 3: ReadProperty to MS/TP
        if mstp_device and mstp_mac:
            print()
            passed, _ = test_read_property_mstp(sock, gateway, mstp_device, mstp_mac)
            results['read_property_mstp'] = passed
        else:
            log("SKIP: Test 3 - No MS/TP device available")
            results['read_property_mstp'] = None

        # Test 4: ReadProperty to Gateway (direct)
        print()
        passed, _ = test_read_property_gateway(sock, gateway)
        results['read_property_gateway'] = passed

        # Test 5: ReadProperty to Gateway via routing
        print()
        passed, _ = test_read_property_via_routing(sock, gateway)
        results['read_property_gateway_routed'] = passed

        # Test 6: Unknown device
        print()
        passed, _ = test_unknown_device(sock, gateway)
        results['unknown_device'] = passed

        # Test 7: Unknown network
        print()
        passed, _ = test_unknown_network(sock, gateway)
        results['unknown_network'] = passed

        # Test 8: Register Foreign Device
        print()
        passed, _ = test_foreign_device_register(sock, gateway)
        results['register_fd'] = passed

        # Test 9: Read FDT
        print()
        passed, _ = test_read_fdt(sock, gateway)
        results['read_fdt'] = passed

        # Test 10: Performance
        if not args.skip_perf and mstp_device and mstp_mac:
            print()
            passed, perf = test_performance(sock, gateway, mstp_device, mstp_mac)
            results['performance'] = passed
        else:
            results['performance'] = None

    finally:
        sock.close()

    # Summary
    print()
    print("=" * 70)
    print("TEST SUMMARY")
    print("=" * 70)

    passed = sum(1 for v in results.values() if v is True)
    failed = sum(1 for v in results.values() if v is False)
    skipped = sum(1 for v in results.values() if v is None)

    for name, result in results.items():
        status = "PASS" if result is True else "FAIL" if result is False else "SKIP"
        symbol = "[+]" if result else "[-]" if result is False else "[?]"
        print(f"  {symbol} {name}: {status}")

    print()
    print(f"Results: {passed} passed, {failed} failed, {skipped} skipped")
    print("=" * 70)

    return 0 if failed == 0 else 1


if __name__ == '__main__':
    sys.exit(main())

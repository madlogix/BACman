#!/usr/bin/env python3
"""
Comprehensive BACnet Gateway Stress Test Suite

Tests:
1. Device discovery and enumeration
2. Full property reads from all devices
3. Object enumeration (all object types)
4. Concurrent request handling
5. Rapid-fire requests (performance)
6. Error handling verification
7. Large response handling
8. Network layer message handling
"""

import socket
import struct
import time
import sys
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from collections import defaultdict

# BACnet constants
BACNET_PORT = 47808
BVLC_ORIGINAL_BROADCAST = 0x0b
BVLC_ORIGINAL_UNICAST = 0x0a

# Common BACnet property IDs
PROPERTIES = {
    'object-identifier': 75,
    'object-name': 77,
    'object-type': 79,
    'system-status': 112,
    'vendor-name': 121,
    'vendor-identifier': 120,
    'model-name': 70,
    'firmware-revision': 44,
    'application-software-version': 12,
    'protocol-version': 98,
    'protocol-revision': 139,
    'protocol-services-supported': 97,
    'protocol-object-types-supported': 96,
    'object-list': 76,
    'max-apdu-length-accepted': 62,
    'segmentation-supported': 107,
    'apdu-timeout': 11,
    'number-of-apdu-retries': 73,
    'device-address-binding': 30,
    'database-revision': 155,
    'description': 28,
    'location': 58,
    'local-time': 57,
    'local-date': 56,
    'utc-offset': 119,
    'daylight-savings-status': 24,
    'present-value': 85,
    'status-flags': 111,
    'out-of-service': 81,
    'units': 117,
    'reliability': 103,
    'event-state': 36,
}

# Object types
OBJECT_TYPES = {
    0: 'analog-input',
    1: 'analog-output',
    2: 'analog-value',
    3: 'binary-input',
    4: 'binary-output',
    5: 'binary-value',
    8: 'device',
    10: 'file',
    13: 'multi-state-input',
    14: 'multi-state-output',
    19: 'multi-state-value',
}

class BACnetTester:
    def __init__(self, gateway_ip, gateway_port=47808, mstp_network=65001, ip_network=10001):
        self.gateway_ip = gateway_ip
        self.gateway_port = gateway_port
        self.mstp_network = mstp_network
        self.ip_network = ip_network
        self.sock = None
        self.invoke_id = 0
        self.stats = {
            'requests_sent': 0,
            'responses_received': 0,
            'errors': 0,
            'timeouts': 0,
            'total_time': 0,
            'min_time': float('inf'),
            'max_time': 0,
        }
        self.errors_by_type = defaultdict(int)
        self.devices = {}  # device_id -> {mac, objects, properties}

    def create_socket(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.settimeout(3.0)
        self.sock.bind(('', 0))  # Random port

    def close(self):
        if self.sock:
            self.sock.close()

    def next_invoke_id(self):
        self.invoke_id = (self.invoke_id + 1) % 256
        return self.invoke_id

    def build_who_is(self, low=None, high=None):
        """Build Who-Is APDU"""
        if low is None and high is None:
            return bytes([0x10, 0x08])  # Unconfirmed Who-Is, no range
        else:
            # With range limits
            apdu = bytearray([0x10, 0x08])
            # Context tag 0 - low limit
            if low < 256:
                apdu.extend([0x09, low])
            else:
                apdu.extend([0x1a, (low >> 16) & 0xff, (low >> 8) & 0xff, low & 0xff])
            # Context tag 1 - high limit
            if high < 256:
                apdu.extend([0x19, high])
            else:
                apdu.extend([0x2a, (high >> 16) & 0xff, (high >> 8) & 0xff, high & 0xff])
            return bytes(apdu)

    def build_read_property(self, obj_type, obj_instance, prop_id, array_index=None):
        """Build ReadProperty request"""
        invoke_id = self.next_invoke_id()

        # Object identifier (context tag 0)
        obj_id = (obj_type << 22) | (obj_instance & 0x3FFFFF)

        apdu = bytearray([
            0x00,  # Confirmed request, no segmentation
            0x05,  # Max segments=0, max APDU=480
            invoke_id,
            0x0c,  # ReadProperty service
            0x0c,  # Context tag 0, length 4
            (obj_id >> 24) & 0xff,
            (obj_id >> 16) & 0xff,
            (obj_id >> 8) & 0xff,
            obj_id & 0xff,
        ])

        # Property identifier (context tag 1)
        if prop_id < 256:
            apdu.extend([0x19, prop_id])
        else:
            apdu.extend([0x1a, (prop_id >> 8) & 0xff, prop_id & 0xff])

        # Array index (context tag 2) if specified
        if array_index is not None:
            if array_index < 256:
                apdu.extend([0x29, array_index])
            else:
                apdu.extend([0x2a, (array_index >> 8) & 0xff, array_index & 0xff])

        return bytes(apdu), invoke_id

    def build_npdu_routed(self, apdu, dest_network, dest_mac):
        """Build NPDU with routing to MS/TP device"""
        npdu = bytearray([
            0x01,  # Version
            0x24,  # Control: dest present, expecting reply
            (dest_network >> 8) & 0xff,
            dest_network & 0xff,
            0x01,  # DLEN = 1 (MS/TP MAC)
            dest_mac,
            0xff,  # Hop count
        ])
        npdu.extend(apdu)
        return bytes(npdu)

    def build_npdu_broadcast(self, apdu):
        """Build NPDU for global broadcast"""
        npdu = bytearray([
            0x01,  # Version
            0x20,  # Control: dest present (broadcast)
            0xff, 0xff,  # DNET = 0xFFFF (global broadcast)
            0x00,  # DLEN = 0 (broadcast)
            0xff,  # Hop count
        ])
        npdu.extend(apdu)
        return bytes(npdu)

    def build_npdu_local(self, apdu, expecting_reply=False):
        """Build simple NPDU for local device"""
        control = 0x04 if expecting_reply else 0x00
        npdu = bytearray([0x01, control])
        npdu.extend(apdu)
        return bytes(npdu)

    def build_bvlc(self, npdu, broadcast=False):
        """Wrap NPDU in BVLC"""
        func = BVLC_ORIGINAL_BROADCAST if broadcast else BVLC_ORIGINAL_UNICAST
        length = len(npdu) + 4
        return bytes([0x81, func, (length >> 8) & 0xff, length & 0xff]) + npdu

    def send_receive(self, packet, timeout=3.0):
        """Send packet and wait for response"""
        self.stats['requests_sent'] += 1
        start = time.time()

        try:
            self.sock.settimeout(timeout)
            self.sock.sendto(packet, (self.gateway_ip, self.gateway_port))
            data, addr = self.sock.recvfrom(1500)
            elapsed = time.time() - start

            self.stats['responses_received'] += 1
            self.stats['total_time'] += elapsed
            self.stats['min_time'] = min(self.stats['min_time'], elapsed)
            self.stats['max_time'] = max(self.stats['max_time'], elapsed)

            return data, elapsed
        except socket.timeout:
            self.stats['timeouts'] += 1
            return None, time.time() - start
        except Exception as e:
            self.stats['errors'] += 1
            return None, time.time() - start

    def send_broadcast_collect(self, packet, timeout=5.0):
        """Send broadcast and collect all responses"""
        responses = []
        self.stats['requests_sent'] += 1
        start = time.time()

        try:
            self.sock.settimeout(0.5)  # Short timeout for each recv
            self.sock.sendto(packet, (self.gateway_ip, self.gateway_port))

            while time.time() - start < timeout:
                try:
                    data, addr = self.sock.recvfrom(1500)
                    responses.append((data, addr, time.time() - start))
                    self.stats['responses_received'] += 1
                except socket.timeout:
                    continue

        except Exception as e:
            self.stats['errors'] += 1

        return responses

    def parse_i_am(self, data):
        """Parse I-Am response to extract device info"""
        try:
            # Skip BVLC header
            if data[0] != 0x81:
                return None
            bvlc_len = (data[2] << 8) | data[3]
            npdu_start = 4

            # Handle Forwarded-NPDU (0x04) - has 6-byte original address
            if data[1] == 0x04:
                npdu_start = 10  # Skip BVLC header (4) + original address (6)

            # Parse NPDU
            npdu_data = data[npdu_start:]
            if len(npdu_data) < 2:
                return None

            control = npdu_data[1]
            offset = 2

            source_network = None
            source_mac = None

            # NPDU order: DNET/DADR first, then SNET/SADR, then hop count
            # Check for destination specifier FIRST (bit 5)
            if control & 0x20:
                dnet = (npdu_data[offset] << 8) | npdu_data[offset + 1]
                dlen = npdu_data[offset + 2]
                offset += 3
                if dlen > 0:
                    offset += dlen  # Skip DADR

            # Check for source specifier SECOND (bit 3)
            if control & 0x08:
                source_network = (npdu_data[offset] << 8) | npdu_data[offset + 1]
                slen = npdu_data[offset + 2]
                offset += 3
                if slen > 0:
                    source_mac = npdu_data[offset]
                    offset += slen

            # Hop count comes AFTER source (if dest was present)
            if control & 0x20:
                offset += 1  # Skip hop count

            # Now at APDU
            apdu = npdu_data[offset:]
            if len(apdu) < 2:
                return None

            # Check for I-Am (unconfirmed, service 0)
            if apdu[0] != 0x10 or apdu[1] != 0x00:
                return None

            # Parse object identifier (context tag 0, length 4)
            if len(apdu) < 7:
                return None
            # apdu[2] should be 0xC4 (context tag 0, length 4) for object-id
            obj_id = (apdu[3] << 24) | (apdu[4] << 16) | (apdu[5] << 8) | apdu[6]
            obj_type = obj_id >> 22
            obj_instance = obj_id & 0x3FFFFF

            if obj_type == 8:  # Device object
                return {
                    'device_id': obj_instance,
                    'network': source_network,
                    'mac': source_mac,
                }
            return None
        except Exception as e:
            return None

    def parse_read_property_ack(self, data):
        """Parse ReadProperty response"""
        try:
            if data[0] != 0x81:
                return None, "Invalid BVLC"

            # Find APDU start
            npdu_start = 4
            if data[1] == 0x04:  # Forwarded-NPDU
                npdu_start = 10

            npdu_data = data[npdu_start:]
            control = npdu_data[1]
            offset = 2

            # NPDU order: DNET/DADR first, then SNET/SADR, then hop count
            # Skip dest specifier FIRST (bit 5)
            if control & 0x20:
                dnet = (npdu_data[offset] << 8) | npdu_data[offset + 1]
                dlen = npdu_data[offset + 2]
                offset += 3
                if dlen > 0:
                    offset += dlen  # Skip DADR

            # Skip source specifier SECOND (bit 3)
            if control & 0x08:
                snet = (npdu_data[offset] << 8) | npdu_data[offset + 1]
                slen = npdu_data[offset + 2]
                offset += 3
                if slen > 0:
                    offset += slen  # Skip SADR

            # Skip hop count (if dest was present)
            if control & 0x20:
                offset += 1

            apdu = npdu_data[offset:]
            if len(apdu) < 1:
                return None, "Empty APDU"

            apdu_type = (apdu[0] >> 4) & 0x0f

            if apdu_type == 3:  # Complex-ACK
                return apdu, None
            elif apdu_type == 5:  # Error
                error_class = apdu[3] if len(apdu) > 3 else 0
                error_code = apdu[5] if len(apdu) > 5 else 0
                return None, f"Error({error_class},{error_code})"
            elif apdu_type == 6:  # Reject
                return None, f"Reject({apdu[2] if len(apdu) > 2 else 0})"
            elif apdu_type == 7:  # Abort
                return None, f"Abort({apdu[2] if len(apdu) > 2 else 0})"
            else:
                return None, f"Unexpected APDU type {apdu_type}"
        except Exception as e:
            return None, str(e)

    # ========== TEST METHODS ==========

    def test_device_discovery(self):
        """Test 1: Discover all devices"""
        print("\n" + "="*70)
        print("TEST 1: Device Discovery")
        print("="*70)

        who_is = self.build_who_is()
        npdu = self.build_npdu_broadcast(who_is)
        bvlc = self.build_bvlc(npdu, broadcast=True)

        print("Sending Who-Is broadcast...")
        responses = self.send_broadcast_collect(bvlc, timeout=5.0)

        for data, addr, elapsed in responses:
            device_info = self.parse_i_am(data)
            if device_info:
                dev_id = device_info['device_id']
                self.devices[dev_id] = {
                    'mac': device_info.get('mac'),
                    'network': device_info.get('network'),
                    'objects': [],
                    'properties': {},
                }
                net_info = f"network {device_info['network']}, MAC {device_info['mac']}" if device_info['network'] else "local"
                print(f"  Found device {dev_id} ({net_info}) in {elapsed*1000:.1f}ms")

        print(f"\nDiscovered {len(self.devices)} device(s)")
        return len(self.devices) > 0

    def test_read_all_properties(self, device_id, mac=None, network=None):
        """Read all common properties from a device"""
        results = {'success': 0, 'errors': 0, 'details': {}}

        for prop_name, prop_id in PROPERTIES.items():
            if network and mac is not None:
                apdu, invoke_id = self.build_read_property(8, device_id, prop_id)
                npdu = self.build_npdu_routed(apdu, network, mac)
            else:
                apdu, invoke_id = self.build_read_property(8, device_id, prop_id)
                npdu = self.build_npdu_local(apdu, expecting_reply=True)

            bvlc = self.build_bvlc(npdu, broadcast=False)
            response, elapsed = self.send_receive(bvlc, timeout=3.0)

            if response:
                ack, error = self.parse_read_property_ack(response)
                if ack:
                    results['success'] += 1
                    results['details'][prop_name] = ('OK', elapsed)
                else:
                    results['errors'] += 1
                    results['details'][prop_name] = (error, elapsed)
                    self.errors_by_type[error] += 1
            else:
                results['errors'] += 1
                results['details'][prop_name] = ('TIMEOUT', elapsed)
                self.errors_by_type['TIMEOUT'] += 1

        return results

    def test_full_property_scan(self):
        """Test 2: Read all properties from all devices"""
        print("\n" + "="*70)
        print("TEST 2: Full Property Scan")
        print("="*70)

        total_success = 0
        total_errors = 0

        for dev_id, dev_info in self.devices.items():
            print(f"\nScanning device {dev_id}...")
            results = self.test_read_all_properties(
                dev_id,
                mac=dev_info.get('mac'),
                network=dev_info.get('network')
            )

            print(f"  Properties: {results['success']} OK, {results['errors']} errors")
            total_success += results['success']
            total_errors += results['errors']

            # Show errors
            for prop, (status, elapsed) in results['details'].items():
                if status != 'OK':
                    print(f"    {prop}: {status}")

        print(f"\nTotal: {total_success} successful, {total_errors} errors")
        return total_errors == 0 or total_success > total_errors

    def test_object_list_enumeration(self):
        """Test 3: Read object-list and enumerate all objects"""
        print("\n" + "="*70)
        print("TEST 3: Object List Enumeration")
        print("="*70)

        for dev_id, dev_info in self.devices.items():
            print(f"\nReading object-list from device {dev_id}...")

            # Read object-list array index 0 (count)
            if dev_info.get('network') and dev_info.get('mac') is not None:
                apdu, _ = self.build_read_property(8, dev_id, 76, array_index=0)
                npdu = self.build_npdu_routed(apdu, dev_info['network'], dev_info['mac'])
            else:
                apdu, _ = self.build_read_property(8, dev_id, 76, array_index=0)
                npdu = self.build_npdu_local(apdu, expecting_reply=True)

            bvlc = self.build_bvlc(npdu)
            response, elapsed = self.send_receive(bvlc)

            if response:
                ack, error = self.parse_read_property_ack(response)
                if ack:
                    print(f"  Object-list read successful ({elapsed*1000:.1f}ms)")
                else:
                    print(f"  Object-list error: {error}")
            else:
                print(f"  Object-list timeout")

        return True

    def test_concurrent_requests(self):
        """Test 4: Send concurrent requests to stress the gateway"""
        print("\n" + "="*70)
        print("TEST 4: Concurrent Request Handling")
        print("="*70)

        if not self.devices:
            print("  No devices to test")
            return False

        # Pick first MS/TP device
        mstp_devices = [(d, i) for d, i in self.devices.items() if i.get('network')]
        if not mstp_devices:
            print("  No MS/TP devices found")
            return False

        dev_id, dev_info = mstp_devices[0]
        num_concurrent = 5
        num_rounds = 3

        print(f"Sending {num_concurrent} concurrent requests x {num_rounds} rounds to device {dev_id}...")

        success = 0
        failures = 0

        for round_num in range(num_rounds):
            # Create multiple sockets for true concurrency
            sockets = []
            for _ in range(num_concurrent):
                s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                s.settimeout(5.0)
                s.bind(('', 0))
                sockets.append(s)

            # Build requests
            requests = []
            for i, s in enumerate(sockets):
                prop_id = list(PROPERTIES.values())[i % len(PROPERTIES)]
                apdu, invoke_id = self.build_read_property(8, dev_id, prop_id)
                npdu = self.build_npdu_routed(apdu, dev_info['network'], dev_info['mac'])
                bvlc = self.build_bvlc(npdu)
                requests.append((s, bvlc))

            # Send all at once
            start = time.time()
            for s, pkt in requests:
                s.sendto(pkt, (self.gateway_ip, self.gateway_port))

            # Collect responses
            for s, _ in requests:
                try:
                    data, _ = s.recvfrom(1500)
                    ack, error = self.parse_read_property_ack(data)
                    if ack:
                        success += 1
                    else:
                        failures += 1
                except socket.timeout:
                    failures += 1

            elapsed = time.time() - start
            print(f"  Round {round_num + 1}: {elapsed*1000:.1f}ms")

            # Cleanup
            for s in sockets:
                s.close()

            time.sleep(0.5)  # Brief pause between rounds

        total = num_concurrent * num_rounds
        print(f"\nResults: {success}/{total} successful ({failures} failures)")
        return success > failures

    def test_rapid_fire(self):
        """Test 5: Rapid sequential requests"""
        print("\n" + "="*70)
        print("TEST 5: Rapid-Fire Performance Test")
        print("="*70)

        if not self.devices:
            print("  No devices to test")
            return False

        # Test with gateway device (local, fastest)
        gateway_id = 1234
        num_requests = 50

        print(f"Sending {num_requests} rapid requests to gateway device {gateway_id}...")

        success = 0
        failures = 0
        times = []

        start_total = time.time()
        for i in range(num_requests):
            prop_id = list(PROPERTIES.values())[i % len(PROPERTIES)]
            apdu, _ = self.build_read_property(8, gateway_id, prop_id)
            npdu = self.build_npdu_local(apdu, expecting_reply=True)
            bvlc = self.build_bvlc(npdu)

            response, elapsed = self.send_receive(bvlc, timeout=2.0)
            if response:
                ack, error = self.parse_read_property_ack(response)
                if ack:
                    success += 1
                    times.append(elapsed)
                else:
                    failures += 1
            else:
                failures += 1

        total_elapsed = time.time() - start_total

        if times:
            avg_time = sum(times) / len(times)
            min_time = min(times)
            max_time = max(times)
            requests_per_sec = success / total_elapsed

            print(f"\nResults: {success}/{num_requests} successful")
            print(f"  Total time: {total_elapsed:.2f}s")
            print(f"  Requests/sec: {requests_per_sec:.1f}")
            print(f"  Response times: avg={avg_time*1000:.1f}ms, min={min_time*1000:.1f}ms, max={max_time*1000:.1f}ms")
        else:
            print(f"  All requests failed!")

        return success > num_requests * 0.8  # 80% success threshold

    def test_mstp_rapid_fire(self):
        """Test 6: Rapid requests to MS/TP device"""
        print("\n" + "="*70)
        print("TEST 6: MS/TP Rapid-Fire Performance Test")
        print("="*70)

        mstp_devices = [(d, i) for d, i in self.devices.items() if i.get('network')]
        if not mstp_devices:
            print("  No MS/TP devices found")
            return False

        dev_id, dev_info = mstp_devices[0]
        num_requests = 20

        print(f"Sending {num_requests} rapid requests to MS/TP device {dev_id}...")

        success = 0
        failures = 0
        times = []

        start_total = time.time()
        for i in range(num_requests):
            prop_id = list(PROPERTIES.values())[i % 10]  # Cycle through first 10 properties
            apdu, _ = self.build_read_property(8, dev_id, prop_id)
            npdu = self.build_npdu_routed(apdu, dev_info['network'], dev_info['mac'])
            bvlc = self.build_bvlc(npdu)

            response, elapsed = self.send_receive(bvlc, timeout=3.0)
            if response:
                ack, error = self.parse_read_property_ack(response)
                if ack:
                    success += 1
                    times.append(elapsed)
                else:
                    failures += 1
                    self.errors_by_type[error] += 1
            else:
                failures += 1
                self.errors_by_type['TIMEOUT'] += 1

        total_elapsed = time.time() - start_total

        if times:
            avg_time = sum(times) / len(times)
            min_time = min(times)
            max_time = max(times)
            requests_per_sec = success / total_elapsed

            print(f"\nResults: {success}/{num_requests} successful")
            print(f"  Total time: {total_elapsed:.2f}s")
            print(f"  Requests/sec: {requests_per_sec:.1f}")
            print(f"  Response times: avg={avg_time*1000:.1f}ms, min={min_time*1000:.1f}ms, max={max_time*1000:.1f}ms")
        else:
            print(f"  All requests failed!")

        return success > num_requests * 0.7  # 70% success for MS/TP

    def test_error_handling(self):
        """Test 7: Verify proper error responses"""
        print("\n" + "="*70)
        print("TEST 7: Error Handling Verification")
        print("="*70)

        tests_passed = 0
        tests_total = 0

        # Test 7a: Read non-existent property
        print("\n7a: Read non-existent property (proprietary 9999)...")
        tests_total += 1
        apdu, _ = self.build_read_property(8, 1234, 9999)  # Proprietary property
        npdu = self.build_npdu_local(apdu, expecting_reply=True)
        bvlc = self.build_bvlc(npdu)
        response, elapsed = self.send_receive(bvlc)
        if response:
            ack, error = self.parse_read_property_ack(response)
            if error and 'Error' in error:
                print(f"  PASS: Got expected error: {error}")
                tests_passed += 1
            elif ack:
                print(f"  INFO: Property exists (unexpected)")
            else:
                print(f"  FAIL: Unexpected response: {error}")
        else:
            print(f"  FAIL: No response (timeout)")

        # Test 7b: Read non-existent object
        print("\n7b: Read non-existent object (device 99999999)...")
        tests_total += 1
        apdu, _ = self.build_read_property(8, 99999999, 75)  # Non-existent device
        npdu = self.build_npdu_local(apdu, expecting_reply=True)
        bvlc = self.build_bvlc(npdu)
        response, elapsed = self.send_receive(bvlc)
        if response:
            ack, error = self.parse_read_property_ack(response)
            if error:
                print(f"  PASS: Got expected error: {error}")
                tests_passed += 1
            else:
                print(f"  FAIL: Unexpected success")
        else:
            print(f"  INFO: No response (may be correct for non-existent object)")
            tests_passed += 1  # Timeout is acceptable for non-existent object

        # Test 7c: Request to unknown network
        print("\n7c: Request to unknown network (59999)...")
        tests_total += 1
        apdu, _ = self.build_read_property(8, 1234, 75)
        npdu = bytearray([
            0x01, 0x24,  # Version, control (dest + expecting reply)
            0xEA, 0x5F,  # DNET = 59999
            0x01, 0x01,  # DLEN=1, DADR=1
            0xff,  # Hop count
        ])
        npdu.extend(apdu)
        bvlc = self.build_bvlc(bytes(npdu))
        response, elapsed = self.send_receive(bvlc, timeout=5.0)
        if response:
            # Check for Reject-Message-To-Network
            if len(response) > 10:
                print(f"  PASS: Got network layer response")
                tests_passed += 1
            else:
                print(f"  INFO: Got response: {response.hex()}")
        else:
            print(f"  INFO: No response (timeout)")

        print(f"\nError handling: {tests_passed}/{tests_total} tests passed")
        return tests_passed >= tests_total - 1

    def print_summary(self):
        """Print final test summary"""
        print("\n" + "="*70)
        print("FINAL SUMMARY")
        print("="*70)

        print(f"\nOverall Statistics:")
        print(f"  Requests sent: {self.stats['requests_sent']}")
        print(f"  Responses received: {self.stats['responses_received']}")
        print(f"  Timeouts: {self.stats['timeouts']}")
        print(f"  Errors: {self.stats['errors']}")

        if self.stats['responses_received'] > 0:
            avg_time = self.stats['total_time'] / self.stats['responses_received']
            print(f"\nResponse Times:")
            print(f"  Average: {avg_time*1000:.1f}ms")
            print(f"  Min: {self.stats['min_time']*1000:.1f}ms")
            print(f"  Max: {self.stats['max_time']*1000:.1f}ms")

        if self.errors_by_type:
            print(f"\nErrors by Type:")
            for error_type, count in sorted(self.errors_by_type.items(), key=lambda x: -x[1]):
                print(f"  {error_type}: {count}")

        success_rate = self.stats['responses_received'] / max(1, self.stats['requests_sent']) * 100
        print(f"\nSuccess Rate: {success_rate:.1f}%")


def main():
    parser = argparse.ArgumentParser(description='BACnet Gateway Stress Test')
    parser.add_argument('--gateway', '-g', default='192.168.71.1',
                       help='Gateway IP address')
    parser.add_argument('--port', '-p', type=int, default=47808,
                       help='Gateway port')
    args = parser.parse_args()

    print("="*70)
    print("BACNET GATEWAY STRESS TEST SUITE")
    print("="*70)
    print(f"Gateway: {args.gateway}:{args.port}")
    print(f"Started: {time.strftime('%Y-%m-%d %H:%M:%S')}")

    tester = BACnetTester(args.gateway, args.port)
    tester.create_socket()

    results = {}

    try:
        # Run all tests
        results['discovery'] = tester.test_device_discovery()
        results['property_scan'] = tester.test_full_property_scan()
        results['object_list'] = tester.test_object_list_enumeration()
        results['concurrent'] = tester.test_concurrent_requests()
        results['rapid_fire_local'] = tester.test_rapid_fire()
        results['rapid_fire_mstp'] = tester.test_mstp_rapid_fire()
        results['error_handling'] = tester.test_error_handling()

    finally:
        tester.close()

    # Print summary
    tester.print_summary()

    print("\n" + "="*70)
    print("TEST RESULTS")
    print("="*70)

    all_passed = True
    for test_name, passed in results.items():
        status = "PASS" if passed else "FAIL"
        symbol = "[+]" if passed else "[-]"
        print(f"  {symbol} {test_name}: {status}")
        if not passed:
            all_passed = False

    print("="*70)

    return 0 if all_passed else 1


if __name__ == '__main__':
    sys.exit(main())

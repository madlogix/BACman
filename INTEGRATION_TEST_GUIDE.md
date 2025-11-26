# BACnet MS/TP Gateway Integration Test Guide

This guide describes how to perform integration testing of the BACnet MS/TP to IP Gateway.

## Prerequisites

### Hardware Required
1. **M5StickC Plus2** with gateway firmware flashed
2. **RS-485 HAT** connected to M5StickC Plus2
3. **MS/TP Device** (e.g., BACnet thermostat, controller) on RS-485 bus
4. **BACnet/IP Device** (optional, for comparison testing)
5. **RS-485 termination resistors** (120 ohm at each end of bus)
6. **Computer** with Ethernet/WiFi on same network as gateway

### Software Required
1. **Rust toolchain** - `cargo` for running test examples
2. **Wireshark** with BACnet dissector (built-in)
3. **bacnet-rs** library examples

## Test 1: Basic Device Discovery

### Purpose
Verify the gateway forwards Who-Is broadcasts to MS/TP and routes I-Am responses back to IP.

### Steps

1. **Start the Gateway**
   - Power on M5StickC Plus2
   - Verify WiFi connection (check display shows IP address)
   - Note the gateway IP address

2. **Run Discovery Test**
   ```bash
   cd bacnet-rs
   cargo run --example gateway_test [gateway_ip]
   ```

3. **Expected Results**
   - Direct BACnet/IP devices show `Connection: Direct`
   - MS/TP devices behind gateway show `Source Network: XXXXX (ROUTED via gateway)`
   - MS/TP address shown for routed devices

### Pass Criteria
- [ ] Gateway displays "WiFi:OK" with IP address
- [ ] MS/TP devices are discovered with routing info (SNET/SADR)
- [ ] Device IDs and vendor names are correctly displayed

## Test 2: Wireshark Protocol Analysis

### Purpose
Verify BACnet protocol compliance and correct NPDU routing.

### Steps

1. **Start Wireshark Capture**
   ```
   Filter: bacnet || udp.port == 47808
   ```

2. **Send Who-Is Broadcast**
   ```bash
   cargo run --example whois_scan
   ```

3. **Analyze Packets**
   - Who-Is should be `Original-Broadcast-NPDU` (0x0B)
   - I-Am from MS/TP should contain SNET and SADR fields
   - NPDU control byte should have bit 3 set (source present)

### Expected NPDU for Routed I-Am
```
NPDU:
  Version: 1
  Control: 0x28 (dest=0, src=1, expecting_reply=0)
  Source Network: 65001 (MS/TP network number)
  Source Address Length: 1
  Source Address: [station_address]
  Hop Count: (not present - no destination)
```

### Pass Criteria
- [ ] Who-Is broadcast received by gateway
- [ ] I-Am responses contain correct source routing info
- [ ] No malformed packets or CRC errors

## Test 3: Read Property Through Gateway

### Purpose
Verify bidirectional communication - read properties from MS/TP device.

### Steps

1. **Identify MS/TP Device**
   - Note device ID from discovery test
   - Note MS/TP station address

2. **Send Read-Property Request**
   ```bash
   cargo run --example test_client -- read [device_id] device:present-value
   ```

3. **Expected Results**
   - Request routed to MS/TP network
   - Response routed back to IP
   - Property value displayed

### Pass Criteria
- [ ] Read-Property request sent successfully
- [ ] Response received within timeout (typically 3-5 seconds for routed)
- [ ] Property value matches expected value

## Test 4: Token Loop Timing

### Purpose
Verify MS/TP token passing is working correctly.

### Steps

1. **Monitor Gateway Display**
   - Check `Loop:` field (token loop time in ms)
   - Check `M:` field (master count)

2. **Expected Values**
   - Token loop time: 10-100ms typical
   - Master count: Number of masters on MS/TP bus

3. **Stress Test (Optional)**
   - Send rapid Who-Is broadcasts
   - Monitor for token loss or recovery

### Pass Criteria
- [ ] Token loop time is stable
- [ ] Master count matches physical devices on bus
- [ ] No persistent token loss

## Test 5: Address Table Aging

### Purpose
Verify address table entries expire after timeout.

### Steps

1. **Discover Devices**
   ```bash
   cargo run --example gateway_test
   ```

2. **Wait for Aging Timeout**
   - Default: 1 hour (configurable)
   - Or manually trigger by disconnecting MS/TP device

3. **Re-Discover**
   - Run discovery again
   - Verify new entries are created

### Pass Criteria
- [ ] Old entries are removed after timeout
- [ ] New entries created for re-discovered devices
- [ ] No memory growth on gateway

## Troubleshooting

### No Devices Found
1. Check gateway WiFi connection (display shows IP)
2. Check MS/TP wiring (A+/B- polarity)
3. Check RS-485 termination
4. Verify baud rate matches (default: 38400)
5. Check MS/TP station addresses (0-127 for masters)

### CRC Errors on Gateway Display
1. Check RS-485 cable quality
2. Verify termination resistors
3. Check for electrical noise sources
4. Try lower baud rate

### Devices Found but No Routing Info
1. Gateway may not be routing (check network numbers)
2. Device may be on IP network (not MS/TP)
3. Check gateway MSTP network number configuration

### Token Loop Time Very High
1. Too many masters polling non-existent addresses
2. Reduce `max_master` setting
3. Check for offline/failed devices

## Test Environment Reference

### Default Gateway Configuration
```
MS/TP Network: 65001
IP Network: 10001
MS/TP Address: 10
Max Master: 127
Baud Rate: 38400
```

### Common MS/TP Device Addresses
- 0-127: Master devices (participate in token passing)
- 128-254: Slave devices (respond only when polled)
- 255: Broadcast

### BACnet/IP Ports
- 47808 (0xBAC0): Standard BACnet/IP port
- UDP broadcast for device discovery

## Running All Tests

```bash
# Run unit tests
cd bacnet-rs
cargo test

# Run gateway integration test
cargo run --example gateway_test

# Run comprehensive device scan
cargo run --example comprehensive_whois_scan

# Run with debug output
RUST_LOG=debug cargo run --example gateway_test
```

## Test Results Log

| Date | Test | Result | Notes |
|------|------|--------|-------|
| | | | |

---

*Last Updated: 2025-11-26*

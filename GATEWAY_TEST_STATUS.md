# BACnet MS/TP to IP Gateway - Test Status

**Date:** 2025-11-27
**Gateway IP:** 192.168.86.141
**Device Instance:** 1234
**MS/TP MAC:** 3
**MS/TP Network:** 65001 (0xFDE9)
**IP Network:** 10001 (0x2711)

## Hardware Setup

- **Gateway:** M5StickC Plus2 with RS-485 HAT
- **MS/TP Bus Participants:**
  - Gateway (MAC 3) - M5Stack
  - Controller 6 (MAC 6, Device 206) - BACnet controller
  - BACrouter (MAC 2, Device 45753) - Commercial BACnet router

## Completed Features

### 1. MS/TP Token Ring ✅
- [x] CRC-8 header calculation (corrected polynomial)
- [x] CRC-16 data calculation (CRC-CCITT 0x8408, NOT MODBUS 0xA001)
- [x] Token passing between stations (MAC 3 ↔ MAC 6 ↔ MAC 2)
- [x] Poll-For-Master / Reply-To-Poll-For-Master
- [x] Master discovery and next_station tracking
- [x] State machine: Initialize → Idle → UseToken → PassToken

### 2. BACnet/IP Interface ✅
- [x] UDP socket on port 47808
- [x] BVLC header parsing (Original-Unicast, Original-Broadcast)
- [x] Web portal at http://192.168.86.141/
- [x] Device discovery via web interface

### 3. Gateway Routing ✅
- [x] MS/TP → IP routing (I-Am responses forwarded to IP)
- [x] IP → MS/TP routing (ReadProperty requests routed to MS/TP)
- [x] NPDU source/destination network handling
- [x] DNET filtering bug FIXED (packets now route instead of local processing)
- [x] I-Am-Router-To-Network announcements

### 4. Local Device ✅
- [x] Who-Is / I-Am handling
- [x] ReadProperty for Device object properties
- [x] Gateway responds as Device 1234

### 5. Device Discovery ✅
- [x] Who-Is broadcast from web portal works
- [x] I-Am responses captured and displayed
- [x] Discovered devices:
  - Device 1234 (Gateway) - local
  - Device 206 (Controller 6) - MAC 6, network 65001
  - Device 45753 (BACrouter) - MAC 2, network 65001

## Bug Fixes Applied This Session

### 1. DNET Filtering Bug (FIXED)
**Location:** `main.rs` lines 1040-1048
**Issue:** Empty if block didn't filter DNET, causing gateway to process all requests locally
**Fix:** Added `local_network` parameter and proper filtering:
```rust
if dnet != 0xFFFF && dnet != local_network {
    return None;  // Route, don't process locally
}
```

### 2. Expecting Reply Bug (FIXED)
**Location:** `main.rs` line 1175
**Issue:** MS/TP frames were sent as `BacnetDataNotExpectingReply (0x06)` instead of checking NPDU
**Fix:** Extract expecting_reply from NPDU control byte:
```rust
let expecting_reply = if mstp_data.len() >= 2 {
    (mstp_data[1] & 0x04) != 0
} else {
    false
};
```

**Result:** Frames now sent as `BacnetDataExpectingReply (0x05)` for confirmed requests. Verified in logs:
```
TX data frame: type=BacnetDataExpectingReply dest=6 len=27
TX RAW FRAME: [55, FF, 05, 06, 03, 00, 1B, F3, ...]
```

## Current Issue: Controller 6 Not Responding

### What Works
- ✅ IP→MS/TP routing: Packets correctly routed to MS/TP network
- ✅ Frame type: `BacnetDataExpectingReply (0x05)` now used for confirmed requests
- ✅ NPDU format: Correct with DNET=65001, DADR=[6], SNET=10001, SADR=[IP:port]
- ✅ Gateway enters WaitForReply state after sending

### What's Not Working
- ❌ Controller 6 doesn't respond to routed ReadProperty
- Reply timeout occurs (~285ms) with no response from MAC 6

### Observations from Serial Logs
1. **MS/TP Bus Noise**: CRC errors and garbled data during WaitForReply
   ```
   Header CRC error: expected 0x7C, got 0x55 (type=85, dest=6, src=127)
   RX_DISCARD: 5 bytes before preamble: [55, 06, 7F, BE, FF]
   ```
2. **Token Ring Activity During Wait**: Station 2 (BACrouter) polling interferes
3. **Possible Causes**:
   - Controller 6 may not support routed confirmed requests
   - TX echo interference corrupting received data
   - Timing issues with token ring vs. reply timeout

### Next Debugging Steps
1. Try a different device (BACrouter at MAC 2, Device 45753)
2. Send unrouted request directly to local device for comparison
3. Check if Who-Is/I-Am still works (confirms device is functional)
4. Review MS/TP Treply_timeout setting

## Remaining Items to Test/Verify

### Priority 1: End-to-End ReadProperty Response
- [x] Send ReadProperty from IP to MS/TP device
- [x] Verify MS/TP frame sent with correct type (0x05)
- [ ] Debug why Controller 6 doesn't respond
- [ ] Verify gateway routes response back to IP client
- [ ] **Current Status:** Frame sent correctly, waiting for device response

### Priority 2: MS/TP → IP Routing for Responses
- [ ] Verify ComplexACK responses from MS/TP devices are routed to IP
- [ ] Check source network/address preservation in responses
- [ ] Test transaction ID matching

### Priority 3: WriteProperty Routing
- [ ] Test WriteProperty from IP to MS/TP device
- [ ] Verify value changes on target device

### Priority 4: Broadcast Handling
- [ ] Global broadcast (DNET=0xFFFF) from IP reaches all MS/TP devices
- [ ] Local broadcast on MS/TP works correctly

### Priority 5: Error Handling
- [ ] Reject-Message-To-Network for unreachable destinations
- [ ] Timeout handling for non-responsive devices
- [ ] Transaction timeout cleanup

## Test Commands

### Monitor Gateway Serial Output
```bash
python3 scripts/serial_monitor.py /dev/ttyACM0 115200
```

### Send Routed ReadProperty to Device 206
```bash
python3 scripts/test_routed_readprop.py
```

### Run MS/TP Sniffer (on separate RS-485 adapter)
```bash
python3 scripts/mstp_sniffer.py /dev/ttyUSB0 38400
```

### Send Who-Is Broadcast
```bash
# Via web portal: http://192.168.86.141/ -> Click "Scan for Devices"
```

## Gateway Log Examples

### Successful IP → MS/TP Routing
```
BIP RX: 22 bytes from 192.168.86.140:52689 BVLC: [81, 0A, 00, 16, 01, 24, FD, E9, 01, 06, FF, ...]
IP->MS/TP routing: 27 bytes to MS/TP dest=6 NPDU: [01, 2C, FD, E9, 01, 06, 27, 11, 06, ...]
IP->MS/TP frame queued successfully
```

### Successful Who-Is Discovery
```
MS/TP RX queue: 25 bytes from MAC 6
  -> APDU extracted: [10, 00, C4, 02, 00, 00, CE, ...]
  -> I-Am detected from MAC 6
Discovered device: instance 206 at MAC 6, vendor 95
```

## Network Diagram

```
                    ┌─────────────────────┐
                    │   IP Network 10001  │
                    │  192.168.86.0/24    │
                    └─────────┬───────────┘
                              │
                              │ UDP:47808
                              │
                    ┌─────────▼───────────┐
                    │  M5Stack Gateway    │
                    │  Device 1234        │
                    │  MAC 3              │
                    │  IP: 192.168.86.141 │
                    └─────────┬───────────┘
                              │
                              │ RS-485 @ 38400
                              │
                    ┌─────────▼───────────┐
                    │ MS/TP Network 65001 │
                    └─────────┬───────────┘
                              │
            ┌─────────────────┼─────────────────┐
            │                 │                 │
      ┌─────▼─────┐     ┌─────▼─────┐     ┌─────▼─────┐
      │Controller 6│     │ BACrouter │     │  (Other)  │
      │Device 206 │     │Device 45753│     │           │
      │  MAC 6    │     │  MAC 2     │     │           │
      └───────────┘     └────────────┘     └───────────┘
```

## Next Steps

1. **Debug Response Path:** Monitor MS/TP traffic to see if Controller 6 is responding to ReadProperty
2. **Check Transaction Matching:** Ensure invoke IDs are preserved through routing
3. **Test with Different Properties:** Try Object-Name (77) or other properties
4. **Verify Source Address in Routed Requests:** Check SNET/SADR are correct for return path

## Files Modified This Session

- `mstp-ip-gateway/src/main.rs` - DNET filtering fix, expecting_reply fix
- `scripts/test_routed_readprop.py` - Test script for routed ReadProperty
- `GATEWAY_TEST_STATUS.md` - This status document

## Test Scripts Available

| Script | Purpose |
|--------|---------|
| `scripts/serial_monitor.py` | Monitor ESP32 serial output |
| `scripts/mstp_sniffer.py` | Decode MS/TP frames from RS-485 |
| `scripts/mstp_simulator.py` | Simulate MS/TP device for testing |
| `scripts/mstp_test_sender.py` | Send test MS/TP frames |
| `scripts/test_routed_readprop.py` | Test routed ReadProperty to MS/TP |

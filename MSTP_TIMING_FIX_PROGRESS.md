# MS/TP Timing Fix Progress

## Date: 2025-11-28

## Problem Summary

The M5Stack gateway (MAC address 3) was intermittently appearing and disappearing from the MS/TP token ring. The other BACnet router (MAC 2) reported dropped tokens, and the gateway would only be discovered briefly every 20-50 tokens.

## Network Setup

| MAC | Device | Role |
|-----|--------|------|
| 2 | BACnet Router | Existing router on the loop |
| 3 | M5Stack Gateway | Our device (BACman) |
| 6 | BACnet Controller | Certified BACnet controller |

## Root Cause Identified

**Wireshark capture revealed the issue:**

```
MS/TP Usage and Timing Maximums:
MAC     Trpfm (Reply to Poll For Master timing)
3       27ms    <-- PROBLEM: Must be < 10ms (Tslot)
```

The M5Stack was taking **27ms** to respond to PollForMaster requests, but the MS/TP specification requires responses within **Tslot = 10ms**.

### Why 27ms?

Multiple `info!()` logging statements in the critical path were causing synchronous serial output, each taking 1-5ms on ESP32:

1. `parse_frames()` - logged BEFORE processing the frame
2. `handle_frame_in_idle()` - logged BEFORE sending reply
3. `send_raw_frame()` - logged BEFORE transmitting

## Code Changes Made

### File: `mstp-ip-gateway/src/mstp_driver.rs`

#### 1. Reordered PollForMaster handling (lines ~541-574)

**Before:** Log first, then send reply
**After:** Send reply IMMEDIATELY, then log/bookkeeping

```rust
Some(MstpFrameType::PollForMaster) => {
    // TIMING CRITICAL: Respond to poll FIRST, then do bookkeeping!
    if dest == self.station_address {
        // Send reply IMMEDIATELY - no logging before this!
        self.send_reply_to_poll(source)?;
        self.no_token_timer = Instant::now();
        // Now safe to log (after time-critical response sent)
        debug!("RPFM sent to {}", source);
    }
    // Record source as discovered master (after reply sent)
    // ...
}
```

#### 2. Removed pre-TX logging for time-critical frames (lines ~1030-1044)

```rust
let is_reply_to_poll = ftype == MstpFrameType::ReplyToPollForMaster;
let is_token = ftype == MstpFrameType::Token;
let skip_pre_log = is_reply_to_poll || is_token;

if !skip_pre_log {
    // Only log non-time-critical frames before TX
    // ...
}
```

#### 3. Process frames before logging (lines ~417-427)

**Before:** Log frame details, then call `handle_received_frame()`
**After:** Call `handle_received_frame()` first, then log

```rust
// Process frame FIRST - logging can wait!
self.handle_received_frame(frame_type, dest, source, data)?;

// Post-process logging (non-critical path)
if data_len > 0 {
    debug!("RX frame: type={:?} src={} dest={} len={}", ...);
}
```

#### 4. Added post-TX trace logging (line ~1192-1195)

```rust
if skip_pre_log {
    trace!("TX: {:?} -> {}", ftype, dest);
}
```

## Build Status

- Code compiles successfully with only warnings (unused functions)
- Release build completed: `target/xtensa-esp32-espidf/release/mstp-ip-gateway`

## VERIFIED: Fix Successful!

Firmware flashed and tested on 2025-11-28.

### Results Comparison

| Metric | Before | After |
|--------|--------|-------|
| **Trpfm (MAC 3)** | 27ms | 0ms (not polled - already stable in ring) |
| **Retries (MAC 2)** | 73 | **0** |
| **Token distribution** | Uneven/intermittent | **Equal (243 tokens each)** |
| **Nodes in ring** | 3 (MAC 3 dropping out) | **4 (all stable)** |

### Analysis

- **Zero retries** confirms no more dropped tokens
- **Equal token distribution** (243 each for MAC 2, 3, 6, 7) shows healthy ring
- **Trpfm=0** for MAC 3 means it wasn't polled because it stayed in the ring the entire capture
- **New node discovered (MAC 7)** - network stable enough for new participants

## Capture File Reference

- Original capture showing the problem: `mstp_20251128080513.cap`
- Post-fix capture confirming success: `mstp_20251128183626.cap`

## Related Documentation

- `MSTP_PROTOCOL_REQUIREMENTS.md` - MS/TP timing requirements
- `MSTP_WIRESHARK_CAPTURE.md` - How to capture MS/TP traffic
- `BACnet_Standard/Annex-G-CALCULATION-OF-CRC.md` - CRC calculations

## Git Status

Changes verified and ready to commit:
- `mstp-ip-gateway/src/mstp_driver.rs` - Timing optimizations (VERIFIED WORKING)

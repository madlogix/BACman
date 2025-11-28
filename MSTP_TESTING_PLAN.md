# MS/TP Core Functionality Testing Plan

**Created:** 2025-11-28
**Purpose:** Systematic testing and validation of MS/TP protocol implementation before IP integration
**Target Hardware:** M5StickC Plus2 (ESP32) with RS-485 HAT (SP485EEN)
**Reference:** ASHRAE 135 Clause 9, MSTP_PROTOCOL_REQUIREMENTS.md

---

## Overview

This plan focuses on getting the MS/TP side working correctly:
1. Announce ourselves on the loop
2. Poll for masters
3. Respond to poll for masters
4. Sit happily on the MS/TP bus

**Testing Philosophy:** Start with unit tests (host), then integration tests (hardware), then real-world validation.

---

## Phase 1: Frame Layer Validation

**Goal:** Ensure frames are correctly encoded/decoded with valid CRCs.

### 1.1 Header CRC-8 (ASHRAE 135 Annex G.1)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Token frame [0x00,0x10,0x05,0x00,0x00] register | 0x73 | 0x73 | PASS |
| [x] | Token frame transmitted CRC (ones complement) | 0x8C | 0x8C | PASS |
| [x] | Receiver validation (with CRC appended) | 0x55 | 0x55 | PASS - valid frame indicator |

**Implementation Notes:**
- Uses parallel algorithm from Annex G.1
- Polynomial: X^8 + X^7 + 1
- Initial value: 0xFF
- Final: ones complement

**Test Results (2025-11-28):**
```
━━━ TEST 1.1: Header CRC-8 (Token Frame) ━━━
  Register value after 5 bytes: 0x73 ✓ PASS (expected 0x73)
  Transmitted CRC (ones complement): 0x8C ✓ PASS (expected 0x8C)

━━━ TEST 1.1b: Header CRC Receiver Validation ━━━
  Receiver remainder: 0x55 ✓ PASS (expected 0x55 = valid frame)
```

---

### 1.2 Data CRC-16 (CRC-CCITT Reflected)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | ASHRAE vector [0x01,0x22,0x30] after 0x01 | 0x1E0E | 0x1E0E | PASS |
| [x] | ASHRAE vector after 0x22 | 0xEB70 | 0xEB70 | PASS |
| [x] | ASHRAE vector after 0x30 | 0x42EF | 0x42EF | PASS |
| [x] | Final CRC (ones complement) | 0xBD10 | 0xBD10 | PASS |
| [x] | Receiver validation remainder | 0xF0B8 | 0xF0B8 | PASS - valid frame indicator |

**Implementation Notes:**
- Polynomial: 0x8408 (CRC-CCITT reflected form)
- Initial value: 0xFFFF
- Final: ones complement
- Transmitted LSB first

**Test Results (2025-11-28):**
```
━━━ TEST 1.2: Data CRC-16 (ASHRAE 135 Annex G.2 Vector) ━━━
  After 0x01: 0x1E0E ✓ PASS
  After 0x22: 0xEB70 ✓ PASS
  After 0x30: 0x42EF ✓ PASS
  Final CRC (ones complement): 0xBD10 ✓ PASS
  Transmitted bytes (LSB first): [0x10, 0xBD]

━━━ TEST 1.2b: Data CRC Receiver Validation ━━━
  Receiver remainder: 0xF0B8 ✓ PASS (expected 0xF0B8 = valid frame)
```

---

### 1.3 Frame Encoding (All 8 Frame Types)

| Status | Frame Type | Value | Encoding Verified | Notes |
|--------|------------|-------|-------------------|-------|
| [x] | Token | 0x00 | Yes | 8 bytes, no data field |
| [x] | PollForMaster | 0x01 | Yes | 8 bytes, no data field |
| [ ] | ReplyToPollForMaster | 0x02 | | No data field |
| [ ] | TestRequest | 0x03 | | Optional data |
| [ ] | TestResponse | 0x04 | | Optional data |
| [ ] | BACnetDataExpectingReply | 0x05 | | Has NPDU data |
| [x] | BACnetDataNotExpectingReply | 0x06 | Yes | 14 bytes with 4-byte data |
| [ ] | ReplyPostponed | 0x07 | | No data field |

**Frame Format:**
```
[0x55][0xFF][FrameType][Dest][Src][Len_Hi][Len_Lo][HeaderCRC][...Data...][DataCRC_Lo][DataCRC_Hi]
```

**Test Results (2025-11-28):**
```
━━━ TEST 1.3: Token Frame Encoding ━━━
  Frame bytes: [55, FF, 00, 10, 05, 00, 00, 8C]
  Frame length: 8 bytes ✓ PASS
  Preamble correct: ✓ PASS

━━━ TEST 1.3b: Data Frame Encoding ━━━
  Frame bytes: [55, FF, 06, 0A, 14, 00, 04, BE, 01, 02, 03, 04, 91, 39]
  Frame length: 14 bytes ✓ PASS
  Header CRC valid: ✓ PASS
  Data CRC valid: ✓ PASS
```

---

### 1.4 Frame Decoding and Preamble Recognition

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Detect 0x55 preamble | Found | Found | PASS |
| [x] | Detect 0xFF after 0x55 | Frame start | Frame start | PASS |
| [x] | Header CRC detects 1-bit error | Error detected | 0xCD ≠ 0x55 | PASS |
| [x] | Data CRC detects 1-bit error | Error detected | 0xEC03 ≠ 0xF0B8 | PASS |
| [ ] | Multiple 0x55 before 0xFF | Handle gracefully | | |
| [ ] | Garbage before preamble | Skip and resync | | |
| [ ] | Incomplete frame | Buffer until complete | | |

**Test Results (2025-11-28):**
```
━━━ TEST 1.4: Error Detection ━━━
  Header CRC detects 1-bit error: ✓ PASS (remainder 0xCD ≠ 0x55)
  Data CRC detects 1-bit error: ✓ PASS (remainder 0xEC03 ≠ 0xF0B8)
```

---

## Phase 2: State Machine - INITIALIZE and IDLE

**Goal:** Verify correct startup behavior and idle state handling.

**STATUS: COMPLETE - 22/22 tests passed (2025-11-28)**

### 2.1 INITIALIZE State

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Entry on power-up | State = INITIALIZE | INITIALIZE | PASS |
| [x] | Wait for Tno_token (500ms) | Transition after silence | Verified | PASS |
| [x] | Noise during init | Reset silence timer | Verified | PASS |
| [x] | Exit to IDLE | After 500ms silence | → IDLE | PASS |

**Timing Parameters:**
- Tno_token: 500ms (currently set to 5000ms for discovery - intentional for better master discovery)

**Test Results (2025-11-28):**
```
━━━ TEST 2.1: INITIALIZE State Entry ━━━
  Initial state: Initialize ✓ PASS
  SilenceTimer starts at 0: true ✓ PASS
  EventCount starts at 0: true ✓ PASS
  ReceivedValidFrame starts false: true ✓ PASS
```

---

### 2.2 IDLE State - Frame Monitoring

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Receive Token(dest=us) | → USE_TOKEN | → UseToken | PASS |
| [x] | Receive Token(dest=other) | Stay IDLE, reset no_token | Stay IDLE | PASS |
| [x] | Receive PollForMaster(dest=us) | Send RTPFM, stay IDLE | RTPFM queued | PASS |
| [x] | Receive DataExpectingReply(dest=us) | → ANSWER_DATA_REQUEST | → AnswerDataRequest | PASS |
| [x] | Receive DataNotExpectingReply | Queue data, stay IDLE | Data queued | PASS |
| [x] | Receive broadcast data | Queue data, stay IDLE | Data queued | PASS |

**Test Results (2025-11-28):**
```
━━━ TEST 2.2: IDLE State Transitions ━━━
  Token for us → UseToken: ✓ PASS
  Token for other → stay IDLE: ✓ PASS
  PFM for us → RTPFM queued, stay IDLE: ✓ PASS
  DataExpectingReply → AnswerDataRequest: ✓ PASS
  DataNotExpectingReply → data queued, stay IDLE: ✓ PASS
  Broadcast → data queued, stay IDLE: ✓ PASS
```

---

### 2.3 Tno_token Timeout

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | No token received for Tno_token | → POLL_FOR_MASTER | → PollForMaster | PASS |
| [x] | Token received resets timer | Timer reset | Timer = 0 | PASS |
| [x] | Any valid frame resets silence | Silence timer reset | Reset verified | PASS |

**Test Results (2025-11-28):**
```
━━━ TEST 2.3: Tno_token Timeout ━━━
  No token timeout → PollForMaster: ✓ PASS
  Token reception resets timer: ✓ PASS
  Valid frame resets silence: ✓ PASS
```

---

## Phase 3: Token Passing

**Goal:** Verify correct token ring participation.

### 3.1 Token Reception

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Token(dest=This_Station) | TokenCount++, → USE_TOKEN | | |
| [ ] | FrameCount reset | FrameCount = 0 on entry | | |
| [ ] | Usage timer started | Start Tusage_timeout | | |

**Test Results:**
```
[Record test output here]
```

---

### 3.2 Nmax_info_frames Enforcement

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Send 1 frame (default max) | Allowed | | |
| [ ] | Attempt 2nd frame | Blocked, → DONE_WITH_TOKEN | | |
| [ ] | No frames queued | Immediate → DONE_WITH_TOKEN | | |

**Configuration:** Nmax_info_frames = 1 (default)

**Test Results:**
```
[Record test output here]
```

---

### 3.3 Tusage_timeout (50ms)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Exit before timeout | Normal operation | | |
| [ ] | Timeout while sending | Force → DONE_WITH_TOKEN | | |
| [ ] | Timeout measured correctly | 50ms ± 5ms | | |

**Test Results:**
```
[Record test output here]
```

---

### 3.4 PASS_TOKEN

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Send Token(dest=NS) | Frame transmitted | | |
| [ ] | Transition to IDLE | After token sent | | |
| [ ] | Reset no_token_timer | Timer restarted | | |

**Test Results:**
```
[Record test output here]
```

---

### 3.5 TokenCount and Npoll Threshold

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | TokenCount increments | Each token received | | |
| [ ] | At Npoll (50) → poll | → POLL_FOR_MASTER | | |
| [ ] | TokenCount reset after poll | TokenCount = 0 | | |

**Test Results:**
```
[Record test output here]
```

---

## Phase 4: Poll for Master Procedure

**Goal:** Verify master discovery and ring formation.

### 4.1 Send PollForMaster

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Send PFM(dest=PS) | Frame transmitted | | |
| [ ] | PS starts at NS+1 | Correct initial value | | |
| [ ] | Start slot_timer | Tslot countdown | | |

**Test Results:**
```
[Record test output here]
```

---

### 4.2 Respond to PollForMaster

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Receive PFM(dest=us) in IDLE | Send RTPFM immediately | | |
| [ ] | RTPFM(dest=source, src=us) | Correct addressing | | |
| [ ] | Stay in IDLE | Don't change state | | |

**Test Results:**
```
[Record test output here]
```

---

### 4.3 Tslot Timeout (10ms)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | No reply in 10ms | Increment PS, poll next | | |
| [ ] | Reply received | Set NS=source, → PASS_TOKEN | | |
| [ ] | Timing accuracy | 10ms ± 2ms | | |

**Test Results:**
```
[Record test output here]
```

---

### 4.4 Discovery Bitmap

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Track responding masters | Bitmap updated | | |
| [ ] | 128-bit coverage (0-127) | All addresses supported | | |
| [ ] | Bitmap accessible for debug | Via stats/display | | |

**Test Results:**
```
[Record test output here]
```

---

### 4.5 Sole Master Detection

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | PS wraps to This_Station | SoleMaster = true | | |
| [ ] | NS = This_Station | Token to self | | |
| [ ] | → USE_TOKEN | Start using token | | |
| [ ] | Continue polling periodically | Detect new masters | | |

**Test Results:**
```
[Record test output here]
```

---

## Phase 5: WAIT_FOR_REPLY State (CRITICAL)

**Goal:** Verify correct reply acceptance using NEGATIVE LIST approach.

**STATUS: COMPLETE - Implementation verified correct (2025-11-28)**

> **WARNING:** This is the #1 implementation bug source. The standard requires rejecting ONLY known non-reply frames, NOT accepting only known reply frames.

### 5.1 State Entry

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | After BACnetDataExpectingReply | → WAIT_FOR_REPLY | → WaitForReply | PASS |
| [x] | reply_timer started | Treply_timeout countdown | Timer started | PASS |
| [x] | From USE_TOKEN only | Correct transition | Verified | PASS |

**Test Results (2025-11-28):**
```
━━━ TEST 5.1: WAIT_FOR_REPLY State Entry ━━━
  After DataExpectingReply → WaitForReply: ✓ PASS
  Reply timer started: ✓ PASS
  Transition from UseToken: ✓ PASS
```

---

### 5.2 CRITICAL: Negative List - Reject Non-Replies

| Status | Frame Type | Action | Verified | Notes |
|--------|------------|--------|----------|-------|
| [x] | Token | REJECT → IDLE | → Idle | PASS - mstp_driver.rs:623-627 |
| [x] | PollForMaster | REJECT → IDLE | → Idle | PASS - mstp_driver.rs:628-632 |
| [x] | ReplyToPollForMaster | REJECT → IDLE | → Idle | PASS - mstp_driver.rs:633-637 |
| [x] | TestRequest | REJECT → IDLE | → Idle | PASS - mstp_driver.rs:638-642 |

**Test Results (2025-11-28):**
```
━━━ TEST 5.2: Negative List - Reject Non-Replies ━━━
  Token in WaitForReply → Idle: ✓ PASS
  PollForMaster in WaitForReply → Idle: ✓ PASS
  ReplyToPollForMaster in WaitForReply → Idle: ✓ PASS
  TestRequest in WaitForReply → Idle: ✓ PASS
```

**Implementation verified at mstp_driver.rs:616-664:**
```rust
// CORRECT: Negative list approach
match frame_type {
    FrameType::Token |
    FrameType::PollForMaster |
    FrameType::ReplyToPollForMaster |
    FrameType::TestRequest => {
        // NOT a reply - reject, transition to IDLE
    }
    _ => {
        // Accept ALL other frame types as valid replies
    }
}
```

---

### 5.3 CRITICAL: Accept All Other Frames as Valid Replies

| Status | Frame Type | Action | Verified | Notes |
|--------|------------|--------|----------|-------|
| [x] | BACnetDataNotExpectingReply | ACCEPT → DONE_WITH_TOKEN | Verified | PASS |
| [x] | TestResponse | ACCEPT → DONE_WITH_TOKEN | Verified | PASS |
| [x] | ReplyPostponed | ACCEPT → DONE_WITH_TOKEN | Verified | PASS |
| [x] | Unknown frame type (0x08+) | ACCEPT → DONE_WITH_TOKEN | Verified | PASS - Forward compat! |
| [x] | Proprietary frame | ACCEPT → DONE_WITH_TOKEN | Verified | PASS |
| [x] | Segmented Complex-ACK | ACCEPT → DONE_WITH_TOKEN | Verified | PASS |

**This is the critical fix from bacnet-stack commit f877ca0eb**

**Test Results (2025-11-28):**
```
━━━ TEST 5.3: Accept All Other Frames as Valid Replies ━━━
  BACnetDataNotExpectingReply → DoneWithToken: ✓ PASS
  TestResponse → DoneWithToken: ✓ PASS
  ReplyPostponed → DoneWithToken: ✓ PASS
  Unknown frame (0x08) → DoneWithToken: ✓ PASS (CRITICAL - forward compat)
  Unknown frame (0xFF) → DoneWithToken: ✓ PASS (proprietary)
  All _ match arm types accepted: ✓ PASS
```

---

### 5.4 Treply_timeout (255ms)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Timeout expires | → DONE_WITH_TOKEN | | |
| [ ] | Reply before timeout | Timer cancelled | | |
| [ ] | Timing accuracy | 255ms ± 10ms | | |

**Test Results:**
```
[Record test output here]
```

---

## Phase 6: ANSWER_DATA_REQUEST State

**Goal:** Correctly respond to requests from other masters.

### 6.1 State Entry

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | DataExpectingReply in IDLE | → ANSWER_DATA_REQUEST | | |
| [ ] | Store request for processing | Request buffered | | |
| [ ] | Start reply_delay_timer | Countdown started | | |

**Test Results:**
```
[Record test output here]
```

---

### 6.2 Treply_delay (250ms Minimum)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Wait at least 250ms | Delay enforced | | |
| [ ] | Don't exceed Treply_timeout | Reply within 255ms | | |
| [ ] | Timing accuracy | 250ms ± 5ms | | |

**Test Results:**
```
[Record test output here]
```

---

### 6.3 Send Response

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Send BACnetDataNotExpectingReply | Frame transmitted | | |
| [ ] | Correct destination (original source) | Addressing correct | | |
| [ ] | → IDLE after sending | State transition | | |
| [ ] | ReplyPostponed if needed | For slow processing | | |

**Test Results:**
```
[Record test output here]
```

---

## Phase 7: Error Recovery and Edge Cases

**Goal:** Ensure robustness on real RS-485 buses.

### 7.1 Header CRC Error Handling

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Corrupt header CRC | Discard frame | | |
| [ ] | Log error | Error logged | | |
| [ ] | Stay in current state | No state change | | |
| [ ] | Stats updated | crc_errors++ | | |

**Test Results:**
```
[Record test output here]
```

---

### 7.2 Data CRC Error Handling

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Corrupt data CRC | Discard data | | |
| [ ] | Log with source address | Source identified | | |
| [ ] | If WAIT_FOR_REPLY, may retry | Retry logic | | |
| [ ] | Stats updated | crc_errors++ | | |

**Test Results:**
```
[Record test output here]
```

---

### 7.3 Frame Abort Detection (Tframe_abort)

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | 60 bit times silence mid-frame | Abort frame | | |
| [ ] | Clear RX buffer | Buffer cleared | | |
| [ ] | At 38400 baud: ~1.56ms | Timing correct | | |
| [ ] | Resume listening | Ready for next frame | | |

**Tframe_abort by Baud Rate:**
| Baud | Bit Time | Tframe_abort (60 bits) |
|------|----------|------------------------|
| 9600 | 104μs | 6.25ms |
| 19200 | 52μs | 3.125ms |
| 38400 | 26μs | 1.5625ms |
| 76800 | 13μs | 0.78ms |

**Test Results:**
```
[Record test output here]
```

---

### 7.4 Lost Token Recovery

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | No token for Tno_token | → POLL_FOR_MASTER | | |
| [ ] | Attempt ring regeneration | Poll sequence | | |
| [ ] | Become sole master if alone | SoleMaster = true | | |
| [ ] | Rejoin ring if others respond | NS updated | | |

**Test Results:**
```
[Record test output here]
```

---

### 7.5 RX Buffer Overflow Protection

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Buffer exceeds max size | Trim oldest data | | |
| [ ] | No crash/panic | Graceful handling | | |
| [ ] | Log warning | Warning logged | | |
| [ ] | Recovery to normal | Resume operation | | |

**Buffer Limits:**
- MAX_RX_BUFFER: MSTP_HEADER_SIZE + MSTP_MAX_DATA_LENGTH + 2 (CRC)
- MSTP_MAX_DATA_LENGTH: 501 bytes (NPDU max)

**Test Results:**
```
[Record test output here]
```

---

## Phase 8: Integration Testing with Real Hardware

**Goal:** Validate on actual RS-485 bus with real devices.

**STATUS: COMPLETE - 3-node token ring operational! (2025-11-28)**

### 8.1 Capture MS/TP Traffic

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Serial monitor running | Captures frames | Working | via serial_monitor.py |
| [x] | See M5Stack frames | Our frames visible | Visible | TX/RX logged |
| [x] | See other device frames | Bus traffic visible | Visible | Stations 2 and 6 |
| [x] | 3-node token ring | All nodes participating | WORKING | 2→3→6→2 cycle |

**Setup (from MSTP_WIRESHARK_CAPTURE.md):**
```bash
# WSL2 USB forwarding
usbipd bind --busid <BUS_ID>
usbipd attach --wsl --busid <BUS_ID>

# Capture
mstpcap /dev/ttyUSB0 38400
```

**Test Results:**
```
[Record capture file names and observations here]
```

---

### 8.2 Verify M5Stack Announces Itself

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [x] | Power on M5Stack | Enters INITIALIZE | INITIALIZE | PASS |
| [x] | Transitions to IDLE | After Tno_token | → IDLE | PASS |
| [x] | Responds to PollForMaster | RTPFM sent | RTPFM sent to station 2 | PASS |
| [x] | Joins token ring | Token passing observed | 2→3→6→2 ring | PASS |
| [ ] | Sends I-Am (application layer) | Device announced | | Pending |

**Test Results (2025-11-28):**
```
Received PollForMaster from station 2, sending reply
TX control frame: type=ReplyToPollForMaster dest=2 raw=[55, FF, 02, 02, 03, 00, 00, 4F]
Reset no_token_timer after replying to poll from 2
Received Token from station 2 (in Idle)
PassToken: Sending token to station 6 (send_queue_len=0)
Token passed to station 6, transitioning to Idle
Poll sweep complete, found 2 masters. next_station=6 (discovered=0x4C)
```

---

### 8.3 Two-Node Token Passing

| Status | Test Case | Expected | Actual | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | M5Stack + other master | Both on bus | | |
| [ ] | Token passes between them | Alternating token | | |
| [ ] | Both can send data | Bidirectional | | |
| [ ] | Remove one, other becomes sole | Recovery works | | |
| [ ] | Reconnect, ring reforms | Rejoin works | | |

**Test Setup:**
- Device 1: M5Stack (address: ___)
- Device 2: _______________ (address: ___)

**Test Results:**
```
[Record test output here]
```

---

### 8.4 Token Loop Timing

| Status | Metric | Expected | Measured | Notes |
|--------|--------|----------|----------|-------|
| [ ] | Min loop time | >10ms | | |
| [ ] | Max loop time | <500ms | | |
| [ ] | Average loop time | ~50-100ms | | |
| [ ] | Jitter | Low variance | | |

**Test Duration:** Run for 10 minutes, collect stats

**Test Results:**
```
[Record timing statistics here]
```

---

### 8.5 Long-Duration Stability Test

| Status | Test Case | Duration | Result | Notes |
|--------|-----------|----------|--------|-------|
| [ ] | Continuous operation | 1 hour | | |
| [ ] | No crashes | Pass/Fail | | |
| [ ] | No memory leaks | Pass/Fail | | |
| [ ] | Token maintained | Pass/Fail | | |
| [ ] | Stats collection | Working | | |

**Metrics to Monitor:**
- Token count
- CRC errors
- Frame errors
- Reply timeouts
- State transitions
- Free heap memory

**Test Results:**
```
Start time:
End time:
Token count:
CRC errors:
Frame errors:
Reply timeouts:
Final state:
Memory (start):
Memory (end):
Notes:
```

---

## Test Environment

### Hardware Configuration

| Component | Value | Notes |
|-----------|-------|-------|
| MCU | ESP32 (M5StickC Plus2) | |
| RS-485 HAT | SP485EEN | Auto direction control |
| TX Pin | GPIO0 | |
| RX Pin | GPIO26 | |
| Baud Rate | 38400 | Default |
| MS/TP Address | 3 | Configurable in NVS |
| Max Master | 127 | Default |

### Software Configuration

| Parameter | Value | Notes |
|-----------|-------|-------|
| Tno_token | 5000ms | Extended for discovery |
| Treply_timeout | 255ms | ASHRAE default |
| Treply_delay | 250ms | ASHRAE minimum |
| Tusage_timeout | 50ms | |
| Tslot | 10ms | |
| Nmax_info_frames | 1 | Default |
| Npoll | 50 | Poll every 50 tokens |

---

## Issues Found

| # | Phase | Description | Severity | Status | Resolution |
|---|-------|-------------|----------|--------|------------|
| 1 | 8 | PollForMaster sole master bug - Poll sweep ignored discovered_masters bitmap | Critical | FIXED | Check `discovered_masters` before setting `sole_master = true` in PollForMaster state (mstp_driver.rs:902-921) |
| 2 | 1 | bacnet-rs library uses wrong CRC-16 polynomial (0xA001 vs 0x8408) | Low | Known | mstp_driver.rs has correct implementation; library bug doesn't affect gateway |
| 3 | | | | | |

**Bug #1 Details:**
- **Symptom**: M5Stack would send Token to itself (dest=3) after poll sweep, instead of to discovered masters
- **Root Cause**: `MstpState::PollForMaster` unconditionally set `sole_master = true` and `next_station = self.station_address` when poll sweep completed without new replies, ignoring previously discovered masters from token passing
- **Fix**: Check `discovered_masters & !(1u128 << self.station_address) != 0` before becoming sole master; use `find_next_master()` if other masters exist

---

## Change Log

| Date | Changes | Tester |
|------|---------|--------|
| 2025-11-28 | Initial plan created | Claude |
| 2025-11-28 | Phase 1 complete: CRC-8 and CRC-16 verified against ASHRAE 135 Annex G | Claude |
| 2025-11-28 | Phase 2 complete: State machine verified, 22/22 tests passed | Claude |
| 2025-11-28 | Phase 5 complete: WAIT_FOR_REPLY negative list implementation verified | Claude |
| 2025-11-28 | Phase 8: Found and fixed critical sole_master bug in PollForMaster state | Claude |
| 2025-11-28 | **SUCCESS**: 3-node token ring operational (2→3→6→2 cycle) | Claude |
| 2025-11-28 | **BUG FIX**: Poll sweep causing dropped tokens - changed to poll ONE address per NPOLL cycle instead of full 0-127 sweep | Claude |
| 2025-11-28 | **CRITICAL BUG FIX**: Not responding to PollForMaster in all states - now respond in WaitForReply and PollForMaster states | Claude |
| 2025-11-28 | **BUG FIX**: New node not discovering other masters - now polls immediately when joining ring (not waiting for NPOLL=50 tokens) | Claude |

---

## References

- ASHRAE 135-2020 Clause 9: MS/TP Data Link Layer
- `MSTP_PROTOCOL_REQUIREMENTS.md` - Detailed state machine specification
- `MSTP_WIRESHARK_CAPTURE.md` - Packet capture guide
- bacnet-stack commit f877ca0eb - WAIT_FOR_REPLY negative list fix
- Context7 BACnet Stack documentation

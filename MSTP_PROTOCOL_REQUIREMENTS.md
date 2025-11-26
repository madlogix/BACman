# BACnet MS/TP Protocol Requirements (ASHRAE 135 Clause 9)

## Overview

This document details the MS/TP (Master-Slave/Token-Passing) protocol requirements from ASHRAE 135 standard for implementing a complete state machine in Rust for the ESP32-based BACnet gateway.

## 1. Complete State Machine States

### Master Node States (per ASHRAE 135 Clause 9.6.1)

| State | Description | Entry Condition | Exit Conditions |
|-------|-------------|-----------------|-----------------|
| **INITIALIZE** | Initial state on power-up or reset | System start, reset | Tno_token expires → IDLE |
| **IDLE** | Waiting to receive token or generate one | Token passed, done receiving | Receive token → USE_TOKEN<br>Tno_token expires → POLL_FOR_MASTER<br>Receive data request → ANSWER_DATA_REQUEST |
| **USE_TOKEN** | Master has token, can send frames | Received token | Sent max frames → DONE_WITH_TOKEN<br>No frames to send → DONE_WITH_TOKEN<br>Tusage_timeout → DONE_WITH_TOKEN |
| **WAIT_FOR_REPLY** | Waiting for reply to Data-Expecting-Reply | Sent frame expecting reply | Receive valid reply → DONE_WITH_TOKEN<br>Treply_timeout → DONE_WITH_TOKEN<br>Receive unexpected frame → IDLE |
| **DONE_WITH_TOKEN** | Finished using token, preparing to pass | Completed token use | Next station determined → PASS_TOKEN<br>Need to poll → POLL_FOR_MASTER |
| **PASS_TOKEN** | Passing token to next station | Done with token | Token sent → IDLE |
| **NO_TOKEN** | Lost token, trying to recover | Token lost, error condition | Tno_token expires → POLL_FOR_MASTER |
| **POLL_FOR_MASTER** | Polling for other masters | No token timeout | Receive ReplyToPollForMaster → PASS_TOKEN<br>Tslot timeout, more to poll → POLL_FOR_MASTER<br>Polled all, found none → USE_TOKEN (sole master)<br>Tno_token overall timeout → IDLE |
| **ANSWER_DATA_REQUEST** | Slave responding to master | Received data request frame | Reply sent → IDLE<br>No reply to send → IDLE |

### Critical State Details

#### INITIALIZE State
- **Purpose**: Allow network to stabilize after startup
- **Actions**:
  - Reset all state variables
  - Clear send/receive queues
  - Start silence timer
- **Exit**: After Tno_token of line silence (typically 500ms)

#### IDLE State
- **Purpose**: Wait for token or data request
- **Actions**: Monitor for incoming frames
- **Transitions**:
  - Token addressed to us → USE_TOKEN
  - Data-Expecting-Reply addressed to us → ANSWER_DATA_REQUEST
  - Tno_token expires without token → POLL_FOR_MASTER
  - Broadcast or data not expecting reply → process and stay in IDLE

#### USE_TOKEN State
- **Purpose**: Send queued frames while holding token
- **Actions**:
  - Send up to Nmax_info_frames (typically 1)
  - If sending Data-Expecting-Reply → WAIT_FOR_REPLY
  - If no more frames or limit reached → DONE_WITH_TOKEN
- **Timeout**: Tusage_timeout (50ms) - maximum time to hold token

#### WAIT_FOR_REPLY State (CRITICAL - See Implementation Note)
- **Purpose**: Wait for reply to Data-Expecting-Reply frame
- **Entry**: Immediately after sending Data-Expecting-Reply frame
- **Actions**:
  - Start reply timer (Treply_timeout)
  - Monitor for reply frame
- **Reply Acceptance (IMPORTANT - Negative List Approach)**:
  - **REJECT** only these frames (non-reply frames):
    - Token
    - PollForMaster
    - ReplyToPollForMaster
    - TestRequest
  - **ACCEPT** all other frames as valid replies:
    - BACnetDataNotExpectingReply
    - TestResponse
    - ReplyPostponed
    - Any unknown/proprietary frame types (for forward compatibility)
    - Complex-ACK segments
- **Exit Conditions**:
  - Receive valid reply → DONE_WITH_TOKEN
  - Treply_timeout expires → DONE_WITH_TOKEN
  - Receive non-reply frame → IDLE (ReceivedUnexpectedFrame)

**CRITICAL IMPLEMENTATION BUG FIX**: Earlier implementations used a "positive list" (only accepting known reply types) which caused segmented Complex-ACK frames and proprietary frames to be dropped. The standard requires a "negative list" approach - reject only frames that are definitively NOT replies. (Reference: bacnet-stack commit f877ca0eb)

#### ANSWER_DATA_REQUEST State
- **Purpose**: Allow master to reply to received Data-Expecting-Reply
- **Entry**: Received Data-Expecting-Reply addressed to us while in IDLE
- **Actions**:
  - Process request
  - Send reply frame if needed
  - Wait Treply_delay before sending (minimum 250ms)
- **Exit**: After sending reply or determining no reply needed → IDLE

#### POLL_FOR_MASTER State
- **Purpose**: Discover other master nodes on network
- **Entry**: From DONE_WITH_TOKEN or NO_TOKEN after Tno_token
- **Actions**:
  - Send PollForMaster to poll_station
  - Wait Tslot for ReplyToPollForMaster
  - If no reply, increment poll_station
  - If poll_station == This_Station, we're sole master
- **Variables**:
  - poll_station: Address being polled
  - next_station: Next station to receive token
- **Exit**:
  - Receive ReplyToPollForMaster → set next_station, go to PASS_TOKEN
  - Polled all addresses, no replies → set sole_master flag, go to USE_TOKEN
  - Overall Tno_token expires → IDLE

## 2. Required Timing Parameters (ASHRAE 135 Clause 9)

### Frame-Level Timeouts

| Parameter | Symbol | Default Value | Description | Clause |
|-----------|--------|---------------|-------------|--------|
| **Frame Abort Timeout** | Tframe_abort | 60 bit times (varies by baud) | Maximum silence within frame before considering it aborted | 9.2.2 |
| **Turnaround Time** | Tturnaround | 40 bit times | Minimum time to switch from receive to transmit | 9.2.3 |

### Baud Rate Specific Timing (bit times at different baud rates)

| Baud Rate | Tframe_abort | Tturnaround |
|-----------|--------------|-------------|
| 9600 bps | 6.25 ms | 4.17 ms |
| 19200 bps | 3.125 ms | 2.08 ms |
| 38400 bps | 1.5625 ms | 1.04 ms |
| 76800 bps | 0.78125 ms | 0.52 ms |

### State Machine Timeouts

| Parameter | Symbol | Default Value | Range | Description | Adjustable |
|-----------|--------|---------------|-------|-------------|-----------|
| **Reply Timeout** | Treply_timeout | 255 ms | 250-300 ms | Maximum wait for reply after sending Data-Expecting-Reply | Yes |
| **Reply Delay** | Treply_delay | 250 ms | 250-255 ms | Minimum delay before sending reply in ANSWER_DATA_REQUEST | No |
| **Usage Timeout** | Tusage_timeout | 20-100 ms | 15-100 ms | Maximum time to hold token in USE_TOKEN state | Yes |
| **No Token Timeout** | Tno_token | 500 ms | 500-1000 ms | Time without token before generating one | Yes |
| **Slot Time** | Tslot | 10 ms | 5-15 ms | Time to wait for reply to PollForMaster | Yes |
| **Usage Delay** | Tusage_delay | 15 ms | N/A | Delay before using token (optional) | Yes |

### Timeout Usage by State

```
INITIALIZE → IDLE: Wait Tno_token (500ms)
IDLE → POLL_FOR_MASTER: After Tno_token (500ms) without token
USE_TOKEN: Exit after Tusage_timeout (50ms) if still sending
WAIT_FOR_REPLY: Exit after Treply_timeout (255ms)
ANSWER_DATA_REQUEST: Wait Treply_delay (250ms) before replying
POLL_FOR_MASTER: Wait Tslot (10ms) per station
```

## 3. Token Passing Algorithm

### Token Management Variables

```rust
// Configuration (set at initialization)
This_Station: u8        // Our address (0-127)
Nmax_master: u8         // Highest master address (default 127)
Nmax_info_frames: u8    // Max frames per token (default 1)

// Runtime state
TokenCount: u8          // Tokens received counter
FrameCount: u8          // Frames sent this token
RetryCount: u8          // Token retry attempts
EventCount: u8          // Events processed counter
NS (Next_Station): u8   // Next station to receive token
PS (Poll_Station): u8   // Station currently being polled
SoleMaster: bool        // True if we're the only master
```

### Token Passing Flow (Normal Operation)

```
1. IDLE: Wait for token
   ↓
2. Receive Token(destination=This_Station)
   ↓
3. USE_TOKEN: FrameCount = 0
   ↓
4. Send frames while FrameCount < Nmax_info_frames
   ↓
5. DONE_WITH_TOKEN
   ↓
6. Determine next_station (NS)
   ↓
7. PASS_TOKEN: Send Token(destination=NS)
   ↓
8. IDLE: TokenCount++
```

### Next Station Calculation

```rust
// In DONE_WITH_TOKEN state:
fn calculate_next_station(&mut self) {
    // Start with station after us
    let mut ns = (self.This_Station + 1) % (self.Nmax_master + 1);

    if self.SoleMaster {
        // We're the only master, token to ourselves
        ns = self.This_Station;
    } else {
        // Normal operation: pass to next in ring
        // (poll_station management in POLL_FOR_MASTER updates this)
        ns = self.NS;
    }

    self.next_station = ns;
}
```

### Token Rotation Guarantee

The algorithm guarantees:
1. Token visits all active masters in sequence
2. Each master gets fair access (limited by Nmax_info_frames)
3. Failed masters are detected and skipped via polling
4. Ring reforms if masters join/leave

## 4. Poll For Master Procedure

### Purpose
Discover and maintain list of active master nodes on the network.

### When to Poll

1. **From DONE_WITH_TOKEN**:
   - If `TokenCount >= Npoll` (typically every 50 tokens)
   - Poll for new masters after this station

2. **From NO_TOKEN**:
   - Lost token, need to regenerate
   - Poll entire ring to find active masters

3. **From IDLE**:
   - Tno_token expired without receiving token
   - Network may be dead, try to regenerate token

### Poll For Master Algorithm

```rust
fn poll_for_master_state(&mut self) {
    // Initial entry: set poll_station
    if self.entering_state {
        self.PS = (self.NS + 1) % (self.Nmax_master + 1);
        self.entering_state = false;
    }

    // Send PollForMaster frame
    self.send_frame(
        FrameType::PollForMaster,
        destination: self.PS,
        source: self.This_Station,
        data: []
    );

    // Start slot timer
    self.slot_timer = now();

    // Wait for reply or timeout
    loop {
        if received_frame(FrameType::ReplyToPollForMaster, source: addr) {
            // Found a master!
            self.NS = addr;  // This is our next_station
            self.state = State::PASS_TOKEN;
            return;
        }

        if timeout(self.slot_timer, Tslot) {
            // No reply, try next address
            self.PS = (self.PS + 1) % (self.Nmax_master + 1);

            if self.PS == self.This_Station {
                // Polled everyone, no response - we're sole master
                self.SoleMaster = true;
                self.NS = self.This_Station;
                self.state = State::USE_TOKEN;
                return;
            }

            // Poll next station
            break;  // Loop again to send next poll
        }

        if timeout(self.no_token_timer, Tno_token) {
            // Overall timeout, give up
            self.state = State::IDLE;
            return;
        }
    }
}
```

### Responding to Poll For Master

When a master receives `PollForMaster` addressed to it:

```rust
fn handle_poll_for_master(&mut self, source: u8) {
    if self.state == State::IDLE {
        // Reply immediately
        self.send_frame(
            FrameType::ReplyToPollForMaster,
            destination: source,
            source: self.This_Station,
            data: []
        );
        // Stay in IDLE, we don't have the token
    }
}
```

### Poll Frequency

```rust
// Configuration
Npoll: u8 = 50  // Poll every 50 tokens

// In DONE_WITH_TOKEN:
if self.TokenCount >= Npoll {
    self.TokenCount = 0;
    // Do full poll sequence
    self.state = State::POLL_FOR_MASTER;
} else {
    // Normal token pass
    self.state = State::PASS_TOKEN;
}
```

## 5. WAIT_FOR_REPLY State Behavior (CRITICAL)

### State Entry
Enter immediately after sending BACnetDataExpectingReply frame from USE_TOKEN state.

### Actions in State

1. **Start Reply Timer**
```rust
fn enter_wait_for_reply(&mut self) {
    self.reply_timer = Some(Instant::now());
    self.state = State::WaitForReply;
}
```

2. **Monitor for Reply**
```rust
fn wait_for_reply_handler(&mut self, frame: &Frame) {
    // Check if frame is addressed to us
    if frame.destination != self.This_Station &&
       frame.destination != BROADCAST_ADDRESS {
        return;  // Not for us, ignore
    }

    // NEGATIVE LIST APPROACH (CRITICAL)
    match frame.frame_type {
        // These are NOT replies - unexpected frames
        FrameType::Token |
        FrameType::PollForMaster |
        FrameType::ReplyToPollForMaster |
        FrameType::TestRequest => {
            // ReceivedUnexpectedFrame event
            log::warn!("Unexpected frame in WAIT_FOR_REPLY");
            self.state = State::IDLE;
        }

        // All other frame types are accepted as valid replies
        _ => {
            // ReceivedReply event
            // This includes:
            // - BACnetDataNotExpectingReply
            // - TestResponse
            // - ReplyPostponed
            // - Unknown/proprietary frame types
            // - Segmented responses

            // Pass to upper layer
            self.receive_queue.push_back((frame.data.clone(), frame.source));

            // Done with this exchange
            self.state = State::DONE_WITH_TOKEN;
        }
    }
}
```

3. **Check Timeout**
```rust
fn check_reply_timeout(&mut self) {
    if let Some(start) = self.reply_timer {
        if start.elapsed() > Duration::from_millis(self.Treply_timeout) {
            // Timeout waiting for reply
            log::warn!("Reply timeout");
            self.reply_timer = None;
            self.state = State::DONE_WITH_TOKEN;
        }
    }
}
```

### State Exit Conditions

| Exit Condition | Event | Next State | Actions |
|----------------|-------|------------|---------|
| Valid reply received | ReceivedReply | DONE_WITH_TOKEN | Queue data for upper layer |
| Treply_timeout expires | ReplyTimeout | DONE_WITH_TOKEN | May retry or abandon |
| Non-reply frame received | ReceivedUnexpectedFrame | IDLE | Log error, release token |

### Common Implementation Errors

**WRONG - Positive List (breaks on unknown frames)**:
```rust
// BAD: Only accepts known reply types
match frame.frame_type {
    FrameType::BacnetDataNotExpectingReply |
    FrameType::TestResponse |
    FrameType::ReplyPostponed => {
        // Accept as reply
    }
    _ => {
        // WRONG: Rejects proprietary and segmented frames!
        self.state = State::IDLE;
    }
}
```

**CORRECT - Negative List (accepts all valid replies)**:
```rust
// GOOD: Only rejects frames that are definitively not replies
match frame.frame_type {
    FrameType::Token |
    FrameType::PollForMaster |
    FrameType::ReplyToPollForMaster |
    FrameType::TestRequest => {
        // Reject non-reply frames
        self.state = State::IDLE;
    }
    _ => {
        // Accept everything else as potential reply
        self.receive_queue.push_back((frame.data, frame.source));
        self.state = State::DONE_WITH_TOKEN;
    }
}
```

## 6. ANSWER_DATA_REQUEST State Behavior

### State Entry
Enter when in IDLE state and receive BACnetDataExpectingReply addressed to us.

### Required Actions

1. **Process the Request**
```rust
fn handle_data_expecting_reply(&mut self, frame: Frame) {
    if self.state != State::IDLE {
        // Can only answer from IDLE
        return;
    }

    // Move to ANSWER_DATA_REQUEST
    self.state = State::ANSWER_DATA_REQUEST;
    self.reply_delay_timer = Instant::now();

    // Store request for processing
    self.pending_request = Some((frame.data, frame.source));
}
```

2. **Wait Reply Delay**
Must wait at least Treply_delay (250ms) before sending reply. This gives:
- Master time to enter WAIT_FOR_REPLY state
- Network time to stabilize
- Prevents collisions

```rust
fn answer_data_request_handler(&mut self) {
    // Wait minimum reply delay
    if self.reply_delay_timer.elapsed() < Duration::from_millis(self.Treply_delay) {
        return;  // Still waiting
    }

    // Process request and generate reply
    if let Some((request_data, destination)) = self.pending_request.take() {
        match self.process_request(&request_data) {
            Some(reply_data) => {
                // Send reply
                self.send_frame(
                    FrameType::BacnetDataNotExpectingReply,
                    destination,
                    self.This_Station,
                    &reply_data
                ).ok();
            }
            None => {
                // No reply needed (could send ReplyPostponed)
            }
        }
    }

    // Return to IDLE
    self.state = State::IDLE;
}
```

3. **Send Reply or ReplyPostponed**
```rust
enum ReplyAction {
    ImmediateReply(Vec<u8>),
    Postponed,
    NoReply,
}

fn process_request(&mut self, data: &[u8]) -> ReplyAction {
    // Parse APDU request
    match parse_apdu(data) {
        Ok(apdu) => {
            // Process service request
            match self.handle_service_request(&apdu) {
                Ok(response) => ReplyAction::ImmediateReply(response),
                Err(WouldBlock) => ReplyAction::Postponed,
                Err(_) => ReplyAction::NoReply,  // Send error
            }
        }
        Err(_) => ReplyAction::NoReply,
    }
}
```

### State Exit
Always exit to IDLE after:
- Sending reply
- Sending ReplyPostponed
- Determining no reply needed
- Error condition

### Timing Constraints

- **Minimum reply delay**: Treply_delay (250ms)
- **Maximum reply delay**: Should reply within Treply_timeout (255ms) of master
- If can't reply in time, send ReplyPostponed frame

## 7. Error Recovery Procedures

### Line Silence Detection

```rust
fn check_frame_abort(&mut self) {
    if self.receiving_frame {
        let silence = self.last_byte_time.elapsed();
        let abort_time = Duration::from_micros(self.Tframe_abort_us);

        if silence > abort_time {
            // Frame aborted due to silence
            log::warn!("Frame abort: {:?} silence", silence);
            self.receiving_frame = false;
            self.rx_buffer.clear();

            // May need to recover token
            if self.state == State::WAIT_FOR_REPLY {
                self.state = State::DONE_WITH_TOKEN;
            }
        }
    }
}
```

### CRC Error Recovery

```rust
fn handle_crc_error(&mut self, error_type: CrcErrorType) {
    match error_type {
        CrcErrorType::HeaderCrc => {
            // Corrupt header, can't trust frame
            log::warn!("Header CRC error");
            self.rx_buffer.clear();
        }
        CrcErrorType::DataCrc => {
            // Corrupt data, but header OK
            log::warn!("Data CRC error from {}", frame.source);

            // If we were waiting for reply, this might be it
            if self.state == State::WAIT_FOR_REPLY {
                // Could retry or treat as timeout
                self.retry_count += 1;
                if self.retry_count >= MAX_RETRIES {
                    self.state = State::DONE_WITH_TOKEN;
                }
            }
        }
    }
}
```

### Lost Token Recovery

```rust
fn handle_lost_token(&mut self) {
    // Haven't received token in Tno_token
    if self.no_token_timer.elapsed() > Duration::from_millis(self.Tno_token) {
        log::warn!("Lost token, attempting recovery");

        // Try to regenerate token by polling
        self.state = State::POLL_FOR_MASTER;
        self.PS = (self.This_Station + 1) % (self.Nmax_master + 1);
        self.no_token_timer = Instant::now();
    }
}
```

### Invalid Frame Recovery

```rust
fn handle_invalid_frame(&mut self, frame_bytes: &[u8]) {
    log::warn!("Invalid frame received: {} bytes", frame_bytes.len());

    // Dump for debugging
    log::trace!("Frame hex: {:02X?}", frame_bytes);

    // Clear buffer and continue
    self.rx_buffer.clear();

    // State recovery depends on current state
    match self.state {
        State::WAIT_FOR_REPLY => {
            // Might be corrupted reply, treat as timeout
            self.state = State::DONE_WITH_TOKEN;
        }
        State::POLL_FOR_MASTER => {
            // Might be corrupted reply to poll
            // Continue polling
        }
        _ => {
            // Stay in current state
        }
    }
}
```

### Receive Buffer Overflow

```rust
const MAX_RX_BUFFER: usize = MSTP_HEADER_SIZE + MSTP_MAX_DATA_LENGTH + 2;

fn check_buffer_overflow(&mut self) {
    if self.rx_buffer.len() > MAX_RX_BUFFER * 2 {
        // Buffer growing without finding valid frame
        log::error!("RX buffer overflow, clearing");

        // Keep only last bytes in case we're mid-frame
        let keep = MAX_RX_BUFFER;
        if self.rx_buffer.len() > keep {
            self.rx_buffer.drain(..self.rx_buffer.len() - keep);
        }
    }
}
```

### Send Queue Overflow

```rust
const MAX_SEND_QUEUE: usize = 16;

pub fn send_frame(&mut self, data: &[u8], dest: u8) -> Result<(), MstpError> {
    if self.send_queue.len() >= MAX_SEND_QUEUE {
        log::warn!("Send queue full, dropping frame");
        return Err(MstpError::BufferFull);
    }

    self.send_queue.push_back((data.to_vec(), dest));
    Ok(())
}
```

### Network Restart Recovery

```rust
pub fn reset(&mut self) {
    log::info!("Resetting MS/TP state machine");

    // Clear all state
    self.state = State::INITIALIZE;
    self.TokenCount = 0;
    self.FrameCount = 0;
    self.RetryCount = 0;
    self.EventCount = 0;
    self.SoleMaster = false;

    // Clear queues (optionally)
    // self.send_queue.clear();
    // self.receive_queue.clear();

    // Clear buffers
    self.rx_buffer.clear();

    // Reset timers
    self.silence_timer = Instant::now();
    self.reply_timer = None;

    // Initialize next_station
    self.NS = (self.This_Station + 1) % (self.Nmax_master + 1);
}
```

## 8. Implementation Checklist

### Core State Machine
- [ ] All 9 states implemented (INITIALIZE, IDLE, USE_TOKEN, WAIT_FOR_REPLY, DONE_WITH_TOKEN, PASS_TOKEN, NO_TOKEN, POLL_FOR_MASTER, ANSWER_DATA_REQUEST)
- [ ] State transition logic per standard
- [ ] State entry/exit actions

### Timing
- [ ] Tframe_abort calculated per baud rate
- [ ] Treply_timeout (255ms default)
- [ ] Treply_delay (250ms minimum)
- [ ] Tusage_timeout (50ms default)
- [ ] Tno_token (500ms default)
- [ ] Tslot (10ms default)
- [ ] Tturnaround implemented

### Token Management
- [ ] Token passing to next_station
- [ ] Next station calculation
- [ ] Sole master detection
- [ ] Token count tracking
- [ ] Nmax_info_frames enforcement

### Polling
- [ ] Poll for master procedure
- [ ] Reply to poll for master
- [ ] Poll frequency (Npoll)
- [ ] Poll timeout handling

### Frame Handling
- [ ] Send token frames
- [ ] Send data frames (expecting/not expecting reply)
- [ ] Receive token frames
- [ ] Receive data frames
- [ ] WAIT_FOR_REPLY negative list logic (CRITICAL)

### Error Recovery
- [ ] Frame abort detection
- [ ] Header CRC error handling
- [ ] Data CRC error handling
- [ ] Lost token recovery
- [ ] Buffer overflow protection
- [ ] Invalid frame handling

### Testing
- [ ] State machine unit tests
- [ ] Token passing with multiple masters
- [ ] Sole master operation
- [ ] Reply timeout handling
- [ ] Poll for master procedure
- [ ] Error injection tests
- [ ] Network analyzer validation

## 9. Reference Implementation Pseudocode

```rust
pub struct MstpStateMachine {
    // Configuration
    This_Station: u8,
    Nmax_master: u8,
    Nmax_info_frames: u8,

    // Timeouts (ms)
    Tframe_abort: u64,
    Treply_timeout: u64,
    Treply_delay: u64,
    Tusage_timeout: u64,
    Tno_token: u64,
    Tslot: u64,
    Npoll: u8,

    // State
    state: MstpState,
    TokenCount: u8,
    FrameCount: u8,
    RetryCount: u8,
    NS: u8,  // Next_Station
    PS: u8,  // Poll_Station
    SoleMaster: bool,

    // Timers
    silence_timer: Instant,
    reply_timer: Option<Instant>,
    usage_timer: Option<Instant>,
    slot_timer: Option<Instant>,
    no_token_timer: Instant,

    // Queues
    send_queue: VecDeque<(Vec<u8>, u8)>,
    receive_queue: VecDeque<(Vec<u8>, u8)>,
}

impl MstpStateMachine {
    pub fn run_state_machine(&mut self) {
        match self.state {
            State::INITIALIZE => self.handle_initialize(),
            State::IDLE => self.handle_idle(),
            State::USE_TOKEN => self.handle_use_token(),
            State::WAIT_FOR_REPLY => self.handle_wait_for_reply(),
            State::DONE_WITH_TOKEN => self.handle_done_with_token(),
            State::PASS_TOKEN => self.handle_pass_token(),
            State::NO_TOKEN => self.handle_no_token(),
            State::POLL_FOR_MASTER => self.handle_poll_for_master(),
            State::ANSWER_DATA_REQUEST => self.handle_answer_data_request(),
        }
    }

    fn handle_initialize(&mut self) {
        // Wait for line silence
        if self.silence_timer.elapsed() >= Duration::from_millis(self.Tno_token) {
            self.state = State::IDLE;
            self.no_token_timer = Instant::now();
        }
    }

    fn handle_idle(&mut self) {
        // Check for token timeout
        if self.no_token_timer.elapsed() >= Duration::from_millis(self.Tno_token) {
            self.state = State::POLL_FOR_MASTER;
            self.PS = (self.This_Station + 1) % (self.Nmax_master + 1);
        }

        // Process received frames (handled in frame_received callback)
    }

    fn handle_use_token(&mut self) {
        // Send frames
        if self.FrameCount < self.Nmax_info_frames {
            if let Some((data, dest)) = self.send_queue.pop_front() {
                // Determine if expecting reply
                let expecting_reply = self.needs_reply(&data);

                self.send_data_frame(&data, dest, expecting_reply);
                self.FrameCount += 1;

                if expecting_reply {
                    self.reply_timer = Some(Instant::now());
                    self.state = State::WAIT_FOR_REPLY;
                    return;
                }
            } else {
                // No frames to send
                self.state = State::DONE_WITH_TOKEN;
            }
        } else {
            // Sent max frames
            self.state = State::DONE_WITH_TOKEN;
        }

        // Check usage timeout
        if let Some(timer) = self.usage_timer {
            if timer.elapsed() >= Duration::from_millis(self.Tusage_timeout) {
                self.state = State::DONE_WITH_TOKEN;
            }
        }
    }

    fn handle_wait_for_reply(&mut self) {
        // Check timeout
        if let Some(timer) = self.reply_timer {
            if timer.elapsed() >= Duration::from_millis(self.Treply_timeout) {
                self.reply_timer = None;
                self.state = State::DONE_WITH_TOKEN;
            }
        }

        // Frame reception handled in callback
    }

    fn handle_done_with_token(&mut self) {
        // Determine next action
        if self.TokenCount >= self.Npoll {
            self.TokenCount = 0;
            self.state = State::POLL_FOR_MASTER;
            self.PS = (self.NS + 1) % (self.Nmax_master + 1);
        } else {
            self.state = State::PASS_TOKEN;
        }
    }

    fn handle_pass_token(&mut self) {
        // Send token to next_station
        self.send_token(self.NS);
        self.state = State::IDLE;
        self.no_token_timer = Instant::now();
    }

    fn handle_poll_for_master(&mut self) {
        // Send poll
        self.send_poll_for_master(self.PS);
        self.slot_timer = Some(Instant::now());

        // Wait for reply (handled in callback)
        // On timeout, increment PS and try again
        if let Some(timer) = self.slot_timer {
            if timer.elapsed() >= Duration::from_millis(self.Tslot) {
                self.PS = (self.PS + 1) % (self.Nmax_master + 1);

                if self.PS == self.This_Station {
                    // Sole master
                    self.SoleMaster = true;
                    self.NS = self.This_Station;
                    self.state = State::USE_TOKEN;
                    self.usage_timer = Some(Instant::now());
                }

                self.slot_timer = None;
            }
        }

        // Overall timeout
        if self.no_token_timer.elapsed() >= Duration::from_millis(self.Tno_token) {
            self.state = State::IDLE;
            self.no_token_timer = Instant::now();
        }
    }

    fn handle_answer_data_request(&mut self) {
        // Implementation depends on application
        // Must wait Treply_delay before replying

        // Simplified:
        self.state = State::IDLE;
    }

    // Frame reception callback
    pub fn frame_received(&mut self, frame: Frame) {
        match self.state {
            State::IDLE => {
                match frame.frame_type {
                    FrameType::Token if frame.destination == self.This_Station => {
                        self.TokenCount += 1;
                        self.FrameCount = 0;
                        self.state = State::USE_TOKEN;
                        self.usage_timer = Some(Instant::now());
                    }
                    FrameType::PollForMaster if frame.destination == self.This_Station => {
                        self.send_reply_to_poll(frame.source);
                    }
                    FrameType::BacnetDataExpectingReply
                        if frame.destination == self.This_Station => {
                        self.state = State::ANSWER_DATA_REQUEST;
                    }
                    FrameType::BacnetDataNotExpectingReply
                        if frame.destination == self.This_Station
                        || frame.destination == BROADCAST => {
                        self.receive_queue.push_back((frame.data, frame.source));
                    }
                    _ => {}
                }
            }
            State::WAIT_FOR_REPLY => {
                // CRITICAL: Negative list approach
                match frame.frame_type {
                    FrameType::Token |
                    FrameType::PollForMaster |
                    FrameType::ReplyToPollForMaster |
                    FrameType::TestRequest => {
                        // Not a reply
                        self.reply_timer = None;
                        self.state = State::IDLE;
                    }
                    _ => {
                        // Valid reply
                        if frame.destination == self.This_Station
                            || frame.destination == BROADCAST {
                            self.receive_queue.push_back((frame.data, frame.source));
                            self.reply_timer = None;
                            self.state = State::DONE_WITH_TOKEN;
                        }
                    }
                }
            }
            State::POLL_FOR_MASTER => {
                if frame.frame_type == FrameType::ReplyToPollForMaster {
                    self.NS = frame.source;
                    self.slot_timer = None;
                    self.state = State::PASS_TOKEN;
                }
            }
            _ => {}
        }
    }
}
```

## 10. Critical Implementation Notes

### WAIT_FOR_REPLY Negative List (MOST IMPORTANT)
This is the single most important implementation detail:
- DO NOT use a positive list of accepted reply types
- DO use a negative list of rejected non-reply types
- Accept unknown frame types as potential replies
- This enables forward compatibility with new frame types

### Timing Precision
- Use high-resolution timers for Tframe_abort and Tturnaround
- These are bit-time based and vary with baud rate
- Use millisecond timers for state machine timeouts

### Token Passing Fairness
- Enforce Nmax_info_frames strictly
- Implement Tusage_timeout to prevent token hogging
- Poll regularly (Npoll) to discover new masters

### Buffer Management
- Limit send queue size to prevent memory exhaustion
- Implement circular receive buffer with overflow protection
- Clear buffers on frame abort

### Error Handling Philosophy
- Fail gracefully on CRC errors
- Always recover to IDLE or token regeneration
- Log errors for debugging
- Don't crash on malformed frames

## 11. Testing Recommendations

### Unit Tests
- Test each state in isolation
- Test all state transitions
- Test timeout handling
- Test frame encoding/decoding
- Test CRC calculations

### Integration Tests
- Two-node token passing
- Multi-node token ring
- Sole master operation
- Token loss recovery
- Poll for master procedure
- Data exchange with replies

### Real Hardware Tests
- Test with BACnet protocol analyzer
- Test with commercial BACnet devices
- Test error conditions (unplug cables, noise injection)
- Performance testing (latency, throughput)
- Long-duration stability tests

## 12. References

- ASHRAE 135-2020 Clause 9: MS/TP Data Link Layer
- BACnet Stack commit f877ca0eb: WAIT_FOR_REPLY negative list fix
- ASHRAE 135-2020 Clause 9.2: Frame Format
- ASHRAE 135-2020 Clause 9.4: Media Access Control
- ASHRAE 135-2020 Clause 9.6: State Machine Specification

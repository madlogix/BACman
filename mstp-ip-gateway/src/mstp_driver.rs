//! MS/TP Driver for ESP32 RS-485 communication
//!
//! This module implements the MS/TP token-passing protocol over RS-485 UART
//! according to ASHRAE 135 Clause 9.
//!
//! Note: The M5Stack RS-485 HAT uses automatic direction control via the SP485EEN
//! chip's built-in transceiver circuit - no manual GPIO direction pin needed.

use esp_idf_svc::hal::uart::UartDriver;
use log::{debug, info, trace, warn};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// MS/TP frame constants
const MSTP_PREAMBLE_55: u8 = 0x55;
const MSTP_PREAMBLE_FF: u8 = 0xFF;
const MSTP_HEADER_SIZE: usize = 8;
const MSTP_MAX_DATA_LENGTH: usize = 501;
const MSTP_BROADCAST_ADDRESS: u8 = 255;

// Polling configuration
const NPOLL: u8 = 255; // Poll for new masters every 255 tokens (reduced frequency for debugging)
const MAX_RETRY: u8 = 3; // Maximum retries for failed transmissions

/// MS/TP frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MstpFrameType {
    Token = 0,
    PollForMaster = 1,
    ReplyToPollForMaster = 2,
    TestRequest = 3,
    TestResponse = 4,
    BacnetDataExpectingReply = 5,
    BacnetDataNotExpectingReply = 6,
    ReplyPostponed = 7,
}

impl MstpFrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Token),
            1 => Some(Self::PollForMaster),
            2 => Some(Self::ReplyToPollForMaster),
            3 => Some(Self::TestRequest),
            4 => Some(Self::TestResponse),
            5 => Some(Self::BacnetDataExpectingReply),
            6 => Some(Self::BacnetDataNotExpectingReply),
            7 => Some(Self::ReplyPostponed),
            _ => None,
        }
    }
}

/// MS/TP node state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MstpState {
    Initialize,
    Idle,
    UseToken,
    WaitForReply,
    PassToken,
    NoToken,
    PollForMaster,
    AnswerDataRequest,
    DoneWithToken,
}

/// MS/TP driver error
#[derive(Debug)]
#[allow(dead_code)]
pub enum MstpError {
    IoError(String),
    InvalidFrame,
    CrcError,
    Timeout,
    BufferFull,
    BusyMedium, // Medium is busy (another station is transmitting)
}

impl std::fmt::Display for MstpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MstpError::IoError(s) => write!(f, "I/O error: {}", s),
            MstpError::InvalidFrame => write!(f, "Invalid frame"),
            MstpError::CrcError => write!(f, "CRC error"),
            MstpError::Timeout => write!(f, "Timeout"),
            MstpError::BufferFull => write!(f, "Buffer full"),
            MstpError::BusyMedium => write!(f, "Medium busy"),
        }
    }
}

/// MS/TP Driver for ESP32
/// Uses M5Stack RS-485 HAT with automatic direction control (no GPIO needed)
#[allow(dead_code)]
pub struct MstpDriver<'a> {
    uart: UartDriver<'a>,

    // Node configuration
    station_address: u8,
    max_master: u8,
    max_info_frames: u8,

    // State machine
    state: MstpState,
    token_count: u8,
    frame_count: u8,

    // Debug counters
    rx_poll_count: u64,
    rx_frame_count: u64,
    tx_frame_count: u64,
    last_rx_status_time: Instant,
    retry_count: u8,
    next_station: u8,
    poll_station: u8,
    sole_master: bool,

    // Token loop tracking
    last_token_time: Option<Instant>,
    token_loop_time_ms: u32,
    discovered_masters: u128, // Bitmap of discovered master addresses (0-127)

    // Error counters
    crc_errors: u64,
    frame_errors: u64,
    reply_timeouts: u64,
    tokens_received: u64,
    token_pass_failures: u64,

    // Token loop timing (for min/max/avg calculation)
    token_loop_min_ms: u32,
    token_loop_max_ms: u32,
    token_loop_sum_ms: u64,      // Sum for calculating average
    token_loop_count: u64,       // Count for calculating average

    // Queues
    send_queue: VecDeque<(Vec<u8>, u8, bool)>, // (data, destination, expecting_reply)
    receive_queue: VecDeque<(Vec<u8>, u8)>, // (data, source)

    // Receive buffer
    rx_buffer: Vec<u8>,

    // Pending request for AnswerDataRequest state
    pending_request: Option<(Vec<u8>, u8)>, // (data, source)

    // Timing
    silence_timer: Instant,
    reply_timer: Option<Instant>,
    usage_timer: Option<Instant>,
    reply_delay_timer: Option<Instant>,
    no_token_timer: Instant,

    // Timeouts (in milliseconds)
    t_no_token: u64,
    t_reply_timeout: u64,
    t_reply_delay: u64,
    t_slot: u64,
    t_usage_timeout: u64,
}

#[allow(dead_code)]
impl<'a> MstpDriver<'a> {
    /// Create a new MS/TP driver
    /// Note: M5Stack RS-485 HAT has automatic direction control, no GPIO needed
    pub fn new(uart: UartDriver<'a>, station_address: u8, max_master: u8) -> Self {
        let next_station = (station_address + 1) % (max_master + 1);
        let now = Instant::now();

        Self {
            uart,
            station_address,
            max_master,
            max_info_frames: 1,
            state: MstpState::Initialize,
            token_count: 0,
            frame_count: 0,
            rx_poll_count: 0,
            rx_frame_count: 0,
            tx_frame_count: 0,
            last_rx_status_time: now,
            retry_count: 0,
            next_station,
            poll_station: station_address,
            sole_master: false,
            last_token_time: None,
            token_loop_time_ms: 0,
            discovered_masters: 1u128 << station_address, // Include ourselves
            crc_errors: 0,
            frame_errors: 0,
            reply_timeouts: 0,
            tokens_received: 0,
            token_pass_failures: 0,
            token_loop_min_ms: u32::MAX,
            token_loop_max_ms: 0,
            token_loop_sum_ms: 0,
            token_loop_count: 0,
            send_queue: VecDeque::new(),
            receive_queue: VecDeque::new(),
            rx_buffer: Vec::with_capacity(MSTP_HEADER_SIZE + MSTP_MAX_DATA_LENGTH + 2),
            pending_request: None,
            silence_timer: now,
            reply_timer: None,
            usage_timer: None,
            reply_delay_timer: None,
            no_token_timer: now,
            t_no_token: 5000,  // Increased to 5s to allow Controller 6 time to complete poll sweep and discover M5Stack (MAC 3)
            t_reply_timeout: 255,
            t_reply_delay: 250,
            t_slot: 10,
            t_usage_timeout: 50,
        }
    }

    /// Queue a frame for transmission
    /// expecting_reply: true if this is a confirmed request expecting a response
    pub fn send_frame(&mut self, data: &[u8], destination: u8, expecting_reply: bool) -> Result<(), MstpError> {
        if self.send_queue.len() >= 16 {
            return Err(MstpError::BufferFull);
        }

        info!("QUEUE: Adding {} bytes to send_queue for dest={}, queue_len_after={}, state={:?}",
              data.len(), destination, self.send_queue.len() + 1, self.state);
        self.send_queue.push_back((data.to_vec(), destination, expecting_reply));
        Ok(())
    }

    /// Queue a frame for transmission (backwards compatibility - assumes not expecting reply)
    pub fn send_frame_simple(&mut self, data: &[u8], destination: u8) -> Result<(), MstpError> {
        self.send_frame(data, destination, false)
    }

    /// Receive a frame (returns None if no frame available)
    pub fn receive_frame(&mut self) -> Result<Option<(Vec<u8>, u8)>, MstpError> {
        // Process incoming bytes
        self.process_uart_rx()?;

        // Run state machine
        self.run_state_machine()?;

        // Return any received data frames
        let result = self.receive_queue.pop_front();
        if let Some((ref data, source)) = result {
            info!(">>> receive_frame() returning {} bytes from MAC {}, queue remaining: {}",
                  data.len(), source, self.receive_queue.len());
        }
        Ok(result)
    }

    /// Process incoming UART bytes
    fn process_uart_rx(&mut self) -> Result<(), MstpError> {
        let mut buf = [0u8; 256];
        let mut total_read = 0usize;

        loop {
            match self.uart.read(&mut buf, 0) {
                Ok(0) => break,
                Ok(n) => {
                    // Log raw bytes as they arrive (use info! for diagnostics)
                    if n > 0 {
                        info!("UART_RX {} bytes: {:02X?}", n, &buf[..n.min(32)]);
                    }
                    self.rx_buffer.extend_from_slice(&buf[..n]);
                    self.silence_timer = Instant::now();
                    total_read += n;
                }
                Err(_) => break,
            }
        }

        // Update debug counter
        self.rx_poll_count += 1;

        // Try to parse frames from buffer
        self.parse_frames()?;

        Ok(())
    }

    /// Parse frames from receive buffer
    fn parse_frames(&mut self) -> Result<(), MstpError> {
        loop {
            // Look for preamble
            let preamble_pos = self.rx_buffer
                .windows(2)
                .position(|w| w[0] == MSTP_PREAMBLE_55 && w[1] == MSTP_PREAMBLE_FF);

            let _start = match preamble_pos {
                Some(pos) => {
                    // Discard bytes before preamble
                    if pos > 0 {
                        info!("RX_DISCARD: {} bytes before preamble: {:02X?}",
                               pos, &self.rx_buffer[..pos.min(16)]);
                        self.rx_buffer.drain(..pos);
                    }
                    0
                }
                None => {
                    // Keep last byte in case it's start of preamble
                    if self.rx_buffer.len() > 1 {
                        let keep = self.rx_buffer.len() - 1;
                        // Log what we're discarding if it's significant
                        if keep > 4 {
                            info!("RX_DISCARD: No preamble, discarding {} bytes: {:02X?}",
                                   keep, &self.rx_buffer[..keep.min(16)]);
                        }
                        self.rx_buffer.drain(..keep);
                    }
                    return Ok(());
                }
            };

            // Need at least header
            if self.rx_buffer.len() < MSTP_HEADER_SIZE {
                return Ok(());
            }

            // Parse header
            let frame_type = self.rx_buffer[2];
            let dest = self.rx_buffer[3];
            let source = self.rx_buffer[4];
            let data_len = ((self.rx_buffer[5] as usize) << 8) | (self.rx_buffer[6] as usize);
            let header_crc = self.rx_buffer[7];

            // Validate header CRC per ASHRAE 135 Annex G.1
            let calculated_crc = calculate_header_crc(&self.rx_buffer[2..7]);

            if calculated_crc != header_crc {
                self.crc_errors += 1;
                // Show full header bytes for debugging
                let hdr_bytes = &self.rx_buffer[..MSTP_HEADER_SIZE.min(self.rx_buffer.len())];
                warn!("Header CRC error: calc=0x{:02X} recv=0x{:02X} type={} dest={} src={} len={}",
                      calculated_crc, header_crc, frame_type, dest, source, data_len);
                warn!("  Header raw: {:02X?}", hdr_bytes);
                // If this looks like a data frame, show more context
                if self.rx_buffer.len() > MSTP_HEADER_SIZE {
                    let preview_len = (self.rx_buffer.len() - MSTP_HEADER_SIZE).min(20);
                    warn!("  Following {} bytes: {:02X?}", preview_len,
                          &self.rx_buffer[MSTP_HEADER_SIZE..MSTP_HEADER_SIZE+preview_len]);
                }
                self.rx_buffer.drain(..2); // Skip preamble and try again
                continue;
            }

            // Check for oversized frames
            if data_len > MSTP_MAX_DATA_LENGTH {
                self.frame_errors += 1;
                warn!("Oversized frame: data_len={} > max={}", data_len, MSTP_MAX_DATA_LENGTH);
                self.rx_buffer.drain(..2); // Skip preamble and try again
                continue;
            }

            // Increment RX counter for valid frames
            self.rx_frame_count += 1;

            // Calculate total frame size
            let frame_size = if data_len > 0 {
                MSTP_HEADER_SIZE + data_len + 2 // +2 for data CRC
            } else {
                MSTP_HEADER_SIZE
            };

            // CRITICAL DEBUG: Log ALL frames with their raw bytes
            // For data frames (type 5 or 6), log extra details
            if frame_type == 5 || frame_type == 6 || data_len > 0 {
                let have_bytes = self.rx_buffer.len();
                let raw_preview = have_bytes.min(35);
                info!(">>> BACNET DATA FRAME: type={} src={} dest={} data_len={} need={} have={}",
                      frame_type, source, dest, data_len, frame_size, have_bytes);
                info!(">>> RAW BUFFER ({} bytes): {:02X?}", have_bytes, &self.rx_buffer[..raw_preview]);
            }

            // Wait for complete frame
            if self.rx_buffer.len() < frame_size {
                if data_len > 0 {
                    info!(">>> Waiting for {} more bytes (have {}, need {})",
                          frame_size - self.rx_buffer.len(), self.rx_buffer.len(), frame_size);
                }
                return Ok(());
            }

            // Extract data if present
            let data = if data_len > 0 {
                let data_start = MSTP_HEADER_SIZE;
                let data_end = data_start + data_len;
                let data = self.rx_buffer[data_start..data_end].to_vec();

                // Validate data CRC per ASHRAE 135 Annex G.2
                let crc_low = self.rx_buffer[data_end];
                let crc_high = self.rx_buffer[data_end + 1];
                let received_crc = (crc_high as u16) << 8 | crc_low as u16;
                let calculated_crc = calculate_data_crc(&data);

                if received_crc != calculated_crc {
                    self.crc_errors += 1;
                    // Verbose debug: show raw frame bytes for CRC debugging
                    let frame_bytes: Vec<u8> = self.rx_buffer[..frame_size].to_vec();
                    warn!("Data CRC error: calc=0x{:04X} recv=0x{:04X} (type={}, src={}, len={})",
                          calculated_crc, received_crc, frame_type, source, data_len);
                    warn!("  Frame raw ({} bytes): {:02X?}", frame_bytes.len(), &frame_bytes[..frame_bytes.len().min(40)]);
                    warn!("  Data ({} bytes): {:02X?}", data.len(), &data[..data.len().min(24)]);
                    warn!("  CRC bytes at [{}..{}]: [{:02X}, {:02X}]", data_end, data_end+2, crc_low, crc_high);
                    self.rx_buffer.drain(..frame_size);
                    continue;
                }

                data
            } else {
                Vec::new()
            };

            // Remove frame from buffer
            self.rx_buffer.drain(..frame_size);

            // Process frame FIRST - logging can wait!
            // PollForMaster (0x01) requires immediate response within Tslot (10ms)
            self.handle_received_frame(frame_type, dest, source, data)?;

            // Post-process logging for debugging (non-critical path)
            // Only log data frames and unexpected frame types at info level
            if data_len > 0 {
                let ftype_name = MstpFrameType::from_u8(frame_type);
                debug!("RX frame: type={:?} src={} dest={} len={}",
                      ftype_name, source, dest, data_len);
            }
        }
    }

    /// Handle a received frame - state-aware processing per ASHRAE 135 Clause 9
    fn handle_received_frame(
        &mut self,
        frame_type: u8,
        dest: u8,
        source: u8,
        data: Vec<u8>,
    ) -> Result<(), MstpError> {
        let ftype = MstpFrameType::from_u8(frame_type);

        // Log data frames at info level for debugging
        if data.len() > 0 || (ftype != Some(MstpFrameType::Token) && ftype != Some(MstpFrameType::PollForMaster)) {
            info!(
                "RX frame: type={:?} dest={} src={} len={} state={:?}",
                ftype, dest, source, data.len(), self.state
            );
        } else {
            trace!(
                "RX frame: type={:?} dest={} src={} len={} state={:?}",
                ftype, dest, source, data.len(), self.state
            );
        }

        // If we're in Initialize and receive a valid frame, transition to Idle
        // This means the bus is active and we should join the network
        if self.state == MstpState::Initialize {
            info!("Received valid frame in Initialize state, transitioning to Idle");
            self.state = MstpState::Idle;
            self.no_token_timer = Instant::now();
        }

        // State-specific frame handling
        match self.state {
            MstpState::Idle => {
                self.handle_frame_in_idle(ftype, dest, source, data)?;
            }
            MstpState::WaitForReply => {
                self.handle_frame_in_wait_for_reply(ftype, dest, source, data)?;
            }
            MstpState::PollForMaster => {
                self.handle_frame_in_poll_for_master(ftype, dest, source)?;
            }
            _ => {
                // For other states, handle basic frames
                self.handle_frame_basic(ftype, dest, source, data)?;
            }
        }

        Ok(())
    }

    /// Handle frame when in IDLE state
    fn handle_frame_in_idle(
        &mut self,
        ftype: Option<MstpFrameType>,
        dest: u8,
        source: u8,
        data: Vec<u8>,
    ) -> Result<(), MstpError> {
        match ftype {
            Some(MstpFrameType::Token) => {
                if dest == self.station_address {
                    // We received the token - transition to UseToken
                    info!("Received Token from station {} (in Idle)", source);
                    self.token_count += 1;
                    self.tokens_received += 1;
                    self.frame_count = 0;
                    self.state = MstpState::UseToken;
                    self.usage_timer = Some(Instant::now());
                    self.no_token_timer = Instant::now(); // Reset no-token timer

                    // Track token loop time with min/max/avg
                    if let Some(last_time) = self.last_token_time {
                        let loop_time = last_time.elapsed().as_millis() as u32;
                        self.token_loop_time_ms = loop_time;

                        // Update min/max
                        if loop_time < self.token_loop_min_ms {
                            self.token_loop_min_ms = loop_time;
                        }
                        if loop_time > self.token_loop_max_ms {
                            self.token_loop_max_ms = loop_time;
                        }

                        // Update running sum for average (with overflow protection)
                        self.token_loop_sum_ms = self.token_loop_sum_ms.saturating_add(loop_time as u64);
                        self.token_loop_count = self.token_loop_count.saturating_add(1);
                    }
                    self.last_token_time = Some(Instant::now());

                    // Record source as a discovered master
                    if source <= 127 {
                        self.discovered_masters |= 1u128 << source;
                    }

                    // We found another master!
                    if self.sole_master {
                        info!("Another master detected, no longer sole master");
                        self.sole_master = false;
                    }

                    // Update next_station to be the next known master after us
                    // This ensures proper token ring operation
                    let old_next = self.next_station;
                    self.next_station = self.find_next_master();
                    if self.next_station != old_next {
                        info!("Updated next_station: {} -> {} (discovered_masters=0x{:X})",
                              old_next, self.next_station, self.discovered_masters);
                    }
                }
            }
            Some(MstpFrameType::PollForMaster) => {
                // TIMING CRITICAL: Respond to poll FIRST, then do bookkeeping!
                // We must reply within Tslot (10ms). Logging and other work comes AFTER.
                if dest == self.station_address {
                    // Send reply IMMEDIATELY - no logging before this!
                    self.send_reply_to_poll(source)?;

                    // Reset no_token timer after replying
                    self.no_token_timer = Instant::now();

                    // Now safe to log (after time-critical response sent)
                    debug!("RPFM sent to {}", source);
                }

                // Record source as a discovered master (after reply sent)
                if source <= 127 {
                    let was_known = (self.discovered_masters & (1u128 << source)) != 0;
                    self.discovered_masters |= 1u128 << source;

                    if !was_known {
                        info!("Discovered new master {} via PollForMaster", source);
                        if self.sole_master {
                            self.sole_master = false;
                        }
                        // Recalculate next_station to include newly discovered master
                        let old_next = self.next_station;
                        self.next_station = self.find_next_master();
                        if self.next_station != old_next {
                            debug!("Updated next_station: {} -> {} after discovering master {}",
                                  old_next, self.next_station, source);
                        }
                    }
                }
            }
            Some(MstpFrameType::BacnetDataExpectingReply) => {
                if dest == self.station_address {
                    // Transition to AnswerDataRequest
                    info!("Received BACnet data (expecting reply) from station {}, {} bytes", source, data.len());
                    self.pending_request = Some((data, source));
                    self.reply_delay_timer = Some(Instant::now());
                    self.state = MstpState::AnswerDataRequest;
                } else if dest == MSTP_BROADCAST_ADDRESS {
                    // Broadcast data expecting reply - just queue it
                    info!("Received BACnet broadcast data from station {}, {} bytes", source, data.len());
                    if self.receive_queue.len() < 16 {
                        self.receive_queue.push_back((data, source));
                    }
                }
            }
            Some(MstpFrameType::BacnetDataNotExpectingReply) => {
                if dest == self.station_address || dest == MSTP_BROADCAST_ADDRESS {
                    // Queue for upper layer
                    info!("Received BACnet data from station {}, {} bytes (dest={})", source, data.len(), dest);
                    if self.receive_queue.len() < 16 {
                        let preview_len = data.len().min(20);
                        info!(">>> QUEUING DATA for upper layer: {} bytes, NPDU preview: {:02X?}", data.len(), &data[..preview_len]);
                        self.receive_queue.push_back((data, source));
                        info!(">>> QUEUE now has {} items", self.receive_queue.len());
                    } else {
                        warn!(">>> QUEUE FULL - dropping frame!");
                    }
                }
            }
            _ => {
                // Other frame types ignored in Idle
            }
        }
        Ok(())
    }

    /// Handle frame when in WAIT_FOR_REPLY state
    /// CRITICAL: Uses NEGATIVE LIST approach per ASHRAE 135
    /// Only reject frames that are definitively NOT replies
    fn handle_frame_in_wait_for_reply(
        &mut self,
        ftype: Option<MstpFrameType>,
        dest: u8,
        source: u8,
        data: Vec<u8>,
    ) -> Result<(), MstpError> {
        // Check if frame is addressed to us
        if dest != self.station_address && dest != MSTP_BROADCAST_ADDRESS {
            return Ok(()); // Not for us, ignore
        }

        // NEGATIVE LIST APPROACH - only reject frames that are NOT replies
        match ftype {
            // PollForMaster MUST ALWAYS be answered, even in WaitForReply state!
            // Per ASHRAE 135, we respond to PFM in any state.
            Some(MstpFrameType::PollForMaster) => {
                if dest == self.station_address {
                    info!("Received PollForMaster from {} while in WaitForReply, sending reply", source);
                    self.send_reply_to_poll(source)?;
                }
                // Don't change state - continue waiting for our reply
            }

            // These are NOT replies - unexpected frames in WaitForReply
            Some(MstpFrameType::Token) |
            Some(MstpFrameType::ReplyToPollForMaster) |
            Some(MstpFrameType::TestRequest) => {
                // ReceivedUnexpectedFrame event
                warn!("Unexpected frame type {:?} in WaitForReply state", ftype);
                self.reply_timer = None;
                self.state = MstpState::Idle;
                self.no_token_timer = Instant::now();
            }

            // ALL OTHER frame types are accepted as valid replies
            // This includes:
            // - BacnetDataNotExpectingReply
            // - TestResponse
            // - ReplyPostponed
            // - Unknown/proprietary frame types (for forward compatibility)
            // - Segmented Complex-ACK frames
            _ => {
                // ReceivedReply event - valid reply received
                debug!("Valid reply received in WaitForReply: {:?}", ftype);

                // Queue for upper layer
                if self.receive_queue.len() < 16 {
                    self.receive_queue.push_back((data, source));
                }

                // Transition to DoneWithToken
                self.reply_timer = None;
                self.state = MstpState::DoneWithToken;
            }
        }
        Ok(())
    }

    /// Handle frame when in POLL_FOR_MASTER state
    fn handle_frame_in_poll_for_master(
        &mut self,
        ftype: Option<MstpFrameType>,
        dest: u8,
        source: u8,
    ) -> Result<(), MstpError> {
        match ftype {
            // PollForMaster from another master - we MUST respond!
            // This can happen when two stations are both trying to establish the ring
            Some(MstpFrameType::PollForMaster) => {
                if dest == self.station_address {
                    info!("Received PollForMaster from {} while in PollForMaster state, sending reply", source);
                    self.send_reply_to_poll(source)?;

                    // The other master found us first - they have priority
                    // Defer to them and wait for the token
                    self.discovered_masters |= 1u128 << source;
                    self.sole_master = false;
                    self.state = MstpState::Idle;
                    self.no_token_timer = Instant::now();
                    info!("Deferring to master {} who polled us first", source);
                }
            }

            Some(MstpFrameType::ReplyToPollForMaster) => {
                // Only accept if addressed to us
                if dest == self.station_address {
                    // Found a master!
                    info!("Received ReplyToPollForMaster from {}", source);

                    // Add to discovered masters bitmap
                    self.discovered_masters |= 1u128 << source;

                    // Update next_station to the newly discovered master
                    self.next_station = source;
                    self.sole_master = false;

                    // Advance poll_station past the discovered master for next poll cycle
                    self.poll_station = (source + 1) % (self.max_master + 1);
                    if self.poll_station == self.station_address {
                        self.poll_station = (self.poll_station + 1) % (self.max_master + 1);
                    }

                    // We generated the token via polling, so we should use it first
                    // before passing to the newly discovered master
                    self.token_count += 1;
                    self.tokens_received += 1;
                    self.frame_count = 0;
                    self.state = MstpState::UseToken;
                    self.usage_timer = Some(Instant::now());
                    info!("New master discovered at {}, next_station={}, poll_station={}",
                          source, self.next_station, self.poll_station);
                } else {
                    debug!("Ignoring ReplyToPollForMaster not addressed to us (dest={}, we are {})", dest, self.station_address);
                }
            }

            _ => {}
        }
        Ok(())
    }

    /// Basic frame handling for other states
    fn handle_frame_basic(
        &mut self,
        ftype: Option<MstpFrameType>,
        dest: u8,
        source: u8,
        data: Vec<u8>,
    ) -> Result<(), MstpError> {
        match ftype {
            Some(MstpFrameType::Token) => {
                if dest == self.station_address {
                    // Unexpected token - go to UseToken anyway
                    self.token_count += 1;
                    self.frame_count = 0;
                    self.state = MstpState::UseToken;
                    self.usage_timer = Some(Instant::now());
                }
            }
            Some(MstpFrameType::PollForMaster) => {
                if dest == self.station_address {
                    self.send_reply_to_poll(source)?;
                }
            }
            Some(MstpFrameType::BacnetDataNotExpectingReply) => {
                if dest == self.station_address || dest == MSTP_BROADCAST_ADDRESS {
                    if self.receive_queue.len() < 16 {
                        self.receive_queue.push_back((data, source));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Run the MS/TP state machine - implements ASHRAE 135 Clause 9
    fn run_state_machine(&mut self) -> Result<(), MstpError> {
        match self.state {
            MstpState::Initialize => {
                // Wait for silence then go to idle
                if self.silence_timer.elapsed() > Duration::from_millis(self.t_no_token) {
                    info!("MS/TP initialized, transitioning to Idle");
                    self.state = MstpState::Idle;
                    self.no_token_timer = Instant::now();
                }
            }

            MstpState::Idle => {
                // Check for no-token timeout
                if self.no_token_timer.elapsed() > Duration::from_millis(self.t_no_token) {
                    // No token received, try to generate one via polling
                    info!("Idle: No token timeout ({}ms), starting PollForMaster", self.t_no_token);
                    self.poll_station = (self.station_address + 1) % (self.max_master + 1);
                    self.send_poll_for_master(self.poll_station)?;
                    self.state = MstpState::PollForMaster;
                    self.silence_timer = Instant::now();
                }
            }

            MstpState::UseToken => {
                // Check usage timeout first
                if let Some(timer) = self.usage_timer {
                    if timer.elapsed() > Duration::from_millis(self.t_usage_timeout) {
                        debug!("Usage timeout, transitioning to DoneWithToken");
                        self.state = MstpState::DoneWithToken;
                        return Ok(());
                    }
                }

                // We have the token, send data if available
                if self.frame_count < self.max_info_frames {
                    if let Some((data, dest, expecting_reply)) = self.send_queue.pop_front() {
                        info!("UseToken: Sending {} bytes to dest={} (expecting_reply={})",
                              data.len(), dest, expecting_reply);
                        self.send_data_frame(&data, dest, expecting_reply)?;
                        self.frame_count += 1;

                        if expecting_reply {
                            // Transition to WaitForReply
                            info!("Sent frame expecting reply, transitioning to WaitForReply");
                            self.reply_timer = Some(Instant::now());
                            self.state = MstpState::WaitForReply;
                        }
                        // Stay in UseToken to potentially send more frames
                    } else {
                        // No frames to send
                        // Per MS/TP spec, we should hold the token briefly to allow
                        // queued data to be sent. Check if we just acquired the token
                        // (frame_count == 0) and haven't held it long enough.
                        // Hold for at least 5ms to give application layer a chance to queue data.
                        if self.frame_count == 0 {
                            if let Some(timer) = self.usage_timer {
                                let hold_time_ms = timer.elapsed().as_millis() as u64;
                                if hold_time_ms < 5 {
                                    // Still waiting for potential queued frames
                                    // Return without changing state to allow caller to yield
                                    return Ok(());
                                }
                            }
                        }
                        // Log queue status for debugging
                        trace!("UseToken: send_queue empty, passing token (frame_count={})", self.frame_count);
                        self.state = MstpState::DoneWithToken;
                    }
                } else {
                    // Sent max frames
                    debug!("UseToken: max_info_frames reached ({}), passing token", self.max_info_frames);
                    self.state = MstpState::DoneWithToken;
                }
            }

            MstpState::WaitForReply => {
                // Check for reply timeout
                if let Some(timer) = self.reply_timer {
                    if timer.elapsed() > Duration::from_millis(self.t_reply_timeout) {
                        warn!("Reply timeout in WaitForReply state");
                        self.reply_timer = None;
                        self.retry_count += 1;
                        self.reply_timeouts += 1;

                        if self.retry_count >= MAX_RETRY {
                            // Max retries exceeded
                            debug!("Max retries exceeded, transitioning to DoneWithToken");
                            self.retry_count = 0;
                            self.token_pass_failures += 1;
                            self.state = MstpState::DoneWithToken;
                        } else {
                            // Could retry the transmission here
                            self.state = MstpState::DoneWithToken;
                        }
                    }
                }
                // Frame reception handled in handle_frame_in_wait_for_reply
            }

            MstpState::AnswerDataRequest => {
                // Wait for minimum reply delay before sending response
                if let Some(timer) = self.reply_delay_timer {
                    if timer.elapsed() >= Duration::from_millis(self.t_reply_delay) {
                        // Ready to send reply
                        if let Some((request_data, source)) = self.pending_request.take() {
                            // Queue the request for upper layer processing
                            // The upper layer should call send_reply() with the response
                            if self.receive_queue.len() < 16 {
                                self.receive_queue.push_back((request_data, source));
                            }
                        }

                        // Return to Idle
                        self.reply_delay_timer = None;
                        self.state = MstpState::Idle;
                        self.no_token_timer = Instant::now();
                    }
                } else {
                    // No timer set, something went wrong
                    self.state = MstpState::Idle;
                    self.no_token_timer = Instant::now();
                }
            }

            MstpState::DoneWithToken => {
                // Check if we should poll for new masters
                if self.token_count >= NPOLL {
                    debug!("Poll interval reached, polling station {}", self.poll_station);
                    self.token_count = 0;

                    // Skip our own address
                    if self.poll_station == self.station_address {
                        self.poll_station = (self.poll_station + 1) % (self.max_master + 1);
                    }

                    // Send poll to current poll_station (incremented by PollForMaster state)
                    self.send_poll_for_master(self.poll_station)?;
                    self.state = MstpState::PollForMaster;
                    self.silence_timer = Instant::now();
                } else {
                    // Normal token pass
                    self.state = MstpState::PassToken;
                }
            }

            MstpState::PassToken => {
                info!("PassToken: Sending token to station {} (send_queue_len={})",
                      self.next_station, self.send_queue.len());
                self.send_token(self.next_station)?;
                info!("Token passed to station {}, transitioning to Idle", self.next_station);
                self.state = MstpState::Idle;
                self.no_token_timer = Instant::now();
            }

            MstpState::NoToken => {
                // Lost token recovery - try to regenerate via polling
                if self.silence_timer.elapsed() > Duration::from_millis(self.t_no_token) {
                    debug!("NoToken state: attempting recovery via PollForMaster");
                    self.poll_station = (self.station_address + 1) % (self.max_master + 1);
                    self.send_poll_for_master(self.poll_station)?;
                    self.state = MstpState::PollForMaster;
                    self.silence_timer = Instant::now();
                }
            }

            MstpState::PollForMaster => {
                // Wait for reply with slot timeout
                // FIX: Per ASHRAE 135 Clause 9, poll only ONE address per NPOLL interval,
                // not a full sweep. Increment poll_station for the NEXT poll cycle.
                if self.silence_timer.elapsed() > Duration::from_millis(self.t_slot) {
                    // No reply from this station - increment poll_station for next time
                    self.poll_station = (self.poll_station + 1) % (self.max_master + 1);

                    // Skip our own address
                    if self.poll_station == self.station_address {
                        self.poll_station = (self.poll_station + 1) % (self.max_master + 1);
                    }

                    debug!("PollForMaster: no reply, next poll will be station {}",
                           self.poll_station);

                    // Now pass the token - don't continue polling
                    // The next NPOLL cycle will poll poll_station
                    self.state = MstpState::PassToken;
                }

                // Check overall no-token timeout (only log once, not repeatedly)
                if self.no_token_timer.elapsed() > Duration::from_millis(self.t_no_token * 2) {
                    trace!("PollForMaster timeout, returning to Idle");
                    self.state = MstpState::Idle;
                    self.no_token_timer = Instant::now();
                }
            }
        }

        Ok(())
    }

    /// Send a reply frame (used by upper layer when responding to DataExpectingReply)
    pub fn send_reply(&mut self, data: &[u8], destination: u8) -> Result<(), MstpError> {
        self.send_raw_frame(MstpFrameType::BacnetDataNotExpectingReply, destination, data)
    }

    /// Send a token frame
    fn send_token(&mut self, dest: u8) -> Result<(), MstpError> {
        self.send_raw_frame(MstpFrameType::Token, dest, &[])
    }

    /// Send poll for master
    fn send_poll_for_master(&mut self, dest: u8) -> Result<(), MstpError> {
        self.send_raw_frame(MstpFrameType::PollForMaster, dest, &[])
    }

    /// Send reply to poll
    fn send_reply_to_poll(&mut self, dest: u8) -> Result<(), MstpError> {
        self.send_raw_frame(MstpFrameType::ReplyToPollForMaster, dest, &[])
    }

    /// Send a data frame
    fn send_data_frame(&mut self, data: &[u8], dest: u8, expecting_reply: bool) -> Result<(), MstpError> {
        let ftype = if expecting_reply {
            MstpFrameType::BacnetDataExpectingReply
        } else {
            MstpFrameType::BacnetDataNotExpectingReply
        };
        self.send_raw_frame(ftype, dest, data)
    }

    /// Send a raw MS/TP frame
    fn send_raw_frame(&mut self, ftype: MstpFrameType, dest: u8, data: &[u8]) -> Result<(), MstpError> {
        let data_len = data.len();

        // Build frame
        let mut frame = Vec::with_capacity(MSTP_HEADER_SIZE + data_len + 2);

        // Preamble
        frame.push(MSTP_PREAMBLE_55);
        frame.push(MSTP_PREAMBLE_FF);

        // Header
        frame.push(ftype as u8);
        frame.push(dest);
        frame.push(self.station_address);
        frame.push((data_len >> 8) as u8);
        frame.push((data_len & 0xFF) as u8);

        // Header CRC
        let header_crc = calculate_header_crc(&frame[2..7]);
        frame.push(header_crc);

        // Data and CRC if present
        if !data.is_empty() {
            frame.extend_from_slice(data);
            let data_crc = calculate_data_crc(data);
            frame.push((data_crc & 0xFF) as u8);
            frame.push((data_crc >> 8) as u8);
        }

        // For time-critical frames (RPFM, Token), skip pre-TX logging entirely
        // Logging is done AFTER transmission completes
        let is_reply_to_poll = ftype == MstpFrameType::ReplyToPollForMaster;
        let is_token = ftype == MstpFrameType::Token;
        let skip_pre_log = is_reply_to_poll || is_token;

        if !skip_pre_log {
            // Log non-time-critical frames before TX
            if (ftype as u8) >= 5 {
                info!("TX data frame: type={:?} dest={} len={} data={:02X?}",
                      ftype, dest, data_len, &data[..data_len.min(20)]);
            } else {
                debug!("TX control frame: type={:?} dest={}", ftype, dest);
            }
        }

        // Tturnaround delay: minimum 40 bit-times between last received byte and first transmitted byte
        // At 38400 baud: 40 bits / 38400 bps = ~1.04ms
        // This is REQUIRED by ASHRAE 135 Clause 9.2.3 for proper RS-485 operation
        //
        // IMPORTANT: We wait for Tturnaround since last RX, then check ONE MORE TIME
        // that no bytes have arrived. If bytes are actively arriving, we're in the
        // middle of receiving another station's frame and must NOT transmit.
        //
        // The key insight is: if we have the token and it's been quiet for Tturnaround,
        // we can transmit. But if bytes are STILL arriving (e.g., buffered in UART FIFO),
        // we need to wait for them to finish.
        // Tturnaround: minimum 40 bit-times = ~1.04ms at 38400 baud
        // CRITICAL: We need to reply within Tslot (10ms) of receiving a poll,
        // but some devices use shorter Tslot (5ms). Keep turnaround minimal.
        let turnaround_us: u64 = 500; // 0.5ms (more than 40 bit-times, but faster response)

        // Wait for initial Tturnaround
        let silence_us = self.silence_timer.elapsed().as_micros() as u64;
        if silence_us < turnaround_us {
            let wait_us = turnaround_us - silence_us;
            std::thread::sleep(std::time::Duration::from_micros(wait_us));
        }

        // For ReplyToPollForMaster: SKIP bus activity check entirely!
        // The polling master just sent us a poll and is waiting for our reply.
        // There should be NO other traffic. Checking the bus adds delay that
        // causes us to miss the Tslot window (10ms from poll TX end).
        //
        // For other frame types: check if another station is transmitting
        // (is_reply_to_poll and is_token already defined above for logging decision)
        let is_time_critical = is_reply_to_poll || is_token;

        if !is_reply_to_poll {
            // Only check bus activity for non-reply frames
            let mut check_buf = [0u8; 64];
            let max_wait_ms = if is_time_critical { 3 } else { 20 };

            if let Ok(n) = self.uart.read(&mut check_buf, 0) {
                if n > 0 {
                    // Bytes are actively arriving - add them to buffer
                    self.rx_buffer.extend_from_slice(&check_buf[..n]);
                    self.silence_timer = Instant::now();

                    // Now wait for true silence - poll until no more bytes arrive
                    // with a reasonable timeout
                    let start = Instant::now();
                    loop {
                        if start.elapsed().as_millis() > max_wait_ms as u128 {
                            // Frame taking too long - go ahead and transmit
                            if is_time_critical {
                                warn!("Time-critical TX: forced TX after {}ms wait (bus busy)", max_wait_ms);
                            }
                            break;
                        }

                        std::thread::sleep(std::time::Duration::from_micros(200));

                        if let Ok(n) = self.uart.read(&mut check_buf, 0) {
                            if n > 0 {
                                self.rx_buffer.extend_from_slice(&check_buf[..n]);
                                self.silence_timer = Instant::now();
                            } else {
                                if self.silence_timer.elapsed().as_micros() as u64 >= turnaround_us {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Save frame copy for echo detection
        let tx_frame_copy = frame.clone();

        // Send the frame
        // Note: M5Stack RS-485 HAT has automatic direction control via SP485EEN chip
        // The TX line controls DE/RE automatically - no GPIO needed
        self.uart.write(&frame).map_err(|e| MstpError::IoError(format!("{:?}", e)))?;

        // Wait for TX to complete
        // At 38400 baud: each byte = 10 bits = ~260us
        // Total frame time = frame.len() * 260us
        // For time-critical frames (ReplyToPollForMaster), use minimal margin
        // For other frames, add more margin to ensure transceiver settles
        let extra_margin_us = if is_reply_to_poll { 200 } else { 2000 };
        let tx_time_us = (frame.len() as u64) * 260 + extra_margin_us;
        std::thread::sleep(std::time::Duration::from_micros(tx_time_us));

        // Note: M5Stack RS-485 HAT uses SP485EEN with automatic direction control
        // This chip should NOT echo our TX back to RX (DE/RE tied together, controlled by TX)
        // However, some RS-485 transceivers do echo. Let's be conservative and only
        // discard bytes received DURING our transmission window, not clear rx_buffer
        // which might contain the start of a legitimate frame from another station.

        // Previously we flushed bytes after TX assuming they were echo.
        // However, the SP485EEN chip on M5Stack RS-485 HAT has automatic direction
        // control and does NOT echo TX back to RX. Any bytes in the UART buffer
        // after transmission are likely the START of a frame from another station
        // that began transmitting immediately after we finished.
        //
        // DO NOT flush these bytes - they should be processed normally.
        // If we were getting echo, we'd see our own frame in the rx_buffer and
        // could filter it out based on matching our TX (but we're not echoing).
        //
        // Read ALL bytes that arrived during our TX - these might be the start
        // of a frame from another station responding to us or transmitting
        let mut rx_buf = [0u8; 256];
        let mut total_received = 0usize;
        loop {
            match self.uart.read(&mut rx_buf, 0) {
                Ok(0) => break,
                Ok(n) => {
                    self.rx_buffer.extend_from_slice(&rx_buf[..n]);
                    total_received += n;
                    self.silence_timer = Instant::now();
                }
                Err(_) => break,
            }
        }
        if total_received > 0 {
            info!("RX during/after TX: {} total bytes", total_received);

            // CRITICAL FIX: Filter TX echo if present
            // Despite docs claiming SP485EEN doesn't echo, we're seeing our own transmissions
            // This causes the router to think our tokens are lost, creating duplicate tokens
            if self.rx_buffer.len() >= tx_frame_copy.len() {
                // Check if start of rx_buffer matches our TX frame
                if &self.rx_buffer[..tx_frame_copy.len()] == tx_frame_copy.as_slice() {
                    warn!("TX ECHO DETECTED! Filtering {} bytes from rx_buffer", tx_frame_copy.len());
                    warn!("Echo frame: {:02X?}", &tx_frame_copy[..tx_frame_copy.len().min(20)]);
                    self.rx_buffer.drain(..tx_frame_copy.len());
                    total_received -= tx_frame_copy.len();
                    if total_received > 0 {
                        info!("Remaining valid RX after echo removal: {} bytes", total_received);
                    }
                }
            }
        }
        // rx_buffer now contains only legitimate frames from other stations

        self.silence_timer = Instant::now();
        self.tx_frame_count += 1;

        // Post-TX logging for time-critical frames (after transmission complete)
        if skip_pre_log {
            trace!("TX: {:?} -> {}", ftype, dest);
        }

        Ok(())
    }

    /// Get frame statistics (rx_count, tx_count)
    pub fn get_frame_stats(&self) -> (u64, u64) {
        (self.rx_frame_count, self.tx_frame_count)
    }

    /// Find the next master station after us in the token ring
    /// Uses the discovered_masters bitmap to find the correct next station
    fn find_next_master(&self) -> u8 {
        // Search from our address + 1 to max_master, then wrap around from 0
        // Skip ourselves
        for offset in 1..=self.max_master as u16 {
            let addr = ((self.station_address as u16 + offset) % (self.max_master as u16 + 1)) as u8;
            if addr != self.station_address && (self.discovered_masters & (1u128 << addr)) != 0 {
                return addr;
            }
        }
        // No other masters discovered - return address after us (will be polled)
        (self.station_address + 1) % (self.max_master + 1)
    }

    /// Get comprehensive MS/TP statistics
    pub fn get_stats(&self) -> MstpStats {
        // Calculate average token loop time
        let token_loop_avg_ms = if self.token_loop_count > 0 {
            (self.token_loop_sum_ms / self.token_loop_count) as u32
        } else {
            0
        };

        // Handle min being u32::MAX when no tokens received yet
        let token_loop_min_ms = if self.token_loop_min_ms == u32::MAX {
            0
        } else {
            self.token_loop_min_ms
        };

        MstpStats {
            rx_frames: self.rx_frame_count,
            tx_frames: self.tx_frame_count,
            crc_errors: self.crc_errors,
            frame_errors: self.frame_errors,
            reply_timeouts: self.reply_timeouts,
            tokens_received: self.tokens_received,
            token_pass_failures: self.token_pass_failures,
            token_loop_time_ms: self.token_loop_time_ms,
            token_loop_min_ms,
            token_loop_max_ms: self.token_loop_max_ms,
            token_loop_avg_ms,
            master_count: self.discovered_masters.count_ones() as u8,
            discovered_masters: self.discovered_masters,
            current_state: self.state as u8,
            next_station: self.next_station,
            poll_station: self.poll_station,
            silence_ms: self.silence_timer.elapsed().as_millis() as u32,
            station_address: self.station_address,
            sole_master: self.sole_master,
            send_queue_len: self.send_queue.len() as u8,
            receive_queue_len: self.receive_queue.len() as u8,
        }
    }

    /// Get current state machine state as a string
    pub fn get_state_name(&self) -> &'static str {
        match self.state {
            MstpState::Initialize => "Initialize",
            MstpState::Idle => "Idle",
            MstpState::UseToken => "UseToken",
            MstpState::WaitForReply => "WaitForReply",
            MstpState::PassToken => "PassToken",
            MstpState::NoToken => "NoToken",
            MstpState::PollForMaster => "PollForMaster",
            MstpState::AnswerDataRequest => "AnswerDataRequest",
            MstpState::DoneWithToken => "DoneWithToken",
        }
    }

    /// Reset statistics counters (keeps discovered_masters)
    pub fn reset_stats(&mut self) {
        self.rx_frame_count = 0;
        self.tx_frame_count = 0;
        self.crc_errors = 0;
        self.reply_timeouts = 0;
        self.tokens_received = 0;
        self.frame_errors = 0;
        self.token_pass_failures = 0;
        self.rx_poll_count = 0;
        // Reset token loop timing stats
        self.token_loop_time_ms = 0;
        self.token_loop_min_ms = u32::MAX;
        self.token_loop_max_ms = 0;
        self.token_loop_sum_ms = 0;
        self.token_loop_count = 0;
        // Keep discovered_masters bitmap - don't clear device knowledge
    }

    /// Check if we currently have the token (UseToken or related states)
    pub fn has_token(&self) -> bool {
        matches!(self.state,
            MstpState::UseToken |
            MstpState::WaitForReply |
            MstpState::DoneWithToken |
            MstpState::AnswerDataRequest
        )
    }

    /// Get the station address
    pub fn get_station_address(&self) -> u8 {
        self.station_address
    }

    /// Get the max master setting
    pub fn get_max_master(&self) -> u8 {
        self.max_master
    }
}

/// MS/TP Statistics
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct MstpStats {
    pub rx_frames: u64,
    pub tx_frames: u64,
    pub crc_errors: u64,
    pub frame_errors: u64,          // Invalid frames (bad type, oversized, etc.)
    pub reply_timeouts: u64,
    pub tokens_received: u64,
    pub token_pass_failures: u64,   // Times we failed to pass token (max retries)
    pub token_loop_time_ms: u32,
    pub token_loop_min_ms: u32,     // Minimum observed token loop time
    pub token_loop_max_ms: u32,     // Maximum observed token loop time
    pub token_loop_avg_ms: u32,     // Rolling average token loop time
    pub master_count: u8,
    pub discovered_masters: u128,
    pub current_state: u8,          // MstpState as u8
    pub next_station: u8,
    pub poll_station: u8,
    pub silence_ms: u32,            // Time since last valid frame
    pub station_address: u8,        // Our station address
    pub sole_master: bool,          // Operating as sole master on bus
    pub send_queue_len: u8,         // Current send queue depth
    pub receive_queue_len: u8,      // Current receive queue depth
}

/// Calculate MS/TP header CRC-8 per ASHRAE 135 Annex G.1
/// Uses polynomial X^8 + X^7 + 1
/// This is the PARALLEL algorithm from the ASHRAE spec - NOT the standard bit-by-bit CRC!
fn calculate_header_crc(header: &[u8]) -> u8 {
    let mut crc = 0xFFu8;

    for &byte in header {
        // XOR C7..C0 with D7..D0
        let mut temp = (crc ^ byte) as u16;

        // Exclusive OR the terms in the table (top down)
        // This implements the polynomial X^8 + X^7 + 1
        temp = temp
            ^ (temp << 1)
            ^ (temp << 2)
            ^ (temp << 3)
            ^ (temp << 4)
            ^ (temp << 5)
            ^ (temp << 6)
            ^ (temp << 7);

        // Combine bits shifted out left hand end
        crc = ((temp & 0xfe) ^ ((temp >> 8) & 1)) as u8;
    }

    !crc
}

/// Calculate MS/TP data CRC-16 per ASHRAE 135 Annex G.2
/// Uses CRC-CCITT polynomial: x^16 + x^12 + x^5 + 1 (reflected form: 0x8408)
/// NOT the same as MODBUS/CRC-16-IBM (0xA001)!
fn calculate_data_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;

    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0x8408;  // CRC-CCITT reflected polynomial
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

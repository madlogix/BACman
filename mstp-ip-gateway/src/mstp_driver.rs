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
const NPOLL: u8 = 50; // Poll for new masters every 50 tokens
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
}

impl std::fmt::Display for MstpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MstpError::IoError(s) => write!(f, "I/O error: {}", s),
            MstpError::InvalidFrame => write!(f, "Invalid frame"),
            MstpError::CrcError => write!(f, "CRC error"),
            MstpError::Timeout => write!(f, "Timeout"),
            MstpError::BufferFull => write!(f, "Buffer full"),
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
            t_no_token: 500,
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
        Ok(self.receive_queue.pop_front())
    }

    /// Process incoming UART bytes
    fn process_uart_rx(&mut self) -> Result<(), MstpError> {
        let mut buf = [0u8; 256];

        loop {
            match self.uart.read(&mut buf, 0) {
                Ok(0) => break,
                Ok(n) => {
                    self.rx_buffer.extend_from_slice(&buf[..n]);
                    self.silence_timer = Instant::now();
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
                        self.rx_buffer.drain(..pos);
                    }
                    0
                }
                None => {
                    // Keep last byte in case it's start of preamble
                    if self.rx_buffer.len() > 1 {
                        let keep = self.rx_buffer.len() - 1;
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
                self.rx_buffer.drain(..2); // Skip preamble and try again
                continue;
            }

            // Check for oversized frames
            if data_len > MSTP_MAX_DATA_LENGTH {
                self.frame_errors += 1;
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

            // Wait for complete frame
            if self.rx_buffer.len() < frame_size {
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
                    self.rx_buffer.drain(..frame_size);
                    continue;
                }

                data
            } else {
                Vec::new()
            };

            // Remove frame from buffer
            self.rx_buffer.drain(..frame_size);

            // Process frame
            self.handle_received_frame(frame_type, dest, source, data)?;
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

        debug!(
            "RX frame: type={:?} dest={} src={} len={} state={:?}",
            ftype, dest, source, data.len(), self.state
        );

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
                    trace!("Received token from station {}", source);
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
                }
            }
            Some(MstpFrameType::PollForMaster) => {
                // Record source as a discovered master
                if source <= 127 {
                    self.discovered_masters |= 1u128 << source;
                }
                if dest == self.station_address {
                    // Reply to poll - we're a master
                    info!("Received PollForMaster from station {}, sending reply", source);
                    self.send_reply_to_poll(source)?;
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
                        self.receive_queue.push_back((data, source));
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
            // These are NOT replies - unexpected frames in WaitForReply
            Some(MstpFrameType::Token) |
            Some(MstpFrameType::PollForMaster) |
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
        _dest: u8,
        source: u8,
    ) -> Result<(), MstpError> {
        if let Some(MstpFrameType::ReplyToPollForMaster) = ftype {
            // Found a master!
            debug!("Received ReplyToPollForMaster from {}", source);
            self.next_station = source;
            self.sole_master = false;
            self.state = MstpState::PassToken;
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
                    debug!("No token timeout, starting PollForMaster");
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
                        self.send_data_frame(&data, dest, expecting_reply)?;
                        self.frame_count += 1;

                        if expecting_reply {
                            // Transition to WaitForReply
                            debug!("Sent frame expecting reply, transitioning to WaitForReply");
                            self.reply_timer = Some(Instant::now());
                            self.state = MstpState::WaitForReply;
                        }
                    } else {
                        // No frames to send
                        self.state = MstpState::DoneWithToken;
                    }
                } else {
                    // Sent max frames
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
                    debug!("Poll interval reached, starting PollForMaster");
                    self.token_count = 0;
                    self.poll_station = (self.next_station + 1) % (self.max_master + 1);
                    if self.poll_station != self.station_address {
                        self.send_poll_for_master(self.poll_station)?;
                        self.state = MstpState::PollForMaster;
                        self.silence_timer = Instant::now();
                    } else {
                        // Would poll ourselves, just pass token
                        self.state = MstpState::PassToken;
                    }
                } else {
                    // Normal token pass
                    self.state = MstpState::PassToken;
                }
            }

            MstpState::PassToken => {
                self.send_token(self.next_station)?;
                debug!("Token passed to station {}", self.next_station);
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
                if self.silence_timer.elapsed() > Duration::from_millis(self.t_slot) {
                    // No reply, try next address
                    self.poll_station = (self.poll_station + 1) % (self.max_master + 1);

                    if self.poll_station == self.station_address {
                        // Polled everyone, no response - we're sole master
                        if !self.sole_master {
                            info!("No other masters found, operating as sole master");
                        }
                        self.sole_master = true;
                        self.next_station = self.station_address;
                        self.state = MstpState::UseToken;
                        self.usage_timer = Some(Instant::now());
                        self.frame_count = 0;
                    } else {
                        // Poll next station
                        self.send_poll_for_master(self.poll_station)?;
                        self.silence_timer = Instant::now();
                    }
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

        // Only log data frames, not token/poll traffic
        if (ftype as u8) >= 5 {
            info!("TX frame: type={:?} dest={} len={}", ftype, dest, data_len);
        }

        // Send the frame
        // Note: M5Stack RS-485 HAT has automatic direction control via SP485EEN chip
        // The TX line controls DE/RE automatically - no GPIO needed
        self.uart.write(&frame).map_err(|e| MstpError::IoError(format!("{:?}", e)))?;

        // Wait for all bytes to be transmitted
        // At 38400 baud: each byte = 10 bits = ~260us
        // Total frame time = frame.len() * 260us
        let tx_time_us = (frame.len() as u64) * 260 + 100; // Add 100us margin
        std::thread::sleep(std::time::Duration::from_micros(tx_time_us));

        self.silence_timer = Instant::now();
        self.tx_frame_count += 1;

        Ok(())
    }

    /// Get frame statistics (rx_count, tx_count)
    pub fn get_frame_stats(&self) -> (u64, u64) {
        (self.rx_frame_count, self.tx_frame_count)
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

/// Calculate MS/TP data CRC (16-bit)
fn calculate_data_crc(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;

    for &byte in data {
        let mut byte = byte;
        for _ in 0..8 {
            let bit = (byte ^ (crc as u8)) & 0x01;
            crc >>= 1;
            if bit != 0 {
                crc ^= 0xA001;
            }
            byte >>= 1;
        }
    }

    !crc
}

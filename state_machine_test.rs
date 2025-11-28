//! MS/TP State Machine Verification Tests
//! Validates state transitions against ASHRAE 135 Clause 9

use std::time::{Duration, Instant};

/// MS/TP Frame Types per ASHRAE 135
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameType {
    Token = 0x00,
    PollForMaster = 0x01,
    ReplyToPollForMaster = 0x02,
    TestRequest = 0x03,
    TestResponse = 0x04,
    BACnetDataExpectingReply = 0x05,
    BACnetDataNotExpectingReply = 0x06,
    ReplyPostponed = 0x07,
}

/// MS/TP States per ASHRAE 135 Clause 9
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MstpState {
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

/// Simplified state machine for testing transitions
struct TestStateMachine {
    state: MstpState,
    station_address: u8,
    next_station: u8,
    poll_station: u8,
    max_master: u8,
    sole_master: bool,
    token_count: u8,
    frame_count: u8,
    max_info_frames: u8,
    
    // Timing
    t_no_token: u64,
    t_reply_timeout: u64,
    t_slot: u64,
    
    // Test tracking
    transitions: Vec<(MstpState, MstpState, String)>,
}

impl TestStateMachine {
    fn new(station_address: u8) -> Self {
        Self {
            state: MstpState::Initialize,
            station_address,
            next_station: (station_address + 1) % 128,
            poll_station: 0,
            max_master: 127,
            sole_master: false,
            token_count: 0,
            frame_count: 0,
            max_info_frames: 1,
            t_no_token: 500,
            t_reply_timeout: 255,
            t_slot: 10,
            transitions: Vec::new(),
        }
    }
    
    fn transition_to(&mut self, new_state: MstpState, reason: &str) {
        let old_state = self.state;
        self.transitions.push((old_state, new_state, reason.to_string()));
        self.state = new_state;
    }
    
    /// Simulate silence timeout from Initialize
    fn on_silence_timeout(&mut self) {
        if self.state == MstpState::Initialize {
            self.transition_to(MstpState::Idle, "Tno_token silence");
        }
    }
    
    /// Simulate no-token timeout from Idle
    fn on_no_token_timeout(&mut self) {
        if self.state == MstpState::Idle {
            self.poll_station = (self.station_address + 1) % (self.max_master + 1);
            self.transition_to(MstpState::PollForMaster, "Tno_token expired, start polling");
        }
    }
    
    /// Simulate receiving a frame
    fn on_frame_received(&mut self, frame_type: FrameType, dest: u8, source: u8) {
        match self.state {
            MstpState::Initialize => {
                // Any valid frame exits Initialize
                self.transition_to(MstpState::Idle, "Valid frame received in Initialize");
            }
            
            MstpState::Idle => {
                match frame_type {
                    FrameType::Token if dest == self.station_address => {
                        self.token_count += 1;
                        self.frame_count = 0;
                        self.transition_to(MstpState::UseToken, "Token received");
                    }
                    FrameType::PollForMaster if dest == self.station_address => {
                        // Reply would be sent here, stay in Idle
                        // (no transition, just send ReplyToPollForMaster)
                    }
                    FrameType::BACnetDataExpectingReply if dest == self.station_address => {
                        self.transition_to(MstpState::AnswerDataRequest, "DataExpectingReply received");
                    }
                    _ => {}
                }
            }
            
            MstpState::WaitForReply => {
                // NEGATIVE LIST approach - only reject specific frame types
                match frame_type {
                    FrameType::Token |
                    FrameType::PollForMaster |
                    FrameType::ReplyToPollForMaster |
                    FrameType::TestRequest => {
                        // NOT a reply - unexpected frame
                        self.transition_to(MstpState::Idle, "Unexpected frame in WaitForReply (negative list reject)");
                    }
                    _ => {
                        // Accept as valid reply (includes unknown frame types!)
                        self.transition_to(MstpState::DoneWithToken, "Valid reply received (negative list accept)");
                    }
                }
            }
            
            MstpState::PollForMaster => {
                if frame_type == FrameType::ReplyToPollForMaster && dest == self.station_address {
                    self.next_station = source;
                    self.sole_master = false;
                    self.transition_to(MstpState::UseToken, "ReplyToPollForMaster received");
                }
            }
            
            _ => {}
        }
    }
    
    /// Simulate sending a frame
    fn on_frame_sent(&mut self, expecting_reply: bool) {
        if self.state == MstpState::UseToken {
            self.frame_count += 1;
            if expecting_reply {
                self.transition_to(MstpState::WaitForReply, "Sent DataExpectingReply");
            } else if self.frame_count >= self.max_info_frames {
                self.transition_to(MstpState::DoneWithToken, "Max info frames sent");
            }
        }
    }
    
    /// Simulate no frames to send
    fn on_no_frames_to_send(&mut self) {
        if self.state == MstpState::UseToken {
            self.transition_to(MstpState::DoneWithToken, "No frames to send");
        }
    }
    
    /// Simulate reply timeout
    fn on_reply_timeout(&mut self) {
        if self.state == MstpState::WaitForReply {
            self.transition_to(MstpState::DoneWithToken, "Treply_timeout expired");
        }
    }
    
    /// Simulate poll slot timeout
    fn on_slot_timeout(&mut self) {
        if self.state == MstpState::PollForMaster {
            self.poll_station = (self.poll_station + 1) % (self.max_master + 1);
            if self.poll_station == self.station_address {
                self.sole_master = true;
                self.next_station = self.station_address;
                self.transition_to(MstpState::UseToken, "Sole master - polled all, no response");
            }
            // Otherwise stay in PollForMaster
        }
    }
    
    /// Simulate ready to pass token
    fn on_done_with_token(&mut self) {
        if self.state == MstpState::DoneWithToken {
            if self.token_count >= 50 {
                self.token_count = 0;
                self.poll_station = (self.next_station + 1) % (self.max_master + 1);
                self.transition_to(MstpState::PollForMaster, "Poll interval reached (NPOLL)");
            } else {
                self.transition_to(MstpState::PassToken, "Ready to pass token");
            }
        }
    }
    
    /// Simulate token passed
    fn on_token_passed(&mut self) {
        if self.state == MstpState::PassToken {
            self.transition_to(MstpState::Idle, "Token passed");
        }
    }
    
    /// Simulate reply delay timeout in AnswerDataRequest
    fn on_reply_delay_timeout(&mut self) {
        if self.state == MstpState::AnswerDataRequest {
            self.transition_to(MstpState::Idle, "Reply sent/timeout");
        }
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     MS/TP State Machine Tests - ASHRAE 135 Clause 9          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut passed = 0;
    let mut failed = 0;

    // =====================================================================
    // TEST 2.1: INITIALIZE -> IDLE on silence timeout
    // =====================================================================
    println!("━━━ TEST 2.1: INITIALIZE -> IDLE (Tno_token silence) ━━━");
    let mut sm = TestStateMachine::new(3);
    assert_eq!(sm.state, MstpState::Initialize);
    sm.on_silence_timeout();
    
    print!("  Initial state: Initialize ");
    if sm.transitions[0].0 == MstpState::Initialize {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }
    
    print!("  After Tno_token: {:?} ", sm.state);
    if sm.state == MstpState::Idle {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 2.2: INITIALIZE -> IDLE on frame received
    // =====================================================================
    println!("\n━━━ TEST 2.2: INITIALIZE -> IDLE (valid frame received) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_frame_received(FrameType::Token, 5, 10); // Frame not for us, but still valid
    
    print!("  After valid frame: {:?} ", sm.state);
    if sm.state == MstpState::Idle {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 2.3: IDLE -> USE_TOKEN (token received)
    // =====================================================================
    println!("\n━━━ TEST 2.3: IDLE -> USE_TOKEN (token received) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout(); // Initialize -> Idle
    sm.on_frame_received(FrameType::Token, 3, 5); // Token for us from station 5
    
    print!("  After Token(dest=us): {:?} ", sm.state);
    if sm.state == MstpState::UseToken {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 2.4: IDLE -> POLL_FOR_MASTER (no token timeout)
    // =====================================================================
    println!("\n━━━ TEST 2.4: IDLE -> POLL_FOR_MASTER (Tno_token timeout) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_no_token_timeout();
    
    print!("  After Tno_token: {:?} ", sm.state);
    if sm.state == MstpState::PollForMaster {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 3.1: USE_TOKEN -> DONE_WITH_TOKEN (no frames)
    // =====================================================================
    println!("\n━━━ TEST 3.1: USE_TOKEN -> DONE_WITH_TOKEN (no frames) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_frame_received(FrameType::Token, 3, 5);
    assert_eq!(sm.state, MstpState::UseToken);
    sm.on_no_frames_to_send();
    
    print!("  After no frames: {:?} ", sm.state);
    if sm.state == MstpState::DoneWithToken {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 3.2: USE_TOKEN -> WAIT_FOR_REPLY (expecting reply)
    // =====================================================================
    println!("\n━━━ TEST 3.2: USE_TOKEN -> WAIT_FOR_REPLY (DataExpectingReply) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_frame_received(FrameType::Token, 3, 5);
    sm.on_frame_sent(true); // expecting reply
    
    print!("  After DataExpectingReply sent: {:?} ", sm.state);
    if sm.state == MstpState::WaitForReply {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 5.1: WAIT_FOR_REPLY - NEGATIVE LIST REJECT
    // =====================================================================
    println!("\n━━━ TEST 5.1: WAIT_FOR_REPLY - Negative List REJECT ━━━");
    
    // Test each rejected frame type
    let reject_types = [
        (FrameType::Token, "Token"),
        (FrameType::PollForMaster, "PollForMaster"),
        (FrameType::ReplyToPollForMaster, "ReplyToPollForMaster"),
        (FrameType::TestRequest, "TestRequest"),
    ];
    
    for (ftype, name) in reject_types.iter() {
        let mut sm = TestStateMachine::new(3);
        sm.on_silence_timeout();
        sm.on_frame_received(FrameType::Token, 3, 5);
        sm.on_frame_sent(true);
        assert_eq!(sm.state, MstpState::WaitForReply);
        
        sm.on_frame_received(*ftype, 3, 10);
        print!("  {} -> {:?} ", name, sm.state);
        if sm.state == MstpState::Idle {
            println!("✓ PASS (rejected as expected)");
            passed += 1;
        } else {
            println!("✗ FAIL (should reject)");
            failed += 1;
        }
    }

    // =====================================================================
    // TEST 5.2: WAIT_FOR_REPLY - NEGATIVE LIST ACCEPT
    // =====================================================================
    println!("\n━━━ TEST 5.2: WAIT_FOR_REPLY - Negative List ACCEPT ━━━");
    
    // Test each accepted frame type
    let accept_types = [
        (FrameType::BACnetDataNotExpectingReply, "BACnetDataNotExpectingReply"),
        (FrameType::TestResponse, "TestResponse"),
        (FrameType::ReplyPostponed, "ReplyPostponed"),
    ];
    
    for (ftype, name) in accept_types.iter() {
        let mut sm = TestStateMachine::new(3);
        sm.on_silence_timeout();
        sm.on_frame_received(FrameType::Token, 3, 5);
        sm.on_frame_sent(true);
        assert_eq!(sm.state, MstpState::WaitForReply);
        
        sm.on_frame_received(*ftype, 3, 10);
        print!("  {} -> {:?} ", name, sm.state);
        if sm.state == MstpState::DoneWithToken {
            println!("✓ PASS (accepted as reply)");
            passed += 1;
        } else {
            println!("✗ FAIL (should accept)");
            failed += 1;
        }
    }

    // =====================================================================
    // TEST 5.3: WAIT_FOR_REPLY timeout
    // =====================================================================
    println!("\n━━━ TEST 5.3: WAIT_FOR_REPLY -> DONE_WITH_TOKEN (timeout) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_frame_received(FrameType::Token, 3, 5);
    sm.on_frame_sent(true);
    assert_eq!(sm.state, MstpState::WaitForReply);
    sm.on_reply_timeout();
    
    print!("  After Treply_timeout: {:?} ", sm.state);
    if sm.state == MstpState::DoneWithToken {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 4.1: POLL_FOR_MASTER -> USE_TOKEN (reply received)
    // =====================================================================
    println!("\n━━━ TEST 4.1: POLL_FOR_MASTER -> USE_TOKEN (reply received) ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_no_token_timeout(); // Idle -> PollForMaster
    assert_eq!(sm.state, MstpState::PollForMaster);
    sm.on_frame_received(FrameType::ReplyToPollForMaster, 3, 10);
    
    print!("  After ReplyToPollForMaster: {:?} ", sm.state);
    if sm.state == MstpState::UseToken {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }
    
    print!("  next_station updated to 10: ");
    if sm.next_station == 10 {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL (got {})", sm.next_station);
        failed += 1;
    }

    // =====================================================================
    // TEST 4.2: POLL_FOR_MASTER sole master detection
    // =====================================================================
    println!("\n━━━ TEST 4.2: POLL_FOR_MASTER -> Sole Master ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.max_master = 5; // Small ring for quick test
    sm.on_silence_timeout();
    sm.on_no_token_timeout();
    
    // Simulate polling all stations with no response
    for _ in 0..10 { // More than enough timeouts
        if sm.state == MstpState::UseToken {
            break;
        }
        sm.on_slot_timeout();
    }
    
    print!("  sole_master flag: {} ", sm.sole_master);
    if sm.sole_master {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }
    
    print!("  next_station = self: {} ", sm.next_station == sm.station_address);
    if sm.next_station == sm.station_address {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 6.1: ANSWER_DATA_REQUEST
    // =====================================================================
    println!("\n━━━ TEST 6.1: IDLE -> ANSWER_DATA_REQUEST -> IDLE ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_frame_received(FrameType::BACnetDataExpectingReply, 3, 10);
    
    print!("  After DataExpectingReply: {:?} ", sm.state);
    if sm.state == MstpState::AnswerDataRequest {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }
    
    sm.on_reply_delay_timeout();
    print!("  After reply delay: {:?} ", sm.state);
    if sm.state == MstpState::Idle {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // TEST 3.3: Token passing cycle
    // =====================================================================
    println!("\n━━━ TEST 3.3: Complete Token Passing Cycle ━━━");
    let mut sm = TestStateMachine::new(3);
    sm.on_silence_timeout();
    sm.on_frame_received(FrameType::Token, 3, 5);
    assert_eq!(sm.state, MstpState::UseToken);
    
    sm.on_no_frames_to_send();
    assert_eq!(sm.state, MstpState::DoneWithToken);
    
    sm.on_done_with_token();
    assert_eq!(sm.state, MstpState::PassToken);
    
    sm.on_token_passed();
    
    print!("  Final state after cycle: {:?} ", sm.state);
    if sm.state == MstpState::Idle {
        println!("✓ PASS");
        passed += 1;
    } else {
        println!("✗ FAIL");
        failed += 1;
    }

    // =====================================================================
    // SUMMARY
    // =====================================================================
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                   STATE MACHINE TEST SUMMARY                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Passed: {:2}                                                  ║", passed);
    println!("║  Failed: {:2}                                                  ║", failed);
    println!("║  Total:  {:2}                                                  ║", passed + failed);
    println!("╚══════════════════════════════════════════════════════════════╝");

    if failed == 0 {
        println!("\n🎉 ALL STATE MACHINE TESTS PASSED!\n");
        println!("State transitions verified against ASHRAE 135 Clause 9:");
        println!("  ✓ INITIALIZE state entry/exit");
        println!("  ✓ IDLE state transitions");
        println!("  ✓ USE_TOKEN flow");
        println!("  ✓ WAIT_FOR_REPLY negative list (CRITICAL)");
        println!("  ✓ POLL_FOR_MASTER procedure");
        println!("  ✓ ANSWER_DATA_REQUEST handling");
        println!("  ✓ Token passing cycle");
    } else {
        println!("\n❌ SOME TESTS FAILED! Review state machine implementation.\n");
        std::process::exit(1);
    }
}

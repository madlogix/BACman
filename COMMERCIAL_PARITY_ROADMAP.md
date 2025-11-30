# BACnet Router Commercial Parity Roadmap

**Project:** MS/TP to IP Gateway
**Target:** Commercial-grade BACnet router feature parity
**Started:** 2025-11-30
**Current Status:** ~60% Complete

---

## Progress Summary

| Phase | Description | Status | Completion |
|-------|-------------|--------|------------|
| Phase 1-4 | Core Routing & Transaction Tracking | COMPLETE | 100% |
| Phase 5 | Production Ready | COMPLETE | 100% |
| Phase 6 | Full Commercial | IN PROGRESS | 70% |
| Phase 7 | Enterprise Features | NOT STARTED | 0% |

---

## Completed Features (Phases 1-4)

### Core Routing
- [x] MS/TP to IP bidirectional packet routing
- [x] NPDU parsing and routing per ASHRAE 135 Clause 6.2.2
- [x] BVLC Original-Unicast-NPDU (0x0A)
- [x] BVLC Original-Broadcast-NPDU (0x0B)
- [x] Source network/address (SNET/SADR) insertion
- [x] Destination network/address (DNET/DADR) handling
- [x] Hop count management
- [x] Final delivery detection and DNET stripping

### Device Discovery
- [x] Who-Is forwarding (IP to MS/TP)
- [x] I-Am forwarding (MS/TP to IP)
- [x] I-Am-Router-To-Network broadcasts
- [x] Gateway local device I-Am response
- [x] Periodic router announcements

### Transaction Management
- [x] Transaction tracking for confirmed services
- [x] Request/response correlation by invoke ID
- [x] Per-service timeout configuration
- [x] Abort on timeout
- [x] Transaction table with capacity limits

### MS/TP Implementation
- [x] Token ring state machine (ASHRAE 135 Clause 9)
- [x] Master discovery and polling
- [x] Frame encoding/decoding
- [x] CRC calculation
- [x] Timing-critical response paths

### BBMD Functions (Basic)
- [x] Foreign Device Registration
- [x] Foreign Device Table with TTL
- [x] Forwarded-NPDU handling (receive)

### Segmentation (Basic)
- [x] Segmented request detection
- [x] Segment reassembly buffer
- [x] Segmented response streaming

### Configuration
- [x] AP mode WiFi configuration
- [x] Web server for WiFi setup
- [x] Network number configuration (compile-time)

---

## Phase 5: Production Ready (Target: 85% Parity)

**Estimated Effort:** 3-4 weeks
**Priority:** CRITICAL for production deployment

### 5.1 Router Device Object ✓ COMPLETE
> Required for BACnet tools (Yabe, VTS, etc.) to discover and query the router

- [x] **5.1.1** Implement Device object (object-identifier, object-name, object-type)
- [x] **5.1.2** Add vendor-identifier property (get ASHRAE vendor ID or use 999)
- [x] **5.1.3** Add model-name property ("BACrust MS/TP Gateway")
- [x] **5.1.4** Add firmware-revision property
- [x] **5.1.5** Add application-software-version property
- [x] **5.1.6** Add protocol-version, protocol-revision properties
- [x] **5.1.7** Add protocol-services-supported property
- [x] **5.1.8** Add protocol-object-types-supported property
- [x] **5.1.9** Add max-apdu-length-accepted property
- [x] **5.1.10** Add segmentation-supported property
- [x] **5.1.11** Add object-list property (list of objects in device)

**Implementation:** `mstp-ip-gateway/src/local_device.rs`

**Testing Checklist:**
- [ ] Device appears in Yabe device list
- [ ] Device appears in VTS device list
- [ ] ReadProperty device,X object-name works
- [ ] ReadPropertyMultiple works for device properties

### 5.2 ReadProperty Service Handler ✓ COMPLETE
> Handle ReadProperty requests to the router's local device

- [x] **5.2.1** Parse ReadProperty requests targeting local device
- [x] **5.2.2** Implement property value encoding for all Device properties
- [x] **5.2.3** Return proper Complex-ACK response
- [x] **5.2.4** Handle object-list property (return array of object IDs)
- [x] **5.2.5** Return Error for unknown properties
- [x] **5.2.6** ReadPropertyMultiple support

**Implementation:** `mstp-ip-gateway/src/local_device.rs` - `handle_read_property()` and `handle_read_property_multiple()`

**Testing Checklist:**
- [ ] ReadProperty device,X object-name returns correct value
- [ ] ReadProperty device,X vendor-identifier returns correct value
- [ ] ReadProperty unknown-property returns Error

### 5.3 Network Port Objects ✓ COMPLETE
> Represent the IP and MS/TP network interfaces

- [x] **5.3.1** Create Network-Port object for BACnet/IP interface
- [x] **5.3.2** Create Network-Port object for MS/TP interface
- [x] **5.3.3** Add network-number property to each
- [x] **5.3.4** Add mac-address property
- [x] **5.3.5** Add link-speed property (for MS/TP baud rate)
- [x] **5.3.6** Add network-type property (IP vs MS/TP)
- [x] **5.3.7** Add additional required properties (ip-address, subnet-mask, max-master, max-info-frames, etc.)

**Implementation:** `mstp-ip-gateway/src/local_device.rs` - NetworkPort struct with full property support

### 5.4 Retry Mechanism ✓ COMPLETE
> Retry failed transmissions before aborting

- [x] **5.4.1** Add retry_count field to PendingTransaction (already existed)
- [x] **5.4.2** Implement configurable max_retries (default: 3) (already existed)
- [x] **5.4.3** On timeout, retry transmission if retries remaining
- [x] **5.4.4** Only send Abort after all retries exhausted
- [x] **5.4.5** Add exponential backoff between retries (1.5x per retry)
- [x] **5.4.6** Store original NPDU for retransmission

**Implementation:**
- `mstp-ip-gateway/src/transaction.rs` - `original_npdu` field, exponential backoff in `retry()`
- `mstp-ip-gateway/src/gateway.rs` - `process_transaction_timeouts()` with retry logic
- `mstp-ip-gateway/src/main.rs` - `drain_mstp_send_queue()` integration

**Testing Checklist:**
- [ ] Noisy network: requests eventually succeed
- [ ] Completely failed network: Abort after max retries

### 5.5 Connection Monitoring ✓ COMPLETE
> Detect network failures and report status

- [x] **5.5.1** Track last successful packet time per network
- [x] **5.5.2** Implement network health check (60-second threshold)
- [x] **5.5.3** Set status flag when network appears down
- [x] **5.5.4** Log network failure/recovery events
- [ ] **5.5.5** Add network-status to Network-Port objects (future)

**Implementation:** `gateway.rs` - `check_network_health()`, `is_network_healthy()`, activity timestamps

### 5.6 Traffic Statistics ✓ COMPLETE
> Operational visibility for troubleshooting

- [x] **5.6.1** Count packets routed IP->MS/TP
- [x] **5.6.2** Count packets routed MS/TP->IP
- [x] **5.6.3** Count routing errors
- [x] **5.6.4** Count transaction timeouts
- [x] **5.6.5** Track bytes transferred per direction
- [x] **5.6.6** Add last-activity timestamp
- [x] **5.6.7** Periodic stats logging (every 60s)

**Implementation:** `gateway.rs` - Extended `GatewayStats`, `get_stats_summary()`

### 5.7 Improved Error Handling ✓ COMPLETE
> Proper BACnet error responses

- [x] **5.7.1** Send Reject-Message-To-Network for unknown DNET
- [x] **5.7.2** Send Reject response for unsupported services (Reject reason 9: unrecognized-service)
- [x] **5.7.3** Handle malformed packets gracefully (BVLC, NPDU validation)
- [x] **5.7.4** Log all error conditions with context (hex dumps, addresses)

**Implementation:**
- `gateway.rs` - `hex_dump()`, enhanced BVLC/NPDU validation, Reject-Message-To-Network
- `local_device.rs` - `build_reject_response()` for unsupported services

---

## Phase 6: Full Commercial (Target: 95% Parity)

**Estimated Effort:** 6-8 weeks
**Priority:** HIGH for enterprise deployment

### 6.1 Web Configuration UI ✓ COMPLETE
> User-friendly configuration without recompiling

- [x] **6.1.1** Create responsive HTML/CSS configuration page
- [x] **6.1.2** WiFi network selection and password entry
- [x] **6.1.3** IP address configuration (DHCP vs static)
- [x] **6.1.4** MS/TP network number configuration
- [x] **6.1.5** IP network number configuration
- [x] **6.1.6** MS/TP MAC address configuration
- [x] **6.1.7** Baud rate selection
- [x] **6.1.8** Device instance number configuration
- [x] **6.1.9** Save configuration button
- [x] **6.1.10** Reboot button
- [x] **6.1.11** Status page showing current configuration
- [x] **6.1.12** Statistics page showing traffic counts
- [x] **6.1.13** Device discovery scan (Who-Is)
- [x] **6.1.14** JSON API endpoints (/api/status, /api/devices, etc.)

**Implementation:** `web.rs` - Full web server with config, status, scan, export

### 6.2 Persistent Configuration (NVS) ✓ COMPLETE
> Survive power cycles

- [x] **6.2.1** Store WiFi credentials in NVS
- [x] **6.2.2** Store network numbers in NVS
- [x] **6.2.3** Store device instance in NVS
- [x] **6.2.4** Store MS/TP MAC address in NVS
- [x] **6.2.5** Store baud rate in NVS
- [x] **6.2.6** Load configuration on boot
- [x] **6.2.7** Factory reset capability (web /reset endpoint)

**Implementation:** `config.rs` - `load_from_nvs()`, `save_to_nvs()`, `clear_nvs()`

### 6.3 Broadcast Distribution Table (BDT)
> Required for multi-subnet BACnet/IP deployments

- [ ] **6.3.1** Implement BDT storage (list of BBMD addresses)
- [ ] **6.3.2** Implement Read-Broadcast-Distribution-Table service
- [ ] **6.3.3** Implement Write-Broadcast-Distribution-Table service
- [ ] **6.3.4** Forward broadcasts to all BDT entries
- [ ] **6.3.5** Store BDT in persistent storage
- [ ] **6.3.6** Web UI for BDT configuration

### 6.4 Segment Retransmission
> Reliable large file transfers

- [ ] **6.4.1** Track acknowledged segments
- [ ] **6.4.2** Detect missing Segment-ACK
- [ ] **6.4.3** Retransmit unacknowledged segments
- [ ] **6.4.4** Handle Segment-NAK with retransmission
- [ ] **6.4.5** Implement segment timeout per ASHRAE 135

### 6.5 Who-Is-Router-To-Network Handler ✓ COMPLETE
> Allow devices to discover routes

- [x] **6.5.1** Parse Who-Is-Router-To-Network requests
- [x] **6.5.2** Respond with I-Am-Router-To-Network for known networks
- [ ] **6.5.3** Forward to other routers for unknown networks (future)

**Implementation:** `gateway.rs` - `handle_network_message_from_mstp()`, `build_i_am_router_to_network()`

### 6.6 Initialize-Routing-Table
> Allow external tools to configure routing

- [ ] **6.6.1** Parse Initialize-Routing-Table request
- [ ] **6.6.2** Update internal routing table
- [ ] **6.6.3** Send Initialize-Routing-Table-Ack response
- [ ] **6.6.4** Persist routing table changes

### 6.7 Display UI ✓ COMPLETE
> Use M5StickC Plus2 LCD for status

- [x] **6.7.1** Show IP address on display
- [x] **6.7.2** Show MS/TP network status
- [x] **6.7.3** Show packet count
- [x] **6.7.4** Show error indicators (CRC errors)
- [x] **6.7.5** Button A: cycle through status screens
- [x] **6.7.6** Button B: trigger AP config mode
- [x] **6.7.7** Multiple screens: Status, Connection, APConfig, Splash

**Implementation:** `display.rs` - Full ST7789V2 LCD driver with multiple screens

---

## Phase 7: Enterprise Features (Target: 100% Parity)

**Estimated Effort:** 8+ weeks
**Priority:** LOW - only for specific enterprise requirements

### 7.1 Multi-Router Support
> For large buildings with multiple MS/TP networks

- [ ] **7.1.1** Dynamic routing table with multiple network entries
- [ ] **7.1.2** Learn routes from I-Am-Router-To-Network
- [ ] **7.1.3** Route table aging and refresh
- [ ] **7.1.4** Support for router-to-router communication

### 7.2 What-Is-Network-Number / Network-Number-Is
> Network number discovery protocol

- [ ] **7.2.1** Handle What-Is-Network-Number requests
- [ ] **7.2.2** Respond with Network-Number-Is
- [ ] **7.2.3** Learn network numbers from other routers

### 7.3 Establish/Disconnect Connection-To-Network
> For dial-up or VPN connections (rarely used)

- [ ] **7.3.1** Parse Establish-Connection-To-Network
- [ ] **7.3.2** Parse Disconnect-Connection-To-Network
- [ ] **7.3.3** Maintain connection state

### 7.4 BACnet/SC (Secure Connect)
> TLS-based secure BACnet - complex implementation

- [ ] **7.4.1** TLS certificate management
- [ ] **7.4.2** Secure WebSocket transport
- [ ] **7.4.3** Hub/Spoke topology support
- [ ] **7.4.4** Certificate validation

**Note:** BACnet/SC is very complex and may not be practical on ESP32 due to memory constraints.

### 7.5 Redundancy
> High-availability for critical systems

- [ ] **7.5.1** Primary/backup router configuration
- [ ] **7.5.2** Heartbeat between redundant routers
- [ ] **7.5.3** Automatic failover
- [ ] **7.5.4** State synchronization

---

## Testing Requirements

### Unit Tests
- [ ] Transaction table operations
- [ ] NPDU parsing edge cases
- [ ] BVLC encoding/decoding
- [ ] Timeout calculations

### Integration Tests
- [ ] Who-Is/I-Am round trip
- [ ] ReadProperty through router
- [ ] WriteProperty through router
- [ ] Segmented transfer (AtomicWriteFile)
- [ ] Multiple simultaneous transactions

### Interoperability Tests
- [ ] JCI CCT device discovery
- [ ] JCI CCT code download
- [ ] Yabe device discovery
- [ ] VTS device discovery
- [ ] Tridium Niagara discovery
- [ ] Siemens Desigo CC discovery

### Stress Tests
- [ ] 100 rapid Who-Is requests
- [ ] Maximum transaction table capacity
- [ ] 24-hour continuous operation
- [ ] Network disconnect/reconnect recovery

---

## Hardware Variants

### Current Target
- M5StickC Plus2 (ESP32, 8MB flash, 8MB PSRAM)

### Future Targets
- [ ] ESP32-S3 based devices (more RAM)
- [ ] Raspberry Pi Pico W (RP2040)
- [ ] Linux-based (Raspberry Pi)
- [ ] Custom PCB design

---

## Documentation TODO

- [ ] User manual (installation, configuration)
- [ ] API documentation
- [ ] Troubleshooting guide
- [ ] Network design best practices
- [ ] BACnet conformance statement (PICS)

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-11-xx | Initial MS/TP token ring |
| 0.2.0 | 2025-11-xx | Basic routing |
| 0.3.0 | 2025-11-xx | Transaction tracking |
| 0.4.0 | 2025-11-30 | JCI CCT device discovery working |
| 0.5.0 | TBD | Phase 5 complete |
| 1.0.0 | TBD | Phase 6 complete - Production release |

---

## Notes

### Commit Hash References
- Phase 1-4 Complete: `01f0a04` (2025-11-30)
- Phase 5.1-5.2 Complete (LocalDevice): Already implemented, verified 2025-11-30

### Known Issues
1. ~~Abort sent for requests to gateway's local device~~ **FIXED** - Device object implemented, Reject sent for unsupported services
2. No retry on timeout (aborts immediately)
3. Configuration requires recompilation

### Dependencies
- ESP-IDF 5.x
- esp-idf-hal
- esp-idf-svc
- embedded-svc

---

*Last Updated: 2025-11-30*

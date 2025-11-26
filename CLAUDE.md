# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## MANDATORY: BACnet Standard Reference

**CRITICAL REQUIREMENT: For ANY work involving BACnet functionality, you MUST consult the BACnet Standard documentation in `BACnet_Standard/` BEFORE implementing or modifying code.**

### How to Use the BACnet Standard Documentation

1. **START HERE**: Read `BACnet_Standard/00-INDEX.md` first - it provides:
   - Quick lookup tables by feature
   - Document categories and descriptions
   - Implementation priority guidance
   - Protocol layer reference diagram

2. **Feature Lookup Quick Reference**:

| Feature | Primary Document | Supporting Docs |
|---------|------------------|-----------------|
| Device Discovery (Who-Is/I-Am) | `16-REMOTE-DEVICE-MANAGEMENT-SERVICES.md` | `Annex-J-BACnet-IP.md` |
| ReadProperty/WriteProperty | `15-OBJECT-ACCESS-SERVICES.md` | `Annex-F-EXAMPLES-OF-APDU-ENCODING.md` |
| COV Subscriptions | `13-ALARM-AND-EVENT-SERVICES.md` | `Annex-M-GUIDE-TO-EVENT-NOTIFICATION-PRIORITY-ASSIGNMENTS.md` |
| Object Types | `12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md` | `Annex-L-DESCRIPTIONS-AND-PROFILES-OF-STANDARDIZED-BACnet-D.md` |
| APDU Encoding | `21-FORMAL-DESCRIPTION-OF-APPLICATION-PROTOCOL-DATA-UNITS.md` | `Annex-F-EXAMPLES-OF-APDU-ENCODING.md` |
| BACnet/IP (UDP) | `Annex-J-BACnet-IP.md` | `Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md` |
| Network Routing/VMAC | `Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md` | `03-DEFINITIONS.md` |
| CRC Calculation (MS/TP) | `Annex-G-CALCULATION-OF-CRC.md` | - |

3. **Document Reference Order** (when implementing features):
   1. Check the relevant clause for service definitions (Clauses 13-17)
   2. Check `Annex-J-BACnet-IP.md` for BACnet/IP transport details
   3. Check `Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md` for network routing/VMAC
   4. Check examples in `Annex-E-EXAMPLES-OF-BACnet-APPLICATION-SERVICES.md` and `Annex-F-EXAMPLES-OF-APDU-ENCODING.md`

### Complete BACnet Standard Document List (`BACnet_Standard/`)

#### Core Protocol Clauses
| Document | Description |
|----------|-------------|
| `00-INDEX.md` | **START HERE** - Master index with feature lookup tables |
| `03-DEFINITIONS.md` | Protocol architecture, terms, abbreviations, network/data link layers |
| `12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md` | All 42 BACnet object types |
| `13-ALARM-AND-EVENT-SERVICES.md` | COV reporting, SubscribeCOV, Event algorithms |
| `14-FILE-ACCESS-SERVICES.md` | AtomicReadFile, AtomicWriteFile |
| `15-OBJECT-ACCESS-SERVICES.md` | ReadProperty, WriteProperty, ReadPropertyMultiple, ReadRange |
| `16-REMOTE-DEVICE-MANAGEMENT-SERVICES.md` | Who-Is/I-Am, Who-Has/I-Have, TimeSynchronization |
| `17-AUTHENTICATION-AND-AUTHORIZATION-SERVICES.md` | Security services, Error/Reject/Abort codes |
| `21-FORMAL-DESCRIPTION-OF-APPLICATION-PROTOCOL-DATA-UNITS.md` | APDU grammar, encoding rules |
| `22-CONFORMANCE-AND-INTEROPERABILITY.md` | Extending BACnet (proprietary properties/objects) |

#### Technical Annexes - Data Link & Transport
| Document | Description |
|----------|-------------|
| `Annex-J-BACnet-IP.md` | **CRITICAL** - BACnet/IP over UDP, BBMD, foreign devices |
| `Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md` | VMAC addressing, network routing |
| `Annex-G-CALCULATION-OF-CRC.md` | CRC calculation for MS/TP |
| `Annex-T-COBS.md` | Byte stuffing for MS/TP |
| `Annex-U-BACnet-IPv6.md` | BACnet over IPv6 |

#### Examples & Encoding Reference
| Document | Description |
|----------|-------------|
| `Annex-E-EXAMPLES-OF-BACnet-APPLICATION-SERVICES.md` | Service request/response examples |
| `Annex-F-EXAMPLES-OF-APDU-ENCODING.md` | **USEFUL** - APDU hex dumps, encoding examples |

#### Interoperability & Profiles
| Document | Description |
|----------|-------------|
| `Annex-A-PROTOCOL-IMPLEMENTATION-CONFORMANCE-STATEMENT.md` | PICS template |
| `Annex-B-GUIDE-TO-SPECIFYING-BACnet-DEVICES.md` | BIBBs and device profiles |
| `Annex-K-BACnet-INTEROPERABILITY-BUILDING-BLOCKS.md` | BIBB definitions |
| `Annex-L-DESCRIPTIONS-AND-PROFILES-OF-STANDARDIZED-BACnet-D.md` | Standard device profiles |

#### Web Services & Data Formats
| Document | Description |
|----------|-------------|
| `Annex-Q-XML-DATA-FORMATS.md` | XML format for BACnet objects |
| `Annex-Z-JSON-DATA-FORMATS.md` | JSON schemas (large file - 226KB) |
| `Annex-W-BACnet-WS-RESTful-WEB-SERVICES-INTERFACE.md` | RESTful web services |

---

## Project-Specific Documentation

| Document | Description |
|----------|-------------|
| `MSTP_PROTOCOL_REQUIREMENTS.md` | **CRITICAL** - Detailed MS/TP state machine requirements from ASHRAE 135 Clause 9. Contains all state definitions, timing parameters, and the critical WAIT_FOR_REPLY negative list approach |
| `PROJECT_COMPLETION_PLAN.md` | Task tracking and remaining work items |

**When implementing MS/TP features, ALWAYS consult:**
1. `MSTP_PROTOCOL_REQUIREMENTS.md` - Contains complete Clause 9 state machine specification with implementation guidance
2. Pay special attention to the **WAIT_FOR_REPLY negative list approach** - this is a critical implementation detail that prevents dropped frames

## Project Overview

This is a BACnet MS/TP to IP Gateway project for M5StickC Plus2 (ESP32). It consists of two main components:

1. **bacnet-rs** - A comprehensive BACnet protocol stack library in Rust
2. **mstp-ip-gateway** - ESP32 firmware that bridges MS/TP (RS-485) and BACnet/IP networks

## Build Commands

### bacnet-rs Library

```bash
cd bacnet-rs

# Build library
cargo build

# Build with release optimizations
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run example
cargo run --example whois_scan

# Run with logging
RUST_LOG=debug cargo run --example whois_scan
```

### mstp-ip-gateway (ESP32)

Requires ESP32 Rust toolchain (`espup install`).

```bash
cd mstp-ip-gateway

# Build for ESP32
cargo build --release

# Build and flash to device (opens monitor)
cargo run --release

# Just flash without monitor
espflash flash target/xtensa-esp32-espidf/release/mstp-ip-gateway
```

## Architecture

### Protocol Stack Layers (bacnet-rs)

The BACnet stack follows ASHRAE 135 layered architecture:

```
┌─────────────────────────────────┐
│  Application (app/, client.rs)  │  Service handlers, APDU processing
├─────────────────────────────────┤
│      Service (service/)         │  Confirmed/Unconfirmed services
├─────────────────────────────────┤
│     Transport (transport/)      │  Segmentation, flow control
├─────────────────────────────────┤
│      Network (network/)         │  NPDU routing, addressing
├─────────────────────────────────┤
│   Data Link (datalink/)         │  BACnet/IP, MS/TP, Ethernet
└─────────────────────────────────┘
```

Key modules:
- `datalink/bip.rs` - BACnet/IP with BVLC, UDP socket handling
- `datalink/mstp.rs` - MS/TP frame encoding/decoding (state machine incomplete)
- `network/mod.rs` - NPDU routing, RouterManager, path discovery
- `encoding/mod.rs` - BACnet data type encoding/decoding
- `object/` - Standard BACnet objects (Device, AnalogInput, etc.)

### Gateway Architecture (mstp-ip-gateway)

```
┌──────────────────────────────────────────┐
│              main.rs                      │
│  WiFi init, UART init, task spawning      │
├─────────────┬────────────────────────────┤
│ mstp_driver │      gateway.rs            │
│ RS-485 I/O  │   NPDU routing logic       │
│ Token pass  │   Address translation      │
│ State mach  │   BVLC wrapping            │
└─────────────┴────────────────────────────┘
```

Hardware configuration (M5StickC Plus2 + RS-485 HAT):
- RS-485 UART: TX=GPIO0, RX=GPIO26, 38400 baud default
- RS-485 HAT uses SP485EEN with automatic direction control (no GPIO needed)
- Display: ST7789V2 LCD (240x135), SPI pins: MOSI=15, SCK=13, CS=5, DC=14, RST=12, BL=27
- Buttons: A=GPIO37 (front), B=GPIO39 (side) - input-only pins with external pull-ups
- WiFi: ESP32 internal
- BACnet/IP port: 47808 (UDP)

### M5Stack Reference Documentation
- `M5Unified/` - Cloned from https://github.com/m5stack/M5Unified for hardware reference
- Official M5StickC Plus2 docs: https://docs.m5stack.com/en/core/M5StickC%20PLUS2

## Configuration

Update WiFi credentials in `mstp-ip-gateway/src/config.rs` before building:

```rust
wifi_ssid: "YOUR_WIFI_SSID".to_string(),
wifi_password: "YOUR_WIFI_PASSWORD".to_string(),
```

Adjust network numbers to match your BACnet installation:
- `mstp_network` - Network number for MS/TP side (default: 1)
- `ip_network` - Network number for IP side (default: 2)

## Feature Flags (bacnet-rs)

```toml
# Full features (default)
bacnet-rs = "0.2"

# Minimal for embedded
bacnet-rs = { version = "0.2", default-features = false, features = ["std"] }

# Async support
bacnet-rs = { version = "0.2", features = ["async"] }
```

## Testing

```bash
# Run all tests
cd bacnet-rs && cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test network::tests

# Test device discovery
# Terminal 1: Start responder
cargo run --example responder_device 12345

# Terminal 2: Run scanner
cargo run --example whois_scan
```

## Key Implementation Notes

### MS/TP Token Passing
The MS/TP driver implements the state machine from ASHRAE 135 Clause 9. Critical states:
- `Initialize` → Wait for silence → `Idle`
- `Idle` → Receive token → `UseToken`
- `UseToken` → Send queued frames → `DoneWithToken`
- `DoneWithToken` → Pass token → `Idle`

### Gateway Routing
The gateway translates between MS/TP and IP networks by:
1. Parsing NPDU to extract destination network
2. Adding source network/address for return routing
3. Wrapping in appropriate link layer (BVLC for IP, MS/TP frame for RS-485)
4. Decrementing hop count for routed messages

### BACnet/IP BVLC Functions
Common function codes in `datalink/bip.rs`:
- `0x0A` - Original-Unicast-NPDU
- `0x0B` - Original-Broadcast-NPDU
- `0x04` - Forwarded-NPDU
- `0x05` - Register-Foreign-Device

## Serial Monitor Notes

**IMPORTANT: Do NOT use `cat /dev/ttyACM0` or similar simple serial commands** - they will hang the terminal and require killing the process.

Instead, use the Python serial monitor script:

```bash
# Monitor ESP32 serial output
python3 scripts/serial_monitor.py /dev/ttyACM0

# Or use espflash monitor (but only in interactive terminal, not in Claude Code)
espflash monitor -p /dev/ttyACM0
```

The `espflash monitor` command requires an interactive terminal and won't work properly in automated/background contexts.

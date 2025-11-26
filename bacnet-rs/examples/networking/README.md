# Networking Examples

Network communication and BACnet/IP protocol demonstrations.

## Examples

### `whois_scan.rs`
**Purpose**: BACnet device discovery using Who-Is/I-Am protocol

**Features**:
- Broadcast Who-Is requests to discover devices
- Listen for I-Am responses with device information
- Multi-subnet scanning capabilities
- Device information parsing and display
- Timeout and retry handling

**Usage**:
```bash
cargo run --example networking/whois_scan
```

**What it demonstrates**:
- BACnet device discovery protocol
- UDP broadcast handling
- I-Am response processing
- Network topology understanding
- Device identification methods

---

### `transport_demo.rs`
**Purpose**: BACnet transport layer functionality

**Features**:
- BVLL (BACnet Virtual Link Layer) message handling
- Foreign device registration with BBMD
- Broadcast Distribution Table (BDT) management
- All BACnet/IP transport functions
- BVLL encoding/decoding examples

**Usage**:
```bash
cargo run --example networking/transport_demo
```

**What it demonstrates**:
- Complete BACnet/IP transport layer
- BVLL message types and encoding
- Foreign device registration process
- Broadcast management mechanisms
- Transport protocol internals

---

### `timeout_demo.rs`
**Purpose**: Comprehensive timeout handling for reliable BACnet communication

**Features**:
- **Socket Timeouts**: Read/write timeout configuration and testing
- **Request Tracking**: Confirmed service timeout management with invoke ID tracking
- **Adaptive Timeouts**: Dynamic timeout calculation based on network performance
- **Retry Logic**: Exponential backoff with configurable parameters
- **Timeout Monitoring**: Statistics collection and performance analysis
- **Condition Waiting**: Timeout-aware condition checking utilities

**Usage**:
```bash
cargo run --example networking/timeout_demo
```

**What it demonstrates**:
- Production-ready timeout management strategies
- Network performance adaptation techniques
- Robust error recovery with exponential backoff
- Request lifecycle tracking and monitoring
- Timeout statistics and diagnostics
- Real-world timeout handling patterns

## Network Architecture

### BACnet/IP Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Local Device  │    │      BBMD       │    │ Remote Device   │
│                 │    │ (Broadcast      │    │                 │
│ ┌─────────────┐ │    │  Management)    │    │ ┌─────────────┐ │
│ │   Objects   │ │    │                 │    │ │   Objects   │ │
│ └─────────────┘ │    │ ┌─────────────┐ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ │    BDT      │ │    │ ┌─────────────┐ │
│ │    APDU     │ │    │ │ (Broadcast  │ │    │ │    APDU     │ │
│ └─────────────┘ │    │ │Distribution │ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ │   Table)    │ │    │ ┌─────────────┐ │
│ │    NPDU     │ │    │ └─────────────┘ │    │ │    NPDU     │ │
│ └─────────────┘ │    │ ┌─────────────┐ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ │    FDT      │ │    │ ┌─────────────┐ │
│ │    BVLL     │◄┼────┼─┤ (Foreign    │◄┼────┼─┤    BVLL     │ │
│ └─────────────┘ │    │ │ Device      │ │    │ └─────────────┘ │
│ ┌─────────────┐ │    │ │   Table)    │ │    │ ┌─────────────┐ │
│ │UDP (47808)  │◄┼────┼─┤             │◄┼────┼─┤UDP (47808)  │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Protocol Flow

1. **Device Discovery**:
   ```
   Scanner → [Who-Is Broadcast] → Network
   Device  → [I-Am Response]    → Scanner
   ```

2. **Foreign Device Registration**:
   ```
   Device → [Register-Foreign-Device] → BBMD
   BBMD   → [Result]                  → Device
   ```

3. **Broadcast Distribution**:
   ```
   Device → [Original-Broadcast-NPDU] → BBMD
   BBMD   → [Distribute-Broadcast]    → Remote Networks
   ```

## Network Testing

### Basic Discovery Test
```bash
# Terminal 1: Start a responder
cargo run --example basic/responder_device 12345

# Terminal 2: Run discovery
cargo run --example networking/whois_scan
```

### Multi-Device Test
```bash
# Start multiple responders
cargo run --example basic/responder_device 12345 &
cargo run --example basic/responder_device 67890 &
cargo run --example basic/responder_device 11111 &

# Discover all devices
cargo run --example networking/whois_scan

# Clean up background processes
kill %1 %2 %3
```

### Real Network Test
```bash
# Scan for real BACnet devices
cargo run --example networking/whois_scan

# Test with specific device
cargo run --example objects/device_objects 192.168.1.100
```

## BVLL Message Types

The transport demo demonstrates all BVLL message types:

| Function Code | Message Type | Purpose |
|---------------|--------------|---------|
| 0x00 | Result | Operation result |
| 0x01 | Write-BDT | Write broadcast distribution table |
| 0x02 | Read-BDT | Read broadcast distribution table |
| 0x03 | Read-BDT-Ack | BDT read acknowledgment |
| 0x04 | Forwarded-NPDU | Forwarded network PDU |
| 0x05 | Register-Foreign-Device | Foreign device registration |
| 0x06 | Read-FDT | Read foreign device table |
| 0x07 | Read-FDT-Ack | FDT read acknowledgment |
| 0x08 | Delete-FDT-Entry | Delete foreign device entry |
| 0x09 | Distribute-Broadcast | Distribute broadcast to network |
| 0x0A | Original-Unicast-NPDU | Original unicast message |
| 0x0B | Original-Broadcast-NPDU | Original broadcast message |
| 0x0C | Secure-BVLL | Secured BVLL message |

## Configuration

### Network Settings
```rust
use bacnet_rs::transport::{BacnetIpConfig, BacnetIpTransport};

let config = BacnetIpConfig {
    bind_address: "0.0.0.0:47808".parse().unwrap(),
    broadcast_enabled: true,
    buffer_size: 1500,
    ..Default::default()
};

let transport = BacnetIpTransport::new(config)?;
```

### Foreign Device Setup
```rust
// Register with BBMD
let bbmd_addr = "192.168.1.1:47808".parse().unwrap();
transport.register_foreign_device(bbmd_addr, 900)?; // 15 minutes TTL
```

## Troubleshooting

### Common Network Issues

1. **No devices discovered**:
   - Check firewall settings (UDP port 47808)
   - Verify network connectivity
   - Ensure broadcast permissions
   - Try different network interfaces

2. **Partial discovery**:
   - Check subnet configurations
   - Verify broadcast addresses
   - Test with known devices
   - Monitor network traffic

3. **Foreign device registration fails**:
   - Verify BBMD address and port
   - Check BBMD configuration
   - Ensure network routing
   - Test connectivity to BBMD

### Debug Commands

```bash
# Enable verbose logging
RUST_LOG=debug cargo run --example networking/whois_scan

# Monitor network traffic
sudo tcpdump -i any -p udp port 47808

# Test network connectivity
ping <target_device_ip>
nc -u <target_device_ip> 47808
```

## Advanced Topics

- **Multiple network interfaces**: Configure specific binding addresses
- **VLAN support**: Cross-VLAN BACnet communication
- **NAT traversal**: Foreign device registration for NAT environments
- **Security**: BACnet/SC secure transport (future implementation)
- **Performance**: Optimizing discovery and communication patterns

## Integration Examples

These networking examples integrate with:
- **Basic examples**: Device responders for testing
- **Object examples**: Remote device object reading
- **Communication examples**: Client-server patterns
- **Debugging examples**: Network troubleshooting
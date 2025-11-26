# Debugging Examples

Tools and utilities for debugging BACnet communication, analyzing protocols, and troubleshooting issues.

## Examples

### `debug_properties.rs`
**Purpose**: Property debugging and raw data analysis

**Features**:
- Raw BACnet data inspection and analysis
- Property encoding/decoding verification
- Data structure visualization
- Error diagnosis and troubleshooting
- Hex dump utilities for packet analysis
- Property parsing step-by-step breakdown

**Usage**:
```bash
cargo run --example debugging/debug_properties
```

**What it demonstrates**:
- Low-level BACnet data analysis
- Property encoding debugging
- Data validation and verification
- Protocol troubleshooting techniques
- Error detection and diagnosis

---

### `debug_formatter.rs`
**Purpose**: Comprehensive debug formatting for BACnet data structures

**Features**:
- **Property Value Formatting**: Human-readable display of all BACnet data types
- **Service Choice Display**: Named service identification and formatting
- **Error Code Translation**: BACnet error class and code interpretation
- **Protocol Structure Analysis**: BVLL, NPDU, and APDU layer breakdown
- **Annotated Hex Dumps**: Hex dumps with field annotations and explanations
- **Complete Packet Analysis**: End-to-end packet structure visualization

**Usage**:
```bash
cargo run --example debugging/debug_formatter
```

**What it demonstrates**:
- Production-ready debug formatting utilities
- Protocol layer analysis and visualization
- Human-readable data type interpretation
- Complete packet structure breakdown
- Error and service code translation
- Advanced debugging and analysis techniques

## Debugging Architecture

### Debug Information Flow
```
Raw BACnet Data
        ↓
  Hex Dump Analysis
        ↓
   Protocol Parsing
        ↓
  Property Extraction
        ↓
   Value Validation
        ↓
  Error Diagnosis
```

### Debug Levels
1. **Raw Data**: Byte-level packet inspection
2. **Protocol**: BVLL, NPDU, APDU layer analysis
3. **Service**: BACnet service parsing
4. **Property**: Object property decoding
5. **Value**: Data type validation

## Data Analysis Tools

### Hex Dump Utilities
```rust
use bacnet_rs::util::hex_dump;

// Analyze raw BACnet packet
let packet_data = &[0x81, 0x0A, 0x00, 0x10, 0x01, 0x00, 0x30, 0x00, 0x0C, 0x02, 0x00, 0x13, 0xB7, 0x19, 0x4D];

println!("BACnet Packet Analysis:");
println!("{}", hex_dump(packet_data, "  "));

// Output:
//   0000: 81 0A 00 10 01 00 30 00  0C 02 00 13 B7 19 4D     |......0......M|
```

### Protocol Layer Parsing
```rust
fn debug_bacnet_packet(data: &[u8]) -> Result<()> {
    println!("=== BACnet Packet Debug ===");
    println!("Total length: {} bytes\n", data.len());
    
    // BVLL Layer
    if data.len() >= 4 {
        println!("BVLL Header:");
        println!("  Type: 0x{:02X} ({})", data[0], 
            if data[0] == 0x81 { "BACnet/IP" } else { "Unknown" });
        println!("  Function: 0x{:02X} ({})", data[1], decode_bvll_function(data[1]));
        let length = u16::from_be_bytes([data[2], data[3]]);
        println!("  Length: {} bytes", length);
        println!();
        
        // NPDU Layer
        if data.len() > 4 {
            debug_npdu_layer(&data[4..])?;
        }
    }
    
    Ok(())
}

fn decode_bvll_function(function: u8) -> &'static str {
    match function {
        0x00 => "Result",
        0x0A => "Original-Unicast-NPDU", 
        0x0B => "Original-Broadcast-NPDU",
        0x05 => "Register-Foreign-Device",
        _ => "Unknown",
    }
}
```

### Property Debugging
```rust
fn debug_property_encoding(property_data: &[u8]) -> Result<()> {
    println!("=== Property Debug ===");
    println!("Raw data: {:02X?}", property_data);
    
    let mut pos = 0;
    while pos < property_data.len() {
        let tag = property_data[pos];
        println!("\nPosition {}: Tag 0x{:02X}", pos, tag);
        
        match tag {
            0x75 => { // Character string
                if let Some((string, consumed)) = debug_character_string(&property_data[pos..]) {
                    println!("  Character String: '{}'", string);
                    println!("  Consumed {} bytes", consumed);
                    pos += consumed;
                } else {
                    println!("  Error parsing character string");
                    break;
                }
            }
            0x44 => { // Real value
                if property_data.len() >= pos + 5 {
                    let bytes = [property_data[pos+1], property_data[pos+2], 
                               property_data[pos+3], property_data[pos+4]];
                    let value = f32::from_be_bytes(bytes);
                    println!("  Real Value: {}", value);
                    println!("  Raw bytes: {:02X?}", bytes);
                    pos += 5;
                } else {
                    println!("  Insufficient data for real value");
                    break;
                }
            }
            0x91 => { // Enumerated
                if property_data.len() >= pos + 2 {
                    let enum_value = property_data[pos + 1];
                    println!("  Enumerated: {}", enum_value);
                    if let Some(units) = decode_engineering_units(enum_value) {
                        println!("  Units: {}", units);
                    }
                    pos += 2;
                } else {
                    println!("  Insufficient data for enumerated");
                    break;
                }
            }
            _ => {
                println!("  Unknown tag, advancing 1 byte");
                pos += 1;
            }
        }
    }
    
    Ok(())
}
```

## Error Diagnosis

### Common BACnet Errors

#### 1. Invalid BVLL Header
```rust
fn diagnose_bvll_error(data: &[u8]) -> String {
    if data.len() < 4 {
        return "BVLL header too short (need 4 bytes)".to_string();
    }
    
    if data[0] != 0x81 {
        return format!("Invalid BVLL type: 0x{:02X} (expected 0x81)", data[0]);
    }
    
    let length = u16::from_be_bytes([data[2], data[3]]);
    if data.len() != length as usize {
        return format!("Length mismatch: header says {} bytes, got {} bytes", 
            length, data.len());
    }
    
    "BVLL header valid".to_string()
}
```

#### 2. Property Parsing Errors
```rust
fn diagnose_property_error(data: &[u8], expected_property: PropertyIdentifier) -> String {
    if data.is_empty() {
        return "No property data".to_string();
    }
    
    let tag = data[0];
    match expected_property {
        PropertyIdentifier::PresentValue => {
            match tag {
                0x44 => "Real value (correct for analog present value)".to_string(),
                0x11 => "Boolean value (correct for binary present value)".to_string(),
                0x21 => "Unsigned int (correct for multistate present value)".to_string(),
                _ => format!("Unexpected tag 0x{:02X} for present value", tag),
            }
        }
        PropertyIdentifier::ObjectName => {
            if tag == 0x75 {
                "Character string (correct for object name)".to_string()
            } else {
                format!("Expected character string (0x75), got 0x{:02X}", tag)
            }
        }
        _ => format!("Cannot diagnose property {:?}", expected_property),
    }
}
```

#### 3. Encoding Issues
```rust
fn diagnose_encoding_issue(original: &[u8], decoded: &[u8]) -> Vec<String> {
    let mut issues = Vec::new();
    
    if original.len() != decoded.len() {
        issues.push(format!("Length mismatch: {} vs {} bytes", 
            original.len(), decoded.len()));
    }
    
    for (i, (orig, dec)) in original.iter().zip(decoded.iter()).enumerate() {
        if orig != dec {
            issues.push(format!("Byte {} differs: 0x{:02X} vs 0x{:02X}", i, orig, dec));
        }
    }
    
    if issues.is_empty() {
        issues.push("Encoding/decoding successful".to_string());
    }
    
    issues
}
```

## Network Debugging

### Packet Capture Analysis
```rust
use std::net::UdpSocket;

fn debug_network_traffic() -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:47808")?;
    println!("Listening for BACnet traffic on port 47808...");
    
    let mut buffer = [0u8; 1500];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((len, source)) => {
                println!("\n=== Packet from {} ===", source);
                println!("Length: {} bytes", len);
                
                // Analyze packet
                analyze_packet(&buffer[..len]);
                
                // Save for further analysis
                save_debug_packet(&buffer[..len], source)?;
            }
            Err(e) => {
                eprintln!("Error receiving packet: {}", e);
            }
        }
    }
}

fn analyze_packet(data: &[u8]) {
    println!("{}", hex_dump(data, ""));
    
    if let Err(e) = debug_bacnet_packet(data) {
        println!("Packet analysis error: {}", e);
    }
}
```

### Communication Timing
```rust
use std::time::Instant;

struct CommunicationDebugger {
    start_times: HashMap<String, Instant>,
    timeouts: HashMap<String, Duration>,
}

impl CommunicationDebugger {
    fn start_request(&mut self, request_id: String) {
        self.start_times.insert(request_id, Instant::now());
    }
    
    fn end_request(&mut self, request_id: &str) -> Option<Duration> {
        self.start_times.remove(request_id).map(|start| start.elapsed())
    }
    
    fn check_timeouts(&self) -> Vec<String> {
        let mut timed_out = Vec::new();
        
        for (request_id, start_time) in &self.start_times {
            if let Some(&timeout) = self.timeouts.get(request_id) {
                if start_time.elapsed() > timeout {
                    timed_out.push(request_id.clone());
                }
            }
        }
        
        timed_out
    }
}
```

## Advanced Debugging Techniques

### Property State Tracking
```rust
#[derive(Debug, Clone)]
struct PropertyState {
    object_id: ObjectIdentifier,
    property: PropertyIdentifier,
    current_value: Option<PropertyValue>,
    last_modified: Instant,
    change_count: u32,
}

struct PropertyDebugger {
    states: HashMap<(ObjectIdentifier, PropertyIdentifier), PropertyState>,
}

impl PropertyDebugger {
    fn track_property_change(
        &mut self, 
        object_id: ObjectIdentifier,
        property: PropertyIdentifier,
        new_value: PropertyValue,
    ) {
        let key = (object_id, property);
        
        match self.states.get_mut(&key) {
            Some(state) => {
                if state.current_value.as_ref() != Some(&new_value) {
                    println!("Property change detected:");
                    println!("  Object: {} {}", object_id.object_type as u16, object_id.instance);
                    println!("  Property: {:?}", property);
                    println!("  Old value: {:?}", state.current_value);
                    println!("  New value: {:?}", new_value);
                    println!("  Time since last change: {:?}", state.last_modified.elapsed());
                    
                    state.current_value = Some(new_value);
                    state.last_modified = Instant::now();
                    state.change_count += 1;
                }
            }
            None => {
                self.states.insert(key, PropertyState {
                    object_id,
                    property,
                    current_value: Some(new_value),
                    last_modified: Instant::now(),
                    change_count: 1,
                });
            }
        }
    }
}
```

### Error Pattern Analysis
```rust
#[derive(Debug, Clone)]
struct ErrorPattern {
    error_type: String,
    frequency: u32,
    last_occurrence: Instant,
    associated_devices: HashSet<u32>,
}

struct ErrorAnalyzer {
    patterns: HashMap<String, ErrorPattern>,
}

impl ErrorAnalyzer {
    fn record_error(&mut self, error: &str, device_id: Option<u32>) {
        let pattern = self.patterns.entry(error.to_string()).or_insert(ErrorPattern {
            error_type: error.to_string(),
            frequency: 0,
            last_occurrence: Instant::now(),
            associated_devices: HashSet::new(),
        });
        
        pattern.frequency += 1;
        pattern.last_occurrence = Instant::now();
        
        if let Some(device) = device_id {
            pattern.associated_devices.insert(device);
        }
    }
    
    fn analyze_patterns(&self) -> Vec<String> {
        let mut analysis = Vec::new();
        
        for pattern in self.patterns.values() {
            if pattern.frequency > 10 {
                analysis.push(format!(
                    "High frequency error: '{}' occurred {} times across {} devices",
                    pattern.error_type, pattern.frequency, pattern.associated_devices.len()
                ));
            }
            
            if pattern.last_occurrence.elapsed() < Duration::from_minutes(5) {
                analysis.push(format!(
                    "Recent error: '{}' last seen {} seconds ago",
                    pattern.error_type, pattern.last_occurrence.elapsed().as_secs()
                ));
            }
        }
        
        analysis
    }
}
```

## Integration with Other Tools

### With Performance Monitoring
```rust
use bacnet_rs::util::performance::PerformanceMonitor;

fn debug_with_performance(monitor: &PerformanceMonitor) {
    // Identify slow operations
    for metric in monitor.get_all_metrics() {
        if metric.avg_duration_ms > 1000.0 {
            println!("Slow operation detected: {} avg {:.1}ms", 
                metric.name, metric.avg_duration_ms);
            
            // Enable detailed debugging for this operation
            enable_debug_logging(&metric.name);
        }
    }
}
```

### With Statistics Collection
```rust
use bacnet_rs::util::statistics::StatsCollector;

fn debug_communication_issues(collector: &StatsCollector) {
    let global_stats = collector.get_global_stats();
    
    if global_stats.success_rate() < 90.0 {
        println!("Low success rate detected: {:.1}%", global_stats.success_rate());
        
        // Analyze per-device statistics
        for device in collector.get_all_device_stats() {
            if device.comm_stats.success_rate() < 80.0 {
                println!("Problem device: {} ({:.1}% success rate)", 
                    device.device_id, device.comm_stats.success_rate());
                
                // Enable detailed debugging for this device
                enable_device_debugging(device.device_id);
            }
        }
    }
}
```

## Debug Configuration

### Logging Levels
```rust
use log::{debug, info, warn, error};

fn configure_debug_logging() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("bacnet_rs::transport", log::LevelFilter::Trace)
        .filter_module("bacnet_rs::service", log::LevelFilter::Debug)
        .init();
}

// Usage in code
debug!("Parsing property data: {:02X?}", data);
info!("Device {} responded in {:.1}ms", device_id, response_time);
warn!("Timeout for device {}, retrying...", device_id);
error!("Failed to parse response from {}: {}", address, error);
```

### Debug Flags
```rust
pub struct DebugConfig {
    pub packet_dump: bool,
    pub property_parsing: bool,
    pub timing_analysis: bool,
    pub error_details: bool,
    pub state_tracking: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            packet_dump: false,
            property_parsing: false,
            timing_analysis: true,
            error_details: true,
            state_tracking: false,
        }
    }
}
```

These debugging tools provide comprehensive analysis capabilities for troubleshooting BACnet communication issues and optimizing system performance.
# Utility Examples

Comprehensive utility functions, performance monitoring, and helper demonstrations.

## Examples

### `util_demo.rs`
**Purpose**: Performance monitoring, statistics collection, and utility functions

**Features**:
- **Performance Monitoring**: Operation timing with RAII scoped timers
- **Statistics Collection**: Communication metrics and device tracking
- **Circular Buffer**: Event history with fixed capacity
- **Retry Configuration**: Exponential backoff algorithms
- **Data Formatting**: Human-readable byte sizes and throughput
- **CRC Calculations**: MS/TP and BACnet/SC checksums

**Usage**:
```bash
cargo run --example utilities/util_demo
```

**What it demonstrates**:
- Production-ready monitoring infrastructure
- Statistical analysis for BACnet systems
- Debugging and diagnostic tools
- Performance optimization techniques

---

### `vendor_lookup.rs`
**Purpose**: BACnet vendor ID management and lookup

**Features**:
- Complete vendor database with 2000+ official assignments
- Vendor information lookup and validation
- Reserved ID ranges for testing
- Official assignment verification
- Vendor name formatting and display

**Usage**:
```bash
cargo run --example utilities/vendor_lookup
```

**What it demonstrates**:
- Official vendor ID management
- Database lookup patterns
- Validation and compliance checking
- Vendor information handling

## Performance Monitoring

### Architecture
```
PerformanceMonitor
├── Metrics Storage (Arc<RwLock<HashMap<String, OperationMetrics>>>)
├── Active Timers (Arc<RwLock<HashMap<String, Instant>>>)
└── ScopedTimer (RAII pattern for automatic timing)
```

### Usage Patterns

#### Basic Operation Timing
```rust
use bacnet_rs::util::performance::{PerformanceMonitor, ScopedTimer};

let monitor = PerformanceMonitor::new();

// Manual timing
monitor.start_timer("database_query");
perform_database_operation();
monitor.stop_timer("database_query");

// RAII timing (preferred)
{
    let _timer = ScopedTimer::new(&monitor, "network_request");
    send_bacnet_request()?;
} // Timer automatically stopped here
```

#### Metrics Analysis
```rust
// Get specific operation metrics
if let Some(metrics) = monitor.get_metrics("device_discovery") {
    println!("Average response time: {:.2} ms", metrics.avg_duration_ms);
    println!("Success rate: {:.1}%", 
        (metrics.count as f64 / total_attempts as f64) * 100.0);
}

// Get all metrics for reporting
for metric in monitor.get_all_metrics() {
    println!("{}: {} calls, avg {:.2} ms", 
        metric.name, metric.count, metric.avg_duration_ms);
}
```

## Statistics Collection

### Communication Statistics
```rust
use bacnet_rs::util::statistics::{CommunicationStats, StatsCollector};

let collector = StatsCollector::new();

// Record communication events
collector.update_global_stats(|stats| {
    stats.record_sent(message_size);
});

collector.update_device_stats(device_id, |device_stats| {
    device_stats.record_response_time(response_time_ms);
    device_stats.comm_stats.record_received(response_size);
});

// Analyze performance
let global_stats = collector.get_global_stats();
println!("Success rate: {:.1}%", global_stats.success_rate());
println!("Total throughput: {}", 
    format_bytes(global_stats.bytes_sent + global_stats.bytes_received));
```

### Device-Specific Tracking
```rust
// Track per-device performance
for device in collector.get_all_device_stats() {
    println!("Device {} ({})", device.device_id, device.address);
    println!("  Status: {}", if device.online { "Online" } else { "Offline" });
    
    if let Some(avg_response) = device.avg_response_time() {
        println!("  Avg response: {:.1} ms", avg_response);
    }
    
    println!("  Success rate: {:.1}%", device.comm_stats.success_rate());
    println!("  Messages: {} sent, {} received", 
        device.comm_stats.messages_sent,
        device.comm_stats.messages_received
    );
}
```

## Data Structures and Utilities

### Circular Buffer
```rust
use bacnet_rs::util::CircularBuffer;

// Event logging with fixed capacity
let mut event_log = CircularBuffer::<String>::new(100);

event_log.push("Device 1234 discovered".to_string());
event_log.push("Connection established".to_string());
event_log.push("Property read successful".to_string());

// Retrieve events (oldest to newest)
for (i, event) in event_log.items().iter().enumerate() {
    println!("[{}] {}", i, event);
}
```

### Retry Configuration
```rust
use bacnet_rs::util::RetryConfig;

let retry_config = RetryConfig {
    max_attempts: 5,
    initial_delay_ms: 100,
    max_delay_ms: 5000,
    backoff_multiplier: 2.0,
};

// Implement retry logic with exponential backoff
for attempt in 0..retry_config.max_attempts {
    match perform_operation() {
        Ok(result) => return Ok(result),
        Err(e) if attempt < retry_config.max_attempts - 1 => {
            let delay = retry_config.delay_for_attempt(attempt);
            thread::sleep(delay);
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

## Data Formatting

### Human-Readable Sizes
```rust
use bacnet_rs::util::{format_bytes, calculate_throughput};

println!("Database size: {}", format_bytes(1048576)); // "1.00 MB"
println!("Network usage: {}", format_bytes(2560));    // "2.50 KB"

// Throughput calculation
let throughput = calculate_throughput(1_000_000, 2.5); // bytes, seconds
println!("Data rate: {}", throughput); // "390.62 KB/s"
```

### BACnet-Specific Formatting
```rust
use bacnet_rs::util::{bacnet_date_to_string, bacnet_time_to_string};

// Format BACnet date/time values
let date_str = bacnet_date_to_string(2024, 3, 15, 5); // "2024/3/15 (Fri)"
let time_str = bacnet_time_to_string(14, 30, 45, 50); // "14:30:45.50"

// Handle wildcard values
let wildcard_date = bacnet_date_to_string(255, 255, 255, 255); // "*/*/* (*)"
```

## CRC and Checksums

### MS/TP CRC-16
```rust
use bacnet_rs::util::crc16_mstp;

let frame_data = b"BACnet MS/TP frame";
let crc = crc16_mstp(frame_data);
println!("MS/TP CRC-16: 0x{:04X}", crc);

// Verify frame integrity
fn verify_mstp_frame(frame: &[u8]) -> bool {
    if frame.len() < 2 { return false; }
    
    let data = &frame[..frame.len()-2];
    let received_crc = u16::from_be_bytes([
        frame[frame.len()-2], 
        frame[frame.len()-1]
    ]);
    
    crc16_mstp(data) == received_crc
}
```

### BACnet/SC CRC-32C
```rust
use bacnet_rs::util::crc32c;

let secure_data = b"BACnet/SC secure message";
let crc = crc32c(secure_data);
println!("BACnet/SC CRC-32C: 0x{:08X}", crc);
```

## Vendor Management

### Vendor Database Lookup
```rust
use bacnet_rs::vendor::{get_vendor_info, get_vendor_name, is_vendor_id_assigned};

// Look up official vendor information
if let Some(vendor_info) = get_vendor_info(15) {
    println!("Vendor: {} ({})", vendor_info.name, vendor_info.organization);
    println!("Contact: {}", vendor_info.url);
}

// Quick name lookup
if let Some(name) = get_vendor_name(15) {
    println!("Vendor 15: {}", name); // "Johnson Controls, Inc."
}

// Validation
if is_vendor_id_assigned(999) {
    println!("Vendor ID 999 is officially assigned");
} else {
    println!("Vendor ID 999 is available or reserved");
}
```

### Vendor ID Validation
```rust
use bacnet_rs::vendor::{is_vendor_id_reserved, format_vendor_display};

fn validate_device_vendor_id(vendor_id: u16) -> Result<(), String> {
    if vendor_id == 0 {
        return Err("Vendor ID 0 is reserved for ASHRAE".into());
    }
    
    if is_vendor_id_reserved(vendor_id) {
        return Err(format!("Vendor ID {} is reserved for testing", vendor_id));
    }
    
    if !is_vendor_id_assigned(vendor_id) {
        return Err(format!("Vendor ID {} is not officially assigned", vendor_id));
    }
    
    Ok(())
}

// Display formatted vendor information
println!("{}", format_vendor_display(15)); // "Johnson Controls, Inc. (ID: 15)"
```

## Network Utilities

### Address Parsing and Validation
```rust
use bacnet_rs::util::{parse_bacnet_address, is_local_network, is_broadcast_network};

// Parse BACnet addresses with default port
let addr = parse_bacnet_address("192.168.1.100")?; // Adds :47808
let addr_with_port = parse_bacnet_address("10.0.0.1:47809")?; // Uses specified port

// Network number validation
assert!(is_local_network(0));        // Local network
assert!(is_broadcast_network(65535)); // Broadcast to all networks
assert!(!is_local_network(100));     // Remote network
```

### Buffer Management
```rust
use bacnet_rs::util::Buffer;

let data = &[0x01, 0x02, 0x03, 0x04, 0x05];
let mut buffer = Buffer::new(data);

// Safe reading with bounds checking
if let Some(byte) = buffer.read_u8() {
    println!("First byte: 0x{:02X}", byte);
}

if let Some(word) = buffer.read_u16() {
    println!("Next word: 0x{:04X}", word);
}

// Check remaining data
println!("Remaining bytes: {}", buffer.remaining());
```

## Integration Examples

### With Performance Monitoring
```rust
// Monitor BACnet operation performance
let monitor = PerformanceMonitor::new();

fn monitored_device_discovery(monitor: &PerformanceMonitor) -> Result<Vec<Device>> {
    let _timer = ScopedTimer::new(monitor, "device_discovery");
    
    // Perform discovery
    let devices = discover_bacnet_devices()?;
    
    // Metrics automatically recorded when timer drops
    Ok(devices)
}
```

### With Statistics Collection
```rust
// Track communication quality
let stats_collector = StatsCollector::new();

fn send_bacnet_request_with_stats(
    request: &[u8], 
    target: SocketAddr,
    stats: &StatsCollector
) -> Result<Vec<u8>> {
    stats.update_global_stats(|s| s.record_sent(request.len()));
    
    let start = Instant::now();
    let response = send_request(request, target)?;
    let response_time = start.elapsed().as_secs_f64() * 1000.0;
    
    let device_id = extract_device_id_from_response(&response)?;
    stats.update_device_stats(device_id, |device_stats| {
        device_stats.record_response_time(response_time);
        device_stats.comm_stats.record_received(response.len());
    });
    
    Ok(response)
}
```

## Production Deployment

### Monitoring Infrastructure
```rust
use std::sync::Arc;

struct BacnetSystemMonitor {
    performance: Arc<PerformanceMonitor>,
    statistics: Arc<StatsCollector>,
    event_log: Arc<Mutex<CircularBuffer<String>>>,
}

impl BacnetSystemMonitor {
    pub fn new() -> Self {
        Self {
            performance: Arc::new(PerformanceMonitor::new()),
            statistics: Arc::new(StatsCollector::new()),
            event_log: Arc::new(Mutex::new(CircularBuffer::new(1000))),
        }
    }
    
    pub fn generate_report(&self) -> SystemReport {
        SystemReport {
            performance_metrics: self.performance.get_all_metrics(),
            communication_stats: self.statistics.get_global_stats(),
            recent_events: self.event_log.lock().unwrap().items(),
            device_status: self.statistics.get_all_device_stats(),
        }
    }
}
```

These utilities provide comprehensive monitoring, analysis, and debugging capabilities essential for production BACnet systems.
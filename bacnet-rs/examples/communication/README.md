# Communication Examples

BACnet device-to-device communication patterns and client implementations.

## Examples

### `test_client.rs`
**Purpose**: BACnet client for testing communication with remote devices

**Features**:
- Confirmed service request handling
- Error response processing
- Timeout and retry mechanisms
- Service choice implementations
- Response validation and parsing

**Usage**:
```bash
cargo run --example communication/test_client
```

**What it demonstrates**:
- Client-side BACnet communication
- Confirmed service patterns
- Error handling strategies
- Response processing workflows
- Communication reliability patterns

## Communication Patterns

### Request-Response Pattern
```
Client                          Server
  │                              │
  ├─ Confirmed Request ─────────→│
  │                              ├─ Process Request
  │                              ├─ Generate Response
  │←─────── Complex Ack ─────────┤
  │                              │
  └─ Process Response            │
```

### Error Handling
```
Client                          Server
  │                              │
  ├─ Confirmed Request ─────────→│
  │                              ├─ Detect Error
  │←────── Error Response ───────┤
  │                              │
  ├─ Handle Error                │
  ├─ Retry Logic                 │
  └─ Alternative Action          │
```

## Service Categories

### Read Services
- **ReadProperty**: Single property read
- **ReadPropertyMultiple**: Batch property read
- **ReadRange**: Historical data reading

### Write Services
- **WriteProperty**: Single property write
- **WritePropertyMultiple**: Batch property write

### Object Services
- **CreateObject**: Dynamic object creation
- **DeleteObject**: Object removal

### File Services
- **AtomicReadFile**: File data reading
- **AtomicWriteFile**: File data writing

### Alarm & Event Services
- **AcknowledgeAlarm**: Alarm acknowledgment
- **GetAlarmSummary**: Active alarm retrieval
- **GetEventInformation**: Event status reading

## Client Implementation Patterns

### Basic Client Structure
```rust
use bacnet_rs::{
    client::BacnetClient,
    object::{ObjectIdentifier, PropertyIdentifier},
    service::ConfirmedServiceChoice,
};

struct MyBacnetClient {
    client: BacnetClient,
    target_device: SocketAddr,
    timeout: Duration,
}

impl MyBacnetClient {
    pub fn read_property(&self, 
        object_id: ObjectIdentifier, 
        property: PropertyIdentifier
    ) -> Result<PropertyValue> {
        // Implementation
    }
    
    pub fn write_property(&self,
        object_id: ObjectIdentifier,
        property: PropertyIdentifier, 
        value: PropertyValue
    ) -> Result<()> {
        // Implementation
    }
}
```

### Batch Operations
```rust
// Read multiple properties efficiently
let properties = vec![
    (ai1_id, PropertyIdentifier::PresentValue),
    (ai2_id, PropertyIdentifier::PresentValue),
    (av1_id, PropertyIdentifier::PresentValue),
];

let values = client.read_properties_batch(&properties)?;
```

### Error Recovery
```rust
fn robust_read_property(
    client: &BacnetClient,
    object_id: ObjectIdentifier,
    property: PropertyIdentifier,
) -> Result<PropertyValue> {
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        match client.read_property(object_id, property) {
            Ok(value) => return Ok(value),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                thread::sleep(Duration::from_millis(100 * attempts));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Service Request Examples

### ReadProperty Request
```rust
use bacnet_rs::service::{ReadPropertyRequest, ConfirmedServiceChoice};

let request = ReadPropertyRequest::new(
    ObjectIdentifier::new(ObjectType::AnalogInput, 1),
    PropertyIdentifier::PresentValue,
    None, // No array index
);

let response = client.send_confirmed_request(
    target_addr,
    ConfirmedServiceChoice::ReadProperty,
    &request.encode()?
)?;
```

### WriteProperty Request
```rust
use bacnet_rs::service::WritePropertyRequest;

let request = WritePropertyRequest::new(
    ObjectIdentifier::new(ObjectType::AnalogValue, 1),
    PropertyIdentifier::PresentValue,
    None, // No array index
    PropertyValue::Real(23.5),
    Some(8), // Priority 8
);

client.send_confirmed_request(
    target_addr,
    ConfirmedServiceChoice::WriteProperty,
    &request.encode()?
)?;
```

## Testing Scenarios

### Local Testing
```bash
# Start a responder device
cargo run --example basic/responder_device 12345 &

# Test client communication
cargo run --example communication/test_client

# Cleanup
kill %1
```

### Remote Device Testing
```bash
# Test with real BACnet device
cargo run --example communication/test_client

# Configure target in source code:
let target_addr = "192.168.1.100:47808".parse()?;
```

### Error Simulation
```bash
# Test timeout handling
cargo run --example communication/test_client

# Test with unreachable device:
let target_addr = "192.168.999.999:47808".parse()?;
```

## Performance Optimization

### Connection Pooling
```rust
use std::collections::HashMap;

struct ConnectionPool {
    clients: HashMap<SocketAddr, BacnetClient>,
    max_connections: usize,
}

impl ConnectionPool {
    fn get_client(&mut self, addr: SocketAddr) -> &mut BacnetClient {
        self.clients.entry(addr).or_insert_with(|| {
            BacnetClient::new(addr)
        })
    }
}
```

### Request Batching
```rust
// Batch multiple requests for efficiency
struct BatchRequest {
    requests: Vec<(ObjectIdentifier, PropertyIdentifier)>,
    target: SocketAddr,
}

impl BatchRequest {
    fn execute(&self, client: &BacnetClient) -> Vec<Result<PropertyValue>> {
        // Use ReadPropertyMultiple for efficiency
        let rpm_request = ReadPropertyMultipleRequest::from_requests(&self.requests);
        client.send_rpm_request(self.target, rpm_request)
    }
}
```

### Async Communication (Future)
```rust
// Future async implementation
use tokio::time::timeout;

async fn async_read_property(
    client: &AsyncBacnetClient,
    object_id: ObjectIdentifier,
    property: PropertyIdentifier,
) -> Result<PropertyValue> {
    timeout(
        Duration::from_secs(5),
        client.read_property_async(object_id, property)
    ).await?
}
```

## Integration with Other Examples

### With Object Database
```rust
// Sync remote device with local database
fn sync_device_properties(
    client: &BacnetClient,
    database: &ObjectDatabase,
    remote_addr: SocketAddr,
) -> Result<()> {
    let objects = database.get_all_objects();
    
    for obj_id in objects {
        if let Ok(remote_value) = client.read_property(
            remote_addr, 
            obj_id, 
            PropertyIdentifier::PresentValue
        ) {
            database.set_property(obj_id, PropertyIdentifier::PresentValue, remote_value)?;
        }
    }
    
    Ok(())
}
```

### With Networking Examples
```rust
// Discover and communicate with all devices
let discovered_devices = whois_scanner.discover_devices()?;

for device in discovered_devices {
    let client = BacnetClient::new();
    let properties = client.read_device_properties(device.address)?;
    println!("Device {}: {:?}", device.device_id, properties);
}
```

## Error Codes and Handling

### Common BACnet Error Classes
- **Device**: Device-related errors (offline, busy)
- **Object**: Object not found or invalid
- **Property**: Property access errors
- **Resources**: Resource exhaustion
- **Security**: Access denied, authentication
- **Services**: Service not supported

### Error Response Processing
```rust
match error_response {
    BacnetError::Device(DeviceError::DeviceBusy) => {
        // Retry after delay
        thread::sleep(Duration::from_secs(1));
        retry_request()?;
    }
    BacnetError::Object(ObjectError::UnknownObject) => {
        // Object doesn't exist, skip
        continue;
    }
    BacnetError::Property(PropertyError::WriteAccessDenied) => {
        // Property is read-only
        return Err("Cannot write to read-only property".into());
    }
    _ => return Err(error_response.into()),
}
```

## Security Considerations

### Access Control
- Validate device credentials
- Implement authentication where required
- Use secure communication channels when available
- Log all communication attempts

### Rate Limiting
```rust
use std::time::{Duration, Instant};

struct RateLimiter {
    last_request: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    fn check_rate_limit(&mut self) -> Result<()> {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.min_interval {
            thread::sleep(self.min_interval - elapsed);
        }
        self.last_request = Instant::now();
        Ok(())
    }
}
```

This communication framework provides robust, efficient, and reliable BACnet client functionality for production use.
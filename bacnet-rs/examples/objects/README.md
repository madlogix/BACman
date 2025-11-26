# Object Examples

BACnet object management, database operations, and remote device interaction.

## Examples

### `object_database.rs`
**Purpose**: Comprehensive object database system demonstration

**Features**:
- Thread-safe object storage with multiple indices
- Property access and modification with validation
- Search and query capabilities by name, type, and property values
- Database statistics and performance monitoring
- Builder pattern for convenient database construction
- Dynamic object management (add/remove)

**Usage**:
```bash
cargo run --example objects/object_database
```

**What it demonstrates**:
- Complete object database architecture
- Multi-threaded object access patterns
- Search and indexing strategies
- Database integrity and validation
- Performance optimization techniques

---

### `device_objects.rs`
**Purpose**: Remote device object discovery and property reading

**Features**:
- ReadPropertyMultiple service implementation
- Object list enumeration from remote devices
- Property parsing with UTF-16 string support
- Comprehensive units display (190+ BACnet units)
- Error handling for diverse device responses
- Real device communication examples

**Usage**:
```bash
# Discover objects in a remote device
cargo run --example objects/device_objects 192.168.1.100

# Test with the demo device
cargo run --example objects/device_objects 10.161.1.211
```

**What it demonstrates**:
- Remote device communication patterns
- ReadPropertyMultiple protocol implementation
- Property parsing and data extraction
- Unicode string handling
- Real-world device compatibility

## Object Database Architecture

### Database Structure
```
ObjectDatabase
├── Objects Storage (Arc<RwLock<HashMap<ObjectIdentifier, Box<dyn BacnetObject>>>>)
├── Type Index (Arc<RwLock<HashMap<ObjectType, Vec<ObjectIdentifier>>>>)
├── Name Index (Arc<RwLock<HashMap<String, ObjectIdentifier>>>)
├── Revision Tracking (Arc<RwLock<u32>>)
└── Statistics (DatabaseStatistics)
```

### Object Lifecycle
```
1. Creation → 2. Validation → 3. Storage → 4. Indexing → 5. Access → 6. Modification → 7. Removal
     ↓             ↓             ↓           ↓          ↓          ↓              ↓
  Builder      Property      HashMap      Multiple    Property   Revision     Index
  Pattern      Validation    Storage      Indices     Access     Update       Cleanup
```

## Supported Object Types

### Standard Objects
| Type | Code | Example Use Case |
|------|------|-----------------|
| Device (DEV) | 8 | Controller/Gateway |
| Analog Input (AI) | 0 | Temperature sensor |
| Analog Output (AO) | 1 | Valve position |
| Analog Value (AV) | 2 | Setpoint |
| Binary Input (BI) | 3 | Door sensor |
| Binary Output (BO) | 4 | Fan control |
| Binary Value (BV) | 5 | Enable/disable |
| Multi-State Input (MSI) | 13 | Mode feedback |
| Multi-State Output (MSO) | 14 | Mode command |
| Multi-State Value (MSV) | 19 | System state |

### Object Properties

#### Required Properties (all objects)
- Object_Identifier
- Object_Name
- Object_Type

#### Common Properties
- Present_Value (I/O/V objects)
- Description
- Status_Flags
- Event_State
- Reliability
- Out_Of_Service

#### Type-Specific Properties
- Units (analog objects)
- Polarity (binary objects)
- State_Text (multi-state objects)
- Number_Of_States (multi-state objects)

## Database Operations

### Basic Operations
```rust
use bacnet_rs::object::{ObjectDatabase, DatabaseBuilder, Device, AnalogInput};

// Create database
let device = Device::new(1234, "My Device".to_string());
let db = ObjectDatabase::new(device);

// Add objects
let ai = AnalogInput::new(1, "Temperature".to_string());
db.add_object(Box::new(ai))?;

// Access properties
let temp = db.get_property(
    ObjectIdentifier::new(ObjectType::AnalogInput, 1),
    PropertyIdentifier::PresentValue
)?;

// Modify properties
db.set_property(
    ObjectIdentifier::new(ObjectType::AnalogInput, 1),
    PropertyIdentifier::PresentValue,
    PropertyValue::Real(22.5)
)?;
```

### Advanced Queries
```rust
// Search by name
let obj_id = db.get_object_by_name("Temperature")?;

// Search by type
let all_ai = db.get_objects_by_type(ObjectType::AnalogInput);

// Search by property value
let active_objects = db.search_by_property(
    PropertyIdentifier::PresentValue,
    &PropertyValue::Real(100.0)
);

// Get statistics
let stats = db.statistics();
println!("Total objects: {}", stats.total_objects);
```

### Builder Pattern
```rust
let db = DatabaseBuilder::new()
    .with_device(Device::new(5000, "Controller".to_string()))
    .add_object(Box::new(AnalogInput::new(1, "Temp 1".to_string())))
    .add_object(Box::new(AnalogInput::new(2, "Temp 2".to_string())))
    .add_object(Box::new(BinaryInput::new(1, "Door".to_string())))
    .build()?;
```

## Remote Device Communication

### ReadPropertyMultiple Pattern
```rust
// 1. Discover device ID
let device_id = discover_device_id(&socket, target_addr)?;

// 2. Read object list
let objects = read_device_object_list(&socket, target_addr, device_id)?;

// 3. Batch property reading
for chunk in objects.chunks(5) {
    let properties = read_objects_properties(&socket, target_addr, chunk)?;
    // Process properties...
}
```

### Property Parsing
```rust
// Extract character strings (with UTF-16 support)
fn extract_character_string(data: &[u8]) -> Option<(String, usize)> {
    let encoding = data[2];
    match encoding {
        0 => String::from_utf8_lossy(&data[3..]).to_string(),
        4 => decode_utf16(&data[3..]),  // UTF-16 support
        _ => String::from_utf8_lossy(&data[3..]).to_string(),
    }
}

// Extract engineering units
fn extract_units(data: &[u8]) -> Option<(String, usize)> {
    if let Some((units_id, consumed)) = decode_enumerated(data) {
        let units_name = match units_id {
            62 => "degrees-celsius",
            63 => "degrees-fahrenheit", 
            // ... 190+ more units
        };
        Some((units_name.to_string(), consumed))
    }
}
```

## Engineering Units Support

The library supports 190+ standard BACnet engineering units:

### Temperature Units
- degrees-celsius (62)
- degrees-fahrenheit (63)
- degrees-kelvin (64)
- degrees-rankine (65)

### Pressure Units
- pascals (53)
- kilopascals (54)
- pounds-force-per-square-inch (56)
- bars (55)

### Flow Units
- cubic-feet-per-minute (94)
- cubic-meters-per-second (96)
- liters-per-second (97)
- gallons-per-minute (98)

### Electrical Units
- volts (87)
- millivolts (88)
- amperes (89)
- watts (114)
- kilowatts (115)
- volt-amperes (116)

## Testing Scenarios

### Local Database Testing
```bash
# Test object database functionality
cargo run --example objects/object_database

# Expected output:
# - 10 objects created (1 Device + 9 I/O objects)
# - Property access demonstrations
# - Search capability tests
# - Dynamic management examples
# - Statistics and performance metrics
```

### Remote Device Testing
```bash
# Test with known BACnet device
cargo run --example objects/device_objects 10.161.1.211

# Expected output:
# - Device discovery (Device ID: 5047)
# - Object list enumeration (40+ objects)
# - Property reading with names, values, units
# - Temperature readings in Celsius
# - Binary status indicators
```

### Integration Testing
```bash
# Start local responder
cargo run --example basic/responder_device 12345 &

# Test remote reading (may have limited objects)
cargo run --example objects/device_objects 127.0.0.1

# Cleanup
kill %1
```

## Performance Considerations

### Database Optimization
- **Read-Write Locks**: Concurrent access optimization
- **Multiple Indices**: Fast lookups by ID, type, and name
- **Batch Operations**: Efficient bulk property access
- **Memory Management**: Smart pointers and minimal copying

### Network Optimization
- **Batch Requests**: ReadPropertyMultiple for efficiency
- **Timeout Handling**: Adaptive timeout strategies
- **Error Recovery**: Graceful handling of device variations
- **Connection Reuse**: Persistent socket management

## Error Handling

### Database Errors
```rust
match db.add_object(Box::new(duplicate_object)) {
    Err(ObjectError::InvalidConfiguration(msg)) => {
        println!("Duplicate object: {}", msg);
    }
    Ok(_) => println!("Object added successfully"),
}
```

### Communication Errors
```rust
match read_device_properties(&socket, target_addr) {
    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
        println!("Device not responding - check network");
    }
    Err(e) => println!("Communication error: {}", e),
    Ok(properties) => process_properties(properties),
}
```

## Advanced Features

### Custom Object Types
```rust
#[derive(Debug, Clone)]
struct CustomObject {
    identifier: ObjectIdentifier,
    object_name: String,
    custom_property: f64,
}

impl BacnetObject for CustomObject {
    fn identifier(&self) -> ObjectIdentifier { self.identifier }
    // ... implement required methods
}
```

### Property Validation
```rust
impl BacnetObject for AnalogInput {
    fn set_property(&mut self, property: PropertyIdentifier, value: PropertyValue) -> Result<()> {
        match property {
            PropertyIdentifier::PresentValue => {
                if let PropertyValue::Real(val) = value {
                    if val >= self.min_pres_value.unwrap_or(f32::MIN) && 
                       val <= self.max_pres_value.unwrap_or(f32::MAX) {
                        self.present_value = val;
                        Ok(())
                    } else {
                        Err(ObjectError::InvalidValue("Value out of range".into()))
                    }
                } else {
                    Err(ObjectError::InvalidPropertyType)
                }
            }
            _ => Err(ObjectError::PropertyNotWritable),
        }
    }
}
```

## Integration Points

These object examples integrate with:
- **Basic examples**: Device creation and configuration
- **Networking examples**: Remote device discovery
- **Communication examples**: Service request/response patterns
- **Utilities examples**: Performance monitoring and statistics
- **Debugging examples**: Property analysis and troubleshooting
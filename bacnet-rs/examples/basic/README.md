# Basic BACnet Examples

Fundamental examples for getting started with BACnet device development.

## Examples

### `simple_device.rs`
**Purpose**: Basic BACnet device creation and configuration

**Features**:
- Device object creation with vendor ID management
- Property access and modification
- Vendor information handling
- Basic configuration patterns

**Usage**:
```bash
cargo run --example basic/simple_device
```

**What it demonstrates**:
- Creating a Device object with official vendor IDs
- Setting and getting device properties
- Vendor ID validation and lookup
- Basic BACnet object patterns

---

### `advanced_device.rs`
**Purpose**: Advanced device with multiple object types

**Features**:
- Multiple object types (AI, AO, AV, BI, BO, BV, MSI, MSO, MSV)
- Property relationships and dependencies
- Status flags and reliability indicators
- Engineering units and scaling

**Usage**:
```bash
cargo run --example basic/advanced_device
```

**What it demonstrates**:
- Complex device configurations
- Object type relationships
- Property validation and constraints
- Real-world device modeling

---

### `responder_device.rs`
**Purpose**: Interactive device for network testing

**Features**:
- Listens for Who-Is requests on UDP port 47808
- Sends I-Am responses with device information
- Configurable device ID
- Network debugging and monitoring

**Usage**:
```bash
# Default device ID (12345)
cargo run --example basic/responder_device

# Custom device ID
cargo run --example basic/responder_device 98765
```

**What it demonstrates**:
- BACnet/IP network communication
- Who-Is/I-Am protocol implementation
- UDP socket handling
- Device discovery patterns

## Learning Progression

1. **Start with `simple_device.rs`** to understand:
   - Basic device object creation
   - Property access patterns
   - Vendor ID management

2. **Move to `advanced_device.rs`** to explore:
   - Multiple object types
   - Complex configurations
   - Real-world scenarios

3. **Use `responder_device.rs`** to test:
   - Network communication
   - Device discovery
   - Protocol implementation

## Common Patterns

### Device Creation
```rust
use bacnet_rs::object::Device;

let mut device = Device::new(12345, "My Device".to_string());
device.vendor_name = "My Company".to_string();
device.model_name = "Model v1.0".to_string();
```

### Property Access
```rust
use bacnet_rs::object::{PropertyIdentifier, PropertyValue};

// Get property
let name = device.get_property(PropertyIdentifier::ObjectName)?;

// Set property  
device.set_property(
    PropertyIdentifier::ObjectName,
    PropertyValue::CharacterString("New Name".to_string())
)?;
```

### Vendor Management
```rust
// Set official vendor ID
device.set_vendor_by_id(999)?;

// Check if vendor ID is official
if device.is_vendor_id_official() {
    println!("Using official vendor ID");
}
```

## Testing Tips

- Use `responder_device.rs` with the networking examples
- Modify device IDs to test multiple devices
- Check vendor ID assignments for your organization
- Monitor network traffic to understand protocols

## Next Steps

After mastering these basics:
- Explore networking examples for device discovery
- Study object examples for advanced object management
- Use utility examples for performance monitoring
- Apply debugging techniques for troubleshooting
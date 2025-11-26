# BACnet Rust Examples

This directory contains comprehensive examples demonstrating various aspects of the BACnet Rust library. Examples are organized by category for easy navigation.

## 📁 Example Categories

### 🔧 Basic (`basic/`)
Fundamental BACnet device examples for getting started.

- **`simple_device.rs`** - Basic BACnet device creation and configuration
- **`advanced_device.rs`** - Advanced device with multiple object types and properties
- **`responder_device.rs`** - Device that responds to Who-Is requests

### 🌐 Networking (`networking/`)
Network communication and protocol examples.

- **`whois_scan.rs`** - BACnet Who-Is scanner for device discovery
- **`transport_demo.rs`** - Transport layer functionality demonstration

### 🏗️ Objects (`objects/`)
BACnet object management and database examples.

- **`object_database.rs`** - Comprehensive object database demonstration
- **`device_objects.rs`** - Read and display objects from remote devices

### 📡 Communication (`communication/`)
Device-to-device communication examples.

- **`test_client.rs`** - BACnet client for testing communication

### 🛠️ Utilities (`utilities/`)
Utility functions and helper demonstrations.

- **`util_demo.rs`** - Performance monitoring, statistics, and utility functions
- **`vendor_lookup.rs`** - BACnet vendor ID lookup and management

### 🐛 Debugging (`debugging/`)
Tools for debugging and analysis.

- **`debug_properties.rs`** - Property debugging and analysis tools

## 🚀 Quick Start

### Basic Device Example
```bash
# Create and run a simple BACnet device
cargo run --example basic/simple_device
```

### Network Discovery
```bash
# Scan for BACnet devices on the network
cargo run --example networking/whois_scan
```

### Object Database
```bash
# Demonstrate object database functionality
cargo run --example objects/object_database
```

### Transport Layer
```bash
# Show transport layer capabilities
cargo run --example networking/transport_demo
```

## 🧪 Testing Device Discovery

To test the complete Who-Is scan functionality:

1. **Start a responder device:**
   ```bash
   cargo run --example basic/responder_device 12345
   ```

2. **Run the scanner in another terminal:**
   ```bash
   cargo run --example networking/whois_scan
   ```

3. **Test with real devices:**
   ```bash
   # Scan a specific device
   cargo run --example objects/device_objects 10.161.1.211
   ```

## 📚 Example Descriptions

### Basic Examples

#### `basic/simple_device.rs`
Demonstrates basic device creation with vendor ID management:
- Device object creation
- Vendor information handling  
- Property access
- Basic configuration

#### `basic/advanced_device.rs`
Shows advanced device features:
- Multiple object types (AI, AO, AV, BI, BO, BV, MSI, MSO, MSV)
- Property relationships
- Status flags and reliability
- Engineering units

#### `basic/responder_device.rs`
Interactive device for testing:
- Listens for Who-Is requests
- Sends I-Am responses
- Configurable device ID
- Network debugging

### Networking Examples

#### `networking/whois_scan.rs`
Network device discovery:
- Broadcast Who-Is requests
- I-Am response processing
- Multi-subnet scanning
- Device information display

#### `networking/transport_demo.rs`
Transport layer demonstration:
- BVLL message handling
- Foreign device registration
- Broadcast distribution management
- BACnet/IP features

### Object Examples

#### `objects/object_database.rs`
Complete object database system:
- Object storage and retrieval
- Property access and modification
- Search and query capabilities
- Database statistics

#### `objects/device_objects.rs`
Remote device object reading:
- ReadPropertyMultiple requests
- Object list enumeration
- Property parsing and display
- UTF-16 string support

### Communication Examples

#### `communication/test_client.rs`
BACnet client functionality:
- Confirmed service requests
- Error handling
- Response processing
- Communication patterns

### Utility Examples

#### `utilities/util_demo.rs`
Comprehensive utility demonstration:
- Performance monitoring
- Statistics collection
- Retry mechanisms
- Data formatting

#### `utilities/vendor_lookup.rs`
Vendor ID management:
- Official vendor database
- ID validation and lookup
- Vendor information display
- Reserved ID handling

### Debugging Examples

#### `debugging/debug_properties.rs`
Property debugging tools:
- Raw data analysis
- Encoding/decoding verification
- Error diagnosis
- Data structure inspection

## 🔧 Configuration

Most examples can be configured through:

- **Command line arguments** - Device IDs, IP addresses, ports
- **Environment variables** - Network settings, debug levels
- **Source code modifications** - Object configurations, timeouts

## 🌐 Network Requirements

- **UDP Port 47808** - Standard BACnet/IP port
- **Broadcast capability** - For device discovery
- **Firewall permissions** - Allow UDP traffic
- **Network segment** - Devices must be reachable

## 🛠️ Troubleshooting

### Common Issues

1. **Device not found**
   - Check network connectivity
   - Verify firewall settings
   - Ensure correct IP address/subnet

2. **Permission denied**
   - Run with elevated privileges for broadcast
   - Check port availability

3. **Timeout errors**
   - Increase timeout values
   - Check device responsiveness
   - Verify BACnet service support

### Debug Tips

- Use `RUST_LOG=debug` for verbose output
- Check network traffic with Wireshark
- Verify BACnet port 47808 accessibility
- Test with known working BACnet devices

## 📖 Learning Path

1. **Start with** `basic/simple_device.rs` to understand device creation
2. **Progress to** `networking/whois_scan.rs` for network discovery
3. **Explore** `objects/object_database.rs` for object management
4. **Study** `utilities/util_demo.rs` for advanced features
5. **Use** debugging examples for troubleshooting

## 🎯 Next Steps

After running the examples:

- Modify object configurations for your use case
- Integrate with existing BACnet networks
- Implement custom object types
- Add application-specific functionality
- Deploy in production environments

For more detailed information, see the main project documentation and individual example source code comments.
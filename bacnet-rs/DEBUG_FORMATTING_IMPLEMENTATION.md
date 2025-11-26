# BACnet Debug Formatting Implementation

This document describes the comprehensive debug formatting functionality that has been implemented for the BACnet library, fulfilling the TODO requirement for debug formatting support in the util module.

## Overview

The debug formatting implementation provides production-ready debugging and analysis capabilities for BACnet protocol structures, making it easier to understand, troubleshoot, and develop with the BACnet library.

## Features Implemented

### 1. Debug Module (`src/util/mod.rs` - debug submodule)

#### Property Value Formatting
- **All BACnet Data Types**: Support for boolean, unsigned/signed integers, real, double, character strings, octet strings, enumerated, date, time, and object identifiers
- **UTF-16 Support**: Proper decoding of UTF-16 character strings (encoding type 4)
- **Human-Readable Output**: Converts raw BACnet encoded data to readable format
- **Type-Specific Formatting**: Each data type formatted according to its semantics

#### Service Choice Display
- **Named Service Identification**: Converts service choice codes to human-readable names
- **Comprehensive Coverage**: All standard confirmed and unconfirmed services
- **Unknown Service Handling**: Graceful handling of non-standard service codes

#### Error Code Translation
- **BACnet Error Classes**: Device, object, property, resources, security, services, VT, communication
- **Error Code Mapping**: Standard BACnet error codes with descriptions
- **Structured Error Display**: Class and code information formatted for easy understanding

#### Protocol Structure Analysis
- **BVLL Layer Analysis**: Complete BACnet Virtual Link Layer structure breakdown
- **NPDU Layer Analysis**: Network Protocol Data Unit parsing and display
- **APDU Layer Analysis**: Application Protocol Data Unit structure interpretation
- **Multi-Layer Visualization**: Complete protocol stack analysis

#### Annotated Hex Dumps
- **Field Annotations**: Hex dumps with field-specific annotations
- **Position-Based Comments**: Annotations tied to specific byte positions
- **Protocol-Aware**: Understanding of BACnet packet structure for intelligent annotations

### 2. Debug Formatting Functions

#### Property Value Formatting
```rust
pub fn format_property_value(data: &[u8]) -> String
```
- Analyzes BACnet encoded property data
- Returns human-readable representation
- Handles all standard BACnet data types
- Provides detailed error information for invalid data

#### Service Choice Formatting
```rust
pub fn format_service_choice(service_choice: u8) -> String
```
- Maps service choice codes to service names
- Covers all standard BACnet services
- Returns formatted string with code and name

#### Error Formatting
```rust
pub fn format_bacnet_error(error_class: u8, error_code: u8) -> String
```
- Translates BACnet error class and code to description
- Provides structured error information
- Handles all standard error classes

#### Protocol Structure Formatters
```rust
pub fn format_apdu_structure(data: &[u8]) -> String
pub fn format_npdu_structure(data: &[u8]) -> String
pub fn format_bvll_structure(data: &[u8]) -> String
```
- Layer-specific protocol analysis
- Detailed field breakdown
- Human-readable protocol information

#### Annotated Hex Dump
```rust
pub fn annotated_hex_dump(data: &[u8], annotations: &[(usize, String)]) -> String
```
- Enhanced hex dump with field annotations
- Position-based annotation system
- Professional packet analysis format

### 3. Comprehensive Debug Example (`examples/debugging/debug_formatter.rs`)

The debug formatter example demonstrates all debug formatting features:

#### Property Value Demonstration
- Boolean values (true/false)
- Real values (IEEE 754 floats)
- Unsigned and signed integers
- ASCII and UTF-16 character strings
- Enumerated values
- Object identifiers
- Date and time values
- Octet strings

#### Service Choice Demonstration
- Common confirmed services (Read Property, Write Property, etc.)
- Service code to name mapping
- Unknown service handling

#### Error Code Demonstration
- Device errors (busy, offline)
- Object errors (not found, access denied)
- Property errors (not supported, out of range)
- Security and resource errors

#### Protocol Analysis Demonstration
- Complete BVLL/NPDU/APDU analysis
- Real-world packet scenarios
- Request/response/error packet breakdown
- Layer-by-layer structure visualization

#### Complete Packet Analysis
- End-to-end packet analysis workflow
- Multi-layer protocol breakdown
- Property value extraction and interpretation
- Real-world communication scenarios

## Technical Implementation Details

### 1. Data Type Parsing
- **Tag-Based Recognition**: Uses BACnet tag encoding to identify data types
- **Length Validation**: Proper bounds checking for all data parsing
- **Error Handling**: Graceful handling of malformed or incomplete data
- **Standards Compliance**: Follows BACnet standard encoding rules

### 2. String Handling
- **Multiple Encodings**: Support for ASCII (0) and UTF-16 (4) character strings
- **Encoding Detection**: Automatic encoding type detection and handling
- **Invalid Data Handling**: Safe handling of malformed string data
- **Memory Safety**: Rust's ownership system prevents buffer overflows

### 3. Protocol Layer Parsing
- **Header Validation**: Proper validation of protocol headers
- **Field Extraction**: Accurate extraction of protocol fields
- **Routing Information**: Handling of network routing information
- **Layer Separation**: Clear separation of BVLL, NPDU, and APDU layers

### 4. Object Identifier Decoding
- **Type Mapping**: Comprehensive mapping of object types to names
- **Instance Extraction**: Proper extraction of object instance numbers
- **Reserved Types**: Handling of reserved and vendor-specific object types
- **Format Consistency**: Consistent formatting across all object types

## Usage Examples

### Basic Property Value Formatting
```rust
use bacnet_rs::util::debug;

let boolean_data = &[0x11, 0x01]; // Boolean true
let formatted = debug::format_property_value(boolean_data);
println!("{}", formatted); // "Boolean(true)"

let real_data = &[0x44, 0x42, 0x28, 0x00, 0x00]; // Real 42.0
let formatted = debug::format_property_value(real_data);
println!("{}", formatted); // "Real(42)"
```

### Service and Error Formatting
```rust
// Service choice formatting
let service = debug::format_service_choice(12);
println!("{}", service); // "readProperty(12)"

// Error formatting
let error = debug::format_bacnet_error(1, 2);
println!("{}", error); // "Error(object class, code 2)"
```

### Protocol Structure Analysis
```rust
let packet = &[0x81, 0x0A, 0x00, 0x10, /* ... */];

// BVLL analysis
println!("{}", debug::format_bvll_structure(packet));

// NPDU analysis (skip BVLL header)
println!("{}", debug::format_npdu_structure(&packet[4..]));
```

### Annotated Hex Dumps
```rust
let data = &[0x81, 0x0A, 0x00, 0x18, /* ... */];
let annotations = vec![
    (0, "BVLL Type".to_string()),
    (1, "BVLL Function".to_string()),
    (2, "Length".to_string()),
];

println!("{}", debug::annotated_hex_dump(data, &annotations));
```

## Object Type Coverage

The debug formatter includes comprehensive object type mapping:

### Standard Objects
- Analog Input/Output/Value (0-2)
- Binary Input/Output/Value (3-5)
- Calendar, Command, Device (6-8)
- Event Enrollment, File, Group (9-11)
- Loop, Multi-state Input/Output/Value (12-14, 19)
- Notification Class, Program, Schedule (15-17)
- And many more standard types...

### Advanced Objects
- Trend Log, Life Safety Point/Zone (20-22)
- Accumulator, Pulse Converter (23-24)
- Event Log, Global Group (25-26)
- Load Control, Structured View (28-29)
- Access Door (30)

## Error Class Coverage

### Complete Error Class Support
- **Device (0)**: Device-related errors
- **Object (1)**: Object access and existence errors
- **Property (2)**: Property access and value errors
- **Resources (3)**: System resource errors
- **Security (4)**: Authentication and authorization errors
- **Services (5)**: Service support and availability errors
- **VT (6)**: Virtual terminal errors
- **Communication (7)**: Network and communication errors

## Service Choice Coverage

### Confirmed Services
- acknowledgeAlarm (0)
- confirmedCOVNotification (1)
- confirmedEventNotification (2)
- getAlarmSummary (3)
- atomicReadFile/atomicWriteFile (6-7)
- readProperty/writeProperty (12, 15)
- readPropertyMultiple/writePropertyMultiple (14, 16)
- createObject/deleteObject (10-11)
- And many more...

### Service Categories
- **Property Services**: Read/write property operations
- **File Services**: Atomic file operations
- **Object Services**: Object lifecycle management
- **Alarm & Event Services**: Alarm and event handling
- **Device Services**: Device management and control

## Performance Considerations

### 1. Efficient Parsing
- **Minimal Allocations**: Efficient string building with minimal memory allocation
- **Bounds Checking**: Fast bounds checking with early validation
- **Lazy Evaluation**: Only parse what's needed for the requested format

### 2. Memory Usage
- **No Heap Allocation for Simple Types**: Direct formatting for basic types
- **String Pooling**: Reuse of common strings and format patterns
- **Bounded Output**: Controlled output size for large data structures

### 3. Error Handling
- **Graceful Degradation**: Continue parsing even with partial errors
- **Error Context**: Detailed error information for debugging
- **Safe Defaults**: Safe fallback behavior for unknown data

## Integration with Existing Code

### Backward Compatibility
- **Non-Intrusive**: Debug formatting doesn't affect existing functionality
- **Optional Usage**: All debug features are optional utilities
- **Standard Interfaces**: Uses standard Rust formatting patterns

### Documentation Integration
- **Comprehensive Examples**: Extensive examples in the debugging section
- **API Documentation**: Full rustdoc documentation for all functions
- **Usage Patterns**: Clear documentation of common usage patterns

## Future Enhancements

### 1. Advanced Analysis
- **Performance Profiling**: Integration with performance monitoring
- **Statistical Analysis**: Communication pattern analysis
- **Historical Tracking**: Long-term debugging data collection

### 2. Output Formats
- **JSON Export**: Machine-readable debug output
- **HTML Reports**: Rich formatting for web viewing
- **Log Integration**: Integration with logging frameworks

### 3. Interactive Features
- **REPL Integration**: Interactive packet analysis
- **Streaming Analysis**: Real-time packet stream analysis
- **Filtering**: Advanced filtering and search capabilities

This debug formatting implementation provides comprehensive, production-ready debugging capabilities that significantly enhance the developer experience when working with the BACnet protocol stack.
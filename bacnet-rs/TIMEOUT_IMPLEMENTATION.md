# BACnet Timeout Implementation

This document describes the comprehensive timeout functionality that has been implemented for the BACnet transport layer, fulfilling the TODO requirement for timeout support.

## Overview

The timeout implementation provides production-ready timeout management for BACnet communication, addressing reliability and performance requirements for real-world deployments.

## Features Implemented

### 1. Transport Layer Timeouts (`src/transport/mod.rs`)

#### Socket-Level Timeouts
- **Read Timeout**: Configurable timeout for UDP socket read operations
- **Write Timeout**: Configurable timeout for UDP socket write operations
- **Dynamic Timeout Updates**: Runtime timeout configuration changes

#### Request-Level Timeouts
- **Confirmed Request Tracking**: Track timeout for each confirmed service request using invoke IDs
- **Request Lifecycle Management**: Complete request tracking from send to response/timeout
- **Concurrent Request Support**: Handle multiple simultaneous requests with individual timeouts

#### Enhanced Configuration
```rust
pub struct BacnetIpConfig {
    // Existing fields...
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub request_timeout: Duration,
    pub registration_timeout: Duration,
}
```

#### New Error Types
- `TransportError::Timeout(String)`: General timeout errors
- `TransportError::RequestNotFound(u8)`: Request tracking errors

### 2. Timeout Management Utilities

#### TimeoutManager
- **Invoke ID Management**: Automatic invoke ID generation and tracking
- **Timeout Checking**: Efficient timeout detection and cleanup
- **Request Statistics**: Track request duration and completion times

#### TimeoutConfig
- **Operation-Specific Timeouts**: Different timeouts for different operations
- **Default Values**: Production-ready default timeout values
- **Customization**: Easy configuration for specific network conditions

### 3. Timeout Utilities (`timeout_utils` module)

#### Condition Waiting
```rust
wait_for_condition(condition, timeout, check_interval) -> Result<()>
```
- Wait for arbitrary conditions with timeout
- Configurable check intervals
- Early termination on condition fulfillment

#### Retry with Exponential Backoff
```rust
retry_with_backoff(operation, max_attempts, initial_delay, max_delay, backoff_multiplier) -> Result<T>
```
- Robust retry logic with exponential backoff
- Configurable parameters for different scenarios
- Production-ready error recovery patterns

#### Adaptive Timeout Calculation
```rust
calculate_adaptive_timeout(recent_times, base_timeout, safety_factor) -> Duration
```
- Dynamic timeout calculation based on historical performance
- Statistical analysis (average + standard deviation)
- Network-aware timeout adaptation

#### Timeout-Aware Operations
- Wrap operations with timeout enforcement
- Measure operation duration vs. timeout limits
- Detailed timeout error reporting

### 4. Enhanced Transport Trait

Extended the `Transport` trait with timeout capabilities:
- `receive_timeout(timeout: Duration)`: Receive with custom timeout
- `send_confirmed_request()`: Send with timeout tracking
- `check_timeouts()`: Check for expired requests
- `complete_request()`: Mark request as completed

### 5. Foreign Device Registration Improvements

- **Registration Confirmation**: Wait for BBMD response with timeout
- **Retry Logic**: Automatic registration retry on timeout
- **Error Handling**: Proper handling of registration failures vs. timeouts

## Constants and Defaults

### Timeout Constants
```rust
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_REGISTRATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_CONCURRENT_REQUESTS: usize = 255;
```

### Default TimeoutConfig
- **Read/Write Operations**: 5 seconds
- **Confirmed Requests**: 10 seconds
- **Foreign Device Registration**: 30 seconds
- **Device Discovery**: 3 seconds
- **Property Operations**: 5 seconds
- **File Transfer**: 60 seconds

## Examples and Testing

### 1. Comprehensive Timeout Demo (`examples/networking/timeout_demo.rs`)

The timeout demo demonstrates all timeout features:

#### Socket Timeout Testing
- Test read timeouts with various durations
- Verify timeout accuracy and tolerance
- Demonstrate timeout configuration

#### Request Timeout Tracking
- Multiple concurrent confirmed requests
- Different timeout values per request
- Request completion and timeout checking

#### Adaptive Timeout Calculation
- Historical performance analysis
- Statistical timeout calculation
- Network condition adaptation

#### Retry Logic Demonstration
- Exponential backoff examples
- Configurable retry parameters
- Success and failure scenarios

#### Timeout Monitoring
- Statistics collection and analysis
- Performance tracking
- Timeout pattern detection

#### Condition Waiting
- Timeout-aware condition checking
- Early termination examples
- Timeout vs. success scenarios

### 2. Usage Examples

#### Basic Timeout Configuration
```rust
let mut config = BacnetIpConfig::default();
config.read_timeout = Some(Duration::from_secs(3));
config.request_timeout = Duration::from_secs(15);

let transport = BacnetIpTransport::new(config)?;
```

#### Confirmed Request with Timeout
```rust
let invoke_id = transport.send_confirmed_request(
    target_addr,
    request_data,
    Some(Duration::from_secs(5))
)?;

// Later, check for timeouts
let timed_out_requests = transport.check_timeouts();
```

#### Adaptive Timeout Example
```rust
let response_times = collect_recent_response_times();
let adaptive_timeout = timeout_utils::calculate_adaptive_timeout(
    &response_times,
    Duration::from_secs(5), // base timeout
    2.0 // safety factor
);
```

#### Retry with Backoff
```rust
let result = timeout_utils::retry_with_backoff(
    || perform_bacnet_operation(),
    5, // max attempts
    Duration::from_millis(100), // initial delay
    Duration::from_secs(5), // max delay
    2.0 // backoff multiplier
)?;
```

## Technical Implementation Details

### 1. Socket Timeout Management
- Uses native UDP socket timeout capabilities
- Platform-specific timeout handling
- Graceful fallback for timeout errors

### 2. Request Tracking
- HashMap-based invoke ID tracking
- Instant-based timestamp recording
- Efficient timeout checking algorithms

### 3. Memory Management
- Automatic cleanup of expired requests
- Bounded concurrent request limits
- Resource leak prevention

### 4. Thread Safety
- Thread-safe timeout management
- Concurrent request handling
- Safe timeout configuration updates

## Performance Considerations

### 1. Efficient Timeout Checking
- O(n) timeout checking where n = active requests
- Batch timeout processing
- Minimal memory allocation

### 2. Adaptive Algorithms
- Statistical calculation optimization
- Bounded history for memory efficiency
- Fast timeout calculation

### 3. Network Optimization
- Reduced retry overhead
- Smart timeout adaptation
- Efficient error recovery

## Error Handling

### 1. Timeout Error Classification
- Socket-level timeouts (I/O errors)
- Request-level timeouts (application errors)
- Registration timeouts (protocol errors)

### 2. Recovery Strategies
- Automatic retry with backoff
- Graceful degradation
- Error reporting and logging

### 3. Diagnostic Information
- Detailed timeout error messages
- Performance statistics
- Request tracking information

## Integration with Existing Code

### Backward Compatibility
- All existing code continues to work
- Optional timeout configuration
- Default timeout values for smooth migration

### Enhanced Features
- New Transport trait methods are additive
- Existing examples work unchanged
- Optional timeout utilities

## Future Enhancements

### 1. Async Support
- Tokio-based async timeouts
- Future-aware timeout management
- Async timeout utilities

### 2. Advanced Analytics
- Timeout pattern analysis
- Network performance metrics
- Predictive timeout calculation

### 3. Configuration Management
- Runtime timeout adjustment
- Dynamic timeout profiles
- Centralized timeout configuration

## Testing and Validation

### Unit Tests
- Timeout configuration validation
- Request tracking correctness
- Timeout calculation accuracy

### Integration Tests
- End-to-end timeout scenarios
- Network condition simulation
- Error recovery validation

### Example Programs
- Comprehensive timeout demonstration
- Real-world usage patterns
- Performance benchmarking

This timeout implementation provides a robust, production-ready foundation for reliable BACnet communication with comprehensive timeout management capabilities.
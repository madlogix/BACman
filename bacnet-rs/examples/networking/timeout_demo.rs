//! Timeout handling demonstration for BACnet transport layer
//!
//! This example demonstrates various timeout scenarios and handling strategies
//! for reliable BACnet communication including:
//! - Socket-level timeouts for read/write operations
//! - Request-level timeouts for confirmed services
//! - Adaptive timeout calculation based on network conditions
//! - Retry logic with exponential backoff
//! - Timeout monitoring and diagnostics

use bacnet_rs::transport::{
    timeout_utils, BacnetIpConfig, BacnetIpTransport, TimeoutConfig, Transport, TransportError,
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging if available
    if std::env::var("RUST_LOG").is_ok() {
        println!("Note: Set RUST_LOG=debug for verbose output");
    }

    println!("=== BACnet Timeout Handling Demo ===\n");

    // 1. Basic timeout configuration
    demo_timeout_configuration()?;

    // 2. Socket timeout handling
    demo_socket_timeouts()?;

    // 3. Request timeout tracking
    demo_request_timeouts()?;

    // 4. Adaptive timeout calculation
    demo_adaptive_timeouts()?;

    // 5. Retry with exponential backoff
    demo_retry_backoff()?;

    // 6. Timeout monitoring
    demo_timeout_monitoring()?;

    println!("\n=== Timeout Demo Complete ===");
    Ok(())
}

fn demo_timeout_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Timeout Configuration");
    println!("========================");

    // Default timeout configuration
    let default_config = TimeoutConfig::default();
    println!("Default timeouts:");
    println!("  Read timeout: {:?}", default_config.read_timeout);
    println!("  Write timeout: {:?}", default_config.write_timeout);
    println!("  Request timeout: {:?}", default_config.request_timeout);
    println!(
        "  Registration timeout: {:?}",
        default_config.registration_timeout
    );
    println!(
        "  Discovery timeout: {:?}",
        default_config.discovery_timeout
    );
    println!(
        "  Property read timeout: {:?}",
        default_config.property_read_timeout
    );
    println!(
        "  File transfer timeout: {:?}",
        default_config.file_transfer_timeout
    );

    // Custom timeout configuration
    let custom_config = TimeoutConfig {
        read_timeout: Duration::from_secs(2),
        write_timeout: Duration::from_secs(3),
        request_timeout: Duration::from_secs(15),
        registration_timeout: Duration::from_secs(45),
        discovery_timeout: Duration::from_secs(5),
        property_read_timeout: Duration::from_secs(8),
        file_transfer_timeout: Duration::from_secs(120),
    };

    println!("\nCustom timeouts:");
    println!("  Read timeout: {:?}", custom_config.read_timeout);
    println!("  Request timeout: {:?}", custom_config.request_timeout);

    // BACnet/IP transport with custom timeouts
    let ip_config = BacnetIpConfig {
        read_timeout: Some(custom_config.read_timeout),
        write_timeout: Some(custom_config.write_timeout),
        request_timeout: custom_config.request_timeout,
        registration_timeout: custom_config.registration_timeout,
        ..Default::default()
    };

    println!("\nBACnet/IP config with custom timeouts created");
    println!("  Socket read timeout: {:?}", ip_config.read_timeout);
    println!("  Socket write timeout: {:?}", ip_config.write_timeout);
    println!("  Request timeout: {:?}", ip_config.request_timeout);

    Ok(())
}

fn demo_socket_timeouts() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Socket Timeout Handling");
    println!("===========================");

    // Create transport with short timeouts for demonstration
    let config = BacnetIpConfig {
        bind_address: "127.0.0.1:0".parse()?, // Use any available port
        read_timeout: Some(Duration::from_millis(500)),
        write_timeout: Some(Duration::from_millis(500)),
        ..Default::default()
    };

    let mut transport = BacnetIpTransport::new(config)?;
    let local_addr = transport.local_address()?;
    println!("Transport bound to: {}", local_addr);

    // Demonstrate read timeout
    println!("\nTesting read timeout (will timeout after 500ms)...");
    let start = Instant::now();

    match transport.receive_timeout(Duration::from_millis(500)) {
        Ok((data, src)) => {
            println!("Unexpected data received from {}: {:?}", src, data);
        }
        Err(TransportError::IoError(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
            println!("✓ Read timeout occurred after {:?}", start.elapsed());
        }
        Err(TransportError::IoError(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
            println!(
                "✓ Read timeout occurred (would block) after {:?}",
                start.elapsed()
            );
        }
        Err(e) => {
            println!("Other error: {:?}", e);
        }
    }

    // Test with different timeout values
    let timeout_values = [
        Duration::from_millis(100),
        Duration::from_millis(250),
        Duration::from_millis(1000),
    ];

    for timeout in timeout_values {
        println!("\nTesting {:?} timeout...", timeout);
        let start = Instant::now();

        match transport.receive_timeout(timeout) {
            Ok(_) => println!("Data received (unexpected)"),
            Err(_) => {
                let elapsed = start.elapsed();
                let tolerance = Duration::from_millis(50); // Allow 50ms tolerance

                if elapsed >= timeout && elapsed <= timeout + tolerance {
                    println!("✓ Timeout occurred within tolerance: {:?}", elapsed);
                } else {
                    println!(
                        "⚠ Timeout outside tolerance: expected ~{:?}, got {:?}",
                        timeout, elapsed
                    );
                }
            }
        }
    }

    Ok(())
}

fn demo_request_timeouts() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Request Timeout Tracking");
    println!("============================");

    let config = BacnetIpConfig::default();
    let mut transport = BacnetIpTransport::new(config)?;

    // Simulate sending confirmed requests
    let target: SocketAddr = "192.168.1.100:47808".parse()?;
    let dummy_data = &[0x01, 0x02, 0x03, 0x04];

    println!("Sending confirmed requests with different timeouts...");

    // Send multiple requests with different timeouts
    let invoke_id1 =
        transport.send_confirmed_request(target, dummy_data, Some(Duration::from_secs(5)))?;
    println!("Request {} sent with 5s timeout", invoke_id1);

    let invoke_id2 =
        transport.send_confirmed_request(target, dummy_data, Some(Duration::from_secs(10)))?;
    println!("Request {} sent with 10s timeout", invoke_id2);

    let invoke_id3 = transport.send_confirmed_request(
        target, dummy_data, None, // Use default timeout
    )?;
    println!("Request {} sent with default timeout", invoke_id3);

    // Show active requests
    println!("\nActive requests: {}", transport.active_request_count());

    // Show requests by remaining time
    let requests_by_time = transport.get_requests_by_remaining_time();
    println!("Requests by remaining time:");
    for (invoke_id, remaining) in requests_by_time {
        println!(
            "  Request {}: {:.2}s remaining",
            invoke_id,
            remaining.as_secs_f64()
        );
    }

    // Simulate completing a request
    if let Some(elapsed) = transport.complete_request(invoke_id1) {
        println!("\nRequest {} completed after {:?}", invoke_id1, elapsed);
    }

    println!(
        "Active requests after completion: {}",
        transport.active_request_count()
    );

    // Simulate timeout checking
    println!("\nWaiting for timeouts...");
    thread::sleep(Duration::from_millis(100));

    let timed_out = transport.check_timeouts();
    if timed_out.is_empty() {
        println!("No requests timed out yet");
    } else {
        println!("Timed out requests: {:?}", timed_out);
    }

    Ok(())
}

fn demo_adaptive_timeouts() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. Adaptive Timeout Calculation");
    println!("================================");

    // Simulate historical response times
    let response_times = vec![
        Duration::from_millis(150),
        Duration::from_millis(200),
        Duration::from_millis(180),
        Duration::from_millis(250),
        Duration::from_millis(190),
        Duration::from_millis(300),
        Duration::from_millis(170),
        Duration::from_millis(220),
    ];

    println!("Historical response times: {:?}", response_times);

    let base_timeout = Duration::from_secs(1);
    let safety_factors = [1.5, 2.0, 3.0];

    for safety_factor in safety_factors {
        let adaptive_timeout =
            timeout_utils::calculate_adaptive_timeout(&response_times, base_timeout, safety_factor);

        println!(
            "Safety factor {}: adaptive timeout = {:?}",
            safety_factor, adaptive_timeout
        );
    }

    // Test with empty history
    let empty_times = vec![];
    let adaptive_timeout =
        timeout_utils::calculate_adaptive_timeout(&empty_times, base_timeout, 2.0);
    println!(
        "Empty history: adaptive timeout = {:?} (should equal base)",
        adaptive_timeout
    );

    // Test with very consistent times
    let consistent_times = vec![
        Duration::from_millis(200),
        Duration::from_millis(201),
        Duration::from_millis(199),
        Duration::from_millis(200),
        Duration::from_millis(202),
    ];

    let consistent_adaptive =
        timeout_utils::calculate_adaptive_timeout(&consistent_times, base_timeout, 2.0);
    println!(
        "Consistent times: adaptive timeout = {:?}",
        consistent_adaptive
    );

    // Test with highly variable times
    let variable_times = vec![
        Duration::from_millis(100),
        Duration::from_millis(500),
        Duration::from_millis(150),
        Duration::from_millis(800),
        Duration::from_millis(200),
    ];

    let variable_adaptive =
        timeout_utils::calculate_adaptive_timeout(&variable_times, base_timeout, 2.0);
    println!("Variable times: adaptive timeout = {:?}", variable_adaptive);

    Ok(())
}

fn demo_retry_backoff() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5. Retry with Exponential Backoff");
    println!("==================================");

    // Simulate a flaky operation that sometimes fails
    let failure_count = Arc::new(Mutex::new(0));
    let max_failures = 3;

    let operation = || {
        let mut count = failure_count.lock().unwrap();
        *count += 1;

        if *count <= max_failures {
            println!("Operation failed (attempt {})", *count);
            Err("Simulated failure")
        } else {
            println!("Operation succeeded (attempt {})", *count);
            Ok("Success!")
        }
    };

    println!("Testing retry with exponential backoff...");
    println!(
        "Operation will fail {} times before succeeding",
        max_failures
    );

    let start = Instant::now();

    match timeout_utils::retry_with_backoff(
        operation,
        5,                          // max attempts
        Duration::from_millis(100), // initial delay
        Duration::from_secs(2),     // max delay
        2.0,                        // backoff multiplier
    ) {
        Ok(result) => {
            println!("✓ Retry succeeded: {}", result);
            println!("Total time elapsed: {:?}", start.elapsed());
        }
        Err(e) => {
            println!("✗ Retry failed: {}", e);
        }
    }

    // Test with insufficient retries
    println!("\nTesting with insufficient retry attempts...");

    let failure_count2 = Arc::new(Mutex::new(0));
    let operation2 = || -> Result<&'static str, &'static str> {
        let mut count = failure_count2.lock().unwrap();
        *count += 1;
        println!("Operation failed (attempt {})", *count);
        Err("Always fails")
    };

    let start2 = Instant::now();

    match timeout_utils::retry_with_backoff(
        operation2,
        3, // max attempts (fewer than needed)
        Duration::from_millis(50),
        Duration::from_millis(500),
        1.5,
    ) {
        Ok(result) => {
            println!("Unexpected success: {}", result);
        }
        Err(e) => {
            println!("✓ Expected failure after retries: {}", e);
            println!("Total time elapsed: {:?}", start2.elapsed());
        }
    }

    Ok(())
}

fn demo_timeout_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n6. Timeout Monitoring");
    println!("=====================");

    // Create a simple timeout monitor
    let mut timeout_stats = TimeoutStats::new();

    // Simulate various operations with different outcomes
    let operations = [
        ("device_discovery", Duration::from_millis(150), false),
        ("property_read", Duration::from_millis(300), false),
        ("device_discovery", Duration::from_millis(2500), true), // timeout
        ("property_read", Duration::from_millis(200), false),
        ("property_write", Duration::from_millis(1500), false),
        ("property_read", Duration::from_millis(5500), true), // timeout
        ("device_discovery", Duration::from_millis(180), false),
    ];

    for (operation, duration, timed_out) in operations {
        timeout_stats.record_operation(operation, duration, timed_out);
    }

    // Display statistics
    timeout_stats.print_stats();

    // Demonstrate timeout condition checking
    println!("\nTesting timeout condition checking...");

    let condition_met = Arc::new(Mutex::new(false));
    let condition_met_clone = condition_met.clone();

    // Spawn a thread to set the condition after 1 second
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        *condition_met_clone.lock().unwrap() = true;
    });

    let check_condition = || *condition_met.lock().unwrap();

    println!("Waiting for condition (should succeed after ~1s)...");
    let start = Instant::now();

    match timeout_utils::wait_for_condition(
        check_condition,
        Duration::from_millis(1500), // timeout
        Duration::from_millis(100),  // check interval
    ) {
        Ok(()) => {
            println!("✓ Condition met after {:?}", start.elapsed());
        }
        Err(e) => {
            println!("✗ Condition not met: {:?}", e);
        }
    }

    // Test timeout case
    println!("\nTesting timeout case (should timeout after 500ms)...");
    let start = Instant::now();

    match timeout_utils::wait_for_condition(
        || false, // condition never met
        Duration::from_millis(500),
        Duration::from_millis(100),
    ) {
        Ok(()) => {
            println!("Unexpected success");
        }
        Err(e) => {
            println!("✓ Expected timeout after {:?}: {:?}", start.elapsed(), e);
        }
    }

    Ok(())
}

/// Simple timeout statistics collector
#[derive(Debug)]
struct TimeoutStats {
    operations: std::collections::HashMap<String, OperationStats>,
}

#[derive(Debug)]
struct OperationStats {
    total_count: u32,
    timeout_count: u32,
    total_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
}

impl TimeoutStats {
    fn new() -> Self {
        Self {
            operations: std::collections::HashMap::new(),
        }
    }

    fn record_operation(&mut self, operation: &str, duration: Duration, timed_out: bool) {
        let stats = self
            .operations
            .entry(operation.to_string())
            .or_insert(OperationStats {
                total_count: 0,
                timeout_count: 0,
                total_duration: Duration::from_secs(0),
                min_duration: Duration::from_secs(u64::MAX),
                max_duration: Duration::from_secs(0),
            });

        stats.total_count += 1;
        if timed_out {
            stats.timeout_count += 1;
        }

        stats.total_duration += duration;
        stats.min_duration = std::cmp::min(stats.min_duration, duration);
        stats.max_duration = std::cmp::max(stats.max_duration, duration);
    }

    fn print_stats(&self) {
        println!("Timeout Statistics:");
        println!(
            "{:<20} {:>8} {:>8} {:>8} {:>10} {:>10} {:>10}",
            "Operation", "Total", "Timeouts", "Rate %", "Avg (ms)", "Min (ms)", "Max (ms)"
        );
        println!("{}", "-".repeat(80));

        for (operation, stats) in &self.operations {
            let timeout_rate = (stats.timeout_count as f64 / stats.total_count as f64) * 100.0;
            let avg_duration = stats.total_duration / stats.total_count;

            println!(
                "{:<20} {:>8} {:>8} {:>7.1}% {:>10} {:>10} {:>10}",
                operation,
                stats.total_count,
                stats.timeout_count,
                timeout_rate,
                avg_duration.as_millis(),
                stats.min_duration.as_millis(),
                stats.max_duration.as_millis(),
            );
        }
    }
}

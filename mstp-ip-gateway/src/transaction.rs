//! Transaction tracking for BACnet confirmed services
//!
//! This module implements transaction state management for routing confirmed BACnet services
//! between MS/TP and IP networks. It tracks pending requests to enable proper response routing
//! back to the original requester.
//!
//! ## Transaction Lifecycle
//!
//! ```text
//! IP Client                    Gateway                    MS/TP Device
//!     |                           |                           |
//!     |--ConfirmedRequest-------->|                           |
//!     |  (invoke_id=42)           | [Track transaction]       |
//!     |                           |--Forward----------------->|
//!     |                           |                           |
//!     |                           |<--Response----------------|
//!     |                           | [Lookup transaction]      |
//!     |<--Response----------------|                           |
//!     |                           | [Remove transaction]      |
//! ```
//!
//! ## Timeout Handling
//!
//! Transactions have service-specific timeouts based on ASHRAE 135 recommendations:
//! - Fast operations (ReadProperty): 10 seconds
//! - File operations (AtomicWriteFile): 60 seconds
//! - Device control (ReinitializeDevice): 30 seconds

use log::{debug, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bacnet_rs::service::ConfirmedServiceChoice;

/// Maximum number of concurrent transactions to prevent memory exhaustion
const MAX_CONCURRENT_TRANSACTIONS: usize = 256;

/// Default timeout for confirmed services (10 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default maximum retries for timed-out transactions
const DEFAULT_MAX_RETRIES: u8 = 3;

/// Errors that can occur during transaction management
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    /// Transaction table is full
    TableFull,
    /// Transaction not found
    NotFound,
    /// Duplicate invoke ID for the same destination
    DuplicateInvokeId,
    /// Invalid invoke ID
    InvalidInvokeId,
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionError::TableFull => write!(f, "Transaction table full"),
            TransactionError::NotFound => write!(f, "Transaction not found"),
            TransactionError::DuplicateInvokeId => write!(f, "Duplicate invoke ID"),
            TransactionError::InvalidInvokeId => write!(f, "Invalid invoke ID"),
        }
    }
}

impl std::error::Error for TransactionError {}

/// Key for looking up transactions in the table
///
/// Combines invoke_id and destination MAC address to uniquely identify a transaction.
/// This allows multiple clients to use the same invoke_id for different destinations.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct TransactionKey {
    /// Invoke ID from the APDU (0-255)
    pub invoke_id: u8,
    /// MS/TP destination address
    pub dest_mac: u8,
}

impl TransactionKey {
    /// Create a new transaction key
    pub fn new(invoke_id: u8, dest_mac: u8) -> Self {
        Self {
            invoke_id,
            dest_mac,
        }
    }
}

/// Pending transaction awaiting a response
///
/// Tracks all information needed to route the response back to the originating IP client.
#[derive(Debug, Clone)]
pub struct PendingTransaction {
    /// Invoke ID from the confirmed request
    pub invoke_id: u8,

    /// IP address of the client that sent the request
    pub source_addr: SocketAddr,

    /// Source network number from the request (if present)
    pub source_network: Option<u16>,

    /// Source MAC address from the request
    pub source_mac: Vec<u8>,

    /// Destination network (MS/TP network)
    pub dest_network: u16,

    /// MS/TP destination address
    pub dest_mac: u8,

    /// Service being requested
    pub service: ConfirmedServiceChoice,

    /// Whether this is a segmented request
    pub segmented: bool,

    /// Timestamp when transaction was created
    pub created_at: Instant,

    /// Timeout duration for this transaction
    pub timeout: Duration,

    /// Number of retries attempted
    pub retries: u8,

    /// Maximum retries allowed
    pub max_retries: u8,

    /// Original NPDU data for retransmission (routed format, ready to send to MS/TP)
    pub original_npdu: Vec<u8>,
}

impl PendingTransaction {
    /// Create a new pending transaction
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        invoke_id: u8,
        source_addr: SocketAddr,
        source_network: Option<u16>,
        source_mac: Vec<u8>,
        dest_network: u16,
        dest_mac: u8,
        service: ConfirmedServiceChoice,
        segmented: bool,
        original_npdu: Vec<u8>,
    ) -> Self {
        let timeout = service_timeout(service);

        Self {
            invoke_id,
            source_addr,
            source_network,
            source_mac,
            dest_network,
            dest_mac,
            service,
            segmented,
            created_at: Instant::now(),
            timeout,
            retries: 0,
            max_retries: DEFAULT_MAX_RETRIES,
            original_npdu,
        }
    }

    /// Check if the transaction has timed out
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }

    /// Get remaining time until timeout
    pub fn remaining_time(&self) -> Duration {
        self.timeout.saturating_sub(self.created_at.elapsed())
    }

    /// Check if retries are exhausted
    pub fn retries_exhausted(&self) -> bool {
        self.retries >= self.max_retries
    }

    /// Increment retry count and reset timestamp with exponential backoff
    ///
    /// Implements exponential backoff: timeout increases by 50% with each retry.
    /// For example, if base timeout is 10s:
    /// - Retry 1: 15s (10s * 1.5)
    /// - Retry 2: 22.5s (15s * 1.5)
    /// - Retry 3: 33.75s (22.5s * 1.5)
    pub fn retry(&mut self) {
        self.retries += 1;
        self.created_at = Instant::now();

        // Apply exponential backoff (50% increase per retry)
        // This gives devices more time to respond on subsequent attempts
        self.timeout = Duration::from_secs_f32(self.timeout.as_secs_f32() * 1.5);

        debug!(
            "Retrying transaction invoke_id={} to MS/TP {} (retry {}/{}, timeout={:.1}s)",
            self.invoke_id, self.dest_mac, self.retries, self.max_retries,
            self.timeout.as_secs_f32()
        );
    }
}

/// Statistics for transaction table
#[derive(Debug, Default, Clone)]
pub struct TransactionStats {
    /// Total transactions created
    pub total_created: u64,
    /// Total transactions completed successfully
    pub total_completed: u64,
    /// Total transactions that timed out
    pub total_timed_out: u64,
    /// Total retries attempted
    pub total_retries: u64,
    /// Current number of active transactions
    pub active_count: usize,
}

/// Transaction table for managing pending confirmed service requests
///
/// Tracks transactions by (invoke_id, dest_mac) key to enable response routing.
pub struct TransactionTable {
    /// Active transactions indexed by key
    transactions: HashMap<TransactionKey, PendingTransaction>,

    /// Maximum number of concurrent transactions
    max_transactions: usize,

    /// Statistics
    stats: TransactionStats,
}

impl TransactionTable {
    /// Create a new transaction table with default capacity
    pub fn new() -> Self {
        Self::with_capacity(MAX_CONCURRENT_TRANSACTIONS)
    }

    /// Create a new transaction table with specified capacity
    pub fn with_capacity(max_transactions: usize) -> Self {
        Self {
            transactions: HashMap::with_capacity(max_transactions.min(256)),
            max_transactions,
            stats: TransactionStats::default(),
        }
    }

    /// Add a new transaction to the table
    ///
    /// Returns an error if:
    /// - The table is full
    /// - A transaction with the same (invoke_id, dest_mac) already exists
    pub fn add(&mut self, transaction: PendingTransaction) -> Result<(), TransactionError> {
        // Check capacity
        if self.transactions.len() >= self.max_transactions {
            warn!(
                "Transaction table full ({}/{}), rejecting new transaction",
                self.transactions.len(),
                self.max_transactions
            );
            return Err(TransactionError::TableFull);
        }

        let key = TransactionKey::new(transaction.invoke_id, transaction.dest_mac);

        // Check for duplicates
        if self.transactions.contains_key(&key) {
            warn!(
                "Duplicate transaction: invoke_id={} dest_mac={}",
                transaction.invoke_id, transaction.dest_mac
            );
            return Err(TransactionError::DuplicateInvokeId);
        }

        debug!(
            "Added transaction: invoke_id={} service={:?} dest={}:{} timeout={:.1}s",
            transaction.invoke_id,
            transaction.service,
            transaction.dest_network,
            transaction.dest_mac,
            transaction.timeout.as_secs_f32()
        );

        self.transactions.insert(key, transaction);
        self.stats.total_created += 1;
        self.stats.active_count = self.transactions.len();

        Ok(())
    }

    /// Look up a transaction by invoke_id and destination MAC
    pub fn get(&self, invoke_id: u8, dest_mac: u8) -> Option<&PendingTransaction> {
        let key = TransactionKey::new(invoke_id, dest_mac);
        self.transactions.get(&key)
    }

    /// Look up a transaction mutably
    pub fn get_mut(&mut self, invoke_id: u8, dest_mac: u8) -> Option<&mut PendingTransaction> {
        let key = TransactionKey::new(invoke_id, dest_mac);
        self.transactions.get_mut(&key)
    }

    /// Remove and return a transaction
    ///
    /// Used when a response is received or transaction is aborted.
    pub fn remove(&mut self, invoke_id: u8, dest_mac: u8) -> Option<PendingTransaction> {
        let key = TransactionKey::new(invoke_id, dest_mac);
        let transaction = self.transactions.remove(&key)?;

        self.stats.total_completed += 1;
        self.stats.active_count = self.transactions.len();

        debug!(
            "Removed transaction: invoke_id={} service={:?} dest={}:{} age={:.1}s",
            transaction.invoke_id,
            transaction.service,
            transaction.dest_network,
            transaction.dest_mac,
            transaction.created_at.elapsed().as_secs_f32()
        );

        Some(transaction)
    }

    /// Check for timed-out transactions and return them
    ///
    /// This should be called periodically (e.g., every 1 second) to detect timeouts.
    /// Returns a vector of transactions that have timed out.
    pub fn check_timeouts(&mut self) -> Vec<PendingTransaction> {
        let mut timed_out = Vec::new();

        // Find timed-out transactions
        let timed_out_keys: Vec<TransactionKey> = self
            .transactions
            .iter()
            .filter(|(_, tx)| tx.is_timed_out())
            .map(|(key, _)| *key)
            .collect();

        // Remove and collect them
        for key in timed_out_keys {
            if let Some(transaction) = self.transactions.remove(&key) {
                warn!(
                    "Transaction timeout: invoke_id={} service={:?} dest={}:{} age={:.1}s",
                    transaction.invoke_id,
                    transaction.service,
                    transaction.dest_network,
                    transaction.dest_mac,
                    transaction.created_at.elapsed().as_secs_f32()
                );
                timed_out.push(transaction);
            }
        }

        if !timed_out.is_empty() {
            self.stats.total_timed_out += timed_out.len() as u64;
            self.stats.active_count = self.transactions.len();
        }

        timed_out
    }

    /// Re-add a transaction after a retry
    ///
    /// Increments the retry count and resets the timestamp.
    pub fn retry(&mut self, mut transaction: PendingTransaction) -> Result<(), TransactionError> {
        transaction.retry();
        self.stats.total_retries += 1;

        let key = TransactionKey::new(transaction.invoke_id, transaction.dest_mac);
        self.transactions.insert(key, transaction);
        self.stats.active_count = self.transactions.len();

        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> &TransactionStats {
        &self.stats
    }

    /// Get number of active transactions
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Clear all transactions (used for testing or emergency reset)
    pub fn clear(&mut self) {
        warn!("Clearing all {} transactions", self.transactions.len());
        self.transactions.clear();
        self.stats.active_count = 0;
    }
}

impl Default for TransactionTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the recommended timeout for a BACnet service
///
/// Based on ASHRAE 135 recommendations and typical device response times.
fn service_timeout(service: ConfirmedServiceChoice) -> Duration {
    use ConfirmedServiceChoice::*;

    match service {
        // Fast property operations (10 seconds)
        ReadProperty | WriteProperty | ReadPropertyMultiple | WritePropertyMultiple => {
            Duration::from_secs(10)
        }

        // File operations (60 seconds - large transfers)
        AtomicReadFile => Duration::from_secs(30),
        AtomicWriteFile => Duration::from_secs(60),

        // Device management (30 seconds - may be slow to respond)
        ReinitializeDevice | DeviceCommunicationControl => Duration::from_secs(30),

        // Object operations (15 seconds)
        CreateObject | DeleteObject | AddListElement | RemoveListElement => {
            Duration::from_secs(15)
        }

        // Alarm/Event services (15 seconds)
        AcknowledgeAlarm
        | GetAlarmSummary
        | GetEnrollmentSummary
        | GetEventInformation
        | ConfirmedEventNotification => Duration::from_secs(15),

        // COV subscriptions (10 seconds)
        SubscribeCOV | SubscribeCOVProperty => Duration::from_secs(10),

        // Range operations (20 seconds - potentially large data)
        ReadRange => Duration::from_secs(20),

        // Virtual terminal (15 seconds)
        VtOpen | VtClose | VtData => Duration::from_secs(15),

        // Security (20 seconds - may involve crypto)
        Authenticate | RequestKey | AuthRequest => Duration::from_secs(20),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_transaction_key() {
        let key1 = TransactionKey::new(42, 10);
        let key2 = TransactionKey::new(42, 10);
        let key3 = TransactionKey::new(42, 11);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_add_transaction() {
        let mut table = TransactionTable::new();
        let transaction = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        assert!(table.add(transaction).is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_duplicate_transaction() {
        let mut table = TransactionTable::new();
        let transaction1 = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );
        let transaction2 = transaction1.clone();

        assert!(table.add(transaction1).is_ok());
        assert_eq!(table.add(transaction2), Err(TransactionError::DuplicateInvokeId));
    }

    #[test]
    fn test_get_transaction() {
        let mut table = TransactionTable::new();
        let transaction = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        table.add(transaction).unwrap();

        let found = table.get(42, 10);
        assert!(found.is_some());
        assert_eq!(found.unwrap().invoke_id, 42);

        let not_found = table.get(43, 10);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_remove_transaction() {
        let mut table = TransactionTable::new();
        let transaction = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        table.add(transaction).unwrap();
        assert_eq!(table.len(), 1);

        let removed = table.remove(42, 10);
        assert!(removed.is_some());
        assert_eq!(table.len(), 0);

        let not_found = table.remove(42, 10);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_timeout_detection() {
        let mut table = TransactionTable::new();
        let mut transaction = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        // Set very short timeout for testing
        transaction.timeout = Duration::from_millis(50);
        table.add(transaction).unwrap();

        // Should not timeout immediately
        let timed_out = table.check_timeouts();
        assert_eq!(timed_out.len(), 0);

        // Wait for timeout
        thread::sleep(Duration::from_millis(100));

        // Should timeout now
        let timed_out = table.check_timeouts();
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].invoke_id, 42);
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_retry_mechanism() {
        let mut table = TransactionTable::new();
        let transaction = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        table.add(transaction).unwrap();

        let mut tx = table.remove(42, 10).unwrap();
        assert_eq!(tx.retries, 0);
        let original_timeout = tx.timeout;

        tx.retry();
        assert_eq!(tx.retries, 1);
        assert!(!tx.retries_exhausted());
        // Check exponential backoff increased timeout
        assert!(tx.timeout > original_timeout);

        tx.max_retries = 1;
        assert!(tx.retries_exhausted());
    }

    #[test]
    fn test_service_timeouts() {
        assert_eq!(
            service_timeout(ConfirmedServiceChoice::ReadProperty),
            Duration::from_secs(10)
        );
        assert_eq!(
            service_timeout(ConfirmedServiceChoice::AtomicWriteFile),
            Duration::from_secs(60)
        );
        assert_eq!(
            service_timeout(ConfirmedServiceChoice::ReinitializeDevice),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_table_capacity() {
        let mut table = TransactionTable::with_capacity(2);

        let tx1 = PendingTransaction::new(
            1,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );
        let tx2 = PendingTransaction::new(
            2,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            11,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );
        let tx3 = PendingTransaction::new(
            3,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            12,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        assert!(table.add(tx1).is_ok());
        assert!(table.add(tx2).is_ok());
        assert_eq!(table.add(tx3), Err(TransactionError::TableFull));
    }

    #[test]
    fn test_statistics() {
        let mut table = TransactionTable::new();

        assert_eq!(table.stats().total_created, 0);
        assert_eq!(table.stats().total_completed, 0);

        let tx = PendingTransaction::new(
            42,
            "192.168.1.100:47808".parse().unwrap(),
            Some(2),
            vec![192, 168, 1, 100, 0xBA, 0xC0],
            1,
            10,
            ConfirmedServiceChoice::ReadProperty,
            false,
            vec![0x01, 0x08, 0x00, 0x01, 0x01, 0x0A], // Mock NPDU
        );

        table.add(tx).unwrap();
        assert_eq!(table.stats().total_created, 1);
        assert_eq!(table.stats().active_count, 1);

        table.remove(42, 10);
        assert_eq!(table.stats().total_completed, 1);
        assert_eq!(table.stats().active_count, 0);
    }
}

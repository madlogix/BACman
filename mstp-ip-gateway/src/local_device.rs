//! Local BACnet Device for the Gateway
//!
//! This module implements a local BACnet Device object that allows the gateway
//! to respond to Who-Is requests and be discoverable on the network.

use log::{debug, info, trace};

/// Vendor ID for Madlogix (using a placeholder - should register with ASHRAE)
/// Per BACnet standard, unregistered vendors should use 0xFFFF or apply for one
const VENDOR_ID: u32 = 65535; // Unregistered vendor

/// APDU types
const APDU_UNCONFIRMED_REQUEST: u8 = 0x10;
const APDU_CONFIRMED_REQUEST: u8 = 0x00;
const APDU_COMPLEX_ACK: u8 = 0x30;
const APDU_ERROR: u8 = 0x50;

/// Unconfirmed service choices
const SERVICE_WHO_IS: u8 = 8;
const SERVICE_I_AM: u8 = 0;

/// Confirmed service choices
const SERVICE_READ_PROPERTY: u8 = 12;
const SERVICE_READ_PROPERTY_MULTIPLE: u8 = 14;

/// Object types
const OBJECT_TYPE_DEVICE: u16 = 8;

/// Segmentation support values
const SEGMENTATION_NOT_SUPPORTED: u32 = 3;

/// Max APDU length for MS/TP (conservative)
const MAX_APDU_LENGTH: u32 = 480;

/// Property identifiers
const PROP_OBJECT_IDENTIFIER: u32 = 75;
const PROP_OBJECT_NAME: u32 = 77;
const PROP_OBJECT_TYPE: u32 = 79;
const PROP_SYSTEM_STATUS: u32 = 112;
const PROP_VENDOR_NAME: u32 = 121;
const PROP_VENDOR_IDENTIFIER: u32 = 120;
const PROP_MODEL_NAME: u32 = 70;
const PROP_FIRMWARE_REVISION: u32 = 44;
const PROP_APPLICATION_SOFTWARE_VERSION: u32 = 12;
const PROP_PROTOCOL_VERSION: u32 = 98;
const PROP_PROTOCOL_REVISION: u32 = 139;
const PROP_PROTOCOL_SERVICES_SUPPORTED: u32 = 97;
const PROP_PROTOCOL_OBJECT_TYPES_SUPPORTED: u32 = 96;
const PROP_MAX_APDU_LENGTH_ACCEPTED: u32 = 62;
const PROP_SEGMENTATION_SUPPORTED: u32 = 107;
const PROP_APDU_TIMEOUT: u32 = 11;
const PROP_NUMBER_OF_APDU_RETRIES: u32 = 73;
const PROP_DATABASE_REVISION: u32 = 155;
const PROP_OBJECT_LIST: u32 = 76;
const PROP_DESCRIPTION: u32 = 28;
const PROP_LOCATION: u32 = 58;
const PROP_MAX_INFO_FRAMES: u32 = 63;
const PROP_MAX_MASTER: u32 = 64;
const PROP_LOCAL_DATE: u32 = 56;
const PROP_LOCAL_TIME: u32 = 57;
const PROP_DEVICE_ADDRESS_BINDING: u32 = 30;

/// Error classes
const ERROR_CLASS_OBJECT: u32 = 1;
const ERROR_CLASS_PROPERTY: u32 = 2;

/// Error codes
const ERROR_CODE_UNKNOWN_OBJECT: u32 = 31;
const ERROR_CODE_UNKNOWN_PROPERTY: u32 = 32;

/// Device status values
const STATUS_OPERATIONAL: u32 = 0;

/// Local BACnet Device
pub struct LocalDevice {
    /// Device instance number
    pub device_instance: u32,
    /// Device name
    pub device_name: String,
    /// Vendor name
    pub vendor_name: String,
    /// Model name
    pub model_name: String,
    /// Firmware revision
    pub firmware_revision: String,
    /// Application software version
    pub application_version: String,
    /// Max master for MS/TP
    pub max_master: u8,
    /// Max info frames for MS/TP
    pub max_info_frames: u8,
}

impl LocalDevice {
    /// Create a new local device
    pub fn new(device_instance: u32) -> Self {
        Self::new_with_mstp(device_instance, 127, 1)
    }

    /// Create a new local device with MS/TP parameters
    pub fn new_with_mstp(device_instance: u32, max_master: u8, max_info_frames: u8) -> Self {
        info!("Creating local BACnet device with instance {}", device_instance);
        Self {
            device_instance,
            device_name: format!("BACman Gateway {}", device_instance),
            vendor_name: "Madlogix".to_string(),
            model_name: "BACman".to_string(),
            firmware_revision: env!("CARGO_PKG_VERSION").to_string(),
            application_version: env!("CARGO_PKG_VERSION").to_string(),
            max_master,
            max_info_frames,
        }
    }

    /// Process an APDU and return a response if applicable
    /// Returns (response_data, is_broadcast_response)
    pub fn process_apdu(&self, apdu: &[u8]) -> Option<(Vec<u8>, bool)> {
        if apdu.is_empty() {
            return None;
        }

        let pdu_type = apdu[0] & 0xF0;

        match pdu_type {
            APDU_UNCONFIRMED_REQUEST => self.process_unconfirmed_request(apdu),
            APDU_CONFIRMED_REQUEST => self.process_confirmed_request(apdu),
            _ => {
                trace!("Ignoring APDU type 0x{:02X}", pdu_type);
                None
            }
        }
    }

    /// Process unconfirmed request (Who-Is, etc.)
    fn process_unconfirmed_request(&self, apdu: &[u8]) -> Option<(Vec<u8>, bool)> {
        if apdu.len() < 2 {
            return None;
        }

        let service_choice = apdu[1];

        match service_choice {
            SERVICE_WHO_IS => self.handle_who_is(&apdu[2..]),
            _ => {
                trace!("Ignoring unconfirmed service {}", service_choice);
                None
            }
        }
    }

    /// Handle Who-Is request
    fn handle_who_is(&self, data: &[u8]) -> Option<(Vec<u8>, bool)> {
        info!("*** Who-Is received! Parsing range from {} bytes ***", data.len());

        // Parse Who-Is parameters (if any)
        let (low_limit, high_limit) = if data.is_empty() {
            // No range specified - matches all devices
            info!("Who-Is: No range specified - matches ALL devices");
            (None, None)
        } else {
            // Try to parse range
            info!("Who-Is: Range data: {:02X?}", data);
            self.parse_who_is_range(data)
        };

        info!("Who-Is: Parsed range: low={:?}, high={:?}, our instance={}",
              low_limit, high_limit, self.device_instance);

        // Check if our device instance is in range
        let matches = match (low_limit, high_limit) {
            (None, None) => true,
            (Some(low), Some(high)) => {
                self.device_instance >= low && self.device_instance <= high
            }
            (Some(low), None) => self.device_instance >= low,
            (None, Some(high)) => self.device_instance <= high,
        };

        if matches {
            info!(
                "Who-Is MATCHES our device {} - generating I-Am response!",
                self.device_instance
            );
            let iam = self.build_i_am();
            info!("I-Am APDU generated: {:02X?}", &iam[..iam.len().min(20)]);
            Some((iam, true)) // I-Am is broadcast
        } else {
            info!(
                "Who-Is does NOT match our device {} (range: {:?}-{:?})",
                self.device_instance, low_limit, high_limit
            );
            None
        }
    }

    /// Parse Who-Is range parameters
    fn parse_who_is_range(&self, data: &[u8]) -> (Option<u32>, Option<u32>) {
        let mut pos = 0;
        let mut low_limit = None;
        let mut high_limit = None;

        // Try to parse context tag 0 (low limit)
        if pos < data.len() {
            if let Some((value, consumed)) = self.decode_context_unsigned(data, pos, 0) {
                low_limit = Some(value);
                pos += consumed;
            }
        }

        // Try to parse context tag 1 (high limit)
        if pos < data.len() {
            if let Some((value, _consumed)) = self.decode_context_unsigned(data, pos, 1) {
                high_limit = Some(value);
            }
        }

        (low_limit, high_limit)
    }

    /// Decode a context-tagged unsigned integer
    fn decode_context_unsigned(&self, data: &[u8], pos: usize, expected_tag: u8) -> Option<(u32, usize)> {
        if pos >= data.len() {
            return None;
        }

        let tag_byte = data[pos];

        // Check if it's a context tag
        if (tag_byte & 0x08) == 0 {
            return None;
        }

        let tag_number = (tag_byte >> 4) & 0x0F;
        if tag_number != expected_tag {
            return None;
        }

        let mut length = (tag_byte & 0x07) as usize;
        let mut consumed = 1;

        // Handle extended length
        if length == 5 && pos + 1 < data.len() {
            length = data[pos + 1] as usize;
            consumed += 1;
        }

        if pos + consumed + length > data.len() {
            return None;
        }

        // Decode the unsigned value
        let value = match length {
            1 => data[pos + consumed] as u32,
            2 => ((data[pos + consumed] as u32) << 8) | (data[pos + consumed + 1] as u32),
            3 => ((data[pos + consumed] as u32) << 16)
                | ((data[pos + consumed + 1] as u32) << 8)
                | (data[pos + consumed + 2] as u32),
            4 => ((data[pos + consumed] as u32) << 24)
                | ((data[pos + consumed + 1] as u32) << 16)
                | ((data[pos + consumed + 2] as u32) << 8)
                | (data[pos + consumed + 3] as u32),
            _ => return None,
        };

        Some((value, consumed + length))
    }

    /// Build I-Am response APDU (public for periodic announcements)
    pub fn build_i_am(&self) -> Vec<u8> {
        let mut apdu = Vec::with_capacity(20);

        // PDU type - Unconfirmed Request
        apdu.push(APDU_UNCONFIRMED_REQUEST);

        // Service choice - I-Am
        apdu.push(SERVICE_I_AM);

        // I-Am Device Identifier (Application Tag 12 - Object Identifier)
        // Tag: 0xC4 = Application tag 12, length 4
        apdu.push(0xC4);
        let object_id = ((OBJECT_TYPE_DEVICE as u32) << 22) | self.device_instance;
        apdu.extend_from_slice(&object_id.to_be_bytes());

        // Max APDU Length Accepted (Application Tag 2 - Unsigned)
        // Tag: 0x22 = Application tag 2, length 2
        apdu.push(0x22);
        apdu.extend_from_slice(&(MAX_APDU_LENGTH as u16).to_be_bytes());

        // Segmentation Supported (Application Tag 9 - Enumerated)
        // Tag: 0x91 = Application tag 9, length 1
        apdu.push(0x91);
        apdu.push(SEGMENTATION_NOT_SUPPORTED as u8);

        // Vendor ID (Application Tag 2 - Unsigned)
        // Tag: 0x22 = Application tag 2, length 2
        apdu.push(0x22);
        apdu.extend_from_slice(&(VENDOR_ID as u16).to_be_bytes());

        debug!("Built I-Am for device {}", self.device_instance);
        apdu
    }

    /// Build I-Am-Router-To-Network NPDU
    /// This is a network layer message (not APDU) announcing this router can reach certain networks
    /// Per BACnet Clause 6.6.3, message type 0x01
    pub fn build_i_am_router_to_network(networks: &[u16]) -> Vec<u8> {
        let mut npdu = Vec::with_capacity(4 + networks.len() * 2);

        // NPDU version
        npdu.push(0x01);

        // Control byte: network layer message (bit 7 = 1)
        npdu.push(0x80);

        // Message type: I-Am-Router-To-Network = 0x01
        npdu.push(0x01);

        // List of network numbers this router can reach
        for &net in networks {
            npdu.push((net >> 8) as u8);
            npdu.push((net & 0xFF) as u8);
        }

        debug!("Built I-Am-Router-To-Network for networks: {:?}", networks);
        npdu
    }

    /// Process confirmed request (ReadProperty, etc.)
    fn process_confirmed_request(&self, apdu: &[u8]) -> Option<(Vec<u8>, bool)> {
        if apdu.len() < 4 {
            return None;
        }

        // Parse confirmed request header
        // Byte 0: PDU type + flags
        // Byte 1: Max response segments + max APDU size
        // Byte 2: Invoke ID
        // Byte 3: Service choice
        let invoke_id = apdu[2];
        let service_choice = apdu[3];

        match service_choice {
            SERVICE_READ_PROPERTY => self.handle_read_property(invoke_id, &apdu[4..]),
            SERVICE_READ_PROPERTY_MULTIPLE => self.handle_read_property_multiple(invoke_id, &apdu[4..]),
            _ => {
                trace!("Ignoring confirmed service {}", service_choice);
                None
            }
        }
    }

    /// Handle ReadProperty request
    fn handle_read_property(&self, invoke_id: u8, data: &[u8]) -> Option<(Vec<u8>, bool)> {
        // Parse ReadProperty request
        // Context tag 0: Object Identifier (4 bytes)
        // Context tag 1: Property Identifier (1-2 bytes)
        // Context tag 2: Property Array Index (optional)

        debug!("ReadProperty request data: {:02X?}", data);

        let mut pos = 0;

        // Parse object identifier (context tag 0, length 4)
        // Context tag format: high nibble = tag number, bit 3 = 1 (context), low 3 bits = length
        // For context tag 0 with length 4: 0x0C = 0000 1100
        if pos >= data.len() {
            debug!("ReadProperty: no data for object ID");
            return self.build_error_response(invoke_id, SERVICE_READ_PROPERTY, ERROR_CLASS_OBJECT, ERROR_CODE_UNKNOWN_OBJECT);
        }

        let tag_byte = data[pos];
        // Check: context tag (bit 3 set), tag number 0 (bits 7-4), length 4 (bits 2-0)
        // 0x0C = 0000_1100 = tag 0, context, length 4
        if tag_byte != 0x0C {
            debug!("ReadProperty: expected context tag 0 length 4, got 0x{:02X}", tag_byte);
            return self.build_error_response(invoke_id, SERVICE_READ_PROPERTY, ERROR_CLASS_OBJECT, ERROR_CODE_UNKNOWN_OBJECT);
        }
        pos += 1;

        if pos + 4 > data.len() {
            debug!("ReadProperty: not enough data for object ID");
            return None;
        }
        let object_id = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let object_type = (object_id >> 22) as u16;
        let object_instance = object_id & 0x3FFFFF;
        pos += 4;

        debug!("ReadProperty: object type={}, instance={}", object_type, object_instance);

        // Check if it's our device object
        if object_type != OBJECT_TYPE_DEVICE || object_instance != self.device_instance {
            debug!(
                "ReadProperty for unknown object: type={}, instance={} (ours is {})",
                object_type, object_instance, self.device_instance
            );
            return self.build_error_response(invoke_id, SERVICE_READ_PROPERTY, ERROR_CLASS_OBJECT, ERROR_CODE_UNKNOWN_OBJECT);
        }

        // Parse property identifier (context tag 1)
        // Context tag 1 with length 1: 0x19 = 0001_1001
        // Context tag 1 with length 2: 0x1A = 0001_1010
        if pos >= data.len() {
            debug!("ReadProperty: no data for property ID");
            return None;
        }
        let tag_byte = data[pos];
        let tag_num = (tag_byte >> 4) & 0x0F;
        let is_context = (tag_byte & 0x08) != 0;
        let length = (tag_byte & 0x07) as usize;

        if !is_context || tag_num != 1 {
            debug!("ReadProperty: expected context tag 1, got tag_num={}, is_context={}", tag_num, is_context);
            return None;
        }
        pos += 1;

        if pos + length > data.len() {
            debug!("ReadProperty: not enough data for property ID");
            return None;
        }

        let property_id = match length {
            1 => data[pos] as u32,
            2 => ((data[pos] as u32) << 8) | (data[pos + 1] as u32),
            3 => ((data[pos] as u32) << 16) | ((data[pos + 1] as u32) << 8) | (data[pos + 2] as u32),
            4 => u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]),
            _ => return None,
        };

        info!("ReadProperty for Device:{} property {} (0x{:02X})", self.device_instance, property_id, property_id);

        // Build response
        self.build_read_property_response(invoke_id, object_id, property_id)
    }

    /// Build ReadProperty response
    fn build_read_property_response(&self, invoke_id: u8, object_id: u32, property_id: u32) -> Option<(Vec<u8>, bool)> {
        let mut apdu = Vec::with_capacity(64);

        // PDU type - Complex ACK
        apdu.push(APDU_COMPLEX_ACK);
        apdu.push(invoke_id);
        apdu.push(SERVICE_READ_PROPERTY);

        // Object Identifier (context tag 0, length 4)
        apdu.push(0x0C);
        apdu.extend_from_slice(&object_id.to_be_bytes());

        // Property Identifier (context tag 1)
        if property_id <= 0xFF {
            apdu.push(0x19);
            apdu.push(property_id as u8);
        } else {
            apdu.push(0x1A);
            apdu.extend_from_slice(&(property_id as u16).to_be_bytes());
        }

        // Property Value (context tag 3 opening)
        apdu.push(0x3E);

        // Encode the property value
        let value_encoded = match property_id {
            PROP_OBJECT_IDENTIFIER => {
                // Application tag 12 (Object ID), length 4
                let mut v = vec![0xC4];
                v.extend_from_slice(&object_id.to_be_bytes());
                v
            }
            PROP_OBJECT_NAME => {
                self.encode_character_string(&self.device_name)
            }
            PROP_OBJECT_TYPE => {
                // Enumerated, Device = 8
                vec![0x91, OBJECT_TYPE_DEVICE as u8]
            }
            PROP_SYSTEM_STATUS => {
                // Enumerated, Operational = 0
                vec![0x91, STATUS_OPERATIONAL as u8]
            }
            PROP_VENDOR_NAME => {
                self.encode_character_string(&self.vendor_name)
            }
            PROP_VENDOR_IDENTIFIER => {
                self.encode_unsigned(VENDOR_ID)
            }
            PROP_MODEL_NAME => {
                self.encode_character_string(&self.model_name)
            }
            PROP_FIRMWARE_REVISION => {
                self.encode_character_string(&self.firmware_revision)
            }
            PROP_APPLICATION_SOFTWARE_VERSION => {
                self.encode_character_string(&self.application_version)
            }
            PROP_PROTOCOL_VERSION => {
                // Unsigned, version 1
                vec![0x21, 1]
            }
            PROP_PROTOCOL_REVISION => {
                // Unsigned, revision 14 (common)
                vec![0x21, 14]
            }
            PROP_PROTOCOL_SERVICES_SUPPORTED => {
                // Bit string - services we support
                // We support: I-Am (bit 26), Who-Is (bit 33), ReadProperty (bit 12)
                // Bit string format: tag, [extended length], unused bits, data bytes
                // BACnet tag encoding: 0x85 = tag 8 (BitString), extended length (next byte)
                // 6 bytes of bit data + 1 unused bits byte = 7 bytes total
                let mut bits = [0u8; 6];
                // Set bit 12 (ReadProperty) - byte 1, bit 4
                bits[1] |= 0x08;
                // Set bit 26 (I-Am) - byte 3, bit 2
                bits[3] |= 0x20;
                // Set bit 33 (Who-Is) - byte 4, bit 1
                bits[4] |= 0x40;

                let mut v = vec![0x85, 0x07, 0x00]; // Tag 8 (BitString), length=7 (extended), 0 unused bits
                v.extend_from_slice(&bits);
                v
            }
            PROP_PROTOCOL_OBJECT_TYPES_SUPPORTED => {
                // Bit string - object types we support
                // We support: Device (bit 8)
                // BACnet tag encoding: 0x85 = tag 8 (BitString), extended length (next byte)
                // 7 bytes of bit data + 1 unused bits byte = 8 bytes total
                let mut bits = [0u8; 7];
                // Set bit 8 (Device) - byte 1, bit 0
                bits[1] |= 0x80;

                let mut v = vec![0x85, 0x08, 0x00]; // Tag 8 (BitString), length=8 (extended), 0 unused bits
                v.extend_from_slice(&bits);
                v
            }
            PROP_MAX_APDU_LENGTH_ACCEPTED => {
                self.encode_unsigned(MAX_APDU_LENGTH)
            }
            PROP_SEGMENTATION_SUPPORTED => {
                // Enumerated
                vec![0x91, SEGMENTATION_NOT_SUPPORTED as u8]
            }
            PROP_APDU_TIMEOUT => {
                // Unsigned, 3000 ms
                self.encode_unsigned(3000)
            }
            PROP_NUMBER_OF_APDU_RETRIES => {
                // Unsigned, 3 retries
                vec![0x21, 3]
            }
            PROP_DATABASE_REVISION => {
                // Unsigned, revision 1
                vec![0x21, 1]
            }
            PROP_OBJECT_LIST => {
                // Array of Object Identifiers - just contains our device object
                // Application tag 12 (Object ID), length 4
                let mut v = vec![0xC4];
                v.extend_from_slice(&object_id.to_be_bytes());
                v
            }
            PROP_DESCRIPTION => {
                self.encode_character_string("BACnet MS/TP to IP Gateway")
            }
            PROP_LOCATION => {
                self.encode_character_string("")
            }
            PROP_MAX_INFO_FRAMES => {
                vec![0x21, self.max_info_frames]
            }
            PROP_MAX_MASTER => {
                vec![0x21, self.max_master]
            }
            PROP_DEVICE_ADDRESS_BINDING => {
                // Empty list - we don't maintain address bindings
                // Return empty sequence (no data between opening/closing tags is fine)
                vec![]
            }
            _ => {
                debug!("Unknown property {} (0x{:02X}) requested", property_id, property_id);
                return self.build_error_response(invoke_id, SERVICE_READ_PROPERTY, ERROR_CLASS_PROPERTY, ERROR_CODE_UNKNOWN_PROPERTY);
            }
        };

        apdu.extend_from_slice(&value_encoded);

        // Property Value (context tag 3 closing)
        apdu.push(0x3F);

        Some((apdu, false)) // ReadProperty response is unicast
    }

    /// Build error response
    fn build_error_response(&self, invoke_id: u8, service: u8, error_class: u32, error_code: u32) -> Option<(Vec<u8>, bool)> {
        let mut apdu = Vec::with_capacity(8);

        // PDU type - Error
        apdu.push(APDU_ERROR);
        apdu.push(invoke_id);
        apdu.push(service);

        // Error class (enumerated, application tag 9)
        apdu.push(0x91);
        apdu.push(error_class as u8);

        // Error code (enumerated, application tag 9)
        apdu.push(0x91);
        apdu.push(error_code as u8);

        Some((apdu, false))
    }

    /// Handle ReadPropertyMultiple request
    fn handle_read_property_multiple(&self, invoke_id: u8, data: &[u8]) -> Option<(Vec<u8>, bool)> {
        debug!("ReadPropertyMultiple request, data len: {}", data.len());

        let mut apdu = Vec::with_capacity(256);

        // PDU type - Complex ACK
        apdu.push(APDU_COMPLEX_ACK);
        apdu.push(invoke_id);
        apdu.push(SERVICE_READ_PROPERTY_MULTIPLE);

        let mut pos = 0;

        // Parse each object specification
        while pos < data.len() {
            // Context tag 0 opening - Object Identifier
            if pos >= data.len() || data[pos] != 0x0C {
                debug!("RPM: Expected context tag 0 (object ID), got 0x{:02X} at pos {}",
                       if pos < data.len() { data[pos] } else { 0 }, pos);
                break;
            }
            pos += 1;

            if pos + 4 > data.len() {
                break;
            }

            let object_id = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            let object_type = (object_id >> 22) as u16;
            let object_instance = object_id & 0x3FFFFF;
            pos += 4;

            debug!("RPM: Object type={}, instance={}", object_type, object_instance);

            // Check if it's our device object
            if object_type != OBJECT_TYPE_DEVICE || object_instance != self.device_instance {
                debug!("RPM: Not our device, skipping");
                // Skip this object's property list
                while pos < data.len() && data[pos] != 0x0C {
                    pos += 1;
                }
                continue;
            }

            // Add object identifier to response (context tag 0)
            apdu.push(0x0C);
            apdu.extend_from_slice(&object_id.to_be_bytes());

            // Opening tag 1 for list of results
            apdu.push(0x1E);

            // Context tag 1 opening - List of property references
            if pos >= data.len() || data[pos] != 0x1E {
                debug!("RPM: Expected opening tag 1, got 0x{:02X}", if pos < data.len() { data[pos] } else { 0 });
                break;
            }
            pos += 1;

            // Parse property references until closing tag
            while pos < data.len() && data[pos] != 0x1F {
                // Property identifier (context tag 0)
                if pos >= data.len() {
                    break;
                }

                let tag_byte = data[pos];
                let tag_num = (tag_byte >> 4) & 0x0F;
                let is_context = (tag_byte & 0x08) != 0;
                let length = (tag_byte & 0x07) as usize;

                if !is_context || tag_num != 0 {
                    debug!("RPM: Expected property ID context tag 0, got tag={}, context={}", tag_num, is_context);
                    pos += 1;
                    continue;
                }
                pos += 1;

                if pos + length > data.len() {
                    break;
                }

                let property_id = match length {
                    1 => data[pos] as u32,
                    2 => ((data[pos] as u32) << 8) | (data[pos + 1] as u32),
                    3 => ((data[pos] as u32) << 16) | ((data[pos + 1] as u32) << 8) | (data[pos + 2] as u32),
                    4 => u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]),
                    _ => {
                        pos += length;
                        continue;
                    }
                };
                pos += length;

                // Skip optional array index (context tag 1)
                if pos < data.len() {
                    let next_tag = data[pos];
                    if (next_tag & 0xF8) == 0x18 { // Context tag 1
                        let len = (next_tag & 0x07) as usize;
                        pos += 1 + len;
                    }
                }

                debug!("RPM: Property {} (0x{:02X})", property_id, property_id);

                // Add property identifier to response (context tag 2)
                if property_id <= 0xFF {
                    apdu.push(0x29);
                    apdu.push(property_id as u8);
                } else {
                    apdu.push(0x2A);
                    apdu.extend_from_slice(&(property_id as u16).to_be_bytes());
                }

                // Get property value
                if let Some(value) = self.get_property_value(object_id, property_id) {
                    // Opening tag 4 for property value
                    apdu.push(0x4E);
                    apdu.extend_from_slice(&value);
                    // Closing tag 4
                    apdu.push(0x4F);
                } else {
                    // Property access error (context tag 5)
                    apdu.push(0x5E); // Opening tag 5
                    apdu.push(0x91); // Error class enumerated
                    apdu.push(ERROR_CLASS_PROPERTY as u8);
                    apdu.push(0x91); // Error code enumerated
                    apdu.push(ERROR_CODE_UNKNOWN_PROPERTY as u8);
                    apdu.push(0x5F); // Closing tag 5
                }
            }

            // Skip closing tag 1F
            if pos < data.len() && data[pos] == 0x1F {
                pos += 1;
            }

            // Closing tag 1 for list of results
            apdu.push(0x1F);
        }

        debug!("RPM: Response size {} bytes", apdu.len());
        Some((apdu, false))
    }

    /// Get encoded property value (without APDU wrapper)
    fn get_property_value(&self, object_id: u32, property_id: u32) -> Option<Vec<u8>> {
        match property_id {
            PROP_OBJECT_IDENTIFIER => {
                let mut v = vec![0xC4];
                v.extend_from_slice(&object_id.to_be_bytes());
                Some(v)
            }
            PROP_OBJECT_NAME => Some(self.encode_character_string(&self.device_name)),
            PROP_OBJECT_TYPE => Some(vec![0x91, OBJECT_TYPE_DEVICE as u8]),
            PROP_SYSTEM_STATUS => Some(vec![0x91, STATUS_OPERATIONAL as u8]),
            PROP_VENDOR_NAME => Some(self.encode_character_string(&self.vendor_name)),
            PROP_VENDOR_IDENTIFIER => Some(self.encode_unsigned(VENDOR_ID)),
            PROP_MODEL_NAME => Some(self.encode_character_string(&self.model_name)),
            PROP_FIRMWARE_REVISION => Some(self.encode_character_string(&self.firmware_revision)),
            PROP_APPLICATION_SOFTWARE_VERSION => Some(self.encode_character_string(&self.application_version)),
            PROP_PROTOCOL_VERSION => Some(vec![0x21, 1]),
            PROP_PROTOCOL_REVISION => Some(vec![0x21, 14]),
            PROP_PROTOCOL_SERVICES_SUPPORTED => {
                let mut bits = [0u8; 6];
                bits[1] |= 0x08; // ReadProperty (bit 12)
                bits[1] |= 0x02; // ReadPropertyMultiple (bit 14)
                bits[3] |= 0x20; // I-Am (bit 26)
                bits[4] |= 0x40; // Who-Is (bit 33)
                let mut v = vec![0x82, 0x07, 0x00];
                v.extend_from_slice(&bits);
                Some(v)
            }
            PROP_PROTOCOL_OBJECT_TYPES_SUPPORTED => {
                let mut bits = [0u8; 7];
                bits[1] |= 0x80; // Device (bit 8)
                let mut v = vec![0x82, 0x08, 0x00];
                v.extend_from_slice(&bits);
                Some(v)
            }
            PROP_MAX_APDU_LENGTH_ACCEPTED => Some(self.encode_unsigned(MAX_APDU_LENGTH)),
            PROP_SEGMENTATION_SUPPORTED => Some(vec![0x91, SEGMENTATION_NOT_SUPPORTED as u8]),
            PROP_APDU_TIMEOUT => Some(self.encode_unsigned(3000)),
            PROP_NUMBER_OF_APDU_RETRIES => Some(vec![0x21, 3]),
            PROP_DATABASE_REVISION => Some(vec![0x21, 1]),
            PROP_OBJECT_LIST => {
                let mut v = vec![0xC4];
                v.extend_from_slice(&object_id.to_be_bytes());
                Some(v)
            }
            PROP_DESCRIPTION => Some(self.encode_character_string("BACnet MS/TP to IP Gateway")),
            PROP_LOCATION => Some(self.encode_character_string("")),
            PROP_MAX_INFO_FRAMES => Some(vec![0x21, self.max_info_frames]),
            PROP_MAX_MASTER => Some(vec![0x21, self.max_master]),
            PROP_DEVICE_ADDRESS_BINDING => Some(vec![]), // Empty list
            _ => None,
        }
    }

    /// Encode a character string with application tag
    fn encode_character_string(&self, s: &str) -> Vec<u8> {
        let bytes = s.as_bytes();
        let len = bytes.len() + 1; // +1 for encoding byte

        let mut result = Vec::with_capacity(len + 3);

        // Application tag 7 (Character String)
        if len < 5 {
            result.push(0x70 | (len as u8));
        } else if len < 254 {
            result.push(0x75);
            result.push(len as u8);
        } else {
            result.push(0x75);
            result.push(254);
            result.extend_from_slice(&(len as u16).to_be_bytes());
        }

        // Character encoding (0 = UTF-8/ANSI X3.4)
        result.push(0);
        result.extend_from_slice(bytes);

        result
    }

    /// Encode an unsigned integer with application tag
    fn encode_unsigned(&self, value: u32) -> Vec<u8> {
        if value <= 0xFF {
            vec![0x21, value as u8]
        } else if value <= 0xFFFF {
            let mut v = vec![0x22];
            v.extend_from_slice(&(value as u16).to_be_bytes());
            v
        } else if value <= 0xFFFFFF {
            let bytes = value.to_be_bytes();
            vec![0x23, bytes[1], bytes[2], bytes[3]]
        } else {
            let mut v = vec![0x24];
            v.extend_from_slice(&value.to_be_bytes());
            v
        }
    }

    /// Build a Who-Is request APDU (broadcast to all devices)
    pub fn build_who_is() -> Vec<u8> {
        vec![
            APDU_UNCONFIRMED_REQUEST,  // PDU type
            SERVICE_WHO_IS,             // Service choice
            // No parameters = request all devices
        ]
    }

    /// Build a Who-Is request APDU with range
    pub fn build_who_is_range(low_limit: u32, high_limit: u32) -> Vec<u8> {
        let mut apdu = vec![
            APDU_UNCONFIRMED_REQUEST,  // PDU type
            SERVICE_WHO_IS,             // Service choice
        ];

        // Context tag 0 - Low Limit
        apdu.extend_from_slice(&encode_context_unsigned(0, low_limit));

        // Context tag 1 - High Limit
        apdu.extend_from_slice(&encode_context_unsigned(1, high_limit));

        apdu
    }
}

/// Encode context-tagged unsigned integer
fn encode_context_unsigned(tag: u8, value: u32) -> Vec<u8> {
    let mut result = Vec::new();

    if value <= 0xFF {
        result.push((tag << 4) | 1);  // Context tag with length 1
        result.push(value as u8);
    } else if value <= 0xFFFF {
        result.push((tag << 4) | 2);  // Context tag with length 2
        result.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= 0xFFFFFF {
        result.push((tag << 4) | 3);  // Context tag with length 3
        let bytes = value.to_be_bytes();
        result.extend_from_slice(&bytes[1..4]);
    } else {
        result.push((tag << 4) | 4);  // Context tag with length 4
        result.extend_from_slice(&value.to_be_bytes());
    }

    result
}

/// Discovered device info from I-Am response
#[derive(Debug, Clone, Default)]
pub struct DiscoveredDevice {
    pub device_instance: u32,
    pub mac_address: u8,
    pub max_apdu_length: u32,
    pub segmentation: u8,
    pub vendor_id: u16,
}

impl DiscoveredDevice {
    /// Parse an I-Am APDU and extract device info
    pub fn from_i_am(apdu: &[u8], mac_address: u8) -> Option<Self> {
        // Minimum I-Am: PDU type (1) + Service (1) + Object ID (5) + Max APDU (3) + Segmentation (2) + Vendor (3) = 15 bytes
        if apdu.len() < 12 {
            return None;
        }

        // Check PDU type and service
        if apdu[0] != APDU_UNCONFIRMED_REQUEST || apdu[1] != SERVICE_I_AM {
            return None;
        }

        let mut pos = 2;

        // Parse Device Object Identifier (Application Tag 12)
        if pos >= apdu.len() || (apdu[pos] & 0xF0) != 0xC0 {
            return None;
        }
        let tag_len = (apdu[pos] & 0x07) as usize;
        pos += 1;

        if pos + tag_len > apdu.len() || tag_len != 4 {
            return None;
        }

        let object_id = u32::from_be_bytes([apdu[pos], apdu[pos + 1], apdu[pos + 2], apdu[pos + 3]]);
        let object_type = (object_id >> 22) as u16;
        let device_instance = object_id & 0x3FFFFF;
        pos += tag_len;

        // Verify it's a device object
        if object_type != OBJECT_TYPE_DEVICE {
            return None;
        }

        // Parse Max APDU Length Accepted (Application Tag 2 - Unsigned)
        let mut max_apdu_length = 480u32;
        if pos < apdu.len() && (apdu[pos] & 0xF0) == 0x20 {
            let len = (apdu[pos] & 0x07) as usize;
            pos += 1;
            if pos + len <= apdu.len() {
                max_apdu_length = match len {
                    1 => apdu[pos] as u32,
                    2 => u16::from_be_bytes([apdu[pos], apdu[pos + 1]]) as u32,
                    _ => 480,
                };
                pos += len;
            }
        }

        // Parse Segmentation Supported (Application Tag 9 - Enumerated)
        let mut segmentation = 3u8;
        if pos < apdu.len() && (apdu[pos] & 0xF0) == 0x90 {
            let len = (apdu[pos] & 0x07) as usize;
            pos += 1;
            if pos + len <= apdu.len() && len >= 1 {
                segmentation = apdu[pos];
                pos += len;
            }
        }

        // Parse Vendor ID (Application Tag 2 - Unsigned)
        let mut vendor_id = 0u16;
        if pos < apdu.len() && (apdu[pos] & 0xF0) == 0x20 {
            let len = (apdu[pos] & 0x07) as usize;
            pos += 1;
            if pos + len <= apdu.len() {
                vendor_id = match len {
                    1 => apdu[pos] as u16,
                    2 => u16::from_be_bytes([apdu[pos], apdu[pos + 1]]),
                    _ => 0,
                };
            }
        }

        Some(DiscoveredDevice {
            device_instance,
            mac_address,
            max_apdu_length,
            segmentation,
            vendor_id,
        })
    }
}

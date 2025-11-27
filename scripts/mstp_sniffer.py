#!/usr/bin/env python3
"""
BACnet MS/TP Protocol Sniffer

Captures and decodes BACnet MS/TP frames from a serial port.
Displays frame type, source/destination addresses, and NPDU/APDU content.

Usage:
    python3 mstp_sniffer.py [port] [baud]

Arguments:
    port - Serial port (default: /dev/ttyUSB0)
    baud - Baud rate (default: 38400)

Example:
    python3 mstp_sniffer.py /dev/ttyUSB0 38400
"""

import sys
import serial
import time
from datetime import datetime
from collections import deque

# MS/TP Frame Types (from ASHRAE 135 Clause 9)
FRAME_TYPES = {
    0x00: "Token",
    0x01: "Poll-For-Master",
    0x02: "Reply-To-Poll-For-Master",
    0x03: "Test_Request",
    0x04: "Test_Response",
    0x05: "BACnet-Data-Expecting-Reply",
    0x06: "BACnet-Data-Not-Expecting-Reply",
    0x07: "Reply-Postponed",
}

# BACnet Service Choices (Confirmed)
CONFIRMED_SERVICES = {
    0x00: "AcknowledgeAlarm",
    0x01: "COVNotification",
    0x02: "EventNotification",
    0x03: "GetAlarmSummary",
    0x04: "GetEnrollmentSummary",
    0x05: "SubscribeCOV",
    0x06: "AtomicReadFile",
    0x07: "AtomicWriteFile",
    0x08: "AddListElement",
    0x09: "RemoveListElement",
    0x0A: "CreateObject",
    0x0B: "DeleteObject",
    0x0C: "ReadProperty",
    0x0D: "ReadPropertyConditional",
    0x0E: "ReadPropertyMultiple",
    0x0F: "WriteProperty",
    0x10: "WritePropertyMultiple",
    0x11: "DeviceCommunicationControl",
    0x12: "ConfirmedPrivateTransfer",
    0x13: "ConfirmedTextMessage",
    0x14: "ReinitializeDevice",
    0x15: "VTOpen",
    0x16: "VTClose",
    0x17: "VTData",
    0x18: "Authenticate",
    0x19: "RequestKey",
    0x1A: "ReadRange",
    0x1B: "LifeSafetyOperation",
    0x1C: "SubscribeCOVProperty",
    0x1D: "GetEventInformation",
}

# BACnet Service Choices (Unconfirmed)
UNCONFIRMED_SERVICES = {
    0x00: "I-Am",
    0x01: "I-Have",
    0x02: "COV-Notification",
    0x03: "Event-Notification",
    0x04: "Private-Transfer",
    0x05: "Text-Message",
    0x06: "Time-Synchronization",
    0x07: "Who-Has",
    0x08: "Who-Is",
    0x09: "UTC-Time-Synchronization",
    0x0A: "WriteGroup",
}

# Colors for terminal output
class Colors:
    RESET = '\033[0m'
    RED = '\033[91m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    MAGENTA = '\033[95m'
    CYAN = '\033[96m'
    WHITE = '\033[97m'
    GRAY = '\033[90m'

def crc8(data):
    """Calculate MS/TP header CRC (CRC-8)"""
    crc = 0xFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8C
            else:
                crc >>= 1
    return crc ^ 0xFF

def crc16(data):
    """Calculate MS/TP data CRC (CRC-16)"""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408  # CRC-CCITT (NOT 0xA001 which is MODBUS)
            else:
                crc >>= 1
    return crc ^ 0xFFFF

def decode_npdu(data):
    """Decode BACnet NPDU header"""
    if len(data) < 2:
        return None, "Too short for NPDU"

    version = data[0]
    control = data[1]

    if version != 0x01:
        return None, f"Unknown NPDU version: {version}"

    info = {
        'version': version,
        'control': control,
        'network_msg': bool(control & 0x80),
        'dnet_present': bool(control & 0x20),
        'snet_present': bool(control & 0x08),
        'expecting_reply': bool(control & 0x04),
        'priority': control & 0x03,
    }

    offset = 2

    # Destination network
    if info['dnet_present'] and len(data) > offset + 3:
        info['dnet'] = (data[offset] << 8) | data[offset + 1]
        dlen = data[offset + 2]
        info['dlen'] = dlen
        offset += 3
        if dlen > 0 and len(data) > offset + dlen:
            info['dadr'] = data[offset:offset + dlen]
            offset += dlen

    # Source network
    if info['snet_present'] and len(data) > offset + 3:
        info['snet'] = (data[offset] << 8) | data[offset + 1]
        slen = data[offset + 2]
        info['slen'] = slen
        offset += 3
        if slen > 0 and len(data) > offset + slen:
            info['sadr'] = data[offset:offset + slen]
            offset += slen

    # Hop count
    if info['dnet_present'] and len(data) > offset:
        info['hop_count'] = data[offset]
        offset += 1

    info['apdu_offset'] = offset
    return info, None

def decode_apdu(data):
    """Decode BACnet APDU"""
    if len(data) < 1:
        return "Empty APDU"

    pdu_type = (data[0] >> 4) & 0x0F

    if pdu_type == 0x00:  # Confirmed Request
        if len(data) < 4:
            return "Confirmed Request (truncated)"
        service = data[3]
        service_name = CONFIRMED_SERVICES.get(service, f"Unknown({service})")
        return f"Confirmed-REQ: {service_name}"

    elif pdu_type == 0x01:  # Unconfirmed Request
        if len(data) < 2:
            return "Unconfirmed Request (truncated)"
        service = data[1]
        service_name = UNCONFIRMED_SERVICES.get(service, f"Unknown({service})")
        return f"Unconfirmed-REQ: {service_name}"

    elif pdu_type == 0x02:  # Simple ACK
        if len(data) < 3:
            return "Simple-ACK (truncated)"
        service = data[2]
        service_name = CONFIRMED_SERVICES.get(service, f"Unknown({service})")
        return f"Simple-ACK: {service_name}"

    elif pdu_type == 0x03:  # Complex ACK
        if len(data) < 3:
            return "Complex-ACK (truncated)"
        service = data[2]
        service_name = CONFIRMED_SERVICES.get(service, f"Unknown({service})")
        return f"Complex-ACK: {service_name}"

    elif pdu_type == 0x04:  # Segment ACK
        return "Segment-ACK"

    elif pdu_type == 0x05:  # Error
        if len(data) < 3:
            return "Error (truncated)"
        service = data[2]
        service_name = CONFIRMED_SERVICES.get(service, f"Unknown({service})")
        return f"Error: {service_name}"

    elif pdu_type == 0x06:  # Reject
        return "Reject"

    elif pdu_type == 0x07:  # Abort
        return "Abort"

    return f"Unknown PDU type: {pdu_type}"

def format_frame(frame_num, frame_type, dst, src, data_len, data, timestamp):
    """Format a decoded MS/TP frame for display"""
    frame_name = FRAME_TYPES.get(frame_type, f"Unknown({frame_type:02X})")

    # Color based on frame type
    if frame_type == 0x00:  # Token
        color = Colors.GRAY
    elif frame_type in (0x01, 0x02):  # Poll
        color = Colors.CYAN
    elif frame_type in (0x05, 0x06):  # BACnet Data
        color = Colors.GREEN
    else:
        color = Colors.YELLOW

    # Basic frame info
    output = f"{color}[{timestamp}] #{frame_num:04d} {frame_name:30s} "
    output += f"Src:{src:3d} -> Dst:{dst:3d}"

    if data_len > 0 and data:
        output += f" Len:{data_len}"

        # Decode NPDU/APDU for BACnet data frames
        if frame_type in (0x05, 0x06) and len(data) >= 2:
            npdu_info, err = decode_npdu(data)
            if npdu_info and not err:
                apdu_offset = npdu_info['apdu_offset']
                if len(data) > apdu_offset:
                    apdu_str = decode_apdu(data[apdu_offset:])
                    output += f" | {Colors.WHITE}{apdu_str}"

                # Show routing info if present
                if npdu_info.get('dnet'):
                    output += f" | DNet:{npdu_info['dnet']}"
                if npdu_info.get('snet'):
                    output += f" | SNet:{npdu_info['snet']}"

    output += Colors.RESET
    return output

def print_hex_dump(data, prefix=""):
    """Print hex dump of data"""
    hex_str = ' '.join(f'{b:02X}' for b in data)
    ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in data)
    print(f"{Colors.GRAY}{prefix}HEX: {hex_str}{Colors.RESET}")

class MstpSniffer:
    """MS/TP frame parser state machine"""

    PREAMBLE1 = 0x55
    PREAMBLE2 = 0xFF

    def __init__(self, verbose=False, show_hex=False):
        self.verbose = verbose
        self.show_hex = show_hex
        self.buffer = bytearray()
        self.frame_count = 0
        self.state = 'IDLE'
        self.frame_type = 0
        self.dst_addr = 0
        self.src_addr = 0
        self.data_len = 0
        self.header_crc = 0
        self.data = bytearray()
        self.data_crc = 0

    def process_byte(self, byte):
        """Process a single byte through the state machine"""
        self.buffer.append(byte)

        if self.state == 'IDLE':
            if byte == self.PREAMBLE1:
                self.state = 'PREAMBLE'
                self.buffer = bytearray([byte])
            else:
                self.buffer.clear()

        elif self.state == 'PREAMBLE':
            if byte == self.PREAMBLE2:
                self.state = 'HEADER'
            elif byte == self.PREAMBLE1:
                pass  # Stay in PREAMBLE
            else:
                self.state = 'IDLE'
                self.buffer.clear()

        elif self.state == 'HEADER':
            if len(self.buffer) == 8:  # Full header received
                self.frame_type = self.buffer[2]
                self.dst_addr = self.buffer[3]
                self.src_addr = self.buffer[4]
                self.data_len = (self.buffer[5] << 8) | self.buffer[6]
                self.header_crc = self.buffer[7]

                # Verify header CRC
                calc_crc = crc8(self.buffer[2:7])
                if calc_crc != self.header_crc:
                    if self.verbose:
                        print(f"{Colors.RED}[CRC ERROR] Header CRC mismatch: calc={calc_crc:02X} recv={self.header_crc:02X}{Colors.RESET}")
                    self.state = 'IDLE'
                    self.buffer.clear()
                    return None

                if self.data_len > 0:
                    self.state = 'DATA'
                    self.data = bytearray()
                else:
                    # No data, frame complete
                    return self._complete_frame()

        elif self.state == 'DATA':
            self.data.append(byte)
            if len(self.data) == self.data_len + 2:  # Data + 2-byte CRC
                self.data_crc = (self.data[-1] << 8) | self.data[-2]
                self.data = self.data[:-2]

                # Verify data CRC
                calc_crc = crc16(self.data)
                if calc_crc != self.data_crc:
                    if self.verbose:
                        print(f"{Colors.RED}[CRC ERROR] Data CRC mismatch{Colors.RESET}")

                return self._complete_frame()

        return None

    def _complete_frame(self):
        """Complete frame processing and return formatted output"""
        self.frame_count += 1
        timestamp = datetime.now().strftime('%H:%M:%S.%f')[:-3]

        output = format_frame(
            self.frame_count,
            self.frame_type,
            self.dst_addr,
            self.src_addr,
            self.data_len,
            self.data,
            timestamp
        )

        hex_data = bytes(self.data) if self.data else None

        # Reset state
        self.state = 'IDLE'
        self.buffer.clear()
        self.data = bytearray()

        return output, hex_data

def main():
    port = sys.argv[1] if len(sys.argv) > 1 else '/dev/ttyUSB0'
    baud = int(sys.argv[2]) if len(sys.argv) > 2 else 38400
    verbose = '-v' in sys.argv or '--verbose' in sys.argv
    show_hex = '-x' in sys.argv or '--hex' in sys.argv

    print(f"{Colors.CYAN}BACnet MS/TP Sniffer{Colors.RESET}")
    print(f"Port: {port} @ {baud} baud")
    print("=" * 70)
    print("Options: -v/--verbose for debug, -x/--hex for hex dumps")
    print("Press Ctrl+C to exit")
    print("=" * 70)
    print()

    sniffer = MstpSniffer(verbose=verbose, show_hex=show_hex)

    try:
        ser = serial.Serial(
            port=port,
            baudrate=baud,
            timeout=0.1,
            bytesize=serial.EIGHTBITS,
            parity=serial.PARITY_NONE,
            stopbits=serial.STOPBITS_ONE
        )

        print(f"{Colors.GREEN}[CONNECTED]{Colors.RESET} {port}")
        print()

        while True:
            if ser.in_waiting > 0:
                data = ser.read(ser.in_waiting)
                for byte in data:
                    result = sniffer.process_byte(byte)
                    if result:
                        output, hex_data = result
                        print(output)
                        if show_hex and hex_data:
                            print_hex_dump(hex_data, "  ")
            else:
                time.sleep(0.001)

    except serial.SerialException as e:
        print(f"{Colors.RED}[ERROR]{Colors.RESET} Could not open {port}: {e}")
        print("\nTroubleshooting:")
        print("  1. Check if device is connected: ls /dev/ttyUSB*")
        print("  2. Check permissions: sudo chmod 666 /dev/ttyUSB0")
        print("  3. For WSL2, use usbipd to attach USB device")
        sys.exit(1)
    except KeyboardInterrupt:
        print(f"\n{Colors.YELLOW}[STOPPED]{Colors.RESET} Captured {sniffer.frame_count} frames")
    finally:
        try:
            ser.close()
        except:
            pass

if __name__ == '__main__':
    main()

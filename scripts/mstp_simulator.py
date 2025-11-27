#!/usr/bin/env python3
"""
MS/TP Simulated Device for testing the BACnet gateway.

This script simulates an MS/TP master station that:
1. Responds to Poll-For-Master frames
2. Accepts token and passes it back
3. Can send/receive BACnet data frames

Usage:
    python3 mstp_simulator.py <serial_port> [baud] [mac_address]

Example:
    python3 mstp_simulator.py /dev/ttyACM1 38400 4
"""

import sys
import serial
import time
import struct
from datetime import datetime

# MS/TP Frame Types per ASHRAE 135 Clause 9
FRAME_TOKEN = 0x00
FRAME_POLL_FOR_MASTER = 0x01
FRAME_REPLY_TO_POLL = 0x02
FRAME_TEST_REQUEST = 0x03
FRAME_TEST_RESPONSE = 0x04
FRAME_BACNET_DATA_EXPECTING_REPLY = 0x05
FRAME_BACNET_DATA_NOT_EXPECTING_REPLY = 0x06
FRAME_REPLY_POSTPONED = 0x07

FRAME_NAMES = {
    0x00: "Token",
    0x01: "Poll-For-Master",
    0x02: "Reply-To-Poll-For-Master",
    0x03: "Test_Request",
    0x04: "Test_Response",
    0x05: "BACnet-Data-Expecting-Reply",
    0x06: "BACnet-Data-Not-Expecting-Reply",
    0x07: "Reply-Postponed",
}

# Timing parameters (in seconds)
T_REPLY_DELAY = 0.000250  # 250us max reply delay
T_SLOT = 0.010  # 10ms slot time
T_USAGE_TIMEOUT = 0.050  # 50ms

# ANSI colors
class C:
    RESET = '\033[0m'
    RED = '\033[91m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    MAGENTA = '\033[95m'
    CYAN = '\033[96m'


def crc8_header(data):
    """Calculate MS/TP header CRC (CRC-8) per ASHRAE 135 Annex G.1.

    Uses polynomial X^8 + X^7 + 1.
    """
    crc = 0xFF

    for byte in data:
        # XOR C7..C0 with D7..D0
        temp = (crc ^ byte) & 0xFF

        # Exclusive OR the terms in the table (top down)
        # This implements the polynomial X^8 + X^7 + 1
        temp = (temp
            ^ (temp << 1)
            ^ (temp << 2)
            ^ (temp << 3)
            ^ (temp << 4)
            ^ (temp << 5)
            ^ (temp << 6)
            ^ (temp << 7)) & 0xFFFF

        # Combine bits shifted out left hand end
        crc = ((temp & 0xFE) ^ ((temp >> 8) & 1)) & 0xFF

    return (~crc) & 0xFF


def crc16_data(data):
    """Calculate MS/TP data CRC (CRC-16) per Annex G.2."""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0x8408  # CRC-CCITT (NOT 0xA001 which is MODBUS)
            else:
                crc >>= 1
    return (~crc) & 0xFFFF


def build_frame(frame_type, dest, src, data=None):
    """Build an MS/TP frame with correct preamble and CRCs."""
    preamble = bytes([0x55, 0xFF])

    if data is None or len(data) == 0:
        header = bytes([frame_type, dest, src, 0x00, 0x00])
        header_crc = crc8_header(header)
        return preamble + header + bytes([header_crc])
    else:
        data_len = len(data)
        header = bytes([frame_type, dest, src, (data_len >> 8) & 0xFF, data_len & 0xFF])
        header_crc = crc8_header(header)
        data_crc = crc16_data(data)
        # Data CRC is sent LSB first
        return preamble + header + bytes([header_crc]) + bytes(data) + struct.pack('<H', data_crc)


def timestamp():
    return datetime.now().strftime('%H:%M:%S.%f')[:-3]


class MstpSimulator:
    """Simulates an MS/TP master station."""

    def __init__(self, ser, mac_address, max_master=127):
        self.ser = ser
        self.mac = mac_address
        self.max_master = max_master

        # State
        self.rx_buffer = bytearray()
        self.state = "IDLE"
        self.token_count = 0
        self.frame_count = 0

        # Master discovery - track known masters on the bus
        self.known_masters = set()  # Set of discovered MAC addresses
        self.next_station = None    # Will be computed from known_masters

        # Statistics
        self.tokens_received = 0
        self.polls_received = 0
        self.data_frames_received = 0
        self.frames_sent = 0

    def log(self, msg, color=C.RESET):
        print(f"{color}[{timestamp()}] [{self.state:6s}] {msg}{C.RESET}")

    def discover_master(self, mac):
        """Record a discovered master station."""
        if mac != self.mac and mac <= self.max_master and mac not in self.known_masters:
            self.known_masters.add(mac)
            self.log(f"*** DISCOVERED MASTER at MAC {mac} *** (known: {sorted(self.known_masters)})", C.YELLOW)
            self._update_next_station()

    def _update_next_station(self):
        """Compute next_station as the next known master after our MAC."""
        if not self.known_masters:
            self.next_station = None
            return

        # Find the next master after our MAC address (wrapping around)
        masters = sorted(self.known_masters)
        for m in masters:
            if m > self.mac:
                self.next_station = m
                self.log(f"Updated next_station to {self.next_station}", C.GREEN)
                return
        # Wrap around to the lowest master
        self.next_station = masters[0]
        self.log(f"Updated next_station to {self.next_station} (wrapped)", C.GREEN)

    def send_frame(self, frame_type, dest, data=None):
        """Send an MS/TP frame."""
        frame = build_frame(frame_type, dest, self.mac, data)
        self.ser.write(frame)
        self.ser.flush()
        self.frames_sent += 1
        self.log(f"TX: {FRAME_NAMES.get(frame_type, 'Unknown')} -> {dest} [{frame.hex()}]", C.GREEN)

    def process_rx(self):
        """Read and buffer incoming bytes."""
        if self.ser.in_waiting > 0:
            data = self.ser.read(self.ser.in_waiting)
            self.rx_buffer.extend(data)
            return True
        return False

    def parse_frame(self):
        """Try to parse a complete frame from the buffer."""
        # Need at least preamble + header
        if len(self.rx_buffer) < 8:
            return None

        # Find preamble
        preamble_pos = -1
        for i in range(len(self.rx_buffer) - 1):
            if self.rx_buffer[i] == 0x55 and self.rx_buffer[i+1] == 0xFF:
                preamble_pos = i
                break

        if preamble_pos < 0:
            # No valid preamble found, keep last byte
            if len(self.rx_buffer) > 1:
                self.rx_buffer = self.rx_buffer[-1:]
            return None

        # Discard bytes before preamble
        if preamble_pos > 0:
            self.rx_buffer = self.rx_buffer[preamble_pos:]

        # Check we have full header
        if len(self.rx_buffer) < 8:
            return None

        # Parse header
        frame_type = self.rx_buffer[2]
        dest = self.rx_buffer[3]
        src = self.rx_buffer[4]
        data_len = (self.rx_buffer[5] << 8) | self.rx_buffer[6]
        header_crc = self.rx_buffer[7]

        # Validate header CRC
        calc_crc = crc8_header(self.rx_buffer[2:7])
        if calc_crc != header_crc:
            self.log(f"Header CRC error: calc=0x{calc_crc:02X} recv=0x{header_crc:02X}", C.RED)
            self.rx_buffer = self.rx_buffer[2:]  # Skip preamble, try again
            return None

        # Calculate total frame size
        if data_len > 0:
            frame_size = 8 + data_len + 2  # header + data + data_crc
        else:
            frame_size = 8  # header only

        # Wait for complete frame
        if len(self.rx_buffer) < frame_size:
            return None

        # Extract data if present
        data = None
        if data_len > 0:
            data = bytes(self.rx_buffer[8:8+data_len])
            # Validate data CRC
            data_crc_recv = self.rx_buffer[8+data_len] | (self.rx_buffer[8+data_len+1] << 8)
            data_crc_calc = crc16_data(data)
            if data_crc_recv != data_crc_calc:
                self.log(f"Data CRC error", C.RED)
                self.rx_buffer = self.rx_buffer[2:]
                return None

        # Remove frame from buffer
        self.rx_buffer = self.rx_buffer[frame_size:]

        return (frame_type, dest, src, data)

    def handle_frame(self, frame_type, dest, src, data):
        """Handle a received frame."""
        frame_name = FRAME_NAMES.get(frame_type, f"Unknown(0x{frame_type:02X})")

        # Only log non-Token frames or tokens for us
        if frame_type != FRAME_TOKEN or dest == self.mac:
            data_info = f" data={data.hex()}" if data else ""
            self.log(f"RX: {frame_name} {src} -> {dest}{data_info}", C.CYAN)

        # Discover masters from traffic:
        # - Any frame source is a master (they're actively participating)
        # - Token destinations are masters (token recipients must be masters)
        # - Reply-To-Poll sources are masters
        self.discover_master(src)
        if frame_type == FRAME_TOKEN:
            self.discover_master(dest)
        elif frame_type == FRAME_REPLY_TO_POLL:
            self.discover_master(src)

        # Check if frame is for us
        if dest != self.mac and dest != 255:  # 255 = broadcast
            return

        if frame_type == FRAME_TOKEN:
            # We received the token!
            self.tokens_received += 1
            self.log(f"*** GOT TOKEN from {src} *** (count={self.tokens_received})", C.YELLOW)
            self.state = "USE_TOKEN"
            self.handle_use_token()

        elif frame_type == FRAME_POLL_FOR_MASTER:
            # Reply to poll
            self.polls_received += 1
            self.log(f"Replying to Poll-For-Master from {src}", C.MAGENTA)
            # Send reply within T_REPLY_DELAY
            time.sleep(T_REPLY_DELAY)
            self.send_frame(FRAME_REPLY_TO_POLL, src)

        elif frame_type == FRAME_BACNET_DATA_EXPECTING_REPLY:
            self.data_frames_received += 1
            self.log(f"BACnet data from {src}: {data.hex() if data else 'empty'}", C.BLUE)
            # TODO: Generate response through gateway

        elif frame_type == FRAME_BACNET_DATA_NOT_EXPECTING_REPLY:
            self.data_frames_received += 1
            self.log(f"BACnet data from {src}: {data.hex() if data else 'empty'}", C.BLUE)

        elif frame_type == FRAME_TEST_REQUEST:
            # Echo back test response
            self.log(f"Test request from {src}, sending response", C.MAGENTA)
            time.sleep(T_REPLY_DELAY)
            self.send_frame(FRAME_TEST_RESPONSE, src, data)

    def handle_use_token(self):
        """Handle having the token - send any queued data, then pass token."""
        self.token_count += 1
        self.frame_count = 0

        # For now, just pass the token immediately
        # In a real implementation, we'd send any queued BACnet data here

        if self.next_station is not None:
            self.log(f"Passing token to {self.next_station} (known masters: {sorted(self.known_masters)})", C.GREEN)
            self.send_frame(FRAME_TOKEN, self.next_station)
        else:
            # No known masters yet - poll for one starting from MAC+1
            poll_target = (self.mac + 1) % (self.max_master + 1)
            self.log(f"No known masters, polling {poll_target}", C.YELLOW)
            self.send_frame(FRAME_POLL_FOR_MASTER, poll_target)
            # Wait for response
            time.sleep(T_USAGE_TIMEOUT)
            # If still no response, just go idle - other masters will pass us token again
        self.state = "IDLE"

    def run(self):
        """Main loop."""
        self.log("Starting MS/TP simulator (with master discovery)", C.YELLOW)
        self.log(f"MAC Address: {self.mac}", C.YELLOW)
        self.log("Next station will be learned from traffic...", C.YELLOW)
        self.log("=" * 60, C.YELLOW)

        try:
            while True:
                # Process incoming data
                self.process_rx()

                # Try to parse frames
                frame = self.parse_frame()
                if frame:
                    self.handle_frame(*frame)

                # Small delay to prevent CPU spinning
                time.sleep(0.001)

        except KeyboardInterrupt:
            print(f"\n{C.YELLOW}--- Statistics ---{C.RESET}")
            print(f"Tokens received: {self.tokens_received}")
            print(f"Polls received: {self.polls_received}")
            print(f"Data frames received: {self.data_frames_received}")
            print(f"Frames sent: {self.frames_sent}")


def main():
    if len(sys.argv) < 2:
        print("Usage: mstp_simulator.py <serial_port> [baud] [mac_address]")
        print("Example: mstp_simulator.py /dev/ttyACM1 38400 4")
        sys.exit(1)

    port = sys.argv[1]
    baud = int(sys.argv[2]) if len(sys.argv) > 2 else 38400
    mac = int(sys.argv[3]) if len(sys.argv) > 3 else 4

    print(f"{C.CYAN}MS/TP Device Simulator{C.RESET}")
    print(f"Port: {port}")
    print(f"Baud: {baud}")
    print(f"MAC:  {mac}")
    print("=" * 40)
    print("Press Ctrl+C to exit")
    print()

    try:
        ser = serial.Serial(
            port=port,
            baudrate=baud,
            timeout=0.1,
            bytesize=serial.EIGHTBITS,
            parity=serial.PARITY_NONE,
            stopbits=serial.STOPBITS_ONE
        )
        ser.reset_input_buffer()

        simulator = MstpSimulator(ser, mac)
        simulator.run()

    except serial.SerialException as e:
        print(f"{C.RED}Error: {e}{C.RESET}")
        sys.exit(1)
    finally:
        try:
            ser.close()
        except:
            pass


if __name__ == "__main__":
    main()

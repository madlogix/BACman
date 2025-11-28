# MS/TP Wireshark Live Capture Setup Guide

This document describes the BACnet MS/TP Wireshark live capture setup configured for this project on WSL2.

## Overview

We have `mstpcap` (from the BACnet Stack project) installed as a Wireshark extcap plugin, enabling live capture and decoding of MS/TP frames directly in Wireshark.

## Installation Details

| Component | Location |
|-----------|----------|
| mstpcap binary | `~/.config/wireshark/extcap/mstpcap` |
| Wireshark version | 4.2.2 |
| Source code | Built from https://github.com/bacnet-stack/bacnet-stack |

### How mstpcap Was Built

```bash
# Clone the BACnet Stack repository
git clone --depth 1 https://github.com/bacnet-stack/bacnet-stack.git /tmp/bacnet-stack

# Build mstpcap for Linux
cd /tmp/bacnet-stack/apps
make mstpcap

# Install to user extcap folder
mkdir -p ~/.config/wireshark/extcap
cp /tmp/bacnet-stack/bin/mstpcap ~/.config/wireshark/extcap/
```

## Hardware Setup

### USB Serial Devices

| Windows Port | USB Bus ID | Device | Purpose |
|--------------|------------|--------|---------|
| COM3 | 2-2 | CH9102 (VID:1a86 PID:55d4) | ESP32 Gateway |
| COM10 | 2-4 | CH343 (VID:1a86 PID:55d3) | RS-485 Sniff Adapter |

### Wiring for Sniffing

```
MS/TP Bus (RS-485)
       │
       ├──── Gateway RS-485 HAT (TX/RX active)
       │     - Connected to ESP32 via UART
       │     - Participates in token ring
       │
       └──── Sniff Adapter (RX only)
             - TX should be disconnected or disabled
             - Passive listener on the bus
```

**Important**: The sniff adapter must be separate from the gateway. You cannot sniff and participate in the token ring on the same port.

## Usage Instructions

### 1. Forward USB Devices to WSL

From Windows PowerShell (Administrator):

```powershell
# List available USB devices
usbipd list

# Attach the sniff adapter (COM10) to WSL
usbipd attach --wsl --busid 2-4

# The ESP32 (COM3) should already be attached
usbipd attach --wsl --busid 2-2
```

### 2. Verify Serial Ports in WSL

```bash
# Check available serial ports
ls -la /dev/ttyUSB* /dev/ttyACM*

# Expected output:
# /dev/ttyACM0 - ESP32 Gateway
# /dev/ttyUSB0 - Sniff Adapter (after attaching COM10)
```

### 3. Verify mstpcap Detects Interfaces

```bash
~/.config/wireshark/extcap/mstpcap --extcap-interfaces

# Expected output:
# interface {value=/dev/ttyACM0}{display=MS/TP Capture on /dev/ttyACM0}
# interface {value=/dev/ttyUSB0}{display=MS/TP Capture on /dev/ttyUSB0}
```

### 4. Launch Wireshark

```bash
# Start Wireshark (requires WSLg or X11 forwarding)
wireshark &
```

### 5. Configure and Start Capture

1. In Wireshark, look for **"MS/TP Capture on /dev/ttyUSBx"** in the interface list
2. Double-click or right-click to configure:
   - **Baud Rate**: 38400 (default, match your MS/TP network)
3. Click **Start** to begin capture

### Supported Baud Rates

| Baud Rate | Common Use |
|-----------|------------|
| 9600 | Legacy devices |
| 19200 | Common |
| 38400 | **Default** - Most common |
| 57600 | Less common |
| 76800 | High speed |
| 115200 | Maximum standard rate |

## Command Line Usage

### Direct Capture to File

```bash
# Capture to pcap file (creates timestamped files)
~/.config/wireshark/extcap/mstpcap --extcap-interface /dev/ttyUSB0 --baud 38400

# Output: mstp_YYYYMMDDHHMMSS.cap
```

### Capture to Named Pipe (for Wireshark)

```bash
# Create a named pipe
mkfifo /tmp/mstp_pipe

# Start capture to pipe
~/.config/wireshark/extcap/mstpcap --extcap-interface /dev/ttyUSB0 --baud 38400 --fifo /tmp/mstp_pipe &

# Open pipe in Wireshark
wireshark -k -i /tmp/mstp_pipe
```

### Analyze Existing Capture

```bash
# Scan a capture file for statistics
~/.config/wireshark/extcap/mstpcap --scan mstp_capture.cap
```

## What Gets Captured

The MS/TP dissector in Wireshark decodes:

- **Frame Header**: Preamble, frame type, destination/source addresses, length, CRC
- **Frame Types**:
  - Token (0x00)
  - Poll For Master (0x01)
  - Reply To Poll For Master (0x02)
  - Test Request/Response (0x03, 0x04)
  - BACnet Data Expecting Reply (0x05)
  - BACnet Data Not Expecting Reply (0x06)
  - Reply Postponed (0x07)
- **NPDU**: Network layer routing information
- **APDU**: Application layer (ReadProperty, WriteProperty, Who-Is, I-Am, etc.)

## Troubleshooting

### No interfaces shown in Wireshark

1. Verify mstpcap is in extcap folder:
   ```bash
   ls -la ~/.config/wireshark/extcap/mstpcap
   ```

2. Check mstpcap can find serial ports:
   ```bash
   ~/.config/wireshark/extcap/mstpcap --extcap-interfaces
   ```

3. Ensure serial ports are accessible:
   ```bash
   # Add user to dialout group if needed
   sudo usermod -a -G dialout $USER
   # Log out and back in for group change to take effect
   ```

### Permission denied on serial port

```bash
# Temporary fix
sudo chmod 666 /dev/ttyUSB0

# Permanent fix - add to dialout group
sudo usermod -a -G dialout $USER
```

### USB device not appearing in WSL

```powershell
# In Windows PowerShell (Admin)
# First, share the device
usbipd bind --busid 2-4

# Then attach to WSL
usbipd attach --wsl --busid 2-4
```

### Capture shows no packets

1. Verify baud rate matches MS/TP network
2. Check RS-485 A/B polarity (swap if needed)
3. Ensure sniff adapter is connected to the bus
4. Verify there is actual traffic on the MS/TP network

## References

- [Steve Karg's MS/TP Wireshark Guide](https://steve.kargs.net/bacnet/bacnet-mstp-wireshark-live-capture/)
- [BACnet Stack Project](https://github.com/bacnet-stack/bacnet-stack)
- [Wireshark Extcap Documentation](https://www.wireshark.org/docs/man-pages/extcap.html)
- [usbipd-win for WSL USB](https://github.com/dorssel/usbipd-win)

## Related Project Files

- `MSTP_PROTOCOL_REQUIREMENTS.md` - MS/TP state machine specification
- `mstp-ip-gateway/src/mstp_driver.rs` - Gateway MS/TP implementation
- `bacnet-rs/src/datalink/mstp.rs` - Library MS/TP frame handling

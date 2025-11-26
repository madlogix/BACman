#!/usr/bin/env python3
"""
Serial Monitor for ESP32 devices.

Usage:
    python3 serial_monitor.py [port] [baud]

Arguments:
    port - Serial port (default: /dev/ttyACM0)
    baud - Baud rate (default: 115200)

Example:
    python3 serial_monitor.py /dev/ttyACM0 115200
"""

import sys
import serial
import time
from datetime import datetime

def main():
    # Parse arguments
    port = sys.argv[1] if len(sys.argv) > 1 else '/dev/ttyACM0'
    baud = int(sys.argv[2]) if len(sys.argv) > 2 else 115200

    print(f"Serial Monitor - {port} @ {baud} baud")
    print("=" * 60)
    print("Press Ctrl+C to exit")
    print("=" * 60)
    print()

    try:
        ser = serial.Serial(
            port=port,
            baudrate=baud,
            timeout=0.1,  # Short timeout for responsive reading
            bytesize=serial.EIGHTBITS,
            parity=serial.PARITY_NONE,
            stopbits=serial.STOPBITS_ONE
        )

        print(f"[{datetime.now().strftime('%H:%M:%S')}] Connected to {port}")
        print()

        while True:
            try:
                # Read available data
                if ser.in_waiting > 0:
                    data = ser.read(ser.in_waiting)
                    try:
                        text = data.decode('utf-8', errors='replace')
                        # Print without extra newlines (ESP-IDF already includes them)
                        sys.stdout.write(text)
                        sys.stdout.flush()
                    except Exception as e:
                        # Print as hex if decode fails
                        print(f"[HEX] {data.hex()}")
                else:
                    time.sleep(0.01)  # Small delay to prevent CPU spinning

            except serial.SerialException as e:
                print(f"\n[ERROR] Serial error: {e}")
                print("Device may have disconnected. Waiting...")
                time.sleep(2)
                try:
                    ser.close()
                    ser.open()
                    print("[INFO] Reconnected")
                except:
                    pass

    except serial.SerialException as e:
        print(f"[ERROR] Could not open {port}: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n\n[INFO] Monitor stopped by user")
    finally:
        try:
            ser.close()
        except:
            pass

if __name__ == '__main__':
    main()

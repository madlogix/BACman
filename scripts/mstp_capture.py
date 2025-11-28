#!/usr/bin/env python3
"""Simple MS/TP frame capture and decoder"""

import sys
import serial
import time
from datetime import datetime

FRAME_TYPES = {
    0: "Token",
    1: "PollForMaster",
    2: "ReplyToPollForMaster",
    3: "TestRequest",
    4: "TestResponse",
    5: "BACnetDataExpectingReply",
    6: "BACnetDataNotExpectingReply",
    7: "ReplyPostponed",
}

def main():
    port = sys.argv[1] if len(sys.argv) > 1 else '/dev/ttyACM1'
    baud = int(sys.argv[2]) if len(sys.argv) > 2 else 38400

    print(f"MS/TP Capture - {port} @ {baud} baud")
    print("=" * 70)

    ser = serial.Serial(port=port, baudrate=baud, timeout=0.1)

    buf = bytearray()
    frame_count = 0

    # Track PFM to station 3 (our M5Stack) and responses
    pfm_to_3 = 0
    reply_from_3 = 0
    token_to_3 = 0

    try:
        start = time.time()
        while time.time() - start < 15:  # 15 second capture
            data = ser.read(256)
            if data:
                buf.extend(data)

            # Parse frames from buffer
            while len(buf) >= 8:
                # Find preamble
                try:
                    idx = buf.index(0x55)
                    if idx > 0:
                        buf = buf[idx:]
                    if len(buf) < 2 or buf[1] != 0xFF:
                        buf = buf[1:]
                        continue
                except ValueError:
                    buf.clear()
                    continue

                if len(buf) < 8:
                    break

                # Parse header
                ftype = buf[2]
                dest = buf[3]
                src = buf[4]
                data_len = (buf[5] << 8) | buf[6]
                hcrc = buf[7]

                frame_size = 8 if data_len == 0 else 8 + data_len + 2

                if len(buf) < frame_size:
                    break

                # Got complete frame
                frame_count += 1
                ftype_name = FRAME_TYPES.get(ftype, f"Unknown({ftype})")

                ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]

                # Track specific frames
                if ftype == 1 and dest == 3:  # PFM to station 3
                    pfm_to_3 += 1
                    print(f"[{ts}] *** PFM -> 3: src={src} (count={pfm_to_3})")
                elif ftype == 2 and src == 3:  # Reply from station 3
                    reply_from_3 += 1
                    print(f"[{ts}] *** REPLY from 3 -> {dest} (count={reply_from_3})")
                elif ftype == 0 and dest == 3:  # Token to station 3
                    token_to_3 += 1
                    print(f"[{ts}] *** TOKEN -> 3: src={src} (count={token_to_3})")
                elif ftype == 0:  # Other tokens
                    print(f"[{ts}] Token: {src} -> {dest}")
                elif ftype == 1:  # Other PFM
                    print(f"[{ts}] PFM: {src} -> {dest}")
                elif ftype >= 5:  # Data frames
                    print(f"[{ts}] {ftype_name}: {src} -> {dest} len={data_len}")

                buf = buf[frame_size:]

    except KeyboardInterrupt:
        pass
    finally:
        ser.close()

    print("=" * 70)
    print(f"Capture Summary:")
    print(f"  Total frames: {frame_count}")
    print(f"  PFM to station 3: {pfm_to_3}")
    print(f"  Reply from station 3: {reply_from_3}")
    print(f"  Token to station 3: {token_to_3}")

if __name__ == '__main__':
    main()

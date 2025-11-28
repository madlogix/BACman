# MS/TP UART Implementation Research

This document summarizes how others have implemented BACnet MS/TP over UART, based on research of open-source implementations.

## Key Architecture Pattern

The standard approach (from [bacnet-stack](https://github.com/bacnet-stack/bacnet-stack)) uses **two separate state machines**:

1. **Receive FSM** - Byte-by-byte frame assembly
2. **Master Node FSM** - Token passing and protocol logic

Plus a **Transmit FSM** for managing outbound data.

---

## UART/RS485 Layer Functions

From the [Xplained port rs485.c](https://github.com/alljoyn/dsb/blob/master/Samples/BACnetAdapter/bacnet-stack-0.8.2/ports/xplained/rs485.c):

| Function | Purpose |
|----------|---------|
| `rs485_init()` | Configure UART, GPIO pins, enable RX interrupt |
| `rs485_byte_available(uint8_t *data)` | Check if byte in RX FIFO, return it |
| `rs485_byte_send(uint8_t data)` | Queue byte for transmission |
| `rs485_rts_enable(bool enable)` | Control DE/RE pins for direction |
| `rs485_silence_reset()` | Reset silence timer on activity |
| `rs485_silence_elapsed()` | Return ms since last activity |
| `rs485_turnaround_elapsed()` | Return ms since TX completed |
| `rs485_baud_rate_set(uint32_t baud)` | Configure baud rate |

---

## Interrupt Handling Pattern

From [STM32F10x port](https://github.com/stargieg/bacnet-stack/blob/master/ports/stm32f10x/dlmstp.c):

```
RX Interrupt Handler:
  1. Read byte from UART data register
  2. Put byte into RX FIFO ring buffer
  3. Reset silence timer (critical for timing!)

TX Complete Interrupt:
  1. Check if more bytes in TX FIFO
  2. If yes, load next byte to UART
  3. If no, disable transmitter, call rs485_rts_enable(false)
```

---

## FIFO Buffer Management

- **512-byte FIFOs** for both RX and TX (sized for BACnet MAX_APDU of 480 bytes)
- Ring buffer implementation with read/write indices
- Power-of-2 size recommended for efficient modulo operations

---

## Direction Control (Half-Duplex)

The DE (Driver Enable) and RE (Receiver Enable) pins are typically tied together:

```
Transmit Mode:  DE=HIGH, RE=HIGH  → Driver enabled
Receive Mode:   DE=LOW,  RE=LOW   → Receiver enabled
```

Key timing:
1. **Before TX**: Enable driver with `rs485_rts_enable(true)`
2. **After TX complete**: Wait for `Tturnaround` (40-bit times)
3. **Then**: Disable driver with `rs485_rts_enable(false)`

---

## Receive State Machine States

```
IDLE → PREAMBLE → HEADER → DATA → IDLE
         ↓           ↓       ↓
      (0x55)      (0xFF)   (CRC check)
```

From [mstp.c](https://github.com/stargieg/bacnet-stack/blob/master/src/mstp.c):

1. **IDLE**: Wait for first preamble byte (0x55)
2. **PREAMBLE**: Wait for second preamble byte (0xFF)
3. **HEADER**: Collect 6 bytes (FrameType, Dest, Src, Length[2], HeaderCRC)
4. **DATA**: Collect payload + 2-byte DataCRC, validate with expected CRC 0xF0B8

---

## Critical Timing Parameters

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `Tframe_abort` | 30ms | Max silence during frame reception |
| `Tno_token` | 500ms | Token loss detection |
| `Treply_timeout` | 255ms | Wait for reply to request |
| `Tslot` | 10ms | Poll For Master window |
| `Tusage_timeout` | 60ms | Wait for successor activity |
| `Tturnaround` | 40 bit times | TX→RX direction switch |

---

## Transmit State Machine States

```
IDLE → SILENCE_WAIT → SEND_WAIT → STOP → IDLE
```

1. **IDLE**: Wait for frame to send
2. **SILENCE_WAIT**: Ensure bus quiet for turnaround time
3. **SEND_WAIT**: Transmit bytes one at a time
4. **STOP**: Confirm TX complete, disable driver

---

## ESP32-Specific Considerations

From [ESP32 forums](https://esp32.com/viewtopic.php?t=15474) and [ESP-IDF docs](https://docs.espressif.com/projects/esp-idf/en/v4.3.2/esp32/api-reference/peripherals/uart.html):

1. **Use `UART_MODE_RS485_HALF_DUPLEX`** - ESP-IDF auto-controls RTS pin
2. **FreeRTOS timing challenges** - Other tasks can affect MS/TP timing precision
3. **Hardware RTS** preferred over GPIO bit-banging for direction control
4. **Official example**: `examples/peripherals/uart/uart_echo_rs485`

---

## Key Takeaways for Implementation

1. **Separate concerns**: RS485 layer handles bytes, MSTP layer handles frames
2. **Silence timer is critical**: Reset on EVERY byte (RX and TX)
3. **Don't echo your own TX**: Only receive when not transmitting
4. **Use interrupt-driven RX**: Feed bytes into FIFO, let state machine poll
5. **Hardware direction control**: Let ESP32 handle RTS automatically if possible

---

## Reference Implementations

### Primary Sources

- [bacnet-stack/bacnet-stack (GitHub)](https://github.com/bacnet-stack/bacnet-stack) - The canonical open-source BACnet implementation
- [STM32F10x dlmstp.c](https://github.com/stargieg/bacnet-stack/blob/master/ports/stm32f10x/dlmstp.c) - Embedded ARM port
- [Xplained port rs485.c](https://github.com/alljoyn/dsb/blob/master/Samples/BACnetAdapter/bacnet-stack-0.8.2/ports/xplained/rs485.c) - Atmel Xplained board port
- [Linux rs485.c](https://github.com/stargieg/bacnet-stack/blob/master/ports/linux/rs485.c) - Linux serial port implementation

### ESP32-Specific Resources

- [ESP-IDF UART Documentation](https://docs.espressif.com/projects/esp-idf/en/v4.3.2/esp32/api-reference/peripherals/uart.html)
- [ESP-IDF RS485 Echo Example](https://github.com/espressif/esp-idf/blob/master/examples/peripherals/uart/uart_echo_rs485/README.md)
- [ESP32 MS/TP Medium Article](https://medium.com/@ilakk2023/268-navigating-hardware-communication-technologies-esp32-bacnet-mstp-and-rs-485-f0ad4883e6e3)

### Other BACnet MS/TP Projects

- [Misty (MS/TP for bacpypes)](https://github.com/riptideio/misty) - Python implementation
- [BACnet Server MSTP Example C++](https://github.com/chipkin/BACnetServerMSTPExampleCPP) - Commercial stack example
- [bacnet-stack-stm32f4](https://github.com/ThePiGrepper/bacnet-stack-stm32f4) - STM32F4 port

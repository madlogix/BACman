# ASHRAE Standard 135-2024 - BACnet Protocol Index

> **Quick Reference for BACScope Flutter App Development**
>
> This index organizes the BACnet standard documents by implementation priority and topic.
> Use this to quickly find relevant protocol specifications.

---

## How to Use This Index

### For Claude/AI Assistants

When implementing BACnet features, reference documents in this order:

1. **Check the relevant clause** for service definitions (Clauses 13-17)
2. **Check Annex-J** for BACnet/IP transport details
3. **Check Annex-H** for network routing/VMAC addressing
4. **Check examples** in Annex-E and Annex-F for encoding patterns

### Quick Lookup by Feature

| Feature | Primary Document | Supporting Docs |
|---------|------------------|-----------------|
| Device Discovery (Who-Is/I-Am) | [16-REMOTE-DEVICE-MANAGEMENT-SERVICES](./16-REMOTE-DEVICE-MANAGEMENT-SERVICES.md) | Annex-J |
| ReadProperty/WriteProperty | [15-OBJECT-ACCESS-SERVICES](./15-OBJECT-ACCESS-SERVICES.md) | Annex-F |
| COV Subscriptions | [13-ALARM-AND-EVENT-SERVICES](./13-ALARM-AND-EVENT-SERVICES.md) | Annex-M |
| Object Types | [12-MODELING-CONTROL-DEVICES](./12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md) | Annex-L |
| APDU Encoding | [21-FORMAL-DESCRIPTION-OF-APDU](./21-FORMAL-DESCRIPTION-OF-APPLICATION-PROTOCOL-DATA-UNITS.md) | Annex-F |
| BACnet/IP (UDP) | [Annex-J-BACnet-IP](./Annex-J-BACnet-IP.md) | Annex-H |
| Network Routing | [Annex-H-COMBINING-NETWORKS](./Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md) | - |

---

## Document Categories

### Front Matter & Reference

| File | Description |
|------|-------------|
| [00-Front-Matter](./00-Front-Matter.md) | ANSI/ASHRAE Standard 135-2024 title, SSPC committee, approval history |
| [25-REFERENCES](./25-REFERENCES.md) | External standards references (stub) |

---

## Core Protocol Clauses

### Clause 3 - Definitions & Architecture

| File | Description |
|------|-------------|
| [03-DEFINITIONS](./03-DEFINITIONS.md) | **FOUNDATIONAL** - Protocol architecture, terms, abbreviations, protocol layering, application layer model, segmentation, state machines, network/data link layers |

### Clause 12 - Object Model

| File | Description |
|------|-------------|
| [12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS](./12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md) | **CRITICAL** - All 42 BACnet object types (Analog I/O/V, Binary I/O/V, Calendar, Device, Schedule, Trend Log, etc.) |

---

## Application Services (Clauses 13-17)

### Clause 13 - Alarm & Event Services

| File | Description |
|------|-------------|
| [13-ALARM-AND-EVENT-SERVICES](./13-ALARM-AND-EVENT-SERVICES.md) | **PRIORITY 1 FEATURE** - COV reporting, Event algorithms, SubscribeCOV, AcknowledgeAlarm, GetEventInformation, LifeSafetyOperation |

### Clause 14 - File Access Services

| File | Description |
|------|-------------|
| [14-FILE-ACCESS-SERVICES](./14-FILE-ACCESS-SERVICES.md) | AtomicReadFile, AtomicWriteFile (stub) |

### Clause 15 - Object Access Services

| File | Description |
|------|-------------|
| [15-OBJECT-ACCESS-SERVICES](./15-OBJECT-ACCESS-SERVICES.md) | **CRITICAL** - ReadProperty, ReadPropertyMultiple, WriteProperty, WritePropertyMultiple, ReadRange, CreateObject, DeleteObject |

### Clause 16 - Remote Device Management

| File | Description |
|------|-------------|
| [16-REMOTE-DEVICE-MANAGEMENT-SERVICES](./16-REMOTE-DEVICE-MANAGEMENT-SERVICES.md) | **CRITICAL** - Who-Is/I-Am, Who-Has/I-Have, DeviceCommunicationControl, ReinitializeDevice, TimeSynchronization |

### Clause 17 - Authentication & Authorization

| File | Description |
|------|-------------|
| [17-AUTHENTICATION-AND-AUTHORIZATION-SERVICES](./17-AUTHENTICATION-AND-AUTHORIZATION-SERVICES.md) | Security services, trust model, AuthRequest; includes Error/Reject/Abort codes |

---

## Protocol Definition (Clauses 21-25)

| File | Description |
|------|-------------|
| [21-FORMAL-DESCRIPTION-OF-APPLICATION-PROTOCOL-DATA-UNITS](./21-FORMAL-DESCRIPTION-OF-APPLICATION-PROTOCOL-DATA-UNITS.md) | APDU formal grammar, confirmed/unconfirmed service productions, error productions, application types |
| [22-CONFORMANCE-AND-INTEROPERABILITY](./22-CONFORMANCE-AND-INTEROPERABILITY.md) | Extending BACnet (proprietary properties, objects, enumerations) |
| [24-DELETED-CLAUSE](./24-DELETED-CLAUSE.md) | Removed from standard |

---

## Technical Annexes - Data Link & Transport

### BACnet/IP (Primary Transport - IMPLEMENTED)

| File | Description |
|------|-------------|
| [Annex-J-BACnet-IP](./Annex-J-BACnet-IP.md) | **CRITICAL** - BACnet/IP over UDP, BBMD operation, broadcast distribution, foreign device registration, B/IP-M multicast |

### Network Layer & Routing

| File | Description |
|------|-------------|
| [Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS](./Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md) | **IMPORTANT** - VMAC addressing, network routing, EUI-48/Random-48 VMAC, conflict detection |

### Alternative Transports (Not Implemented)

| File | Description |
|------|-------------|
| [Annex-O-BACnet-OVER-ZigBee-AS-A-DATA-LINK-LAYER](./Annex-O-BACnet-OVER-ZigBee-AS-A-DATA-LINK-LAYER.md) | ZigBee data link, VMAC table management, ZigBee Cluster Library frames |
| [Annex-U-BACnet-IPv6](./Annex-U-BACnet-IPv6.md) | BACnet over IPv6, Foreign Device Table, IPv6 VMAC management |
| [Annex-T-COBS](./Annex-T-COBS.md) | Constant Overhead Byte Stuffing for MS/TP, CRC examples |
| [Annex-G-CALCULATION-OF-CRC](./Annex-G-CALCULATION-OF-CRC.md) | CRC calculation for MS/TP, hardware CRC generators, shift register operations |

---

## Technical Annexes - Data Formats & Encoding

| File | Description |
|------|-------------|
| [Annex-P-BACnet-ENCODING-OF-STANDARD-AUTHENTICATION-FACTOR-](./Annex-P-BACnet-ENCODING-OF-STANDARD-AUTHENTICATION-FACTOR-.md) | Authentication factor encodings (CHUID, IPv6, CBEFF biometric) |
| [Annex-Q-XML-DATA-FORMATS](./Annex-Q-XML-DATA-FORMATS.md) | XML format for BACnet objects and properties |
| [Annex-Z-JSON-DATA-FORMATS](./Annex-Z-JSON-DATA-FORMATS.md) | **LARGE (226KB)** - JSON schemas, metadata extensions, includes Annex AA (Time Series Data Exchange) |

---

## Technical Annexes - Interoperability & Conformance

| File | Description |
|------|-------------|
| [Annex-A-PROTOCOL-IMPLEMENTATION-CONFORMANCE-STATEMENT](./Annex-A-PROTOCOL-IMPLEMENTATION-CONFORMANCE-STATEMENT.md) | PICS template, character encoding, gateway options |
| [Annex-B-GUIDE-TO-SPECIFYING-BACnet-DEVICES](./Annex-B-GUIDE-TO-SPECIFYING-BACnet-DEVICES.md) | Using BIBBs and device profiles for system design |
| [Annex-K-BACnet-INTEROPERABILITY-BUILDING-BLOCKS](./Annex-K-BACnet-INTEROPERABILITY-BUILDING-BLOCKS.md) | BIBB definitions (AA-DAC-A, AA-SAC-A, etc.), device interoperability |
| [Annex-L-DESCRIPTIONS-AND-PROFILES-OF-STANDARDIZED-BACnet-D](./Annex-L-DESCRIPTIONS-AND-PROFILES-OF-STANDARDIZED-BACnet-D.md) | Standard device profiles (B-EM Elevator Monitor, etc.) |

---

## Technical Annexes - Advanced Features

| File | Description |
|------|-------------|
| [Annex-I-COMMANDABLE-PROPERTIES-WITH-MINIMUM-ON-AND-OFF-TIM](./Annex-I-COMMANDABLE-PROPERTIES-WITH-MINIMUM-ON-AND-OFF-TIM.md) | Minimum on/off time behavior, priority array interaction |
| [Annex-M-GUIDE-TO-EVENT-NOTIFICATION-PRIORITY-ASSIGNMENTS](./Annex-M-GUIDE-TO-EVENT-NOTIFICATION-PRIORITY-ASSIGNMENTS.md) | Event priorities (Life Safety, Property Safety, Supervisory, etc.) |
| [Annex-X-EXTENDED-DISCOVERY-OF-DEVICES,-PROFILES,-AND-VIEWS](./Annex-X-EXTENDED-DISCOVERY-OF-DEVICES,-PROFILES,-AND-VIEWS.md) | Extended discovery, CSML, PICS in XML, device views |
| [Annex-Y-ABSTRACT-DATA-MODEL](./Annex-Y-ABSTRACT-DATA-MODEL.md) | Abstract data model metadata, property identifiers |

---

## Technical Annexes - Web Services

| File | Description |
|------|-------------|
| [Annex-W-BACnet-WS-RESTful-WEB-SERVICES-INTERFACE](./Annex-W-BACnet-WS-RESTful-WEB-SERVICES-INTERFACE.md) | RESTful web services (stub - content in Annex-Z) |
| [Annex-N-FORMER-BACnet-WS-WEB-SERVICES-INTERFACE](./Annex-N-FORMER-BACnet-WS-WEB-SERVICES-INTERFACE.md) | Legacy SOAP services (deprecated) |
| [Annex-V-MIGRATION-FROM-SOAP-SERVICES](./Annex-V-MIGRATION-FROM-SOAP-SERVICES.md) | SOAP to REST migration guide |

---

## Examples & Reference (Informative)

| File | Description |
|------|-------------|
| [Annex-E-EXAMPLES-OF-BACnet-APPLICATION-SERVICES](./Annex-E-EXAMPLES-OF-BACnet-APPLICATION-SERVICES.md) | Service request/response examples with data |
| [Annex-F-EXAMPLES-OF-APDU-ENCODING](./Annex-F-EXAMPLES-OF-APDU-ENCODING.md) | **USEFUL** - APDU encoding hex dumps, PDU type demonstrations |
| [Annex-R-MAPPING-NETWORK-LAYER-ERRORS](./Annex-R-MAPPING-NETWORK-LAYER-ERRORS.md) | Network layer error mappings |

---

## Removed/Deprecated Content

| File | Status |
|------|--------|
| [Annex-C-REMOVED](./Annex-C-REMOVED.md) | Removed from standard |
| [Annex-D-REMOVED](./Annex-D-REMOVED.md) | Removed from standard |
| [Annex-S-Removed](./Annex-S-Removed.md) | Removed from standard |

---

## Implementation Priority Guide

### Currently Implemented (BACScope v1.0)

These documents cover features already working in the app:

1. **[15-OBJECT-ACCESS-SERVICES](./15-OBJECT-ACCESS-SERVICES.md)** - ReadProperty, ReadPropertyMultiple, WriteProperty
2. **[16-REMOTE-DEVICE-MANAGEMENT-SERVICES](./16-REMOTE-DEVICE-MANAGEMENT-SERVICES.md)** - Who-Is/I-Am discovery
3. **[Annex-J-BACnet-IP](./Annex-J-BACnet-IP.md)** - UDP/IP transport on port 47808
4. **[12-MODELING-CONTROL-DEVICES](./12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md)** - Object type definitions

### Priority 1: Real-Time Updates (COV)

For implementing Change of Value subscriptions:

1. **[13-ALARM-AND-EVENT-SERVICES](./13-ALARM-AND-EVENT-SERVICES.md)** - SubscribeCOV, COVNotification
2. **[Annex-M-GUIDE-TO-EVENT-NOTIFICATION-PRIORITY-ASSIGNMENTS](./Annex-M-GUIDE-TO-EVENT-NOTIFICATION-PRIORITY-ASSIGNMENTS.md)** - Priority levels

### Priority 2: Historical Data (Trend Logs)

For implementing trend log reading:

1. **[15-OBJECT-ACCESS-SERVICES](./15-OBJECT-ACCESS-SERVICES.md)** - ReadRange service
2. **[12-MODELING-CONTROL-DEVICES](./12-MODELING-CONTROL-DEVICES-AS-A-COLLECTION-OF-OBJECTS.md)** - Trend Log object type

### Priority 3: Advanced Routing

For multi-network support:

1. **[Annex-H-COMBINING-BACnet-NETWORKS](./Annex-H-COMBINING-BACnet-NETWORKS-WITH-NON-BACnet-NETWORKS.md)** - VMAC, routing
2. **[03-DEFINITIONS](./03-DEFINITIONS.md)** - Network layer architecture

---

## Protocol Layer Reference

```
┌─────────────────────────────────────────────────────────────────┐
│  Application Layer (Clauses 13-17)                              │
│  Services: ReadProperty, WriteProperty, Who-Is, SubscribeCOV    │
├─────────────────────────────────────────────────────────────────┤
│  APDU Layer (Clause 21)                                         │
│  Encoding: Confirmed/Unconfirmed requests, Complex-ACK, Errors  │
├─────────────────────────────────────────────────────────────────┤
│  Network Layer (Annex H)                                        │
│  NPDU: Routing, VMAC addressing, network numbers                │
├─────────────────────────────────────────────────────────────────┤
│  Data Link Layer (Annex J - BACnet/IP)                          │
│  BVLC: UDP/IP on port 47808, BBMD, foreign devices              │
├─────────────────────────────────────────────────────────────────┤
│  Physical Layer                                                  │
│  Ethernet, Wi-Fi (UDP/IP transport)                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## File Statistics

| Category | Files | Notes |
|----------|-------|-------|
| Core Clauses | 10 | Clauses 3, 12, 13-17, 21-22, 24-25 |
| Technical Annexes | 22 | Annexes A-B, E-Z (excluding removed) |
| Removed Content | 3 | Annexes C, D, S |
| Front Matter | 1 | Title/committee info |
| **Total** | **36** | |

**Largest File**: Annex-Z-JSON-DATA-FORMATS.md (226 KB) - Contains JSON schemas and Annex AA

---

*Last Updated: 2025-11-26*
*BACnet Standard Version: ASHRAE 135-2024*

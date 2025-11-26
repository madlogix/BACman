# BACnet-RS vs BACnet-Stack Comparison Issues

## Issues Found

### 1. ReadProperty Service Encoding Issues

**Problem**: The ReadProperty request encoding in bacnet-rs uses incorrect hardcoded tags instead of proper context tag encoding.

**Current implementation**:
```rust
buffer.push(0x0C); // Context tag 0, length 4  // WRONG!
buffer.extend_from_slice(&device_id.to_be_bytes());

buffer.push(0x19); // Context tag 1, length 1  // WRONG!
buffer.push(self.property_identifier as u8);   // Should be enumerated!
```

**Correct implementation (from bacnet-stack)**:
- Object identifier should use `encode_context_object_id()` (context tag 0)
- Property identifier should use `encode_context_enumerated()` (context tag 1)
- Array index should use `encode_context_unsigned()` (context tag 2)

### 2. I-Am Service Encoding Issues

**Problem**: The I-Am service encoding uses application tags when it should use context tags for some fields.

**Current implementation**:
```rust
buffer.push(0xC4); // Application tag 12 (ObjectIdentifier), length 4
```

**Correct implementation**: I-Am uses application tags, so this is actually correct, but the segmentation field should be enumerated, not just a byte.

### 3. Missing Context Tag Functions

**Problem**: Several context tag encoding functions are missing:
- `encode_context_object_id()`
- `encode_context_enumerated()`
- `decode_context_object_id()`
- `decode_context_enumerated()`

### 4. WriteProperty Service Issues

Similar to ReadProperty, WriteProperty also uses incorrect tag encoding.

### 5. Property Identifier Size

**Problem**: Property identifiers are being encoded as single bytes, but they should support full enumerated values (up to 4 bytes).

### 6. Array Index Handling

**Problem**: The special value `BACNET_ARRAY_ALL` (0xFFFFFFFF) is not defined or handled properly when array_index is None.

## Recommended Fixes

1. Add missing context tag encoding functions to the encoding module
2. Fix all service encodings to use proper context tags
3. Define constants for special values like BACNET_ARRAY_ALL
4. Update property identifier encoding to support full range
5. Add proper enumerated encoding for segmentation values
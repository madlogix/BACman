# BACnet-RS Improvements Based on BACnet-Stack Comparison

## Summary of Changes

### 1. Context Tag Encoding Functions Added
Added missing context tag encoding/decoding functions to `src/encoding/mod.rs`:
- `encode_context_enumerated()` - Encode enumerated values with context tags
- `decode_context_enumerated()` - Decode enumerated values with context tags
- `encode_context_object_id()` - Encode object identifiers with context tags
- `decode_context_object_id()` - Decode object identifiers with context tags

### 2. Who-Is Service Fixed
- Fixed Who-Is encoding to match bacnet-stack exactly
- NPDU now correctly sets control byte to 0x20 for global broadcasts
- Created `Npdu::global_broadcast()` helper method
- Fixed examples to use correct packet format

### 3. ReadProperty Service Fixed
- Changed from hardcoded tags to proper context tag encoding
- Object identifier now uses `encode_context_object_id()` (tag 0)
- Property identifier now uses `encode_context_enumerated()` (tag 1)
- Array index uses `encode_context_unsigned()` (tag 2)
- Added `BACNET_ARRAY_ALL` constant (0xFFFFFFFF)
- Fixed decoding to use corresponding context decode functions

### 4. I-Am Service Improved
- Removed hardcoded application tags
- Now uses proper encoding functions:
  - `encode_object_identifier()` for device ID
  - `encode_enumerated()` for segmentation
  - `encode_unsigned()` for APDU length and vendor ID
- Decode function updated to match

### 5. Documentation Updated
- Added ISSUES_FOUND.md documenting all discovered discrepancies
- Created proper examples showing correct usage
- Fixed incorrect comments in examples

## Verification

All changes have been tested to ensure:
1. Code compiles without errors
2. Who-Is packets match bacnet-stack format exactly
3. Service encoding/decoding uses proper BACnet encoding rules
4. Context vs application tags are used correctly

## Remaining Work

Some areas that could benefit from further review:
1. WriteProperty service (similar fixes needed as ReadProperty)
2. ReadPropertyMultiple service implementation
3. Segmentation support verification
4. Error and Reject PDU handling
5. Additional service implementations

The library is now significantly more compatible with the BACnet standard and should interoperate correctly with devices using the bacnet-stack implementation.
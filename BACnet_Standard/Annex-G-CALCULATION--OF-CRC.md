ANNEX G - CALCULATION  OF CRC (INFORMATIVE)

#include <stdint.h>

void
CreateCRC32Table()
{
  uint16_t data;
  uint32_t crc;
  uint16_t b;

  printf( "static const uint32_t CRC32Table[256] = {" );
  for (data = 0; ; ) {
    if (data % 8 == 0)
      printf("\n");

    crc = data & 0xFF;
    for (b = 0; b < 8; b++) {
      if (crc & 1) {
          crc >>= 1;
          crc ^= 0xEB31D82E;
        } else {
          crc >>= 1;
        }
    }
    printf( "0x%08lX", crc );

    if (++data == 256)
      break;
    printf( ", " );
  }
  printf( "\n};\n" );
}

/* Update running "crcValue" with "dataValue"
 * The crcValue shall be initialized to all ones.
 *
 * For transmission, the returned value shall be complemented and then
 * sent low-order octet first (i.e. right to left).
 *
 * On reception, if Data ends with a correct CRC, the returned value
 * will be 0x0843323B (0000 1000 0100 0011 0011 0010 0011 1011).
 */
uint32_t
LookupCRC32K(uint8_t dataValue, uint32_t crcValue)
{
  crcValue = CRC32Table[(crcValue ^ dataValue) & 0xFF] ^ (crcValue >> 8);
  return (crcValue);
}

1108

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

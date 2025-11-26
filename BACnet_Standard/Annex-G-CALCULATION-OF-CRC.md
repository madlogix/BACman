ANNEX G - CALCULATION OF CRC (INFORMATIVE)

(This annex is not part of this standard but is included for informative purposes only.)

Historically,  CRC  generators  have  been  implemented  as  shift  registers  with  exclusive  OR  feedback.  This  provides  an
inexpensive way to process CRC information on a serial bit stream in hardware. Since commercial UARTs do not provide
hardware calculation of CRC, UART-based protocols such as those described in Clauses 9 and 10 must perform this calculation
with  software.  While  this  can  be  done  one  bit  at  a  time,  simulating  a  shift  register  with  feedback,  a  much  more  efficient
algorithm is possible that computes the CRC on an entire octet at once. This annex shows how the CRC may be computed in
this manner. This algorithm is presented as an example and is not intended to restrict the vendor's implementation of the CRC
calculation.

G.1 Calculation of the Header CRC

We begin with the diagram of a hardware CRC generator as shown in Figure G-1. The polynomial used is

X8 + X7 + 1

Figure G-1. Hardware header-CRC generator.

The hardware implementation operates on a serial bit stream, whereas our calculation must operate on entire octets. To this
end, we follow the operation of the circuit through eight bits of data. The CRC shift register is initialized to X0 (input end) to
X7 (output end), the input data is D0 to D7 (least significant bit first, as transmitted and received by a UART). Within each
block below, the terms are exclusive OR'ed vertically.

input   register contents
  |--------------------------------|
D0| X0  X1  X2  X3  X4  X5  X6  X7 |
  |--------------------------------|
D1|     X0  X1  X2  X3  X4  X5  X6 |
  | D0                          D0 |
  | X7                          X7 |
  |--------------------------------|
D2|         X0  X1  X2  X3  X4  X5 |
  | D1  D0                      D1 |
  | X6  X7                      X6 |
  | D0                          D0 |
  | X7                          X7 |
  |--------------------------------|
D3|             X0  X1  X2  X3  X4 |
  | D2  D1  D0                  D2 |
  | X5  X6  X7                  X5 |
  | D1  D0                      D1 |
  | X6  X7                      X6 |
  | D0                          D0 |
  | X7                          X7 |
  |--------------------------------|

ANSI/ASHRAE Standard 135-2024

1095

InputOutputZ-7XORXORZ-1Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

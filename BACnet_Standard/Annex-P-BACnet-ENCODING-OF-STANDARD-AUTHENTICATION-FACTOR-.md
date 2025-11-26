ANNEX P – BACnet ENCODING OF STANDARD AUTHENTICATION FACTOR FORMATS (NORMATIVE)

Authentication Factor Format
Description
Complete Card Holder Unique
Identifier stored as data string.
The data elements are decoded
using the CHUID tags which
are embedded in the data string.

See SP 800-73 Section 1.8.3
(Figure 1 & 2  pg 12 of the TIG
2.3)
Global unique identifier
represented as IPv6 address

Authentication Factor Value Encoding1,3

Octet String Size = n (maximum size = 3397)

Octet[1..n] = CHUID data string

-- Octet encoding is defined in SP 800-73 (Figure 1 & 2
of the TIG 2.3) using CHUID Tags.

Octet String Size = 16

-- Refer to RFC 2373 for format description and
encoding

Common Biometric Exchange
File Format  (CBEFF) Patron
format A

Octet String Size = n

Octet[1..n] = CBEFF data

-- NIST CBEFF Patron Format A (CBEFF) content
formatted

Common Biometric Exchange
File Format  (CBEFF) Patron
format B

Octet String Size = n

Octet[1..n] = CBEFF data

-- NIST CBEFF Patron Format B (BioAPI) content
formatted

Common Biometric Exchange
File Format  (CBEFF) Patron
format C

Octet String Size = n

Octet[1..n] = CBEFF data

-- NIST CBEFF Patron Format C (ANSI Standard
X9.84) content formatted

USER_PASSWORD

User name and password

Octet String Size = n,

Octet[1] = length of user name string  in octets including
character set specifier (max 255)

Octet[2] = character set specifier for user name string (as
specified in Clause 20.2.9 excluding DBCS, i.e. a value of
X'01')

Octet[3..m] = string of characters for user name (encoded
as specified in Clause 20.2.9)

Octet[m+1] = length of password string in octets
including character set specifier (max 255)

Octet[m+2] = character set specifier for password string
(as specified in Clause 20.2.9 excluding DBCS, i.e. a
value of X'01')

Octet[m+3..n] = string of characters for password
(encoded as specified in Clause 20.2.9)

1  Multi-octet fields shall be conveyed with the most significant octet first.
2  In BCD (binary coded decimal) format, each octet holds two 4-bit BCD encoded decimal digits. Bits 7 to 4

convey the most significant digit, while Bits 3 to 0 convey the least significant digit.

3  Data fields specified for an encoding which are not contained on the credential shall be set to zero for

unsigned
values or all zeros for BCD values. BCD values which use less than the allocated space shall be padded with
leading zeros in the most significant nibbles as necessary.

ANSI/ASHRAE Standard 135-2024

1265

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

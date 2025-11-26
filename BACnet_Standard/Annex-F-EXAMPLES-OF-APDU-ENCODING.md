ANNEX F – EXAMPLES OF APDU ENCODING (INFORMATIVE)

X'00'
X'01'
X'53'
X'17'

PDU Type=0 (BACnet-Confirmed-Request-PDU, SEG=0, MOR=0, SA=0)
Maximum APDU Size Accepted=128 octets
Invoke ID=83
Service Choice=23 (VT-Data-Request)

X'21'
X'05'
X'65'
X'15'
X'465245440D0A456E7465722050617373776F72643A'"FRED{cr}{lf}Enter Password:"
X'21'
X'01'

Application Tag 2 (Unsigned Integer, L=1) (VT Session Identifier)
5
Application Tag 6 (Octet String, L>4) (VT New Data)
Extended Length=21

Application Tag 2 (Unsigned Integer, L=1) (VT Data Flag)
1

To which the target device would respond:

X'30'
X'53'
X'17'
X'09'
X'01'

Terminal sign-off:

X'00'
X'01'
X'54'
X'16'

X'21'
X'1D'

Response:

X'20'
X'54'
X'16'

PDU Type=3 (BACnet-ComplexACK-PDU, SEG=0, MOR=0)
Invoke ID=83
Service Choice=23 (VT-Data-ACK)
SD Context Tag 0 (All New Data Accepted, L=1)
1 (TRUE)

PDU Type=0 (BACnet-Confirmed-Request-PDU, SEG=0, MOR=0, SA=0)
Maximum APDU Size Accepted=128 octets
Invoke ID=84
Service Choice=22 (VT-Close-Request)

Application Tag 2 (Unsigned Integer, L=1) (List Of Remote VT Session Identifiers)
29

PDU Type=2 (BACnet-SimpleACK-PDU)
Invoke ID=84
Service ACK Choice=22 (VT-Close)

1094

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

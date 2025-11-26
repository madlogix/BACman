ANNEX E – EXAMPLES OF BACnet APPLICATION SERVICES (INFORMATIVE)

'VT-session Identifier' = 5
'VT-new Data' =
'VT-data Flag' =

"FRED{cr}{lf}Enter Password:"
1

Our operator interface display queue is empty so it can accept all of the incoming characters. Our VT-User therefore issues 'Result
(+)':

'All New Data Accepted' = TRUE

For some reason, FRED decides to cancel this virtual terminal session and signals the operator interface program to do so. The
operator interface program issues a VT-Close request:

Service =
'List of Remote VT Session Identifiers' =  (29)

VT-Close

1062

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

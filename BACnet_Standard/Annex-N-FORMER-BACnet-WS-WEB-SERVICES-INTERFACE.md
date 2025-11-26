ANNEX N - FORMER BACnet/WS WEB SERVICES INTERFACE (INFORMATIVE)

WS_ERR_COMMUNICATION_FAILED

WS_ERR_READBACK_FAILED

24

25

"Communication with the
Remote Device Failed"
"The Readback Failed"

N.14 Extending BACnet/WS

The data model defined by this standard can be extended in the following ways:

1.  Extended  information  that  might  be  considered  to  be  a  property  of  a  node  may  be  modeled  by  adding  children

nodeswith a NodeType of "Property". This allows for the extended property data to be arbitrarily complex.

2.  Node classification can be extended by local application of the NodeSubtype attribute. Any string value can be used
for the localized value of the Units attribute. However, if the corresponding canonical value of the Units attribute
cannot be expressed as defined in Clause N.8.11, then the canonical value of that attribute shall  be "other".

ANSI/ASHRAE Standard 135-2024

1255

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

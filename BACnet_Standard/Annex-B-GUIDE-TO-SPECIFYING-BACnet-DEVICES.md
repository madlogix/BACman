ANNEX B - GUIDE TO SPECIFYING BACnet DEVICES (INFORMATIVE)

(This annex is not part of this standard but is included for informative purposes only.)

The BIBBs (Annex K) and standardized BACnet device profiles (Annex L) are intended to be a useful tool for people who
design,  specify,  or  operate  building  automation  systems  that  contain  BACnet  devices.  This  classification  approach  is  a
compromise between two conflicting goals. The first goal is to promote interoperability by limiting the various combinations
of  BACnet  object  types  and  services  that  can  be  supported  and  still  conform  to  this  standard.  The  other  goal  is  to  avoid
unnecessarily  restricting  manufacturers  of  BACnet  devices  in  the  sense  that  they  would  be  required  to  provide  BACnet
functionality that would never be used by a device except to meet a conformance requirement. Maximum interoperability would
be achieved by requiring all BACnet devices to support exactly the same combination of standard object types and application
services. On the other hand, complete flexibility for manufacturers would inevitably lead to such widespread variation in the
particular  object  types  and  application  services  that  are  supported  that  many  devices  would  only  partially  interoperate.
Interoperability would be limited to the intersection of the application services and object types supported by the devices.

The  idea behind the BIBB and device  profile model is to combine the portions of the  BACnet protocol that are needed to
perform particular functions, and to identify those functions that a system designer would expect in a certain type of device.
When designing or specifying BACnet devices for an automation system, it is appropriate to specify the device profile that best
meets the needs of the application and any additional BIBBs that are also required. Devices can be expected to interoperate
with respect to a given BIBB so long as one device implements the A-side functionality and the other device implements the
B-side functionality.

A particular manufacturer may decide to build a product that supports more BIBBs than required by its device profile. This can
be determined from the PICS.

1042

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

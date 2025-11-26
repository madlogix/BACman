ANNEX H - COMBINING BACnet NETWORKSWITH NON-BACnet NETWORKS (NORMATIVE)

Table H-11. PossibleValues and WritableValues Attribute Mappings
BACnet Property or Datatype Mapping

BACnet Object Type

Binary Input
Binary Output
Binary Value
Command
Device
Event Enrollment
Life Safety Point
Life Safety Zone
Multistate Input
Multistate Output
Multistate Value
Schedule

Active_Text, Inactive_Text properties
Active_Text, Inactive_Text properties
Active_Text, Inactive_Text properties
Action_Text property
BACnetDeviceStatus enumeration
BACnetEventState enumeration
BACnetLifeSafetyState enumeration
BACnetLifeSafetyState enumeration
State_Text property
State_Text property
State_Text property
(varies)

H.6.1.7 Overridden

This boolean attribute may correspond to the OVERRIDDEN flag in the BACnet property StatusFlags. If the OVERRIDDEN
flag of that property is set to true, then Overridden shall be true.

H.7 Virtual MAC Addressing

H.7.1 General

With the exception of LonTalk, a data link layer with a MAC address size greater than 6 octets shall expose a BACnet Virtual
MAC (VMAC) address of 6 octets or fewer to the BACnet network layer.

The VMAC address shall function analogously as the MAC address of the technologies of Clauses 7, 8, 9, 11, and Annex J.

A VMAC table shall exist within the data link layer on all BACnet nodes on a BACnet network that employs VMAC addresses.
A  VMAC  table  shall  be  used  to  map  native  MAC  addresses  of  the  data  link  layer  to  VMAC  addresses.  The  VMAC  table
contains VMAC entries corresponding to nodes in the BACnet network.

The data link layer uses native MAC addresses when communicating over its data link. The data link translates from VMAC
addresses to native MAC addresses when BACnet messages are sent out. The data link translates from native MAC addresses
to VMAC addresses when BACnet messages are received. If the address translation fails, the NPDU shall be dropped.

The methods used to maintain a VMAC table are dependent on the specific data link that is using a VMAC table.

H.7.2 Using Device Instance as a VMAC Address

When a particular data link layer specifies that each node's BACnet device instance is to be used as the VMAC address for the
node, then the device instance as a VMAC address shall be transmitted as 3 octets, with the high order octet first, and formatted
as follows:

   Bit Number:   7   6   5   4   3   2   1   0
               |---|---|---|---|---|---|---|---|
               | 0 | 0 |    High 6 Bits        |
               |---|---|---|---|---|---|---|---|
               |           Middle Octet        |
               |---|---|---|---|---|---|---|---|
               |            Low Octet          |
               |---|---|---|---|---|---|---|---|

Nodes that do not have a BACnet device instance configured shall generate and use a random instance VMAC address. The
generation  and  use  of  a  random  instance  VMAC  address  does  not  affect  the  BACnet  device  instance  which  remains  not
configured.  To  ensure  that  the  random  instance  VMAC  is  not  used  by  another  node,  the  node  shall  attempt  to  resolve  the
generated VMAC in the network. If the node detects that another node is already using the random instance VMAC it has

ANSI/ASHRAE Standard 135-2024

1125

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

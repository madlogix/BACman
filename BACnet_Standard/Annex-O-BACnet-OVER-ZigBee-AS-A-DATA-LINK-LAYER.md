ANNEX O - BACnet OVER ZigBee AS A DATA LINK LAYER (NORMATIVE)

To facilitate removal of an obsolete VMAC entry, the following procedure shall be used: After an interval, if there has been no
activity indicating a node's VMAC address for a node represented by a VMAC entry, then the BZLL shall issue a Read Attribute
command requesting the node's Protocol Address attribute (VMAC address). If the node fails to respond, the VMAC entry
shall be removed. The interval of no activity before a node's Protocol Address attribute is requested is a local matter.

If a node advertises or responds with a new VMAC address, the node's VMAC entry shall be updated.

There shall be no duplicate VMAC addresses in the VMAC table. If a duplicate address is received, a BACnet router shall keep
only the most recently verified VMAC address. Otherwise, the means by which a node detects, verifies and prevents duplicate
VMAC addresses is a local matter.

Other than the requirements above, the means by which a node maintains a VMAC table is a local matter.

O.6.2 BZLL Transfer NPDU

A BZLL on a node shall transfer BACnet NPDUs to a BZLL on another node using the APSDE-DATA primitives described
in the ZigBee Specification. A BACnet NPDU shall be transferred as a ZigBee ASDU from an output BACnet Protocol Tunnel
cluster to an input BACnet Protocol Tunnel cluster.

The ZigBee ASDU that is passed to the APSDE-DATA.request to transfer a BACnet NPDU shall be a ZigBee Cluster Library
(ZCL) client to server frame as shown in Figure O-3.

Frame Control

1 octet

X'01'

Transaction  Sequence
Number

1 octet

X'00'  to  X'FF',  incrementing  with  each  new
request command

Command Identifier

1 octet

X'00' Transfer NPDU request command

Frame Payload

N octets

BACnet NPDU

Figure O-3. ZCL Frame as ZigBee ASDU with BACnet NPDU Payload.

A BACnet unicast NPDU shall be transferred using a ZigBee unicast by specifying the EUI64 and BACnet endpoint of the
target as parameters of the APSDE-DATA.request.

A BACnet broadcast NPDU shall be transferred using the BACnet Protocol Tunnel cluster as a destination. The cluster and
source endpoint will be used to resolve, through a ZigBee binding, to a ZigBee group.

O.6.3 BZLL Generic Tunnel Cluster Support

The BZLL shall support the ZigBee Generic Tunnel cluster attributes described below.

O.6.3.1 Maximum Incoming Transfer Size

The Maximum Incoming Transfer Size attribute shall be the maximum ZigBee ASDU size, in octets, that may be received by
the BZLL. This value is related to the maximum BACnet APDU size described in Clause O.7.

O.6.3.2 Maximum Outgoing Transfer Size

The Maximum Outgoing Transfer Size attribute shall be the maximum ZigBee ASDU size, in octets, that can be sent by the
BZLL. This value is related to the maximum BACnet APDU size described in Clause O.7.

O.6.3.3 Protocol Address

The Protocol Address attribute shall be the VMAC address of the BZLL, which is the BACnet device instance.

O.7 Maximum Payload Size

Each BACnet endpoint shall support a ZigBee ASDU size that includes the maximum BACnet APDU size plus the octets in
the  ZCL  and  BACnet  NPDU  headers.  The  ZigBee  ASDU  may  be  fragmented  at  the  source  node  and  reassembled  at  the
destination node. The ZigBee stack options controlling fragmentation/reassembly and payload sizes will ultimately determine
the maximum ZigBee APDU size and therefore shall be set accordingly.

O.8 Vendor Specific Commands

The  ZigBee  Cluster  Library  frame  specification  defines  a  method  for  sending  vendor  specific  commands.  Use  of  these
commands is a local matter.

1260

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

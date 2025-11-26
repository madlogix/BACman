ANNEX J - BACnet/IP (NORMATIVE)

J.8.3 B/IP-M BBMD Operation

BBMDs function as described in Clause J.4.5 except that the BBMD serving the B/IP-M group shall also be a member of the
group and that the B/IP-M group address is used analogously to the B/IP broadcast address with respect to the B/IP-M group. It is
also required that the BDT entry for each BBMD serving a B/IP-M group shall use a broadcast distribution mask of all 1's to force
"two-hop" BBMD-to-BBMD broadcast distribution. This is to prevent the multiple receipt of broadcast messages that would occur
if the B/IP-M BBMD were on the same IP subnet as any of the B/IP-M devices themselves and a "directed broadcast" were used.
The following paragraphs summarize the relevant operations of BBMDs that serve a B/IP-M group:

Upon receipt of an Original-Broadcast-NPDU via its B/IP-M group address, a BBMD shall forward the message to other entries
in its BDT (as well as to any devices in its FDT if the BBMD also supports foreign device registration) as described in  Clause
J.4.5.

Upon receipt of a Forwarded-NPDU from a peer BBMD, the BBMD shall re-transmit the message using the B/IP-M group address
(as well as direct it to any devices in its FDT if the BBMD also supports foreign device registration).

Upon receipt of a BVLL Distribute-Broadcast-To-Network message from a registered foreign device, the receiving BBMD shall
transmit a BVLL Forwarded-NPDU message using the B/IP-M group address as the destination address. In addition, a Forwarded-
NPDU message shall be sent to each entry in its BDT as described in  Clause  J.4.5 as well as directly to each foreign device
currently in the BBMD's FDT except the originating node. Error processing is as described in Clause J.4.5,

ANSI/ASHRAE Standard 135-2024

1143

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

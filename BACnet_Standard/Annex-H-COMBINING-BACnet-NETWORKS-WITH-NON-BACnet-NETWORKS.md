ANNEX H - COMBINING BACnet NETWORKS WITH NON-BACnet NETWORKS (NORMATIVE)

generated, it shall generate another random instance VMAC address. Once a node obtains a BACnet device instance, the node
shall cease using the random VMAC and shall start using the regular device instance VMAC as described above.

The random portion of a random instance VMAC address is a number in the range 0 to 4194303. The resulting random instance
VMAC address is in the range 4194304 to 8388607 (X'100000' to X'7FFFFF '). The generation of a random instance VMAC
shall yield any number in the entire range with equal probability. A random instance VMAC is formatted as follows:

   Bit Number:   7   6   5   4   3   2   1   0
               |---|---|---|---|---|---|---|---|
               | 0 | 1 |    High 6 Bits        |
               |---|---|---|---|---|---|---|---|
               |           Middle Octet        |
               |---|---|---|---|---|---|---|---|
               |            Low Octet          |
               |---|---|---|---|---|---|---|---|

H.7.3 EUI-48 and Random-48 VMAC Address

When a particular data link layer specifies that a EUI-48 VMAC is to be used, then the device shall use a 6-octet VMAC in the
form of an IEEE EUI-48 identifier. The means of obtaining or generating the EUI-48 identifier is a local matter. For example,
if a device has a physical Ethernet adapter, and there is only one BACnet device hosted by that adapter on a particular BACnet
network, then the Ethernet hardware address would be an appropriate choice for the initial value of the VMAC.

The Random-48 VMAC is a 6-octet VMAC address in which the least significant 4 bits (Bit 3 to Bit 0) in the first octet shall
be B'0010' (X'2'), and all other 44 bits are randomly selected to be 0 or 1. The generation of a Random-48 VMAC shall yield
any Random-48 VMAC in the entire range with equal probability.

To ensure that the VMAC is not used by another device, the device shall attempt to resolve its own VMAC in the network. If
the device detects that another device is already using this VMAC, the device shall generate a new Random-48 VMAC address
and try again.

The values X'000000000000' and X'FFFFFFFFFFFF' are not valid as VMAC addresses for a device and can have other uses
defined by the data link.

1126

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

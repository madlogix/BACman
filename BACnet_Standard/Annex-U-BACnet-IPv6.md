ANNEX U -  BACNET/IPv6 (NORMATIVE)

Foreign-Device  message  to  an  appropriate  BBMD  and  receiving  a  BVLC-Result  message  containing  a  result  code  of
'Successful completion' indicating the successful completion of the registration.

U.4.5.2

BACnet /IPv6 Foreign Device Table

The FDT shall consist of zero or more FDT entries. Each entry shall contain the B/IPv6 address and the TTL of the registered
foreign device.

U.4.5.3 Use of the BVLL Register-Foreign-Device Message

Upon  receipt  of  a  BVLL  Register-Foreign-Device  message,  a  BBMD  configured  to  accept  foreign  device  registration  and
having available table entries, shall add an entry to its FDT  as described in  Clause  U.4.5.2 and reply with a BVLC-Result
message containing a result code of 'Successful completion' indicating the successful completion of the registration. A BBMD
that does not have an available table entry of that is not configured to accept foreign device registrations shall return a BVLC-
Result message containing a result code of 'Register-Foreign-Device NAK' indicating that the registration has failed.

Upon receipt of a BVLL Register-Foreign-Device message, a BACnet/IPv6 device that is not configured as a BBMD shall
return a BVLC-Result message containing a result code of 'Register-Foreign-Device NAK' indicating that the registration has
failed.

U.4.5.4 Use of the BVLL Delete-Foreign-Device-Table-Entry Message

Upon receipt of a BVLL Delete-Foreign-Device-Table-Entry message, a BBMD shall search its foreign device table for an
entry corresponding to the B/IPv6 address supplied in the message. If an entry is found, it shall be deleted and the BBMD shall
return a BVLC-Result message to the originating device with a result code of 'Successful completion'. Otherwise, the BBMD
shall return a BVLC-Result message to the originating device with a result code of 'Delete-Foreign-Device-Table-Entry NAK'
indicating that the deletion attempt has failed.

Upon receipt of a BVLL Delete-Foreign-Device-Table-Entry message, a BACnet/IPv6 device that is not configured as a BBMD
shall return a BVLC-Result message containing a result message containing a result code of 'Delete-Foreign-Device-Table-
Entry NAK' indicating that the deletion attempt has failed.

U.4.5.5 Foreign Device Table Timer Operation

Upon receipt of a BVLL Register-Foreign-Device message, a BBMD shall start a timer with a value equal to the Time-to-Live
parameter supplied plus a fixed grace period of 30 seconds. If, within the period during which the timer is active, another BVLL
Register-Foreign-Device message from the same device is received, the timer shall be reset and restarted. If the time expires
without the receipt of another BVLL Register-Foreign-Device message from the same foreign device, the FDT entry for this
device shall be cleared.

U.5 BACnet /IPv6 VMAC Table Management

The Virtual MAC address table shall be updated using the respective parameter values of the incoming messages. For outgoing
messages to a VMAC address that is not in the table, the device shall transmit an Address-Resolution message. The Virtual
MAC Address table shall be updated with the values conveyed in the Address-Resolution-ACK message.

To learn the VMAC address of a remote BACnet device with a known B/IPv6 address, a B/IPv6 node may send a Virtual-
Address-Resolution message to that device and use the information of the Virtual-Address-Resolution-ACK message to update
the VMAC table.

Upon receipt of a Virtual-Address-Resolution message, the receiving node shall construct a Virtual-Address-Resolution-ACK
message whose Source-Virtual-Address contains its virtual address and transmit it via unicast to the B/IPv6 node that originally
initiated the Virtual-Address-Resolution message.

Upon  receipt  of  an  Address-Resolution  or  Forwarded-Address-Resolution  message  whose  target  virtual  address  is  itself,  a
B/IPv6 node shall construct an Address-Resolution-ACK message and send it via unicast to the B/IPv6 node that originally
initiated the Address-Resolution message.

In addition to forwarding NPDUs to other BBMDs and foreign devices, a B/IPv6 BBMD is used in determining the VMAC
address of a B/IPv6 node that is not reachable by multicasts or is registered as a foreign device. See Clause U.4.4.

1296

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

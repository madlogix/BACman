ANNEX R - MAPPING NETWORK LAYER ERRORS (NORMATIVE)

(This annex is part of this standard and is required for its use.)

This annex describes the mapping of network layer and BVLL layer errors to application errors to allow for reporting of errors
up the BACnet stack to the application program. This allows recording of errors by the application entity in a singular format.

There is no requirement that all of these errors be passed to the application layer, but when errors are provided to the application
layer, these mappings shall be used. There are cases, such as the receipt of Reject-Message-To-Network messages, where there
is no simple method for associating the error with the original request.

Table R-2. Mapping Reject-Message-To-Network Reasons to Error Class and Error Code Pairs

Reject-Message-To-Network Reason
0
1
2
3
4
5
6

Error Class / Error Code

COMMUNICATION / OTHER
COMMUNICATION / NOT_ROUTER_TO_DNET
COMMUNICATION / ROUTER_BUSY
COMMUNICATION / UNKNOWN_NETWORK_MESSAGE
COMMUNICATION / MESSAGE_TOO_LONG
COMMUNICATION / SECURITY_ERROR
COMMUNICATION / ADDRESSING_ERROR

Table R-3. Mapping BVLL Errors to Error Class and Error Code Pairs

Error Condition

Error Class / Error Code
Write-Broadcast-Distribution-Table NAK  COMMUNICATION / WRITE_BDT_FAILED
Read-Broadcast-Distribution-Table NAK  COMMUNICATION / READ_BDT_FAILED
Register-Foreign-Device NAK
Read-Foreign-Device-Table NAK
Delete-Foreign-Device-Table-Entry NAK  COMMUNICATION / DELETE_FDT_ENTRY_FAILED
Distribute-Broadcast-To-Network NAK

COMMUNICATION / REGISTER_FOREIGN_DEVICE_FAILED
COMMUNICATION / READ_FDT_FAILED

COMMUNICATION / DISTRIBUTE_BROADCAST_FAILED

1278

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX K – BACnet INTEROPERABILITY BUILDING BLOCKS (BIBBs) (NORMATIVE)

K.9 Authentication and Authorization BIBBs

K.9.1 BIBB - Dynamic Authorization Client - A (AA-DAC-A)

The A device is an authorization client that is capable of getting its own access tokens from an authorization server. When a
protected operation fails with an authorization error code like CONFIG_SCOPE_REQUIRED, a device claiming conformance
to AA-DAC-A will attempt to retrieve an access token for the required scope from the authorization server configured into its
Authorization_Server property.  See Clause 17.

BACnet Service

AuthRequest

Initiate
x

Execute

Devices claiming conformance to this BIBB are interoperable with devices claiming conformance to AA-AS-B and AA-AT-
B.

K.9.2 BIBB - Static Authorization Client - A (AA-SAC-A)

The  A  device  is  an  authorization  client  that  accepts  configuration  of  access  tokens  by  a  helper  tool.  The  manner  of  such
configuration is a local matter. The capacity of such configuration shall be claimed on the PICS.  The device shall support a
minimum capacity to hold two access tokens. The device shall support the Authorization_Cache property with a minimum
capacity  of  two  tokens.  However,  it  is  recommended  that  the  Authorization_Cache  have  the  capacity  to  indicate  the  same
number of tokens that can be configured. See Clause 17.

Devices claiming conformance to this BIBB are interoperable with devices claiming conformance to AA-AT-B.

K.9.3 BIBB - Authorization Target - B (AA-AT-B)

The  B  device  is  a  target  device  that  has  protected  operations  that  require  authorization.  The  B  device  has  an
Authorization_Server property that supports both of the signing keys and a group membership list with at least two members.
The Authorization_Policy property shall be present and support at least four distributed policies. Support for all optional policy
fields is required. See Clause 17.

The B device shall process access tokens presented to it by an authorization client.

The B device shall generate appropriate authorization error codes like CONFIG_SCOPE_REQUIRED.  If the device supports
non-standard  scopes,  then  the  device  shall  also  generate  "Hint"  data  attribute  for  the  error  responses  and  the
Authorization_Scopes property shall be present.

Devices claiming conformance to this BIBB are interoperable with devices claiming conformance to AA-DAC-A and AA-
SAC-A.

K.9.4 BIBB - Non-secure Authorization Target - B (AA-NAT-B)

The B device  is a non-secure device  that nonetheless supports authorization policies to create an  "allow list" for protected
operations to defend against misconfigured devices. The Authorization_Policy property shall be present and support at least
four distributed policies. Support for all optional policy fields is required. See Clause 17.

K.9.5 BIBB - Authorization Server - B (AA-AS-B)

The A device is an authorization server that is capable of issuing access tokens to authorization clients or their helpers. See
Clause 17.

AuthRequest

BACnet Service

Initiate

Execute
x

Devices claiming conformance to this BIBB are interoperable with devices claiming conformance to AA-DAC-A.

ANSI/ASHRAE Standard 135-2024

1195

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

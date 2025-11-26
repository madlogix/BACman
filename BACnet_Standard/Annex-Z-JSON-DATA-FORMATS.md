ANNEX Z - JSON DATA FORMATS (NORMATIVE)

(c)  A  period  character  ".".  While  not  required,  it  is  recommended  that  proprietary  attributes  beginning  with  a  period
character  also  use  a  vendor-specific  prefix,  following  the  required  period  character,  to  prevent  conflicts  among
proprietary attributes. Option 3 is retained for historical compatibility; new implementations are required to use option
1 or 2.

While this clause provides the syntax and method for extending the standard metadata, it makes no requirement that consumers
of this JSON understand or process any of these extensions. Consumers are allowed to consume extensions that are known to
the consumer and to ignore the rest.

The following example shows the standard metadata, 'maximumLength', being extended with the standard metadata 'writable'
and 'writeEffective', and a standard String being extended with a proprietary extension "555-UIGroup".

{ "$$definitions": {
       "555-ExampleObject": {
          "$base":"Object",
          "write-me":{
              "$base":"String",
              "$writable":true,
              "$maximumLength":{"$value":50,"$writable":true,"$writeEffective":"on-device-restart"},
              "$extensions":{
                   "555-UIGroup": { "$base":"Integer", "$value":6 }
               }
          }
      }
   }
}

ANSI/ASHRAE Standard 135-2024

1451

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AA - TIME SERIES DATA EXCHANGE FILE FORMAT (NORMATIVE)

ANNEX AA –FILE FORMATS

(This annex is part of this standard and is required for its use.)

AA.1 Time Series Data Exchange File Format (NORMATIVE)

Collected  trend  data  has  value  in  many  third-party  applications,  including  building  energy  optimization  using  energy
information  systems,  trend  analysis  for  one-time  building  assessments,  continuous  commissioning,  and  fault  detection and
diagnostics. To facilitate standard exchange of this data, its format in text files is defined here.

This  format  is  designed  for  export  from  databases  in  servers  or  workstation-class  computers  with  user  interfaces.  It  is  not
intended to replace the BACnet ReadRange  service  in devices or the queryable history services provided by BACnet Web
Services.

This file format defines a series of records, each with a single time stamp associated with value(s) for one or more sources
selected by the user. The number of sources present in a single file is a local matter and is limited only by the capabilities of
the generating software and the intended consuming software.

AA.1.1 File Format

Trend  data  shall  be  exported  as  UTF-8  text  files  in  CSV  format  as  specified  by  RFC  4180,  with  columns  comprised  of  a
timestamp and associated data values. Note that RFC 4180 specifies the rules for quoting when fields contain commas or quotes
and the requirements for line terminations.

The header line shall be present to describe the data in each column. Subsequent rows shall contain the time stamp in column
one and the associated data values in other columns. Rows shall be in ascending order of date and time.

The column name for column one shall be "DateTime". The names for the other columns shall be nonempty printable strings,
each limited to 80 characters.

The timestamp column shall use the format defined by XML Schema xs:dateTime, Fractional seconds are optional. However,
the time zone indicator is required.

AA.1.2 Representation of Data

Each data value shall be represented as a string that is appropriate for the data type, and shall be formatted as if returned in
'plain text' from the services described in Clause W.9, plus the requirements for quoting specified by RFC 4180. Only primitive
data types can be represented. The following table summarizes the requirements made by Clause W.9, which in turn references
clauses in Annex Q and Annex Y.

Source Data Type

Serialization Type

Examples

BitString

xs:string

Boolean

xs:boolean

Date
Date with unspecified
value
DatePattern

xs:date
xs:string

xs:string

DateTime

xs:dateTime

DateTime with
unspecified value
DateTimePattern

xs:string

xs:string

fault
fault;overridden
<empty string>

true
false
2018-01-24
----/--/--

*-01-24
2018-01-*
*-*-*
2018-01-24T08:56:00+01:00
2018-01-24T07:56:00.00Z
----/--/--T--:--:--Z

Notes
A semicolon separated
list of the names of the
bits that are true. i.e., an
empty string means that
all bits are false.
See Clause Y.12.11

See Clause W.9

See Clause Y.12.14

Time zone indicator is
required for CSV files
See Clause W.9

2018-01-24 10:*:*.*

See ClauseY.12.16

1452

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AA -  TIME SERIES DATA EXCHANGE FILE FORMAT (NORMATIVE)

Double
Enumerated

xs:double
xs:string

xs:integer
Integer
xs:string
Link
ObjectIdentifier
xs:string
ObjectIdentifierPattern  xs:string
OctetString

xs:hexBinary

Raw

Real
String

StringSet
Time

xs:hexBinary

xs:float
xs:string

xs:string
xs:time

Time with unspecified
value
TimePattern

xs:string

xs:string

*-*-* 3 10:00:00.00
*-*-* *:*:*.*
123456789.00
high
idle
1234
http://example.com/abc
calendar,12
calendar,*
0103CAFEBABE99
0103cafebabe99
0103CAFEBABE99
0103cafebabe99
1234.56
hello world
"hello, world"

foo;bar;baz
12:05:22
12:05:22.55
--:--:--

10:24:*.*
*:*:*.*

Unsigned
WeekNDay

xs:nonNegativeInteger  1234
xs:string

"1,1,*"

See Clause Y.12.12

See Clause Y.20.1
See Clause Y.20.2

Embedded commas
need to be quoted in
CSV
See Clause Y.12.10.
Fractional seconds is
optional
See Clause W.9

See Clause Y.12.18

See Clause Y.20.3

Missing data shall be represented by a string consisting of a question mark followed by a space followed by a decimal error
number defined by Table W-14. See Clause W.40.

AA.1.3 File Generation

As an example implementation, a system for collecting or archiving trends could provide a menu in the user interface of the
system that allows the export of trend data in a standardized format. The export function would incorporate options to limit the
data exported to certain time periods and/or to certain trend sources. The means of specifying what to export is a local matter.

A properly formatted value shall be present for each source for each time stamp. If a value for a source is not available for a
particular timestamp, an error string, as specified in Clause AA.2, shall be present in that column. It is a local matter what
"available" means. As an example implementation, the generating software could have user selectable options for whether to
interpolate or otherwise generate an appropriate value for a given time stamp.

The names of the columns shall be under user control within the limitations specified by Clause AA.1. The generating software
shall, by default, enforce that the column names are unique within the file, unless explicitly overridden by the user.

The name of the exported file is a local matter with the exception that either the user shall be given the opportunity to name the
file or the name of trend source shall be incorporated into the file name so as to make it recognizable to the user and unique
among other exported files.

AA.1.4 Example Files

Example of a simple single-value CSV file:

DateTime,B8-Plant-CH3-CHWS-Temp-F
2019-06-16T13:01:02-08:00,42.0
2019-06-16T13:06:02-08:00,42.5
2019-06-16T13:11:02-08:00,42.3

Example of a CSV file with multiple values and empty bitstrings:

ANSI/ASHRAE Standard 135-2024

1453

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AA - TIME SERIES DATA EXCHANGE FILE FORMAT (NORMATIVE)

DateTime,B8-Plant-CH3-CHWS-Temp-F,Status_Flags,Running
2019-06-16T21:01:02Z,42.0,,active
2019-06-16T21:06:02Z,0.0,fault;alarm,active
2019-06-16T21:11:02Z,42.3,,active

Example of a CSV file with a missing value:

DateTime,SomeData1,SomeData2
2019-06-16T21:01:02Z,74.0,75.5
2019-06-16T21:06:02Z,? 24,75.7
2019-06-16T21:11:02Z,74.2,75.3

AA.2 Certificate Authority Requirements Interchange File Format (NORMATIVE)

This annex describes an interoperable file format that allows one or more devices to package their Certificate Signing Request
(CSR) files into a single file. This file is processed by the site Certificate Authority (CA) and, if successful, the CA appends
each device's Issuer and Operational certificate files to the received file. This annex does not specify the delivery mechanism
to exchange these files.

AA.2.1 File Format

The file format is a strict structure of folders and files that provides the information and context necessary to allow the site CA
to  process  CSR  files  and  generate  operational  and  issuer  certificates  for  devices  specified  in  the  file.  This  folder  and  file
structure is compressed using the zip file format into a single file.

The content of the request file format for the CA is specified in Clause AA.2.1.1 and response content from the CA is specified
in Clause AA.2.1.2.

The file format shall contain only folders and files specified in clauses AA.2.1.1 and AA.2.1.2. All text files shall be UTF-8
encoded.

AA.2.1.1 Request File Format

cert1/

vendor-data
request-notes.txt
device-<instance>/

port-<id>/

device-<instance>/

port-<id>/

device-<instance>/
router/
port-<id>/

port-<id>/

csr-<string>.pem

hub/
csr-<string>.pem

csr-<string>.pem

hub/
csr-<string>.pem

Figure AA.2-1 provides the request file hierarchy of folders and files destined for the CA.

Figure AA.2-1 Example Request File Format

The required root folder of the request file shall be 'cert1' and contains all the folders and files required for the CA to generate
device certificates.

1454

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AA -  TIME SERIES DATA EXCHANGE FILE FORMAT (NORMATIVE)

The optional file named 'vendor-data' contains vendor specific data. The content of this file is a local matter but shall be limited
to 1 megabyte.

The optional file named 'request-notes.txt' is a free form human readable text file. The content of this file is a local matter but
shall be limited to 10 kilobytes. This file can be used to document any aspect of the request phase of the exchange. The file
could be used to provide comments for the approver.

Each request file shall contain one or more device folders with the name of the device folder made up of 'device-' concatenated
with the device's object instance.

If a device routes between BACnet/SC networks, it shall contain an empty sub-folder named 'router'. See Clause 6.6.

Each device folder shall contain one or more port folders. The name of each port folder shall be 'port-<id>/' where <id> is a
vendor specific value and shall be unique for each port folder. <id> shall be any printable character except for '<' (less than),
'>' (greater than), ':' (colon), '"' (double quote), '/' (forward slash), '\' (backslash), '|' (vertical bar or pipe), '?' (question mark),
and '*' (asterisk).

Each  port  folder  shall  contain  a  file  named  'csr-<string>.pem'  where  'string'  shall  be  any  printable  string  that  contains  any
characters except for '<' (less than), '>' (greater than), ':' (colon), '"' (double quote), '/' (forward slash), '\' (backslash), '|' (vertical
bar or pipe), '?' (question mark), and '*' (asterisk). This file is the PKCS#10 Certificate Signing Request file for this port. The
port folder can optionally contain a key-<string>.pem file that is the private key corresponding to the CSR file. This file is
ignored by the server but will be preserved in the response file.

If a port represents a hub function, the port folder shall contain an empty folder named 'hub'. See Clause AB.1.2.

AA.2.1.2 Response File Format

cert1/

vendor-data
request-notes.txt
response-notes.txt
errors.txt
device-<instance>/

port-<id>/

device-<instance>/

port-<id>/

device-<instance>/
router/
port-<id>/

csr-<string>.pem
opr-<string>.pem

hub/
csr-<string>.pem
opr-<string>.pem

csr-<string>.pem

opr-<string>.pem

port-<id>/

hub/
csr-<string>.pem
opr-<string>.pem

issuer/

iss-1.pem
iss-2.pem

Figure AA.2-2 Example Response File Format

Figure AA.2-2 provides the response file hierarchy of folders and files.

ANSI/ASHRAE Standard 135-2024

1455

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AA - TIME SERIES DATA EXCHANGE FILE FORMAT (NORMATIVE)

The response file format shall contain all files and folders included in the request file and the files and folders specified in this
clause. If any of the files specified below exist in the request file, they will be overwritten or deleted in the response file.

The optional file named 'response-notes.txt' is a free form human readable text file. The content of this file is a local matter and
shall be limited to 10 kilobytes. This file could indicate why a certificate request was denied.

The conditional 'errors.txt' file contains a single text line for every error that is encountered during the processing of the CSR
files. If errors are encountered, the 'errors.txt' shall be present, otherwise it shall be absent.

If the operational certificate file exists for the corresponding CSR file, the port folder shall contain the operational certificate
file. The operational certificate file shall be in PEM format and named 'opr-<string>.pem' where 'string' matches the 'string' in
the name of 'csr-<string>.pem'. This file is destined for the file referenced by the Operational_Certificate_File property of the
Network Port object for the port. See Clause 12.56.8 and Clause AB.7.4.1.1.

If the operational certificate file does not exist for the corresponding CSR file, the errors.txt file shall contain an error that is a
tab separated string with 'device-<instance>', 'port-<id>', and an optional human readable description of the error.

Each response file shall contain a subfolder of 'cert1' named 'issuer'. This folder shall contain at least one and no more than two
issuer certificate files. These certificate files are destined for the files referenced by the Issuer_Certificate_Files property of the
Network Port objects. See Clause 12.56.99. These files shall be named 'iss-1.pem' and 'iss-2.pem'.

1456

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

ANNEX AB – BACnet Secure Connect (NORMATIVE)

(This annex is part of this standard and is required for its use.)

This annex defines a data link protocol by which BACnet devices can transfer messages utilizing the WebSocket protocol as
specified in RFC 6455. The Request For Comments (RFC) documents that define the WebSocket protocol are maintained by
the Internet Engineering Task Force (IETF).

AB.1 BACnet Secure Connect Data link

The BACnet Secure Connect, or BACnet/SC, data link layer specifies a microprotocol enabling the use of WebSocket based
connections, specifically the TLS-secured variant, for the exchange of BACnet messages between nodes. As a WebSocket
based protocol, this data link can be implemented on any IPV4 or IPv6 network, including Ethernet, Fiber, WiFi, RFC 8163
MS/TP, and many others.

The logical topology of a BACnet/SC network generally follows a hub-and-spoke model consisting of multiple BACnet/SC
nodes and a hub function. See Figure AB-1. A BACnet/SC node wishing to participate in a BACnet/SC network establishes a
hub connection to a hub function. This annex specifies the BACnet/SC hub function based on BACnet/SC connections which
all BACnet/SC nodes shall be able to connect to.

Optionally,  for  transmitting  unicast  BACnet  messages,  BACnet/SC  nodes  may  support  direct  connections  with  other
BACnet/SC nodes on the same BACnet network, as a BACnet/SC connection initiating peer, or as BACnet/SC connection
accepting peer.

BACnet/SC connections use the secure variant of the WebSocket connections for bi-directional BACnet Virtual Link Layer
Control (BVLC) messages.

Figure AB-1. BACnet/SC Logical Network Topology

For enhanced availability of the central hub function for a BACnet/SC network, the hub connector specifies a failover hub
concept in which the failover hub can be used in the case that the primary hub function is not available or not reachable.

AB.1.1 BACnet/SC Nodes

A BACnet/SC node is a network port that implements a BACnet/SC Virtual Link Layer (BVLL) entity for link control and
NPDU transport, and the hub connector for connecting to the hub function to participate in the BACnet/SC network.

Figure AB-2 illustrates a BACnet device implementing a BACnet/SC node.

ANSI/ASHRAE Standard 135-2024

1457

IPv4 and/or IPv6 Network InfrastructureBACnet/SC NodeHub Connection Hub FunctionWebSocketClientWebSocketServerDirect Connection(Optional)WebSocketClientWebSocketServerCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Figure AB-2. Example BACnet/SC Device

The BVLL for BACnet/SC defines the BACnet/SC Virtual Link Control (BVLC) messages that are used to control the virtual
link and to convey BACnet NPDUs.

AB.1.1.1 BVLL Entity

The BVLL entity of a network port is the initiating and executing entity of BVLC messages and is identified in the BACnet/SC
network by the VMAC of the node. See Clause AB.1.5.2.

AB.1.1.2 Hub Connector

The hub connector is required in a BACnet/SC node and maintains one initiated hub connection to a hub function at a time.
For enhanced availability of the hub function of a BACnet/SC network, the hub connector  shall support initiating a connection
to the primary hub function, referred to as the primary hub connection, and shall support initiating a connection to the failover
hub function, referred to as the failover hub connection and to be used when the primary hub function is not available. See
Clause AB.3.

The Hub Connector shall support connecting to the BACnet/SC hub function by initiating BACnet/SC connections for the
primary hub connection and for the failover hub connection.

AB.1.1.3 Optional Node Switch and Direct Connections

Optionally, a BACnet/SC node may support initiating or accepting WebSocket connections as BACnet/SC direct connections.
See Clause AB.1.3. The support of direct connections requires a node switch function which is the endpoint of all WebSocket
connections initiated or accepted by the BACnet/SC node as direct connections.

Figure AB-3 illustrates a BACnet device implementing a BACnet/SC node with support of direct connections.

1458

ANSI/ASHRAE Standard 135-2024

BACnet/SC DeviceBACnet/SC Virtual Link Layer(BVLL) EntityPrimary Hub ConnectionInitiated to Hub FunctionBACnet Network LayerVMACBACnet ApplicationApplication Layer BACnet/SC NodeHub ConnectorFailover Hub ConnectionInitiated to Hub Function(If no primary hub connection)Network PortCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Figure AB-3. Example BACnet/SC Device Supporting Direct Connections

The  BACnet/SC  node  switch  function  dispatches  messages  from  the  local  BVLL  entity  to  a  direct  connection  or  the  hub
connector, and from the hub connector or a direct connection to the local BVLL entity. For direct connections and the node
switch function, see Clause AB.4.

AB.1.2 Hub Function

For every BACnet/SC network, one hub function is required. This hub function is referred to as the primary hub function for
the BACnet/SC nodes.

Optionally, for enhanced availability, an additional hub function may be present and is used by the BACnet/SC nodes as the
failover hub function. The distinction of which is the primary hub function, and which is the failover hub function, is a site
specific determination, and configured into the BACnet/SC nodes accordingly.

Figure AB-4. Example Failover Situation

ANSI/ASHRAE Standard 135-2024

1459

BACnet/SC DeviceBACnet/SC Virtual Link Layer(BVLL) EntityPrimary Hub ConnectionInitiated to Hub FunctionBACnet Network LayerVMACBACnet Application  Application LayerBACnet/SC NodeHub ConnectorFailover Hub ConnectionInitiated to Hub Function(If no primary hub connection)Node SwitchInitiated Direct ConnectionsInitiated WebSockets Accepted Direct ConnectionsAccepted WebSockets Network PortBACnet/SC NetworkNormal SituationBACnet/SC NodePrimary Hub ConnectionsFailover Hub ConnectionsPrimary Hub FunctionPPFailover Hub FunctionFFFailover SituationPFCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

The  hub  function  forwards  unicast  messages  received  from  one  hub  connection  to  another  hub  connection  and  distributes
broadcast messages to all hub connections.

This annex defines the BACnet/SC hub function that all BACnet/SC nodes are required to be able to connect to.

For the BACnet/SC hub function see Clause AB.5.3. The BACnet/SC hub function accepts secure WebSocket connections as
BACnet/SC connections. See Clause AB.1.3.

AB.1.3 BACnet/SC Connections

For each hub connection to the BACnet/SC hub function and for each direct connection, one WebSocket connection is used
for one BACnet/SC connection for bi-directional BACnet Virtual Link Control (BVLC) message exchange.

After establishing a WebSocket connection, the BACnet/SC connection establishment phase detects error situations before the
BACnet/SC connection is established and can be used for general and bi-directional BVLC message transmission. See Clause
AB.6.1.

The  WebSocket protocol as define  in RFC 6455 is used for establishment of WebSocket connections for BACnet/SC. See
Clause AB.7.

AB.1.4 Service Specification

This clause describes the primitives and parameters associated with the services the BACnet/SC BVLL entity is providing to
the BACnet network layer. The parameters are described in an abstract sense, which does not constrain the implementation
method. Primitives and their parameters are described in a form that echoes their specification in ISO 8802-2. This is intended
to provide a consistent interface to the BACnet network layer.

In addition to other data link service primitives, these primitives support a 'data_attributes' parameter that specifies attributes
to the 'data' parameter that can be forwarded by the BACnet network layer to reach the final destination of the Encapsulated-
NPDU's payload. See Clause 6.6.

In BACnet/SC, the 'data_attributes' parameter is supported and conveys the data options to be sent or received.

AB.1.4.1 DL-UNITDATA.request

AB.1.4.1.1 Function

This primitive is the service request primitive for the unacknowledged connectionless-mode data transfer service.

AB.1.4.1.2 Semantics of the Service Primitive

The primitive shall provide parameters as follows:

DL-UNITDATA.request (
source_address,
destination_address,
data,
priority,
data_attributes
)

Each source and destination address consists of the logical concatenation of a medium access control (MAC) address and a link
service access point (LSAP). For the case of BACnet/SC network ports, since the data link interface supports only the BACnet
network layer, the LSAP is omitted and these parameters consist of only the VMAC address. See Clause AB.1.5.2

The 'data' parameter specifies the link service data unit (LSDU) to be transferred by the BACnet/SC network port.

The 'priority' parameter specifies the priority desired for the data unit transfer. The priority parameter is ignored by BACnet/SC.

The 'data_attributes' parameter provides attributes for the content of the 'data' parameter. For a BACnet/SC network port, this
parameter specifies the header options which shall be included in the 'Data Options' parameter in all BVLC messages resulting
from this primitive.

1460

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.1.4.1.3 When Generated

This primitive is passed from the network layer to the BACnet/SC BVLL entity to request that a network protocol data unit
(NPDU) be sent to one or more remote LSAPs using unacknowledged connectionless-mode procedures.

AB.1.4.1.4 Effect on Receipt

Receipt  of  this  primitive  causes  the  BACnet/SC  BVLL  entity  to  attempt  to  send  the  NPDU  using  unacknowledged
connectionless-mode procedures.

AB.1.4.2 DL-UNITDATA.indication

AB.1.4.2.1 Function

This primitive is the service indication primitive for the unacknowledged connectionless-mode data transfer service.

AB.1.4.2.2 Semantics of the Service Primitive

The primitive shall provide parameters as follows:

DL-UNITDATA.indication (

source_address,
destination_address,
data,
priority,
data_attributes
)

Each source and destination address consists of the logical concatenation of a medium access control (MAC) address and a link
service  access  point  (LSAP).  For  the  case  of  BACnet/SC  devices,  since  the  data  link  interface  supports  only  the  BACnet
network layer, the LSAP is omitted and these parameters consist of only the VMAC address. See Clause AB.1.5.2.

The 'data' parameter specifies the link service data unit (LSDU) received by the BACnet/SC network port.

The  'priority'  parameter  specifies  the  priority  desired  for  the  data  unit  transfer.  The  priority  parameter  is  not  provided  by
BACnet/SC.

The 'data_attributes' parameter provides attributes for the content of the 'data' parameter. For a BACnet/SC network port, this
parameter includes the information that was received in the 'Data Options' parameter of the BVLC message received.

AB.1.4.2.3 When Generated

This primitive is passed from the BACnet/SC entity to the network layer to indicate the arrival of an NPDU from the specified
remote entity.

AB.1.4.2.4 Effected on Receipt

The effect of receipt of this primitive by the network layer is specified in Clause 6.

AB.1.4.3 DL-RELEASE.request

AB.1.4.3.1 Function

This primitive is the service request primitive for the request to release the data link state machine from waiting on a response
message.

AB.1.4.3.2 Semantics of the Service Primitive

The primitive shall not provide any parameters as follows:

DL-RELEASE.request()

AB.1.4.3.3 When Generated

This primitive is passed from the network layer to the BACnet/SC BVLL entity to indicate that no reply is available from the
higher layers.

ANSI/ASHRAE Standard 135-2024

1461

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.1.4.3.4.Effected on Receipt

In BACnet/SC, there is no effect of receipt of this primitive.

AB.1.5 Addressing within BACnet/SC Networks

AB.1.5.1 Network Location of Nodes

The network and resource location of the BACnet/SC hub function and the node switch of BACnet/SC nodes accepting direct
connections are specified by WebSocket URIs as defined by the "wss" URI scheme in RFC 6455, Section 3.

The URIs to be used to connect to a hub function shall be configurable and shall be used only to connect to the hubs. See Clause
AB.5.

For a direct connection to a node, the WebSocket URIs of that node where direct connections are accepted can be requested by
initiating an Address-Resolution BVLC message sent to that node through the hub function. The Address-Resolution-ACK
BVLC response can provide the possible WebSocket URIs where that node accepts WebSocket connections for BACnet/SC
direct connections. See Clause AB.4.

Optionally, the  WebSocket URIs for a  direct connection to a  node may be configured in the initiating node and shall take
precedence over what is received in Address-Resolution-ACK messages from the responding node.

AB.1.5.2 VMAC Addressing of Nodes

For the BVLC message exchange, BACnet/SC nodes are identified by their 6-octet virtual MAC address as defined in Clause
H.7.3.

For broadcast BVLC messages that need to reach all nodes of the BACnet/SC network, the destination VMAC address shall
be the non-EUI-48 value X'FFFFFFFFFFFF', referred to as the Local Broadcast VMAC address.

The reserved EUI-48 value X'000000000000' is not used by this data link and therefore can be used internally to indicate that
a VMAC is unknown or uninitialized.

AB.1.5.3 Device UUID

Every BACnet device that supports one or more BACnet/SC network ports shall have a Universally Unique ID (UUID) as
defined in RFC 4122. This UUID identifies the device regardless of its current VMAC address or device instance number and
is referred to as the device UUID.

This device UUID shall be generated before first deployment of the device in an installation, shall be persistently stored across
device restarts, and shall not change over the entire lifetime of a device.

If  a  device  is  replaced  in  an  installation,  the  new  device  is  not  required  to  re-use  the  UUID  of  the  replaced  device.  For
BACnet/SC, it is assumed that existing connections to the device being replaced are all terminated before the new device comes
into operation.

AB.1.6 BACnet/SC Network Definition

A BACnet network based on the BACnet/SC data link option is referred to as a BACnet/SC network. A BACnet/SC network
is  a  set  of  two or  more  BACnet/SC  nodes  in  which  all  nodes  connect  to  the  same  primary  hub function.  In  a  BACnet/SC
network, one hub function used as the primary hub shall be present. The presence of a hub function used as the failover hub is
optional.

Direct connections between nodes shall only be established between nodes of the same BACnet/SC network. Nodes using
direct connections shall remain connected to the hub. Broadcast BVLC messages shall always and only be sent to the hub
function for distribution.

Only one direct connection shall exist at a time between any two BACnet/SC nodes, regardless of which node initiated or
accepted the WebSocket connection.

1462

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.1.7 Remote MAC Addressing of Devices on BACnet/SC Networks

In  BACnet  network  layer  services,  application  layer  services,  and  in  data  of  type  BACnetAddress,  the  MAC  address  of  a
BACnet/SC node shall be the node's virtual MAC address as defined in Clause AB.1.5.2.

AB.1.8 BACnet/SC Network Port Objects

Participation  in  a  BACnet/SC  network  is  represented  by  single  network  port  regardless  of  the  number  of  connections  and
initiated or accepted WebSocket connections in use by the network port.

For BACnet/SC network port implementations less than protocol revision 17, the configuration of BACnet/SC network ports
is a local matter and cannot be represented by Network Port objects.

For  BACnet/SC  network  port  implementations  with  a  Protocol_Revision  17  through  Protocol_Revision  23,  BACnet/SC
network ports shall be represented by a Network Port object at the BACNET_APPLICATION protocol level with a proprietary
network type value. For the required standard properties to be present see Clause 12.56.

For BACnet/SC network port implementations with a Protocol_Revision 24 and higher, BACnet/SC network ports shall be
represented  by  a  Network  Port  object  at  the  BACNET_APPLICATION  protocol  level  with  network  type  of
SECURE_CONNECT. For the required standard properties to be present see Clause 12.56.

AB.2 BACnet/SC Virtual Link Layer Messages

The BACnet/SC Virtual Link Layer (BVLL) provides the interface between the BACnet Network Layer (See Clause 6) and
the  underlying  capabilities  of  the  communication  subsystem  based  on  WebSockets  (RFC  6455).  This  annex  specifies  the
BACnet Virtual Link Control (BVLC) functions required to transport unicast and broadcast BACnet messages, and to control
the BVLL operation. The purpose and format of each BVLC message is described in the following subclauses. The BVLL
behavior is defined in Clause AB.6.

The following table lists the BVLC messages defined for BACnet/SC.

Table AB-1 BACnet/SC BVLC Messages

BVLC Message

X'00' BVLC-Result
X'01' Encapsulated-NPDU
X'02' Address-Resolution
X'03' Address-Resolution-ACK
X'04' Advertisement
X'05' Advertisement-Solicitation
X'06' Connect-Request
X'07' Connect-Accept
X'08' Disconnect-Request
X'09' Disconnect-ACK

X'0A' Heartbeat-Request
X'0B' Heartbeat-ACK
X'0C' Proprietary-Message

BVLC Function
Respond with ACK or NAK with error details
Convey an NPDU.
Request for the WebSocket URIs accepting direct connections.
Return WebSocket URIs accepting direct connections if any.
Inform about the sender node's current status
Request for the current status of the destination node.
Request to accepting peer to accept a WebSocket connection for BACnet/SC
Response to initiating peer to accept a WebSocket connection for BACnet/SC
Request and last message sent to request disconnection of the connection.
Response and last message sent to confirm disconnection to the connection
peer
Request a heartbeat from the connection peer.
Heartbeat response to connection peer.
Proprietary extension messages

Unicast BVLC messages are addressed to a single destination node. Broadcast BVLC messages are addressed to all nodes of
the BACnet/SC network and sent by a node to the hub function for distribution to all other nodes.

Response BVLC messages are unicast BVLC messages and are returned as an immediate response to a BVLC message. The
following are response messages: BVLC-Result, Address-Resolution-ACK, Connect-Accept, Disconnect-ACK, and Heartbeat-
ACK. No response message shall be sent when a broadcast or response message is received.

AB.2.1 General BVLC Message Format

The following table shows the general BVLC message format for BACnet/SC.

ANSI/ASHRAE Standard 135-2024

1463

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Table AB-2 BACnet/SC BVLC Messages Structure

Field

Length

Description

BVLC Function
Control Flags
Message ID
Originating Virtual Address

1-octet
1-octet
2-octets
6-octets

Destination Virtual Address

6-octets

Destination Options

Data Options

Payload

Variable

Variable

Variable

BVLC function
Determines presence of optional fields.
The message identifier
Optional field, originating node VMAC
address
Optional field, destination VMAC
address
Optional field, list of header options for
the destination node
Optional field, list of header options
accompanying a payload containing
data for upper layers
Optional field, the payload of the BVLC
message

The 1-octet 'BVLC Function' field identifies the specific function to be carried out in support of the indicated communication
subsystem or microprotocol type.

The 1-octet 'Control Flags' field indicates which optional parts are present. See Clause AB.2.2

The 2-octet 'Message ID' field is a numeric identifier of the message being sent. See Clause AB.3.1.3.

The optional 6-octet 'Originating Virtual Address' field indicates the VMAC address of the node that originally initiated the
BVLC message. If the sender of the message is also the originator of the message, then the 'Originating Virtual Address' field
shall be omitted and the receiver shall assume the 'Originating Virtual Address' to be the VMAC of the sender.

The optional 6-octet 'Destination Virtual Address' field indicates the VMAC address of the destination node or the broadcast
VMAC. If the immediate receiver of a unicast BVLC message is also the final destination of the message, then the 'Destination
Virtual Address' field shall be omitted.

The optional and variable size  'Destination Options' field contains a list of zero or more header options for the destination
BACnet/SC node. See Clause AB.2.3

The optional and variable size 'Data Options' field contains a list of zero or more header options accompanying a data payload
intended for upper layers. See Clause AB.2.3.

The remaining octets of the BVLC message, if any, are the variable size 'Payload' parameter conveying the payload of the
BVLC message. See BVLC message definitions in Clause AB.2.4 and subsequent clauses.

All multi-octet numeric values are encoded with most significant octet first.

For encodings of example BVLC messages see Clause AB.2.17.

AB.2.2 Control Flags

The 'Control Flags' field indicates the presence or absence of optional fields in the BVLC message.

Bit 7..4

Reserved

Shall be zero.

Bit 3:

Bit 2:

Originating Virtual
Address Flag

1 = Originating Virtual Address is present
0 = Originating Virtual Address is absent

Destination Virtual
Address Flag

1 = Destination Virtual Address is present
0 = Destination Virtual Address is absent

Bit 1:

Destination Options

1 = Destination Options field is present

1464

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Flag

0 = Destination Options field is absent

Bit 0:

Data Options
Flag

1 = Data Options field is present
0 = Data Options field is absent

AB.2.3 Header Options

BVLC messages allow conveying header options in addition to defined payloads. Multiple header options  with the same or
different header option type may be present in each of the header options list parameters of a BVLC message.

The optional 'Destination Options' parameter is a list of header options. The header options in this list are addressed to the
destination node or nodes addressed by the 'Destination VMAC Address' parameter. Destination options are used at the data
link layer only and are limited to destination nodes within the same BACnet/SC network. Therefore, destination header options
are never passed up to or received from the network layer or application layer.

The  optional 'Data Options' parameter is a  list of header options that accompany data  payloads that are intended for upper
layers. For standard BVLC messages, this parameter shall only be present in BVLC messages that convey an NPDU, in which
case, the header options in this list are associated with an NPDU that originates at the source BACnet device and accompany
the NPDU to the ultimate destination device or devices. Because these are BACnet/SC options, they can only be conveyed to
the ultimate destination device if that device is also a BACnet/SC device and the message has not passed through any non-
BACnet/SC network segments while being routed.

When routing of an NPDU to a BACnet network of a type that does not support conveying data header options with the NPDU,
the data header options will be silently dropped and are not conveyed with the NPDU on that network. See Clause 6.5.

Each header option includes a 'Header Marker' identifying the type of the option, a 'Header Length' field, and the 'Header Data'
for the content of the header.

Header Marker

Header Length

1-octet

Flags for the header option and numeric header option type.

0 or 2-octets  Optional length of the 'Header Data' field, in octets. Present if and

only if the 'Header Data Flag' flag is set (1).

Header Data

Variable

Optional octet string as defined for the header option type. Present if
and only if the 'Header Data Flag' flag is set (1).

The 'Header Marker' octet for 'Destination Options' and 'Data Options' includes the fields as follows:

Bit 7

More Options

1 = Another header option follows in the current header option list.
0 = This is the last header option in the current header option list.

Bit 6:

Must Understand

1 = This header option must be understood for consuming the message.
0 = This header option can be ignored if not understood.

Bit 5:

Header Data Flag

1 = The 'Header Length' and 'Header Data' fields are present
0 = The 'Header Length' and 'Header Data' fields are absent

Bits 4..0:

Header Option Type  1..31, The numeric header option type.

The 'Header Marker' octet for 'Data Options' includes the fields as follows:

Bit 7

More Options

1 = Another header option follows in the current header option list.
0 = This is the last header option in the current header option list.

Bit 6:

Every Segment

1 = This header option shall be sent with every segment.
0 = This header option shall be sent with the first, or only, segment and shall
not be sent with subsequent segments.

Bit 5:

Header Data Flag

1 = The 'Header Length' and 'Header Data' fields are present

ANSI/ASHRAE Standard 135-2024

1465

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Bits 4..0:

Header Option Type  1..31, The numeric header option type.

0 = The 'Header Length' and 'Header Data' fields are absent

The 'More Options' flag indicates if the header option is the  last option in the current header options list (0), or at least one
more header option follows in the current header options list (1).

For the handling of the 'Must Understand' flag and the processing of header options when sending, forwarding, broadcasting,
or receiving BVLC messages with header options, see Clause AB.3.1.4.

For the handling of attributes designated as 'Every Segment' and the processing of header options when sending or receiving
NPDUs with data options, see Clause 5.2.1.1 and the subclauses of Clause 5.4.

The following table lists the header option types defined by this standard and assigns the numeric header option type used in
the 'Header Marker'.

Header Option Type

Secure Path
Hello
Identity
Hint
Token
Proprietary Header Option

Table AB-3 BVLC Header Options

Numeric Header
Option Type

Description

1
2
3
4
5
31

See Clause AB.2.3.1
See Clause AB.2.3.1.2
See Clause AB.2.3.1.3
See Clause AB.2.3.1.4
See Clause AB.2.3.1.5
See Clause AB.2.3.2

All other header options and numeric header option types are reserved for definition by ASHRAE.

The optional 2-octet 'Header Length' field indicates the length in octets of the 'Header Data' field. It shall be present if and only
if the 'Header Data Flag' of the header marker is set (1).

The optional and variable size 'Header Data' field is an octet string whose content is defined by the respective header option
type indicated by the 'Header Marker'. Shall be present if and only if the 'Header Data Flag' of the header marker is set (1). If
zero data octets are present, the 'Header Data' field is considered empty.

AB.2.3.1 Standard Header Options

AB.2.3.1.1 Secure Path Header Option

The 'Secure Path' header option specifies, by its presence, whether the service being requested represents a message which has
only been transferred by BACnet/SC data links and secure connect BACnet routers.

The 'Secure Path' header option consists of the following fields.

Header Marker

1-octet

'Last Option' = 0 or 1, 'Must Understand' = 1,
'Header Data Flag' = 0, 'Header Option Type' = 1

This header option, if present, shall be a data option in the 'Data Options' parameter of BVLC messages conveying an NPDU.
This header option shall be initially provided by the network or application entity initiating the payload of the NPDU being
conveyed. It shall remain with the NPDU as long as the message does not pass through any BACnet network of a type that
does not support conveying this data header option while being routed. The processing of this information when received by
the NPDU's payload final destination device's network or application entity is a local matter.

1466

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.2.3.1.2 Hello Header Option

The 'Hello' header option specifies the capabilities of a connecting peer.

The 'Hello' header option consists of the following fields.

Header Marker

Header Length
Header Data
       Capabilities

1-octet

2-octets
1-octet
1-octet

'More Options' = 0 or 1, 'Must Understand' = 0, 'Header
Data Flag' = 1, 'Header Option Type' = 2

Length of 'Header Data' field, in octets. Shall be 1
Required to include:
8 bit flags. Bit 0 is 'identity relay'. Bits 1-7 are reserved
and shall each be set to 0 and shall be ignored by the
receiver.

This  header  option,  if  present,  shall  be  a  destination  option  in  the  'Destination  Options'  parameter  of  the  BVLC  messages
'Connect Request' and 'Connect Accept'.

This header option shall be included with the 'Connect Request' and 'Connect Accept' messages and shall only be included with
those messages. An SC connection shall record the capabilities of the connecting peer and this information will remain for the
lifetime of the connection.  If the capabilities of a peer change, that peer shall drop the connection so that a new Hello Option
can be sent upon reconnection.

AB.2.3.1.3 Identity Header Option

The 'Identity' header option specifies the identity information for the sender of a message.

When passed to or from the network layer, a 'Identity' option is conveyed as data attribute with the encapsulated NPDU. It is
designated as a "every segment" attribute. See Clause 17.3.2 for its definition and usage.

The 'Identity' header option consists of the following fields.

Header Marker

Header Length
Header Data
       Device

1-octet

2-octets
3-octets
3-octets

'More Options' = 0 or 1, 'Must Understand ' = 0,
'Header Data Flag' = 1, 'Header Option Type' = 3

Length of 'Header Data' field, in octets. Shall be 3
Required to include:
Device instance number, with most significant octet first

This header option, if present, shall be a data option in the 'Data Options' parameter of BVLC messages conveying an NPDU.

AB.2.3.1.4 Hint Header Option

The 'Hint' header option provides information about what authorization is required for a failed operation.  See Clause 17.4.8
for meaning and usage.

When passed to or from the network layer, a 'Hint' option is conveyed as data attribute with the encapsulated NPDU. It is
designated as a "first segment" attribute.

A 'Hint' header option consists of the following fields.

Header Marker

Header Length
Header Data
       Scope

1-octet

'More Options' = 0 or 1, 'Must Understand ' = 0,
'Header Data Flag' = 1, 'Header Option Type' = 4

Length of 'Header Data' field, in octets.

2-octets
5-N octets  Required to include:
Variable

The required scope represented as the ASN.1 production
of a BACnetAuthorizationScope

This header option, if present, shall be a data option in the 'Data Options' parameter of BVLC messages conveying an NPDU.

ANSI/ASHRAE Standard 135-2024

1467

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.2.3.1.5 Token Header Option

The 'Token' header option conveys an access token to authorize a protected operation.  See Clause 17.4.8 for meaning and
usage.

When passed to or from the network layer, a 'Token' option is conveyed as data attribute with an encapsulated NPDU. It is
designated as a "first segment" attribute.

A 'Token' header option consists of the following fields.

Header Marker

Header Length
Header Data
       Token

1-octet

'More Options' = 0 or 1, 'Must Understand ' = 0,
'Header Data Flag' = 1, 'Header Option Type' = 5

2-octets
N octets
Variable

Length of 'Header Data' field, in octets.
Required to include:
The ASN.1 production of a BACnetAccessToken

This header option, if present, shall be a data option in the 'Data Options' parameter of BVLC messages conveying an NPDU.

AB.2.3.2 Proprietary Header Options

Vendors may define and use proprietary header options. In order to distinguish vendor specific header options, the first two
octets of the header data shall contain the vendor identifier code of the defining organization. See Clause 23.

Any proprietary header option shall consist of the following fields:

Header Marker

Header Length
Header Data
       Vendor Identifier

       Proprietary Option Type
       Proprietary Header Data

1-octet
Variable

1-octet

'More Options' = 0 or 1, 'Must Understand' = 0 or 1,
'Header Data Flag' = 1, 'Header Option Type' = 31

Length of 'Header Data' field, in octets.

2-octets
3-N octets  Required to include:
2-octets

Vendor Identifier, with most significant octet first, of the
organization defining this option.
An indication of the proprietary header option type.
A proprietary string of octets. Can be zero length.

For BVLC messages received, the processing of proprietary header options is a local matter.

For BVLC messages sent, the insertion of proprietary header options in the BVLC message is a local matter.

AB.2.4 BVLC-Result

This  unicast  BVLC  message  provides  a  mechanism  to  acknowledge  the  result  of  those  BVLC  messages  that  require  an
acknowledgment, whether successful (ACK) or unsuccessful (NAK). For standard BVLC messages, it is only used to indicate
an unsuccessful result as described in subsequent clauses. It can be used for Proprietary messages that require acknowledgement
of successful and/or unsuccessful results. This message shall be returned for an unknown or unsupported destination option
whose ‘Must Understand’ flag is set (see Clause AB.3.1.4) and for BVLC message errors (see Clause AB.3.1.5). This message
is the result of a BVLC function request and is not a response to the payload or data options. This response message is generated
by a BACnet/SC node's BVLL entity and shall not convey data options.

AB.2.4.1 BVLC-Result Format

The BVLC-Result message consists of the following fields:

BVLC Function
Control Flags
Message ID

1-octet
1-octet
2-octets

X'00'

BVLC-Result
Control flags.
The message identifier of the message for which
this message is the result.

1468

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

Originating Virtual Address
Destination Virtual Address
Destination Options
Data Options
Payload
      Result For BVLC Function  1-octet
1-octet
      Result Code

0 or 6-octets
0 or 6-octets
Variable
0-octets

      Error Header Marker
      (Conditional)

1-octet

      Error Class (Conditional)

2-octets

      Error Code (Conditional)

2-octets

      Error Details (Conditional)  Variable

ANNEX AB - BACnet Secure Connect (NORMATIVE)

If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.

Function  BVLC function for which this is a result
X'00'

X'01'

ACK: Successful completion. The 'Error Header
Marker' and all subsequent parameters shall be
absent.
NAK: The BVLC function failed. The 'Error Header
Marker', the 'Error Class', the 'Error Code', and the
'Error Details' shall be present.
The header marker of the destination option that
caused the BVLC function to fail. If the NAK is
unrelated to a header option, this parameter shall be
X'00'.
The 'Error Class' field of the 'Error' datatype defined
in Clause 21.
The 'Error Code' field of the 'Error' datatype defined
in Clause 21.
UTF-8 reason text. Can be an empty string using no
octets. Note that this string is not encoded as defined
in Clause 20.2.9, has no character set indication octet,
and no trailing zero octets. See BVLC-Result
examples in Clause AB.2.17.

AB.2.5 Encapsulated-NPDU

This unicast or broadcast BVLC message is used to send NPDUs to another BACnet/SC node, or  broadcast NPDUs to all
nodes.

AB.2.5.1 Encapsulated-NPDU Format

The Encapsulated-NPDU message consists of the following fields:

BVLC Function
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address
Destination Options
Data Options
Payload
      BACnet NPDU

1-octet
1-octet
2-octets
0 or 6-octets
0 or 6-octets
Variable
Variable

Variable

AB.2.6 Address-Resolution

X'01'

Encapsulated-NPDU
Control flags
The message identifier
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Optional, 0 to N header options

This  unicast  BVLC  message  is  sent  by  BACnet/SC  nodes  to  request  the  list  of  possible  WebSocket  URIs  at  which  the
destination node accepts direct connections. See Clause AB.4.

AB.2.6.1 Address-Resolution Format

The Address-Resolution message consists of the following fields:

X'02'

BVLC Function
Control Flags
Message ID
Originating Virtual Address
Destination Virtual Address
Destination Options
Data Options

1-octet
1-octet
2-octets
0 or 6-octets
0 or 6-octets
Variable
0-octets

Address-Resolution
Control flags
The message identifier
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.

ANSI/ASHRAE Standard 135-2024

1469

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.2.7 Address-Resolution-ACK

This unicast BVLC message is the  response  to the Address-Resolution message. The Address-Resolution-ACK message is
directed to the node that originally initiated the Address-Resolution message. See Clause AB.4.1.

AB.2.7.1 Address-Resolution-ACK Format

The Address-Resolution-ACK message consists of the following fields:

BVLC Function
Control Flags
Message ID

1-octet
1-octet
2-octets

X'03'

Originating Virtual Address
Destination Virtual Address
Destination Options
Data Options
Payload
      WebSocket-URIs

0 or 6-octets
0 or 6-octets
Variable
0-octets

Variable

Address-Resolution-ACK
Control flags
The message identifier of the message for which this
message is the response.
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.

UTF-8 string containing a list of WebSocket URIs as
of RFC 3986, separated by a single space character
(X'20'), where the source BACnet/SC node accepts
direct  connections.  Can  be  an  empty  string  using
zero octets. See Clause AB.3.3.

AB.2.8 Advertisement

This unicast BVLC message is advertising the configuration and status of the source node. See Cause AB.3.2.

AB.2.8.1 Advertisement Format

The Advertisement message consists of the following fields:

X'04'

X'00'
X'01'
X'02'
X'00'

X'01'

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options
Payload
      Hub Connection Status

1-octet
1-octet
2-octets
0 or 6-octets
0 or 6-octets
Variable
0-octets

1-octet

      Accept Direct Connections

1-octet

      Maximum BVLC Length

2-octet

      Maximum NPDU Length

2-octets

AB.2.9 Advertisement-Solicitation

Advertisement
Control flags
The message identifier
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.

No hub connection.
Connected to primary hub.
Connected to failover hub.
The  node  does  not  support  accepting  direct
connections.
The node supports  accepting  direct connections.
The  maximum  BVLC  message  size  that  can  be
received  and  processed  by  the  node,  in  number  of
octets.
The  maximum  NPDU  message  size  that  can  be
handled  by  the  node's  network  entity,  in  number  of
octets.

This  unicast  BVLC  message  is  sent  to  a  node  to  solicit  that  node  to  send  an  Advertisement  message  in  a  manner  that  the
requesting node can receive.

AB.2.9.1 Advertisement-Solicitation Format

The Advertisement-Solicitation message consists of the following fields:

1470

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options

1-octet
1-octet
2-octets
0 or 6-octets
0 or 6-octets
Variable
0-octets

AB.2.10 Connect-Request

X'05'

Advertisement-Solicitation
Control flags
The message identifier
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.

This unicast BVLC message is sent to the connection accepting peer node to request acceptance of the connection established.
See Clause AB.6.2.

AB.2.10.1 Connect-Request Format

The Connect-Request message consists of the following fields:

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options
Payload
      VMAC Address
      Device UUID
      Maximum BVLC Length

1-octet
1-octet
2-octets
0-octets
0-octets
Variable
0-octets

6-octets
16-octet
2-octet

      Maximum NPDU Length

2-octets

AB.2.11 Connect-Accept

X'06'

Connect-Request
Control flags
The message identifier
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent.

The VMAC address of the requesting node.
The device UUID of the requesting node
The maximum BVLC message size that can be
received and processed by the requesting node, in
number of octets.
The maximum NPDU message size that can be
handled by the requesting node's network entity, in
number of octets.

This unicast BVLC message is the response to the Connect Request message. It is sent to the connection requesting peer node
to confirm acceptance of the connection established. See Clause AB.6.2.

AB.2.11.1 Connect-Accept Format

The Connect-Accept message consists of the following fields:

X'07'

BVLC Function:
Control Flags
Message ID

Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options
Payload
      VMAC Address

1-octet
1-octet
2-octets

0-octets
0-octets
Variable
0-octets

6-octets

      Device UUID

16-octets

Connect-Accept
Control flags
The message identifier of the message for which this
message is the response.
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent.

For direct connections, the VMAC of the accepting
node. For hub connections, the VMAC of the node in
the network port that contains the hub function.
For direct connections, the device UUID of the
accepting node. For hub connections, the UUID of the
device that contains the network port that contains the
hub function.

ANSI/ASHRAE Standard 135-2024

1471

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

      Maximum BVLC Length

2-octets

      Maximum NPDU Length

2-octets

AB.2.12 Disconnect-Request

The maximum BVLC message size that can be
received and processed by the accepting node, in
number of octets.
The maximum NPDU message size that can be
handled by the accepting node's network entity, in
number of octets.

This unicast BVLC message is sent to the connection peer node to request disconnection of the connection. See Clause AB.6.2.

AB.2.12.1 Disconnect-Request Format

The Disconnect-Request message consists of the following fields:

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options

1-octet
1-octet
2-octets
0-octets
0-octets
Variable
0-octets

AB.2.13 Disconnect-ACK

X'08'

Disconnect-Request
Control flags
The message identifier
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent.

This unicast BVLC message is the response to the Disconnect Request message. It is sent to the connection peer node to confirm
the disconnection. See Clause AB.6.2

AB.2.13.1 Disconnect-ACK Format

The Disconnect-ACK message consists of the following fields:

BVLC Function:
Control Flags
Message ID

Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options

AB.2.14 Heartbeat-Request

1-octet
1-octet
2-octets

0-octets
0-octets
Variable
0-octets

X'09'

Disconnect-ACK
Control flags
The message identifier of the message for which
this message is the response.
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent.

This unicast BVLC message is sent to the connection peer node to probe that the connection and connection peer node is still
alive. See Clause AB.6.3.

AB.2.14.1 Heartbeat-Request Format

The Heartbeat-Request message consists of the following fields:

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options

1-octet
1-octet
2-octets
0-octets
0-octets
Variable
0-octets

AB.2.15 Heartbeat-ACK

X'0A'

Heartbeat-Request
Control flags
The message identifier
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent.

This unicast BVLC message is the response to the Heartbeat Request message. It is sent to the connection peer node to indicate
that the sending node and the connection is alive. See Clause AB.6.3.

1472

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.2.15.1 Heartbeat-ACK Format

The Heartbeat-ACK message consists of the following fields:

BVLC Function:
Control Flags
Message ID

Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options

AB.2.16 Proprietary Message

1-octet
1-octet
2-octets

0-octets
0-octets
Variable
0-octets

X'0B'

Heartbeat-ACK
Control flags
The message identifier of the message for which
this message is the response.
Absent, is always from connection peer node
Absent, is always for connection peer node
Optional, 0 to N header options
Shall be absent

This unicast or broadcast BVLC message is for proprietary extensions at the data link level. The payload portion of the BVLL
message  shall  always  have  a  vendor  identifier,  a  proprietary  function  identifier  defined  by  that  vendor,  and  an  optional
proprietary data field. The use and processing of proprietary messages is a local matter. Recipients of unexpected proprietary
messages  can  either  drop  the message  or  respond  with  a  negative  BVLC-Result.  The  error  class  and  error  code  to  use  for
negative responses is a local matter, however, note that the error code BVLC_PROPRIETARY_FUNCTION_UNKNOWN is
available as a generic response.

AB.2.16.1 Proprietary Message Format

The Proprietary-Request message consists of the following fields:

BVLC Function:
Control Flags
Message ID
Originating Virtual Address:
Destination Virtual Address:
Destination Options
Data Options
Payload

1-octet
1-octet
2-octets
0 or 6-octets
0 or 6-octets
Variable
Variable
3-N octets

       Vendor Identifier

2-octets

       Proprietary Function
       Proprietary Data

1-octet
Variable

AB.2.17 BVLC Message Encoding Examples

X'0C'

Proprietary-Message
Control flags
The message identifier
If absent, message is from connection peer node
If absent, message is for connection peer node
Optional, 0 to N header options
Shall be absent.
The payload shall consist of at least the vendor
identifier and the proprietary function octet.
Vendor Identifier, with most significant octet first, of
the organization defining this message.
The vendor-defined function code.
Optional vendor-defined payload data

Figure AB-5 illustrates the encoding of an example BVLC message conveying a ReadProperty request and is being sent to a
BACnet/SC hub function for forwarding to the destination VMAC. Note that the hub will insert the 'Originating Virtual
Address' before forwarding to the hub connection to the destination node.

BVLC Function
Control Flags

X'01'
X'07'

Message ID
Destination
Virtual Address
Destination Options  X'BF'

Encapsulated-NPDU
Control octet = B'00000111': Originating Virtual Address is
absent. Destination Virtual Address, Destination Options, and
Data Options are present.
The Message ID chosen for this BVLC message.

X'B5EC'
X'927BF71A96A2'  6 octets unicast Destination Virtual Address

Header Marker = B'10111111'. More Options follow, not required
to understand, and Header Data present
Proprietary Header Option (31)
Header Length in octets = 7
Vendor Identifier = 555
5 octets of proprietary header data

X'0007'
X'022B'
X'BAC5ECC099'

ANSI/ASHRAE Standard 135-2024

1473

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Data Options

Payload

X'3F'

X'0003'
X'0309'
X'39'
X'01'

X'01'
X'04'

X'0000010C0C
000000051955'

Header Marker = B'00111111'. Last Option, not required to
understand, and Header Data present
Proprietary Header Option (31)
Header Length in octets = 3.
Vendor Identifier = 777
1 octet of proprietary header data
Header Marker = B'00000001': Last Data Option, not required to
understand, and Header Data absent
Secure Path Header Option (1)
NPDU: Version = 1
NPDU: Control = B'00000100': Conveys APDU, no address
fields, reply expected, and NORMAL network priority.
APDU: ReadProperty confirmed request, see decoding in Clause
F.3.5

Figure AB-5. Example Encapsulated-NPDU BVLC Message

Figure AB-6 illustrates the encoding of an example BVLC-Result NAK message conveying an error on one of the destination
header options of the Encapsulated-NPDU message shown in Figure AB-5. This BVLC-Result example includes a UTF-8
encoded 'Error Details' parameter.

BVLC Function
Control Flags

X'00'
X'08'

Message ID

X'B5EC'

Originating
Virtual Address
Payload

X'927BF71A96A2'

X'01'
X'01'
X'BF'
X'0007'
X'0111'
X'556E6DC3B6676C69
6368657220436F646521'

BVLC-Result
Control octet = B'00001000'. Originating Virtual Address is
present.  Destination  Virtual  Address  and  Destination
Options are absent.
The Message ID from the BVLC message to which this is a
result.
6 octets unicast Originating Virtual Address

Result for BVLC Function = Encapsulated-NPDU
Result Code = NAK
Error Header Marker of header that caused the error.
Error Class = COMMUNICATION
Error Code = Proprietary error code (= 273)
Error Details = "Unmöglicher Code!"

Figure AB-6. Example BVLC-Result Message with 'Error Details'

Figure AB-7 illustrates the encoding of an example BVLC-Result NAK message conveying an error on one of the destination
header options of the Encapsulated-NPDU message shown in Figure AB-5. This BVLC-Result example does not include an
'Error Details' parameter.

BVLC Function
Control Flags

X'00'
X'08'

Message ID

X'B5EC'

Originating
Virtual Address
Payload

X'927BF71A96A2'

BVLC-Result
Control octet = B'00001000'. Originating Virtual Address is
present.  Destination  Virtual  Address  and  Destination
Options are absent.
The Message ID from the BVLC message to which this is a
result.
6 octets unicast Originating Virtual Address

X'01'

Result for BVLC Function = Encapsulated-NPDU

1474

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

X'01'
X'3F'
X'0007'
X'0117'

Result Code = NAK
Error Header Marker of header that caused the error.
Error Class = COMMUNICATION
Error Code = Proprietary error code (= 279)

Figure AB-7. Example BVLC-Result Message without 'Error Details'

AB.3 BACnet/SC Node Operation

AB.3.1 BVLC Message Exchange

AB.3.1.1 Response BVLC Messages

A BVLC-Result received shall not generate a BVLC-Result message in response. The handling of a BVLC-Result message
received that cannot be matched to a BVLC message sent is a local matter.

AB.3.1.2 Virtual Address Parameters in BVLC Messages

In every BVLC message, the 'Originating Virtual Address', if present, shall always be the VMAC of the node that originally
initiated the BVLC message. If absent, the message originated at the connection peer node. When forwarding, the connection
peer node's VMAC shall be inserted as the 'Originating Virtual Address' parameter.

When a  broadcast BVLC message is sent to all nodes of the BACnet/SC network, the  Local Broadcast VMAC address as
defined in Clause AB.1.5.2 shall be used as the 'Destination Virtual Address' parameter. In this case, the 'Destination Virtual
Address' parameter shall always remain present in the BVLC message so the recipient can determine if the BVLC message
was unicast or broadcast.

When a unicast BVLC message is sent to a destination BACnet/SC node other than the connection peer, the VMAC of the
destination node shall be used as the 'Destination Virtual Address' parameter. When a unicast BVLC message is sent to the
connection peer, the 'Destination Virtual Address' parameter shall be absent.

If a response BVLC message is returned on a BVLC message with no 'Originating Virtual Address' parameter, then the response
BVLC message 'Destination Virtual Address' parameter shall be absent and the response BVLC message shall be sent through
the connection from which that BVLC message was received. In all other response messages, the 'Destination Virtual Address'
shall be the VMAC indicated in the 'Originating Virtual Address' parameter of the message being responded to. Note that an
Advertisement  message  is  not  considered  a  "response"  to  an  Advertisement-Solicitation  message;  however,  the  solicited
Advertisement shall be sent in a manner that the soliciting node will receive it using the rules above.

Note that a BVLC-Result received without an 'Originating Virtual Address' was produced by the connection peer, which is the
hub function for hub connections, or is the destination node for direct connections.

AB.3.1.3 Message ID Parameter

When a BVLC request message is originally created, the determination of the 'Message ID' parameter value is a local matter.

To allow for matching the BVLC messages sent with response BVLC messages received, the message ID may be selected to
be  unique  among  all  pending  initiated  BVLC  messages  within  some  maximum  time  the  node  waits  for  a  response.  The
maximum time to wait for a response is a local matter.

For response BVLC messages, the message ID shall be the message ID of the causing message. Note that an Advertisement
message is not considered a "response message" to an Advertisement-Solicitation message and does not copy the message ID
of the solicitation.

When forwarding a BVLC message, e.g. by the hub function, the message ID shall not be changed.

AB.3.1.4 Header Options Processing and 'Must Understand'

The destination BACnet/SC node shall process the header options present in 'Destination Options'. Destination options whose
'Must Understand' flag is cleared (0) shall be ignored when not supported.

If a destination option is present whose 'Must Understand' flag is set (1) but the option is unknown or not supported by the
BVLL entity of the destination node, then if the original message was a unicast BVLC message, a BVLC-Result NAK for the

ANSI/ASHRAE Standard 135-2024

1475

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

BVLC  message  shall  be  returned  indicating  an
'Error  Code'  of
HEADER_NOT_UNDERSTOOD. If the original message was a broadcast BVLC message, no BVLC-Result message shall
be returned. The broadcast BVLC message shall be ignored.

'Error  Class'  of  COMMUNICATION  and  an

For the handling of 'Data Options' see Clause AB.3.4. The hub function and the source and destination node's BVLL entity
shall forward and not alter any of the data options.

The remaining parts of the BVLC message shall be processed as required.

AB.3.1.5 Common Error Situations

If a BVLC message is received that does not contain a Message ID, a BVLC-Result NAK shall not be returned. The message
shall be discarded and not be processed.

If a BVLC message is received that is truncated, for example there are missing fields or incomplete fields, a BVLC-Result
NAK shall be returned if it was a unicast message, indicating an 'Error Class' of COMMUNICATION and 'Error Code' of
MESSAGE_INCOMPLETE. The message shall be discarded and not be processed.

If a BVLC message is received that is an unknown BVLC function, a BVLC-Result NAK shall be returned if it was a unicast
message  indicating  an  'Error  Class'  of  COMMUNICATON  and  'Error  Code'  of  BVLC_FUNCTION_UNKNOWN.  The
message shall be discarded and not be processed.

If  a  BVLC  message  is  received  for  which  a  payload  is  required, but  no payload  is  present,  a  BVLC-Result  NAK  shall  be
returned  if  it  was  a  unicast  message  indicating  an
'Error  Code'  of
PAYLOAD_EXPECTED. The message shall be discarded and not be processed.

'Error  Class'  of  COMMUNICATON  and

If a BVLC message is received in which a header has encoding errors, a BVLC-Result NAK shall be returned if it was a unicast
message indicating an 'Error Class' of COMMUNICATON and 'Error Code' of HEADER_ENCODING_ERROR. The message
shall be discarded and not be processed.

If a BVLC message is received in which any control flag has an unexpected value, then a BVLC-Result NAK shall be returned
if
'Error  Code'  of
indicating  an
PARAMETER_OUT_OF_RANGE. The message shall be discarded and not be processed.

'Error  Class'  of  COMMUNICATION  and  an

it  was  a  unicast  message,

If a BVLC message is received in which any parameter, field of a known header, or parameter in a BACnet/SC defined payload,
is  out  of  range,  then  a  BVLC-Result  NAK  shall  be  returned  if  it  was  a  unicast  message,  indicating  an  'Error  Class'  of
COMMUNICATION and an 'Error Code' of PARAMETER_OUT_OF_RANGE. The message shall be discarded and not be
processed.

If a BVLC message is received in which any data inconsistency exists in any parameter, field of a known header, or parameter
in  a  BACnet/SC  defined  payload,  then  a  BVLC-Result  NAK  shall  be  returned,  indicating  an  'Error  Class'  of
COMMUNICATION and an 'Error Code' of INCONSISTENT_PARAMETERS. The message shall be discarded and not be
processed.

If a BVLC message is received that is longer than expected, a BVLC-Result NAK shall be returned if it was a unicast message,
indicating  an  'Error  Class'  of  COMMUNICATION  and  'Error  Code'  of  UNEXPECTED_DATA.  The  message  shall  be
discarded and not be processed.

AB.3.2 Advertisement Exchange

Nodes may initiate Advertisement or Advertisement-Solicitation messages to other nodes at any time, e.g., for synchronization
or update of status information.

On receipt of an Advertisement message, the node shall update its status information of the sending node as provided by the
Advertisement message.

1476

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

On receipt of an Advertisement-Solicitation message, the node shall respond with an Advertisement message. Note that even
though the Advertisement message is sent in response to the solicitation, it is not considered a "response message" and thus
does not copy the message ID of the Address-Solicitation message.

AB.3.3 Address Resolution

An Address-Resolution message can be initiated to another node for requesting an Address-Resolution-ACK response returning
the WebSocket URIs where the responding node accepts direct connections. See Clause AB.4.1. The WebSocket URIs returned
may or may not be valid for the network context of the requesting node. See Cause AB.1.5.1.

On receipt of an Address-Resolution message, an Address-Resolution-ACK message shall be returned if the node accepts direct
connections. An empty string shall be returned if no such WebSocket URIs are currently known but the node supports accepting
direct connections.

If accepting direct connections is not supported, a BVLC-Result NAK for the Address-Resolution message shall be returned,
indicating
of
OPTIONAL_FUNCTIONALITY_NOT_SUPPORTED.

COMMUNICATION

Class'

'Error

'Error

Code'

and

an

an

of

AB.3.4 NPDU Exchange

The following message exchange procedures are performed by the BVLL entity of nodes for BVLC messages conveying an
NPDU as payload.

On receipt of an Encapsulated-NPDU BVLC message from the node switch function or hub connector, the NPDU shall be
extracted and forwarded to the local network entity in the 'data' parameter of the data link indication primitive. Data options
present  in  the  message  shall  be  forwarded  to  the  local  network  entity  in  the  'data_attributes'  parameter  of  the
DL_UNITDATA.indication primitive.

On receipt of an NPDU from the local network entity in the 'data' parameter of the DL_UNITDATA.request primitive, and a
destination VMAC address is provided; the BVLL entity shall create an Encapsulated-NPDU BVLC message with the NPDU
as payload and forward it to the hub connector or node switch if present to send the message to the destination node. The
'Destination Virtual Address' shall be the destination VMAC address provided. Data options as provided by the network entity
in the 'data_attributes' parameter shall be the 'Data Options' parameter in the Encapsulated-NPDU message.

On receipt of an Encapsulated-NPDU from the local network entity and an empty destination MAC address is provided; the
BVLL  shall  create  an  Encapsulated-NPDU  message  with  the  NPDU  as  payload  and  the  Local  Broadcast  VMAC  as  the
'Destination Virtual Address' and provide it to the hub connector for being sent. Data options as provided by the network entity
in the 'data_attributes' parameter shall be the 'Data Options' parameter in the Encapsulated-NPDU message forwarded to the
hub connector.

AB.4 Node Switch and Direct Connections

BACnet/SC nodes can optionally support BACnet/SC connections between BACnet/SC nodes of a BACnet/SC network for
direct  connections,  in  addition  to  the  hub  connections.  BACnet/SC  node  implementations  can  optionally  support  initiating
direct  connections,  or  accepting  direct  connections,  or  both.  The  WebSocket  subprotocol  name  for  BACnet/SC  direct
connections shall be used, and no other subprotocol shall be accepted, to establish the underlying WebSocket connection. See
Clause AB.7.1.

BACnet/SC nodes supporting direct connections are required to implement the BACnet/SC node switch function.

Only unicast BVLC messages addressed to the connection peer node shall be sent through direct connections. All other BVLC
messages are required to be sent through the hub connection to the hub function.

Nodes may optionally accept direct connections as an accepting peer and may optionally initiate direct connections to other
nodes as an initiating peer.

If a node supports direct connections as an initiating peer, the method to determine when to initiate, reconnect and terminate a
direct  connection  is  a  local  matter.  The  reconnect  timeout  configured  for  the  node  shall  be  respected  between  attempts  to
reconnect a direct connection to the same accepting peer node. See Clause AB.6.1.

ANSI/ASHRAE Standard 135-2024

1477

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.4.1 URIs For Direct Connections

The WebSocket URIs for initiating a direct connection to an accepting peer shall use the "wss" scheme.

The WebSocket URIs to use for direct connections may be statically configured or dynamically discovered. If no WebSocket
URIs are statically configured for a particular node, then the WebSocket URIs can be requested from that node by sending an
Address-Resolution message to the node through the hub function.

If WebSocket URIs are provided with the Address-Resolution-ACK message in response, these WebSocket URIs can be used
to attempt a direct connection to the node. The selection of the URI to use from those returned in the message is a local matter.
The WebSocket URIs returned are not required to be valid for the network location and context of the Address-Resolution
message initiating node. If none of the returned URI's result in a connection, then a direct connection cannot be established;
however, communication to the node through the hub function is still available.

AB.4.2 Node Switch Function

The BACnet/SC node's optional switch function is the peer for the direct connections of the BACnet/SC node. The node switch
function forwards BVLC messages between the direct connections, the hub connector, and the local BVLL entity. See Figure
AB-8. All direct connections are required to be established as BACnet/SC connections. See Clause AB.6.2.

The  'Destination  Virtual  Address'  of  BVLC  messages  received  from  the  local  BVLL  entity  is  used  to  select  the  direct
connection, the hub connector, or the local BVLL entity to forward a message to.

The node switch function shall maintain knowledge of the connection peer node's VMAC address and the connection peer
node's Device UUID while the connection is established. This information is learned from the connect message exchange for
the BACnet/SC connection after establishment of the WebSocket connection. See Clause AB.6.2

For direct connection messages, both the source and destination VMAC address shall be omitted.

The node switch function dispatches messages between the local BVLL entity, direct connections, and the hub connector. See
Figure AB-8 showing the node switch function of an example node that is connected to a hub function and has initiated one
direct connection and has accepted two direct connections.

Figure AB-8. Node Switch Function

1478

ANSI/ASHRAE Standard 135-2024

xBVLL EntityBACnet Network Layer and higher ...Directed BVLC Messages FlowBVLL EntityBACnet Network Layer and higher ...Outbound MessagesInbound MessagesA,B,...DA = CDA = DDA = EOther DADA = A or Absent VMAC of NodeNode Switch DA = Destination VMAC AddressNode Switch WebSocket ConnectionHub ConnectorNodeDNodeExOther DABroadcast BVLC Messages FlowAA NodeCHub FunctionHub ConnectorHub FunctionNodeDNodeE NodeCCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.4.2.1 Outbound Messages

On receipt of a unicast BVLC message from the local BVLL entity and a destination VMAC address is provided, the unicast
BVLC message shall be sent through the direct connection with the connection peer node matching the destination VMAC, if
one exists. If no such connection exists, the unicast BVLC message shall be forwarded to the hub connector.

On receipt of a unicast BVLC message from the local BVLL entity where the destination VMAC address is omitted and the
direct connection to use is indicated by the local BVLL entity, the message shall be sent through that direct connection. The
method of indication of the direct connection to be used is a local matter.

On  receipt  of  a  broadcast  BVLC  message  from  the  local  BVLL  entity,  the  BVLC  message  shall  be  forwarded  to  the  hub
connector.

Unicast BVLC messages being sent through a  direct connection shall omit both the 'Destination VMAC Address', and the
'Originating VMAC Address' parameters.

All BVLC messages forwarded to the hub connector shall include both the 'Destination VMAC Address' parameter and the
'Originating VMAC Address' parameter, where the latter shall be the VMAC address of the BVLL entity.

AB.4.2.2 Inbound Messages

On receipt of a unicast BVLC message from any current direct connection or the hub connector whose destination VMAC is
the VMAC of the local BVLL entity, or the destination VMAC address is absent, the message shall be forwarded to the local
BVLL entity. All other unicast BVLC messages shall be discarded.

On receipt of a broadcast BVLC message from the hub connector, the message shall be forwarded to the local BVLL entity.

On receipt of a broadcast BVLC message from a direct connection, the message shall be discarded.

For unicast BVLC messages received from a direct connection whose destination VMAC address is absent, the hub switch
shall indicate the VMAC address of the local BVLL entity as the destination VMAC address to the local BVLL entity.

For unicast BVLC messages  received from a direct connection in which the originating VMAC address is absent,  the hub
switch shall forward the connection peer node's VMAC address as the originating VMAC address to the local BVLL entity.

AB.5 Hub Function and Hub Connector

For  BACnet/SC  networks,  forwarding  and  distribution  of  BVLC  messages  among  and  between  the  BACnet/SC  nodes  is
required. This functionality is performed by the hub function and the hub connectors of the BACnet/SC nodes connecting to
the hub function.

The hub function can be used by nodes as the primary or failover hub function. This distinction is made by the BACnet/SC
nodes only.

All hub connectors shall support connections to both the primary and to the failover hub function.

ANSI/ASHRAE Standard 135-2024

1479

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.5.1 Hub Function Requirements

Figure AB-9. Hub Function Overview

The hub function is required to accept hub connections initiated by the hub connector of BACnet/SC nodes.

Unicast BVLC messages received from a hub connection shall be forwarded by the hub function to the hub connection where
the destination VMAC address matches the VMAC address of the connection peer node. If there is no match, the unicast BVLC
message shall be discarded.

Broadcast BVLC messages received from a hub connection shall be duplicated and a copy shall be sent to all hub connections
except the one from which it was received.

The hub function shall support the forwarding and distribution of BVLC messages that convey, at the least, NPDU sizes of
1497 octets and 4192 octets of data options and destination options.

The hub function shall not send a BVLC message, or any copy of it, to the hub connection from which it was received.

AB.5.2 Hub Connector Requirements

The hub function and method of connection to use to connect to the hub function is expected to be indicated by the URI scheme
used for the primary and failover hub URIs configured for the hub connector.

The hub connector of the BACnet/SC node shall support configuration of the WebSocket URI for the primary hub function
and the WebSocket URI for the failover hub function.

The hub connector of every BACnet/SC node shall support connecting to a primary hub function and to a failover hub function.
See Clause AB.5.4.

The URI for connecting to a BACnet/SC hub function is identified by the "wss" scheme. A local BACnet/SC hub function is
referenced by a "wss" scheme URI with "localhost" used as the hostname.

The hub connector shall establish and maintain a hub connection to the hub function indicated by the URI configured for the
primary hub. If a hub connection to the primary hub function cannot be established, the hub connector shall attempt to establish

1480

ANSI/ASHRAE Standard 135-2024

BVLL EntityBACnet Network Layer and higher ...Directed BVLC Messages FlowA,B,… VMAC of NodeDA = Destination VMAC AddressPrimary or Failover Hub ConnectionHub ConnectorBroadcast BVLC Messages FlowAHub FunctionBACnet /SC NodeBACnet/SC NodeHub ConnectorBroadcast DistributionDA=BDA=DBBACnet /SC NodeCBACnet /SC NodeDDA=BDA=QXBACnet/SC DeviceHub ConnectorHub ConnectorCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

a hub connection to the hub function indicated by the URI for the failover hub, if configured. While the hub connection to the
failover hub function is established, attempts to re-establish the hub connection to the primary hub function shall be continued
respecting the reconnect timeout. If an established primary hub connection is lost, the hub connector shall first attempt to  re-
establish the primary hub connection.

One established hub connection shall be maintained and used at a time. If the connection to the primary hub function can be
restored, the failover hub connection shall be terminated.

The hub connector shall forward BVLC messages from the local BVLL entity, or via the node switch if present, to the hub
connection currently established.

The hub connector shall forward BVLC messages received from the established hub connection to the local BVLL entity, or
to the node switch if present, which will then forward the message to the BVLL entity.

AB.5.3 BACnet/SC Hub Function

The  BACnet/SC  Hub  Function  accepts  BACnet/SC  connections  as  hub  connections  and  performs  the  hub  function.  The
BACnet/SC  hub  function  is  conceptually  present  in  a  BACnet/SC  network  port  that  includes  a  BACnet/SC  node  and  hub
connector. The BACnet/SC hub function acts as an independent endpoint of BACnet/SC connections that are used exclusively
as hub connections and does not use a VMAC address for its operation. The local BACnet/SC node hub connector connects to
the local hub function in the same manner as to a remote hub function.

AB.5.3.1 Hub Connections

The BACnet/SC hub function accepts BACnet/SC connections from BACnet/SC hub connectors. The hub function accepts at
most one hub connection from each BACnet/SC node. The  WebSocket subprotocol name for BACnet/SC hub connections
shall be used, and shall only be accepted, to establish the underlying WebSocket connection. See Clause AB.7.1.

All BVLC message types related to the hub connection shall be initiated and sent to connection peers. See Clause AB.6.2. The
VMAC address provided by the hub in a Connect-Accept message shall be the VMAC address of the BACnet/SC node of the
network port in which the hub function resides. Note that the VMAC is provided to the hub connector to allow a client to know
the device which is hosting the hub function. It is otherwise unused by this protocol.

If the hub function determines that the connection or the connection peer might not be alive, the hub function shall test with a
Heartbeat-Request prior to terminating the connection. See Clause AB.2.14.

The hub function's URI on which it accepts hub connections is a WebSocket URI identified by the "wss" scheme.

Figure AB-10 illustrates a BACnet/SC Device that also includes a BACnet/SC hub function. This hub function is used as the
primary hub function by the BACnet/SC node of the network port.

ANSI/ASHRAE Standard 135-2024

1481

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Figure AB-10. Example BACnet/SC Device with BACnet/SC Hub Function

AB.5.3.2 Unicast BVLC Messages Forwarding

On receipt of a unicast BVLC message from any current hub connection, the BVLC message shall be sent out through the hub
connection  with  the  connection  peer  node  VMAC  matching  the  destination  VMAC.  If  no  such  connection  exists,  the  hub
function shall discard the unicast BVLC message.

When forwarding a unicast BVLC message, the 'Originating Virtual Address' parameter shall be added, indicating the VMAC
address of the connection peer node of the hub connection from which the message was received, and the 'Destination Virtual
Address' parameter shall be removed.

AB.5.3.3 Broadcast  BVLC Messages Forwarding

On receipt of a broadcast BVLC message from a hub connection, the hub function shall send a copy of the received message
through each current hub connection, except the hub connection through which it was received.

When  forwarding  a  broadcast  BVLC  message,  the  'Originating  Virtual  Address'  parameter  shall  be  added,  indicating  the
VMAC address of the connection peer node of the hub connection from which the broadcast BVLC message was received, and
the 'Destination Virtual Address' shall remain in the message so that the receiving node can determine that the message was a
broadcast.

AB.5.4 Hub Connector for the BACnet/SC Hub Function

The hub connector of every BACnet/SC node shall support connecting to a primary hub function and to a failover hub function.
See Clause AB.5.2.

The URI for connecting to the BACnet/SC hub function as the primary hub function shall be configurable and is required to be
a valid "wss" URI for the hub connector to establish a BACnet/SC connection as the hub connection to the primary hub. A
malformed URI or any URI scheme other than "wss" is not supported by the BACnet/SC hub connector and the hub connector
shall not initiate the hub connection. A configuration error may be reported to a local management entity.

1482

ANSI/ASHRAE Standard 135-2024

BACnet/SC DeviceBACnet/SC Virtual Link Layer(BVLL) EntityPrimary Hub ConnectionInitiated to the local Hub FunctionBACnet Network LayerVMACBACnet ApplicationApplication Layer BACnet/SC NodeHub ConnectorFailover Hub ConnectionInitiated to a remote Hub Function(If no primary hub connection)Network PortNode SwitchBACnet/SC Hub FunctionPrimary Hub Connection Connects to Local BACnet/SC Hub Function.Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

The URI for connecting to the failover hub function shall be configurable and a valid "wss" URI. For installations where there
is no failover hub function in use, this URI shall be empty, or otherwise marked as unconfigured, and the hub connector shall
not attempt to connect to a failover hub.

The BACnet/SC hub connector shall initiate and continue to keep alive the BACnet/SC connections to the BACnet/SC hub
function as defined in  Clause  AB.6.2. The  WebSocket subprotocol name for BACnet/SC hub connections shall be  used to
establish the underlying WebSocket connection. See Clause AB.7.1.

When sending a BVLC message to the BACnet/SC hub function to be forwarded, the 'Destination Virtual Address' shall be
present, and the 'Originating Virtual Address' parameter shall be absent in the BVLC message.

On receiving a BVLC message from the BACnet/SC hub function for the local BVLL entity, if the 'Destination Virtual Address'
parameter is present and is not the broadcast VMAC address, the BVLC message received shall be dropped.

AB.6 BACnet/SC Connections

BACnet/SC connections are based on secured WebSocket connections as defined by RFC 6455, and are used for bi-directional
BVLC message exchange. For the application of the WebSocket protocol for BACnet/SC connections, see Clause AB.7.

The connection peer initiating the WebSocket connection is referred  to as the initiating peer. The connection peer accepting
the WebSocket connection is referred to as the accepting peer.

For  direct  connections,  the  node  switch  is  the  initiating  peer  or  the  accepting  peer  of  BACnet/SC  connections.  For  hub
connections to the BACnet/SC hub function, the hub connector of a BACnet/SC node is the initiating peer, and the BACnet/SC
hub function is the accepting peer.

While not needed for protocol operations, it may be useful for a node to know the identity of the BACnet device that is hosting
the BACnet/SC hub function that it is connecting to. For example, this could provide additional checks that the URI is correct
or provide access to other information about the hosting device. The use of this information is a local matter. To provide this
information, in the Connect-Accept message from the hub function, the 'VMAC Address' parameter shall be the VMAC of the
network port containing the hub function, and the 'Device UUID' parameter shall be the Device UUID of the BACnet device.

AB.6.1 BACnet/SC Reconnect Timeout

The minimum time for the initiating peer to wait between initiation attempts to reconnect a WebSocket connection is specified
by the reconnect timeout. If the minimum reconnect timeout is configurable, the initiating peer shall support a range of 2..300
seconds for the minimum reconnect timeout. A fixed minimum reconnect timeout shall have a value in the range 10..30 seconds.

Increasing reconnect timeouts should be applied between unsuccessful attempts to connect. The algorithm for increasing is a
local matter, however the reconnect timeout shall not be increased beyond 600 seconds.

As of Protocol_Revision 24, the minimum reconnect timeout shall be configurable.

AB.6.2 BACnet/SC Connection Establishment and Termination

Once a WebSocket connection is established as specified in Clause AB.7.5, the connection is required to be established as a
BACnet/SC  connection  for  general  BVLC  message  exchange.  To  establish  and  close  a  BACnet/SC  connection,  both  the
initiating  peer  and  the  accepting  peer  execute  a  state  machine  and  exchange  BVLC  messages  to  verify  and  accept  the
WebSocket connection to be a BACnet/SC connection, and to terminate such BACnet/SC connection.

While waiting for a Connect-Request, or for a response to a Connect-Request, a connect wait timer shall be applied using the
connect  wait  timeout.  On  expiration  of  the  connect  wait  timer,  the  initiating  or  accepting  peer  shall  close  the  WebSocket
connection and enter the IDLE state.

The connect wait timeout shall be configurable. The BACnet/SC node shall support a minimum range of 5..300 seconds. The
recommended default value is 10 seconds.

While waiting for a response after sending a Disconnect-Request, a disconnect wait timer shall be applied using the disconnect
wait timeout. The duration of this timeout is a local matter.

ANSI/ASHRAE Standard 135-2024

1483

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Closing an existing WebSocket connection, when one exists before entering the IDLE state, shall be performed as specified in
Clause AB.7.5.5.

On  unexpected  failure,  unexpected  close  by  connection  peer,  or  loss  of  the  WebSocket  connection,  the  local  initiating  or
accepting peer shall enter the IDLE state.

AB.6.2.1 Duplicate Connections and VMAC Address Collisions

The BACnet/SC connections shall ensure that only one connection exists in a given context.

For the node switch, this shall ensure that only one direct connection to another node is used at a time. The local BVLL entity
with its VMAC address and Device UUID is considered a connection peer of the node switch as well.

For the BACnet/SC hub function, this shall ensure that only one hub connection to a node is used at a time. The BACnet/SC
node in the network port shall not be considered a connection peer unless it is currently connected to the local hub function by
its hub connection.

The duplicate connection detection also provides detection of VMAC address collisions of initiating peers.

AB.6.2.2 BACnet/SC Connection Initiating Peer State Machine

The initiating peer state machine enters the IDLE state before a WebSocket connection is initiated. The time of initiation of a
WebSocket connection is determined by the initiating peer.

If  an  initiating  peer  receives  a  BVLC-Result  NAK  on  the  Connect-Request  message  with  an  'Error  Code'  of
NODE_DUPLICATE_VMAC,  then  the  initiating  peer  BACnet/SC  node  shall  choose  a  new  Random-48  VMAC  before  a
reconnection is attempted.

For common error situations see also Clause AB.3.1.5.

Figure AB-11 depicts the connection state machine for the BACnet/SC connection initiating peer.

Figure AB-11. BACnet/SC Connection Initiating Peer Connection State Machine

In any state, before the events specific to the state are considered:

WebSocket Failure

1484

ANSI/ASHRAE Standard 135-2024

Initiate WebSocketWebSocket EstablishedDisconnect-RequestReceivedIDLEAWAITING_ WEBSOCKETAWAITING_ ACCEPTCONNECTEDBVLC-Result NAK Received Connect-Accept ReceivedDISCONNECTINGLocal DisconnectionDisconnect-ACK Received, orBVLC-Result NAK ReceivedWebSocket FailureConnect Wait Timeout ExpiredDisconnect Wait Timeout ExpiredCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

On failure to establish, or unintended disconnection of the WebSocket connection, enter the IDLE state.

In state IDLE

Initiating a WebSocket

On initiation of the WebSocket connection, enter the AWAITING_WEBSOCKET state.

In state AWAITING_WEBSOCKET

WebSocket established

On establishment of the WebSocket connection, send a Connect-Request, start the connect wait timer, and enter the
AWAITING_ACCEPT state.

In state AWAITING_ACCEPT

BVLC-Result NAK received, VMAC collision

On receipt of a BVLC-Result NAK message with an 'Error Code' of NODE_DUPLICATE_VMAC, the initiating
peer's node shall choose a new Random-48 VMAC, close the WebSocket connection, and enter the IDLE state.

BVLC-Result NAK received

On receipt of a BVLC-Result NAK message on the Connect-Request message initiated, close the WebSocket
connection and enter the IDLE state.

Connect-Accept received

On receipt of a Connect-Accept message enter the CONNECTED state.

Connect Wait Timeout expired

On expiration of the connect wait timer, close the WebSocket connection and enter the IDLE state.

In state CONNECTED

Local disconnection

On locally determined disconnection of the connection, send a Disconnect-Request message to the connection peer
node, start the disconnect wait timer, and enter the DISCONNECTING state.

Disconnect-Request received

On receipt of a Disconnect-Request message from the accepting peer, respond with a Disconnect-ACK message to
the accepting peer, close the WebSocket connection, and enter the IDLE state.

In state DISCONNECTING

Disconnect-ACK received

On receipt of a Disconnect-ACK message from the accepting peer, close the WebSocket connection, and enter the
IDLE state.

BVLC-Result NAK received

On receipt of a Result-NAK response to the Disconnect-Request, close the WebSocket connection, and enter the
IDLE state.

Disconnect Wait Timeout expired

On expiration of a disconnect wait timeout, close the WebSocket connection and enter the IDLE state.

AB.6.2.3 BACnet/SC Connection Accepting Peer State Machine

The accepting peer state machine enters the IDLE state before a WebSocket connection is accepted. In any state, in case of an
existing WebSocket connection is lost, the accepting peer shall enter the IDLE state. Figure AB-12 depicts the connection state
machine for the BACnet/SC connection accepting peer.

ANSI/ASHRAE Standard 135-2024

1485

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Figure AB-12. BACnet/SC Connection Accepting Peer Connection State Machine

In any state, before the events specific to the state are considered:

WebSocket Failure

On unintended disconnection of an established WebSocket connection, enter the IDLE state.

In state IDLE

Accepting a WebSocket

On accepting a WebSocket connection, start the connect wait timer and enter the AWAITING_REQUEST state.

In state AWAITING_REQUEST

Connect-Request received, new Device UUID, no VMAC collision

On receipt of a Connect-Request message from the initiating peer whose 'VMAC Address' is not equal to the
accepting peer's VMAC address, and not equal to any of the initiating peers' VMAC addresses of existing
connections, and the 'Device UUID' is not equal to any connection peer's Device UUID, then return a Connect-
Accept message and enter the CONNECTED state.

Connect-Request received, known Device UUID

On receipt of a Connect-Request message from the initiating peer whose 'Device UUID' is equal to the initiating
peer device UUID of an existing connection, then return a Connect-Accept message, disconnect and close the
existing connection to the connection peer node with matching Device UUID, and enter the CONNECTED state.

Connect-Request received, new Device UUID, VMAC collision

On receipt of a Connect-Request message from the initiating peer whose 'VMAC Address' is equal to the accepting
peer's VMAC address, or equal to any initiating peer VMAC addresses of an existing connection, and the 'Device
UUID' is not equal to any initiating peer Device UUID of an existing connection, then return a BVLC-Result NAK
message with an 'Error Class' of COMMUNICATION and an 'Error Code' of NODE_DUPLICATE_VMAC' close
the WebSocket connection and enter the IDLE state

Connect Wait Timeout expired

On expiration of the connect wait timeout, close the WebSocket connection and enter the IDLE state.

In state CONNECTED

1486

ANSI/ASHRAE Standard 135-2024

WebSocket AcceptedConnect-Request Received And AcceptedDisconnect-Request ReceivedIDLEAWAITING_ REQUESTCONNECTEDDISCONNECTINGLocal DisconnectionDisconnect-ACK Received, or BVLC-Result NAK ReceivedWebSocket FailureConnect-Request Received and RejectedConnect Wait Timeout ExpiredDisconnect Wait Timeout ExpiredCopyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Local disconnection

On locally determined disconnection of the connection, send a Disconnect-Request message to the initiating peer,
start the disconnect wait timer, and enter the DISCONNECTING state.

Disconnect-Request received

On receipt of a Disconnect-Request message from the initiating peer node, respond with a Disconnect-ACK
message, close the WebSocket connection, and enter the IDLE state.

In state DISCONNECTING

Disconnect-ACK received

On receipt of a Disconnect-ACK message from the initiating peer, close the WebSocket connection, and enter the
IDLE state.

BVLC-Result NAK received

On receipt of a BVLC-Result NAK response to the Disconnect-Request, close the WebSocket connection, and enter
the IDLE state.

Disconnect Wait Timeout expired

On expiration of a disconnect wait timeout, close the WebSocket connection and enter the IDLE state.

AB.6.3 Connection Keep-Alive

Initiating peers shall keep established BACnet/SC connections alive through periodic initiation of Heartbeat-Request messages
to the accepting peer.

An initiating peer shall send a Heartbeat-Request message to the accepting peer if the initiating peer has not received a BVLC
message over the connection within the heartbeat timeout.

On receipt of Heartbeat-Request, the accepting peer shall respond with a Heartbeat-ACK message to the initiating peer.

As of Protocol_Revision 24, the heartbeat timeout shall be configurable and if a Heartbeat-ACK message is not received from
the accepting peer, the initiating peer shall initiate the ‘Local disconnection’ procedure. See Clause AB.6.2.2.

If the heartbeat timeout is configurable, it shall support a minimum range of 3..300 seconds. A fixed heartbeat timeout shall
have a value in the range 30..300 seconds.

The  connections  may  be  kept  alive  for  as  long  as  the  WebSocket  connection  maximum  lifetime  allows.  Note  that  the
determination of the maximum lifetime is a local matter. See Clause AB.7.5.4.

AB.7 Application of WebSockets in BACnet/SC

All secure WebSocket connections established for BACnet/SC connections shall apply the WebSocket protocol as specified in
RFC  6455  and  in  the  following  subclauses.  Only  TLS-secured  WebSocket  connections  are  used.  Support  of  TLS  V1.3  is
required.

Note that the minimum requirement of RFC 6455 is HTTP 1.1. Therefore, for interoperability on the HTTP level, WebSocket
servers and WebSocket clients for BACnet/SC connections are expected to be able to fall back to HTTP 1.1.

Secure WebSocket connections are used for BACnet/SC connections for bi-directional exchange of binary encoded BACnet/SC
BVLC messages. A BACnet/SC network port may initiate and/or accept one or more WebSocket connections for BACnet/SC.
Each WebSocket connection shall be used exclusively for one BACnet/SC connection.

In WebSocket connections for BACnet/SC connections, the connection peer that initiated the WebSocket connection is referred
to as the initiating peer (i.e. "client" in RFC6455), and the connection peer that accepted the WebSocket connection is referred
to as the accepting peer (i.e. "server" in RFC6455).

ANSI/ASHRAE Standard 135-2024

1487

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.7.1 The WebSocket Protocol

The  use  and  support  of  the  WebSocket  protocol  for  BACnet/SC  connections,  and  the  purpose  of  the  connection,  shall  be
indicated by the WebSocket subprotocol.

For BACnet/SC hub connections, the subprotocol name "hub.bsc.bacnet.org" shall be used.

For BACnet/SC direct connections, the subprotocol name "dc.bsc.bacnet.org" shall be used.

These subprotocol identifiers are registered with IANA as defined in RFC 6455.

The use of the WebSocket Ping and Pong message exchange is optional and implementations shall not rely on its use by the
connection peer.

AB.7.2 WebSocket URIs

The WebSocket URIs used for identifying peers accepting BACnet/SC connections shall be of "wss" URI scheme as defined
in RFC 6455, Section 3.

AB.7.3 WebSocket Binary Data Payload Format

For BACnet/SC connections, fully encoded BVLC messages shall be sent as binary data payload frames over the WebSocket
connection using data frame opcode 0x2. See RFC 6455 Section 5.6.

AB.7.4 Connection Security

The use of secure WebSocket connections as of RFC 6455 and TLS V1.3 as of RFC 8446 for BACnet/SC connections provides
for confidentiality, integrity, and authenticity of BVLC messages transmitted across the connection.

The establishment of a secure WebSocket connection shall be performed as defined in RFC 6455. For establishing a secure
WebSocket connection, mutual TLS authentication shall be performed. "Mutual authentication" in this context means that both
the initiating peer and the accepting peer shall:

(a) Validate that the peer's operational certificate is well formed.
(b) Validate that the peer's operational certificate is active as of the current date and not expired.
(c) Validate that the peer's operational certificate is not revoked, if such information is available.
(d) Validate that the peer's operational certificate is directly signed by one of the locally configured issuer certificates.

An operational certificate is considered to be directly signed by an issuer certificate if the signature can be validated using the
issuer certificate’s public key.

To ensure interoperability, no additional checks  of the operational certificate or issuer certificate  beyond the above shall be
performed by default, and none are required to be supported. Any additional checks, e.g., Common Name, Distinguished Name,
or Subject Alternate Names matches, shall only be performed if specifically enabled, as directed by the installation. The support
and update of revocation information is a local matter.

Note that Clause 17 uses an optional Subject Alternative Name in the certificate for authentication of the device identity for
authorization policies.  This is not used as a connection criteria at the datalink layer.  If the above criteria are satisfied, the
BACnet/SC  connection  shall  succeed  regardless  of  the  presence  or  content  of  any  Subject  Alternative  Name  fields  in  the
certificate.

In BACnet/SC, it is assumed that both the initiating and accepting peer of an established WebSocket connection are trusted,
including all code they execute. The validation of such code and its origins is outside the scope of this standard.

BACnet/SC  implementations  shall  support  TLS  version  1.3  as  specified  in  RFC  8446.  BACnet/SC  implementations  shall
support the following TLS V1.3 cipher suite application profile. For the definition of the terms in quotes see RFC 8446:

(a)  TLS cipher suite "TLS_AES_128_GCM_SHA256",
(b)  digital signature with "ecdsa_secp256r1_sha256", and
(c)  key exchange with "secp256r1".

1488

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

Support  of  other  versions  of  TLS  or  other  cipher  suites,  digital  signatures,  or  key  exchanges  is  a  local  matter.  Additional
supported TLS versions, and additional supported cipher suites, digital signatures, or key exchanges shall be listed in the PICS.
See Annex A.

AB.7.4.1 Certificate Management

Secure WebSocket connections require the use of TLS. The creation of private keys, public certificates and the management
of  the  certificate  signing  authority,  or  authorities,  are  site-specific  deployment  options  beyond  the  scope  of  this  standard.
However, to ensure interoperability, a BACnet/SC implementation shall support the storage and use of certificates as defined
in the following subclauses.
Devices claiming Protocol_Revision 24 or greater shall support certificate configuration through the Network Port object.

AB.7.4.1.1 Operational Credentials

Operational credentials include the certificate of a device, the related private key, and the accepted issuer certificates that are
used to connect to a BACnet/SC network of an installation. A device may have other  certificates and private keys being used
for manufacturer specific communication. These credentials are not considered operational credentials and may be considered
to be part of the factory default condition of the device. See Clause AB.7.4.2.

Before deployment to an active network, the connection peers shall be configured with  an issuer certificate store containing
one  or  more  issuer  certificates  of  those  signing  CAs  that  are  accepted  to  have  signed  the  peer's  certificate,  and  a  unique
operational certificate with matching private key. The operational certificate shall be issued and directly signed by a signing
CA whose issuer certificate is configured in the issuer certificate store. This allows peer-to-peer mutual authentication so that
the accepting peer and the initiating peer can each verify that the certificate presented to it was signed by one of the  signing
CAs in its issuer certificate store.

AB.7.4.1.2 Signing CA

The choice of one or multiple CAs to sign the operational certificates used in a site shall be dictated by site policy. Each signing
CA shall be controlled by the site and can be a root CA or an intermediate CA.

The signing CAs shall support processing of certificate signing requests in Privacy Enhanced Mail (PEM) format (RFC 7468)
conveying a certificate signing request and return the signed certificates in PEM formatted PKCS7 structure.

AB.7.4.1.3 Configuring Operational Certificates

The configuration of operational credentials is performed by the configuration tool of the device. The configuration tool shall
support the exchange of certificate signing requests and signed certificates in PEM format as of RFC 7468 with the signing CA
of the installation. The protocol used by the tool to communicate to the signing CA for this exchange is outside the scope of
this standard.

For devices that cannot generate their own public/private key pairs, the key pair needs to be generated by a configuration tool.
In this case, the tool shall generate the key pair and create a certificate signing request based on certificate parameters defined
by the installation. The tool shall submit the certificate signing request to the signing CA for the installation. The operational
certificate returned from the signing CA, the private key, and the issuer certificates required for the installation are configured
into the device by the tool. The private key shall only be transferred in a secured environment, or over communication secured
by TLS.

A device that supports an internal security function that allows it to generate and store its private keys by itself is not allowed
to  expose  the  private  keys,  and  may  not  be  allowed  to  accept  a  private  key  from  a  configuration  tool.  To  create  a  signed
operational certificate, the configuration tool provides certificate parameters of the installation to the device  and initiates  a
private key and certificate signing request generation by the device. The certificate signing request is sent to a signing CA of
the installation. The operational certificate returned from the signing CA, and the issuer certificates required for the installation
are configured into the device by the tool.

If the effective operational certificate of an active connection is changed, the connection shall be re-established.

If an issuer certificate is removed from the set of effective issuer certificates and the issuer certificate was used, or might have
been used, to validate a peer's certificate for a connection, the connection shall be re-established.

ANSI/ASHRAE Standard 135-2024

1489

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

AB.7.4.2 Factory Defaults Condition

In the factory defaults condition, a connection peer shall not have operational credentials configured, and the device shall not
contain any site specific sensitive data.

AB.7.4.2.1 Reset to Factory Defaults

Devices  shall  provide  a  suitably  secure  out-of-band  mechanism  to  place  itself  into  "factory  defaults"  condition.  It  is
recommended that this requires physical access to the device.

Performing a reset to "factory defaults" condition shall erase all operational certificates and respective private keys and all CA
certificates from all BACnet/SC network ports. Any sensitive data the device contains shall also be erased. It is not allowed to
simply block access to existing sensitive data while in the factory defaults condition because an attacker with physical access
can use this condition to insert new operational credentials and then use that false trust relationship to access sensitive data that
was not erased.

AB.7.5 WebSocket Connection Operation

WebSocket connections shall be initiated, accepted and terminated by the peers as defined in RFC6455.

AB.7.5.1 Initiating WebSocket Connections

For BACnet/SC, the initiation of secured WebSocket connections over TLS V1.3 shall be supported.

If  the  WebSocket  URI  provided  indicates  a  URI  scheme  other  than  "wss",  no  WebSocket  connection  to  that  URI  shall  be
initiated. If applicable, an 'Error Code' of WEBSOCKET_SCHEME_NOT_SUPPORTED shall be indicated.

If the DNS resolution of the host name in the WebSocket URI fails, the following error codes can be used to indicate DNS
error  conditions,  if  known.  If  the  specific  DNS  error  is  unknown, or  no  specific  code  is  available,  DNS_ERROR  shall  be
indicated.

Situation

Error Code

DNS is unknown or not reachable
The host name cannot be resolved to its
IP or IPv6 address.
There  is  an  error  in  the  local  DNS
resolver that prevents it from resolving
the host name.
Any other DNS error situation

DNS_UNAVAILABLE
DNS_NAME_RESOLUTION_FAILED

DNS_RESOLVER_FAILURE

DNS_ERROR

If the host with the IP address resulting from DNS host name resolution is not reachable, and the respective IP error is available,
then the following error codes can be used to indicate IP error conditions. If the specific IP error is unknown, or no specific
code is available, IP_ERROR shall be indicated.

Situation
IP address not reachable
Any other IP error situation

Error Code

IP_ADDRESS_NOT_REACHABLE
IP_ERROR

If the TCP connection to the IP address and port cannot be established, and the respective TCP error is available, then the
following error codes can be used to indicate TCP error conditions. If the specific TCP error is unknown, or no specific code
is available, TCP_ERROR shall be indicated.

Situation
The connection could not be established
due to no response within timeout.
The  connection  is  not  accepted  by  the
peer.
Any other TCP error situation

Error Code

TCP_CONNECT_TIMEOUT

TCP_CONNECTION_REFUSED

TCP_ERROR

1490

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

If  the  TLS  session  on  the  TCP  connection  cannot  be  established,  and  the  respective  fatal  TLS  error  is  available,  then  the
following error codes can be used to indicate TLS error conditions. If the specific TLS error is unknown, or no specific code is
available, TLS_ERROR shall be indicated.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

from

prevents

Situation
The client certificate contains an error
that
being
it
authenticated.
The  server  certificate  contains  an
error  that  prevents  it  from  being
authenticated.
Authentication of the client failed.
Authentication of the server failed.
Client  certificate  validity  window
does not include current time.
Server  certificate  validity  window
does not include current time.
Client certificate revoked
Server certificate is revoked
Any other TLS error situation

TLS_CLIENT_CERTIFICATE_ERROR

Error Code

TLS_SERVER_CERTIFICATE_ERROR

TLS_CLIENT_AUTHENTICATION_FAILED
TLS_SERVER_AUTHENTICATION_FAILED
TLS_CLIENT_CERTIFICATE_EXPIRED

TLS_SERVER_CERTIFICATE_EXPIRED

TLS_CLIENT_CERTIFICATE_REVOKED
TLS_SERVER_CERTIFICATE_REVOKED
TLS_ERROR

If  the  HTTP  exchange  for  upgrade  to  the  WebSocket  protocol  fails,  and  the  respective  HTTP  error  is  available,  then  the
following error codes can be used to indicate HTTP error conditions. If the specific HTTP error is unknown, or no specific
code is available, HTTP_ERROR shall be indicated.

in  values  of

in  HTTP  response

Situation
Server  reports  unexpected  response
code.
Server does not accept upgrade to the
WebSocket protocol.
Redirect  to  another  location  of  the
peer WebSocket port received.
Proxy Authentication failed
No  response  from  server  within
timeout.
Syntax  error
received.
Errors
response received.
Missing header fields in response.
Response contains any other error in
HTTP header fields.
No upgrade request was received by
the server.
Upgrading  to  WebSocket  protocol
failed.
No  more  HTTP  connections  are
available currently.
No  inbound  requests  supported.  The
host is not an HTTP server.
Any other error situation

the  HTTP

HTTP_UNEXPECTED_RESPONSE_CODE

Error Code

HTTP_NO_UPGRADE

HTTP_RESOURCE_NOT_LOCAL

HTTP_PROXY_AUTHENTICATION_FAILED
HTTP_RESPONSE_TIMEOUT

HTTP_RESPONSE_SYNTAX_ERROR

HTTP_RESPONSE_VALUE_ERROR

HTTP_RESPONSE_MISSING_HEADER
HTTP_WEBSOCKET_HEADER_ERROR

HTTP_UPGRADE_REQUIRED

HTTP_UPGRADE_ERROR

HTTP_TEMPORARY_UNAVAILABLE

HTTP_NOT_A_SERVER

HTTP_ERROR

AB.7.5.2 Accepting WebSocket Connections

A  network  port  that  accepts  WebSocket  connections  implements  an  HTTP  server  and  supports  HTTP  upgrades  to  the
WebSocket  protocol.  If  serving  as  a  BACnet/SC  network  port,  it  shall  accept  WebSocket  connections  for  the  appropriate
BACnet/SC WebSocket subprotocol, and exchange binary payloads as defined by this Annex.

ANSI/ASHRAE Standard 135-2024

1491

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AB - BACnet Secure Connect (NORMATIVE)

For BACnet/SC, WebSocket connections secured with TLS V1.3 shall be accepted.

For accepted WebSocket connections that fail or terminate unintentionally, the error codes defined in Clause AB.7.5.1 can be
used to indicate error situations to local higher layers or to a management entity.

AB.7.5.3 BACnet/SC BVLC Message Exchange

All BVLC messages are binary encoded as defined in Clause AB.2 and shall be transmitted as WebSocket binary data frames.
BVLC messages can be sent through a WebSocket connection in both directions.

If any other type of Websocket data frame is received, then the WebSocket connection shall be closed with a status code of
1003 -WEBSOCKET_DATA_NOT_ACCEPTED.

If the length of a BVLC message received through a WebSocket connection exceeds the maximum BVLC length supported by
the receiving node, the BVLC message shall be discarded and not be processed.

AB.7.5.4 Refreshing WebSocket Connections

WebSocket connections may be required to be refreshed such as when new key material must be generated periodically for
TLS. TLS mechanisms shall be used to force session key refreshes. The method to determine the time of refreshing is a local
matter.  Security  requirements,  network  load  produced,  and  processing  power  requirements  shall  be  considered  in  this
determination.

AB.7.5.5 Closing WebSocket Connections

WebSocket connections may be closed by either end at any time. See RFC 6455.

The WebSocket close handshake shall be performed when intentionally closing a WebSocket connection. When a WebSocket
connection is closed, the resulting close status shall be indicated for the associated WebSocket connection. The close status
code received from the WebSocket connection shall map to error codes as follows. For the meaning of WebSocket response
codes, see RFC 6455 Section 7.4.1.

WebSocket Close Status Code

Error Code

1000
1001
1002
1003
1006
1007
1008
1009
1010
1011
all other codes

WEBSOCKET_CLOSED_BY_PEER
WEBSOCKET_ENDPOINT_LEAVES
WEBSOCKET_PROTOCOL_ERROR
WEBSOCKET_DATA_NOT_ACCEPTED
WEBSOCKET_CLOSED_ABNORMALLY
WEBSOCKET_DATA_INCONSISTENT
WEBSOCKET_DATA_AGAINST_POLICY
WEBSOCKET_FRAME_TOO_LONG
WEBSOCKET_EXTENSION_MISSING
WEBSOCKET_REQUEST_UNAVAILABLE
WEBSOCKET_ERROR

1492

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (INFORMATIVE)

(This annex is not part of this standard but is included for informative purposes only.)

These are examples of the various flows of data used in authentication and authorization. In many cases, only the interesting
items are shown for clarity.  For example, "Secure Path" option is assumed. The absence of any required items should not be
considered significant.

The following abbreviations are used in the examples:

AA
non-AA
cert
n/a

an auth-aware device, following the rules in Clause 17.
a non-auth-aware device, predating the rules in Clause 17
this shows just the "..." part of the "bacnet://..." Subject Alternative Name URI
not applicable to the example; may or may not be present and its value is not important

AC.1 Authentication Examples

The following examples show the flow of information for authenticating a client to a target, either directly or relayed through
authorized intermediaries.

AC.1.1 AA to AA interactions

AA hub receives original message, adds Identity
AA destination receives Identity from authorized identity relay --> success

Device 12                           AA Hub 34                    AA Device 56
 cert: 12                          cert: 34?hub                     cert: 56
      |                                  |                             |
      |                                  |     Hello:relay             |
      |                                  |---------------------------->| mark as AA
      |                                  |                             |
      |--------------------------------->| OK, add Identity            |
      |                                  |                             |
      |                                  |     Identity:12             |
      |                                  |---------------------------->| OK, hub
      |                                  |                             |

AA hub receives Identity from router --> success

AA Device 12                          AA Hub 34                    AA Device 56
cert: 12?router                     cert: 34?hub                     cert: 56
      |                                   |       Hello:relay           |
      |                                   |---------------------------->| mark as AA
      |       Hello:relay                 |                             |
      |---------------------------------> | mark as AA                  |
      |                                   |                             |
      |      Identity:99
|                             |
      |---------------------------------> | OK, router                  |
      |                                   |                             |
      |                                   |     Identity:99             |
      |                                   |---------------------------->| OK, hub
      |                                   |                             |

ANSI/ASHRAE Standard 135-2024

1493

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

AC.1.2 Non-AA to AA interactions

An evil device trying to sneak a Identity through a non-AA path will fail, eventually.  It is temporarily a lie in transit, but it
will be dropped at the first AA peer because there is no innocent way for this to happen. Since AA peers will remove Identity
received from a non-AA peer, the only way it can be sent by a non-AA peer is a hack upstream.

Evil Device 99   non-AA Hub 7   non-AA Router 12     AA Hub            AA Device
cert: n/a         cert: n/a       cert: n/a        cert: 34?hub         cert: 56
   |                |                 |                |                  |
   | {evil}         |                 |                |                  |
   | Identity:11    |                 |                |                  |
   |--------------->| no check        |                |                  |
   |                |                 |                |                  |
   |                | {innocent}      |                |                  |
   |                | Identity:11     |                |                  |
   |                |---------------->| no check       |                  |
   |                |                 |                |                  |
   |                |                 | {innocent}     |                  |
   |                |                 | Identity:11    |                  |
   |                |                 |--------------->| No, not AA       |
   |                |                 |                | Remove Identity  |
   |                |                 |                |                  |
   |                |                 |                |   (no Identity)  |
   |                |                 |                |----------------->|OK,
   |                |                 |                |                  |but no
   |                |                 |                |                  |Identity

A non-AA peer with a new cert is still a non-AA peer. It can’t relay a Identity.  Just because a non-AA peer has been given a
new cert does not make it an authorized identity relay.

non-AA Router 12                        AA Hub                       AA Device
cert: 12?router                      cert: 34?hub                     cert: 56
      |                                    |                             |
         |                             |
      |     Identity:99
      |---------------------------------->| Not AA, Remove Identity     |
      |                                   |                             |
      |                                   |     (no Identity
      |
      |                                   |---------------------------->|

AC.1.3 AA to non-AA interactions

A non-AA end device might receive a Identity option and/or a Token.  This is harmless since the non-AA device does not
understand those options and their presence does not grant any extra permissions.

Device 12             AA Hub                      non-AA Device
cert: 12           cert: 34?hub                     cert: n/a
      |                  |                             |
      |     Token        |                             |
      |----------------->| OK, adds Identity           |
      |                  |                             |
      |                  |  Token, Identity:12         |
      |                  |---------------------------->| Identity and Token ignored

1494

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

AC.2 Authorization Examples

The following examples show the flow of information for authorizing the operations of an authenticated client by the target
device.  In these diagrams, an ellipsis in the message arrows indicates that it is not material to the example whether the path is
direct or relayed through hubs and routers.

AC.2.1 Typical Token request flow

The client device does not have a distributed policy configured at the target. So, when a protected operation is attempted, it is
denied.  The client uses the error code to know what kind of access token to request for the protected operation.  The client
requests a token with that scope and is granted a token.  The client then retries the protected operation, this time presenting the
token, and the operation succeeds.

To simplify these examples, a hub is not shown.  They apply to either Hub Connect (where Identity would be present) or Direct
Connect (where Identity would be absent).

AA Device 12                ...                 AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,CONFIG_SCOPE_REQUIRED)     |
      |<--------------------...-----------------------|
      |
      |
      |                                    Authorization Server 99
      |                                               |
      |              AuthRequest-Request              |
      |            token-request(client 12,           |
      |           audience 56, scope config)          |
      |---------------------...---------------------->| consult database,
      |                                               | approve request
      |                AuthRequest-ACK                |
      |           token-response(XXXX..XXXX)          |
      |<--------------------...-----------------------|
      |
      |
      |                                          AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |              Token(XXXX..XXXX)                |
      |---------------------...---------------------->| evaluate token
      |                                               | accept
      |                  Result(+)                    |
      |<--------------------...-----------------------|
      |                                               |

ANSI/ASHRAE Standard 135-2024

1495

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

AC.2.2 Extended Scope

The target device requires an extended (non-standard) scope identifier for a protected operation.  It returns that requirement in
a Hint data attribute.

AA Device 12                ...                 AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,EXTENDED_SCOPE_REQUIRED)   |
      |          Hint(scope "555-twiddle")            |
      |<--------------------...-----------------------|
      |
      |
      |                                    Authorization Server 99
      |                                               |
      |              AuthRequest-Request              |
      |            token-request(client 12,           |
      |        audience 56, scope "555-twiddle")      |
      |---------------------...---------------------->| consult database,
      |                                               | approve
      |                AuthRequest-ACK                |
      |           token-response(XXXX..XXXX)          |
      |<--------------------...-----------------------|
      |
      |
      |                                          AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |              Token(XXXX..XXXX)                |
      |---------------------...---------------------->| evaluate token
      |                                               | accept
      |                  Result(+)                    |
      |<--------------------...-----------------------|
      |                                               |

1496

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

AC.2.3 No Policy Exists, Resolved with Authorization Server

The Authorization server does not have a policy for a requested token, so there is a delay while the owner configures a policy
to allow the token to be issued.  Client also retries original operation in case a distributed policy has been created.

AA Device 12                ...                 AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,CONFIG_SCOPE_REQUIRED)     |
      |<--------------------...-----------------------|
      |
      |
      |                                    Authorization Server 99
      |                                               |
      |              AuthRequest-Request              |
      |            token-request(client 12,           |
      |            audience 56, scope config)         |
      |---------------------...---------------------->| consult database,
      |                                               | deny request,
      |                                               | no policy found,
      |                   Result(-)                   | alert owner...
      |         Error(SERVICE,NO_POLICY_FOUND)        |
      |<--------------------...-----------------------|

                      ... time passes ...
              ... client periodically retries ...

      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,CONFIG_SCOPE_REQUIRED)     |
      |<--------------------...-----------------------|
      |                                               |
      |                                               |
      |              AuthRequest-Request              |
      |            token-request(client 12,           |
      |            audience 56, scope config)         |
      |---------------------...---------------------->| consult database,
      |                                               | deny request,
      |                                               | no policy found,
      |                   Result(-)                   | alert owner...
      |         Error(SERVICE,NO_POLICY_FOUND)        |
      |<--------------------...-----------------------|

         ...owner configures a policy to allow request...

      |              AuthRequest-Request              |
      |            token-request(client 12,           |
      |         audience 56, scope config)            |
      |---------------------...---------------------->| consult database,
      |                                               | approve request.
      |                AuthRequest-ACK                |
      |           token-response(XXXX..XXXX)          |
      |<--------------------...-----------------------|
      |

ANSI/ASHRAE Standard 135-2024

1497

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

      |
      |                                          AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |              Token(XXXX..XXXX)                |
      |---------------------...---------------------->| evaluate token,
      |                                               | accept
      |                  Result(+)                    |
      |<--------------------...-----------------------|
      |                                               |

1498

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AC - EXAMPLES OF AUTHENTICATION AND AUTHORIZATION (NORMATIVE)

AC.2.4 No Policy Exists, Resolved with Distributed Policy

The target device does not have a policy for a requested operation, so there is a delay while the owner configures a distributed
policy in the Authorization_Policy property to allow the operation to succeed.

AA Device 12                ...                 AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,CONFIG_SCOPE_REQUIRED)     |
      |<--------------------...-----------------------|
      |
      |

                      ... time passes ...
              ... client periodically retries ...
    ... client may or may not also try to obtain a token ...

      |            WriteProperty-Request              |
      |---------------------...---------------------->| No distributed policy
      |                                               | Not allowed
      |                  Result(-)                    |
      |     Error(SECURITY,CONFIG_SCOPE_REQUIRED)     |
      |<--------------------...-----------------------|
      |

     ...owner configures a distributed policy to allow request...

      |                                          AA Device 56
      |                                               |
      |            WriteProperty-Request              |
      |---------------------...---------------------->| distributed policy found,
      |                                               | accept
      |                  Result(+)                    |
      |<--------------------...-----------------------|
      |                                               |

ANSI/ASHRAE Standard 135-2024

1499

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

(This annex is part of this standard and is required for its use.)

This annex introduces the BACnet Energy Services Interface (BACnet ESI) for the access of complex building data via BACnet
web services (Annex W).

AD.1 Introduction to BACnet ESI

The BACnet ESI enables a building Energy Data Client (Figure AD-1) to access complex structured energy service data e.g.,
data  received  from  an  energy  service  provider  (ESP)  along  with  data  tied  to  building  response  to  the  energy  service.  ESP
information may include demand response (DR) signals via OpenADR, utility-validated meter data via Green Button standard
formats and weather data from a weather data provider. The BACnet ESI version 1 focuses on demand response. In the DR use
case, DR signals arrive and are acted upon by the BACnet Energy Services Interface Energy Manager (ESI-EM, Figure AD-
1). The ESI-EM communicates externally with the ESP and Energy Data Clients, and internally with building system controllers
and database.

The BACnet ESI enables the energy data client to access complex structured data via BACnet Web Services (B/WS). The
building itself could have a control network that does not use BACnet. B/WS is a generic web services protocol designed for
communicating  building  control  information,  including  complex  signal  data  communicated  via  the  external  energy  service
protocols.

Figure AD-1. BACnet ESI architecture for retrieving structured energy data. That data may come from energy service
providers or from internal building system controllers.

Figure AD-1 shows a typical configuration with a BACnet ESI Energy Manager hosted by the building automation system.
The Facility Smart Grid Information Model describes the ESI-EM as a top-level energy manager that interacts with outside
energy service providers via the ESP Interfaces. Internally, the ESI-EM interacts with various devices (Figure AD-1 right side)
according to some policy for device control on the building network side to carry out energy management in response to ESP
signals. For example, if a DR event signal is received indicating an elevated level of required DR, the EM policy may specify
raising office temperature setpoints and shedding some low priority loads. The ESI-EM may issue commands to device control
points or use the BACnet Load Control Object as an indirect way to pass on event information to a subordinate energy manager.
However, the specific implementation in a building is not in scope in this annex

In some cases, there may be no control actions tied to interactions with the ESP. For example, the facility may receive weather
alerts or utility-validated meter data. The BACnet ESI will enable an energy data client to access these weather alerts and meter
data. An energy auditor may want access to meter data that comes directly from a facility meter as well as validated meter data
accessed via Green Button from the ESP.

The scope of this annex is the Energy Data Client interactions at the top of Figure AD-1. This annex presents data compositions
that collect information linked to specific energy services and explains how to access that data via B/WS. The data definitions
are  made  available  in  CSML  at  data.ashrae.org.  Clause  AD.2  presents  three  data  compositions  specific  to  DR  event  data
retrieval. Future versions of this annex may include additional data compositions such as weather data access or information
related to energy market interactions.

1500

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

In order to provide energy service-related data to clients, the BACnet ESI-EM must have access to data passing through the
ESP interfaces to capture and store these messages in a database. The information in the messages must then be composed to
provide them in the format specified in this annex and made available via B/WS. B/WS (Annex W) and the CSML information
model (Annex Y) provide the building data access methods and formats. A filtered query (as defined in Clause W.8) might be
used to retrieve event information for a specific event or for all events within a specific date range.

The data tree is as shown in Figure AD-2. The .energy tree organizes energy-related data in a building. The tree shown here
identifies “electric” data under .energy. Other energy services might include energy market transactions, which may not be
only electric, e.g., gas, water or steam. Under the electric branch, one category is “demandResponse” which is the focus of this
annex in its initial form. Other electric services might include power quality data provision, or provision of standard electric
tariff data

Figure AD-2. BACnet web services .energy data tree.

AD.2 BACnet ESI Data Classes

AD.2.1 DR Event Data

This subclause presents three data compositions and methods for retrieval of DR event-related data with formats derived from
the OpenADR schema.

1.  DREventSummary provides a summary of event information for one or more events.
2.  DRProgram provides basic OpenADR metadata describing the program plus a pointer to associated DR meter data.
3.  DRMessageLog provides a log of DR messages received or sent which may include multiple events across more than

one DR program.

OpenADR is the international standard (IEC 62746-10-1) for demand response communications via event signals, typically
used for the connection from utility to aggregator or utility to customer energy management system. OpenADR defines the
messages that move between a virtual top node (VTN, typically a utility) and virtual end node (VEN, at the customer facility).
An  event  message  provides  event  details  such  as  start  time,  duration,  level  (severity)  or  price,  and  status.  There  may  be
modifications to event details as time progresses leading up to an event. An event ID will remain unchanged for each event.

OpenADR is a service-oriented protocol, not control protocol. It is up to a facility controller to implement some local control
policy. The amount of load response (typically for peak shaving application) may or may not be mandated in a DR program
agreement.  Measurement  and  verification of  response  is  typically  performed  by  comparison  of utility  meter  data  against  a
calculated baseline. Whether there is a baseline or not and how that baseline is calculated is DR program-specific.

The identity of an OpenADR VTN is unique to a DR program, and each program participant is represented by a VEN specific
to the DR program. A given building owner may possibly participate in more than one DR program. OpenADR events are tied
to a specific DR program and the building energy performance in response to event signals are tied to energy use as recorded
by one or more utility meters.

The DREventSummary data formats are based primarily on OpenADR 3.0, but the BACnet ESI data formats are not identical
and do not include all of the OpenADR data model. This Annex defines the formats for serving data to the energy data client
and does not specify where those data come from. How a DR protocol (OpenADR or other) might be mapped to the BACnet
ESI schema is out of scope.

When querying the BACnet ESI for event data, a building client may request information on events tied to a specific program,
a specific event, all events, or only active, pending or completed events during a given time period. DREventSummary provides

ANSI/ASHRAE Standard 135-2024

1501

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

information about each event and indicates the DR program that the event is associated with. A client may also request DR
program information to learn which meters serve which DR programs.  Finally, a client may request a log of messages via the
DRMessageLog.

Section AD.3.1 presents data access methods for retrieving DR event, program and message log data.

Section AD.4.1 presents the data definitions for these event, program and log components.

AD.2.1.1 Event Summary

The  DREventSummary  composition  defined  in  Clause  AD.4.1.1  composes  event  information  from  different  OpenADR
messages exchanged between the OpenADR VTN and the customer VEN. DREventSummary includes event ID, VEN ID,
program ID, start time and duration of each event, with event signal data, modification times and other data elements. The
program ID serves as the connector between an event and a program.

Note that the number of events that are retained, or the number of days or months over which event messages and summaries
are retained is a local matter.

AD.2.1.2 DR Program Metadata

DRProgram information includes basic program information (programID, venID, and programDescription) plus a link pointing
to  some  site-specific  meter  metadata.  This  link  can  identify  a  meter  ID  or  name  (using  “urn”  or  “tag”  as  a  non-locating
“identifier”, for example, “tag:wossamotta.edu,2022:Meter12”) or may point to a meter object.

The  method  for  evaluation  of  building  performance  relative  to  DR  response  requirements  is  specific  to  each  DR  program
agreement. Understanding DR program requirements and performance evaluation are not in scope. The programDescription
URI(s) should provide understanding of the DR program requirements.

AD.2.1.3 DR Event Message Log

DRMessageLog  provides  a  record  of  all  DR  messages  stored  in  memory.  A  client  may  want  to  review  each  message  for
forensics purposes or other reasons (e.g., to look for some specific event metadata such as Target that is not included in the
current BACnet ESI DR data model) for a specific event or for some time interval.

An OpenADR VEN may be configured in PULL or PUSH mode. If in PULL mode, there may be a very large number of
repetitive  messages  that  consume  storage.  A  BACnet  ESI  may  be  configured  to  retain  only  messages  with  new  event
information or utilize some other filtering method as a local matter. If the BACnet ESI is configured to not store repetitive
messages, it is also a local matter whether the BACnet ESI indicates the number of repetitions of the dropped messages or any
other metadata.

AD.3 Data Access

AD.3.1 Demand Response

AD.3.1.1 DR Event Summary

The DREventSummary of OpenADR events can be accessed via a GET request to:

(a){prefix}/.energy/electric/demandResponse/eventSummaries/events/eventID, for a specific event,
(b){prefix}/.energy/electric/demandResponse/eventSummaries/programs/venID, for events of a specific DR program,
(c){prefix}/.energy/electric/demandResponse/eventSummaries/pending, for pending events,
(d){prefix}/.energy/electric/demandResponse/eventSummaries/active, for active events,
(e){prefix}/.energy/electric/demandResponse/eventSummaries/completed, for completed events,
(f){prefix}/.energy/electric/demandResponse/eventSummaries/all, for all events,

where {prefix} is found per Clause W.2. These events may additionally be filtered by a date range. Event summary data is
returned according to the format in Clause AD.4.1.1.

If more than one event summary is returned (multiple instances of DREventSummaryDetails), results for each event shall be
returned in order of event start time (start) from newest to oldest.

1502

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

AD.3.1.2 DR Program Metadata

A  GET  request  to  {prefix}/.energy/electric/demandResponse/programs  will  return  program  metadata  per  the  DRProgram
composition definitions in Clause AD.4.1.2. This metadata includes a list of links that may be used to point to multiple meters
associated with each DR program that has a VEN hosted by the BACnet ESI.

AD.3.1.3 DR Event Message Log

A  GET  request  to  {prefix}/.energy/electric/demandResponse/eventMessages  will  return  the  DR  message  log,  AD.4.1.3.
DRMessageLog returns a record of all DR messages retained in a database. The request may be filtered by a time window, e.g.,
events in the past 3 months, or a narrow window around a specific event.

If  a  client  wants  only  the  messages  associated  with  a  specific  DR  event,  this  can  be  accessed  with  a  GET  request  to
{prefix}/.energy/demandResponse/eventMessages/events/eventID.

If  a  client  wants  only  the  messages  associated  with  a  specific  DR  program,  this  can  be  accessed  with  a  GET  request  to
{prefix}/.energy/demandResponse/eventMessages/programs/venID.

 AD.4 Data Definitions

The CSML corresponding to the tables in this clause is available at data.ashrae.org.

AD.4.1 Demand Response

AD.4.1.1 DREventSummary

This section provides the data model components for event summaries.

Components
drEventSummaryDetails

Type, optionality
LIST of
DREventSummaryDetails

Description
Each  DREventSummaryDetails  composition  provides  a
summary of a single OpenADR event

Table AD-4.1.1 DREventSummary composition

Components
programName
venID
problem
event

Components
status

detail

type
title
instance

Table AD-4.1.1.1 DREventSummaryDetails composition

Type, optionality
String
String
Problem, OPTIONAL
Event

Description
facility user-provided name for DR program
ven/venID

key  information  elements  from  the  Event  component  of  the
most recent DR event message

Type, optionality
Integer, OPTIONAL

Table AD-4.1.1.1.1 Problem composition
Description
http status code in response from VEN back to VTN if there is an
error
provides a human readable explanation specific to this occurrence of
the problem, e.g. "Connection to database timed out"
URI identifies specific problem type
short description of problem type
URI identifying specific problem

String, OPTIONAL

Link, OPTIONAL
String, OPTIONAL
Link, OPTIONAL

ANSI/ASHRAE Standard 135-2024

1503

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

Table AD-4.1.1.1.2 Event composition
Description
Unique event ID
Server timestamp on event creation

Components
eventID
createdDateTime

modificationDateTime
programID
eventName
priority
start

Type, optionality
String, OPTIONAL
BACnetDateTime,
OPTIONAL
String, OPTIONAL
String
String, OPTIONAL
Integer, OPTIONAL
BACnetDateTime

duration

Integer

randomizeStart

Integer, OPTIONAL

payloadType

String

values

ARRAY of String

units
currency

String, OPTIONAL
String, OPTIONAL

Most recent event modification timestamp (if any)
ID attribute of program object this event is associated with
User-defined name for event (e.g., for user interface)
Relative priority of event from 0 to 3, 0 being highest priority
Start  time  of  the  event.  BACnet  ESI  supports  multi-interval  price
events where  intervals have the  same  durations, but not multi-part
events with varying durations.
duration  of  each  interval,  seconds.  If  more  than  one  interval,  then
each interval is of the same duration.
indicates  a  randomization  time  that  may  be  applied  to  start,  in
seconds.
From OA3 eventPayloadDescriptor/payloadType , e.g., “SIMPLE”
or “PRICE”.
OA3 event level (SIMPLE) is type INT, PRICE is type REAL. OA3
Data values : event/payloads/valuesMap/values

Note: Duration provides the length of each interval, and the length
of this values array times duration provides the total event duration
(if more than one interval).
eventPayloadDescriptor/units
eventPayloadDescriptor/currency

AD.4.1.2 DR Program type definitions

DR Program provides program metadata with meter metadata for each DR program.

Components
programID
programName
programLongName
retailerName
programType
venID
programDescription

drMeter

Type, optionality
String
String
String, OPTIONAL
String, OPTIONAL
String, OPTIONAL
String, OPTIONAL
LIST
of
OPTIONAL
LIST
of
OPTIONAL

Table AD-4.1.2 DRProgram composition
Description
VTN assigned program ID
User provided short name for program, e.g., ComTOU
User provided long name for program, e.g., Commercial TOU-A
Name of energy retailer providing the program.
user-defined program category (e.g., “Pricing_Tariff”)
Virtual end node object ID
URI(s)  pointing  to  human/machine-readable  content.  This  element
was previously called marketContext.
Links point to some site-specific meter metadata. A link may point to
a  meter  object,  where  "http"  or  “https"  points  to  a  BACnet/WS
accessible resource, or may identify a meter ID or name using “urn”
or
example,
“tag:wossamotta.edu,2022:Meter12”).  There may be more than one
meter associated with a DR program.

non-locating

“identifier”

Link,

Link,

“tag”

(for

as

a

1504

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX AD - BACNET ENERGY SERVICES INTERFACE (NORMATIVE)

AD.4.1.3 DRMessageLog type definition

Log of DR messages exchanged with facility which may include messages for more than one event and messages associated
with more than one DR program. Message log may be constrained by available storage or may not include all messages per
policy.

Components
logStartTime
logEndTime
logDescription
drLog

Table AD-4.1.3 DRMessageLog type definition

memberType, optionality
DateTime
DateTime
String OPTIONAL
LIST of DRLog

Description
start date and time for log record
end date and time for log record
description of log contents
message log for each event of each DR program

Table AD-4.1.3.1 DRLog composition

Components
venID
programID
programDescription
eventID
drLogMessageText

Type, optionality
String
String
LIST of Link, OPTIONAL
String
LIST of DRLogMessageText

Description
VEN ID with format specified by DR program
DR program-specific ID
URL(s) pointing to human/machine-readable content
unique ID for each event with format defined by DR Program
flattened text of each DR event-related message

Table AD-4.1.3.1.1 DRLogMessageText composition

Components
messageTime
messageBody

Type, optionality
DateTime
String

Description
datetime message was received at or sent from the VEN
flattened message text

AD.5 Examples

Consider that a building client is not already configured to read DR program information or meter data. In this case, the client
may request a summary of all DR events:

{prefix}/.energy/electric/demandResponse/eventSummaries/all

The server will return the DREventSummary composition with a summary of all event information in memory for each event
that has been completed or is pending or active. This event summary data will include the programID which may be used to tie
a given eventID to a specific meter by using a GET request to:

{prefix}/.energy/electric/demandResponse/programs

This will return the metadata for each DR program and links to meters associated with those DR program(s). The building
client will then know about every stored event and know which meter(s) serve for measurement and verification for any specific
event.

ANSI/ASHRAE Standard 135-2024

1505

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

HISTORY OF REVISIONS

Protocol

Version
1

Revision
NA

Summary of Changes to the Standard

ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee June 28, 1995; by the ASHRAE
Board of Directors June 29, 1995; and by the American National Standards Institute
December19, 1995.

1

NA

Addendum a to ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee January 23, 1999; by the ASHRAE
Board  of  Directors  January  27,  1999;  and  by  the  American  National  Standards
Institute October 1, 1999.

1.  Add Annex J - BACnet/IP and supporting definitions

1

1

Addendum b to ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee February 5, 2000; by the ASHRAE
Board  of  Directors  February  10,  2000;  and  by  the  American  National  Standards
Institute April 25, 2000.

1.

Inconsistencies are eliminated in the definitions of the Analog and Binary
Value object types

2.  Any  device  that  receives  and  executes  UnconfirmedEventNotification

service requests must support programmable process identifiers

3.  Modify each event-generating object type to contain the last timestamp for

each acknowledgeable transition

4.  Modify the Notification Class object by requiring that the 'Notification Class'
property  be  equivalent  to  the  instance  number  of  the  Notification  Class
object

5.  Modify  the  Event  Notification  services  to  make  the  'To  State'  parameter

mandatory for notifications of type ACK_NOTIFICATION

6.  A new BACnetDeviceObjectPropertyReference production is added and its

use in the Event Enrollment and Schedule object types is specified

7.  Add a Multi-state Value object type
8.  Add an Averaging object type
9.  Change all 'Process Identifier' properties and parameters to Unsigned32
10.  Change  the  Multi-state  Input  object  type  to  correct  flaws  related  to  fault
detection and reporting and achieve  consistency with the  proposed Multi-
state Value object type

11.  Add a Protocol_Revision property to the Device object type
12.  The  File  object  type  is  changed  to  allow  truncation  and  partial  deletion

operations

13.  A new ReadRange service is added to permit reading a range of data items

from a property whose datatype is a list or array of lists

14.  A new UTCTimeSynchronization service is introduced and related changes

are made to properties in the Device object type

15.  Add a Trend Log object type
16.  The UnconfirmedCOVNotification service is extended to allow notifications
without prior subscription as a means of distributing globally important data
to a potentially large number of recipients
17.  Add eight new BACnet engineering units.

1506

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

2

Addendum c to ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee June 23, 2001; by the ASHRAE
Board of Directors June 28, 2001; and by the American National Standards Institute
September 7, 2001.

1.  Add a new Life Safety Point object type that represents the characteristics of
initiating  and  indicating  devices  in  the  fire,  life  safety,  and  security
applications

2.  Add a new Life Safety Zone object type that represents the characteristics
associated  with  an  arbitrary  group  of  BACnet  Life  Safety  Point  and  Life
Safety Zone objects

3.  Add functionality to the existing BACnet alarm and event features needed to

support the Life Safety Point and Life Safety Zone object types

4.  Add  a  new  LifeSafetyOperation  service  that  provides  silence  and  reset

capabilities needed for life safety systems

5.  Add a new clause to 19 to describe the use of existing BACnet services to

provide backup and restore capability

6.  Define a new service, SubscribeCOVProperty, to allow COV notifications
for  arbitrary  properties  of  an  object  with  subscriber-specified  COV
increments

7.  Add Vendor ID to proprietary MS/TP frames
8.  Add a new service, GetEventInformation, that provides enough information

to acknowledge alarms

1

2

Addendum d to ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee June 23, 2001; by the ASHRAE
Board of Directors June 28, 2001; and by the American National Standards Institute
September 7, 2001.

1.  Replace  Clause  22  with  a  new  clause  entitled  "Conformance  and

Interoperability".

2.  Update Annex A, "Protocol Implementation Conformance Statement".
3.  Add  a  new  Annex  K  entitled  "BACnet  Interoperability  Building  Blocks

(BIBBs)".

4.  Add  a  new  Annex  L  entitled  "Descriptions  and  Profiles  of  Standardized

BACnet Devices".

1

2

Addendum e to ANSI/ASHRAE 135-1995
Approved by the ASHRAE Standards Committee June 23, 2001; by the ASHRAE
Board of Directors June 28, 2001; and by the American National Standards Institute
September 7, 2001.

1.  Define the PTP connection status when the half-router can and cannot re-

establish the connection.

2.  Add Object Profiles and Extensions.
3.  Add the capability for devices to advertise the maximum number of

segments of a segmented APDU that they can receive.

1

1

2

2

ANSI/ASHRAE 135-2001
A consolidated version of the standard that incorporates all of the known errata and
Addenda a, b, c, d, and e to ANSI/ASHRAE 135-1995.

ANSI/ASHRAE 135-2001 (reprinted May, 2002)
This reprinted version incorporated all errata known as of April 12, 2002.

ANSI/ASHRAE Standard 135-2024

1507

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

3

Addendum b to ANSI/ASHRAE 135-2001
Approved by the ASHRAE Standards Committee January 25, 2003; by the ASHRAE
Board  of  Directors  January  30,  2003;  and  by  the  American  National  Standards
Institute April 3, 2003.

1.  Remove UTC timestamps from Trend Logs and guarantee Trend Log record

ordering.

1

1

3

4

EN ISO 16484-5 2003
This  ISO  standard  contains  the  same  technical  content  as Version  1  Revision  3  of
ANSI/ASHRAE Standard 135-2001. It also includes all errata approved as of April
24, 2003.

Addendum a to ANSI/ASHRAE 135-2001
Approved by the ASHRAE Standards Committee October 5, 2003; by the ASHRAE
Board  of  Directors  January  29,  2004;  and  by  the  American  National  Standards
Institute February 15, 2004.

1.  Add Partial Day Scheduling to the Schedule object.
2.  Enable reporting of proprietary events by the Event Enrollment object.
3.  Allow detailed error reporting when all ReadPropertyMultiple accesses

fail.

4.  Remove the Recipient property from the Event Enrollment object.
5.  Add the capability to issue I-Am responses on behalf of MS/TP

subordinate devices.

6.  Add a new silenced mode to the DeviceCommunicationControl service.
7.  Add 21 new engineering units.
8.  Specify the behavior of a BACnetARRAY when its size is changed.
9.  Clarify the behavior of a BACnet router when it receives an unknown

network message type.

1

4

Addendum c to ANSI/ASHRAE 135-2001
Approved by the ASHRAE Standards Committee October 5, 2003;by the ASHRAE
Board  of  Directors  January  29,  2004;  and  by  the  American  National  Standards
Institute February 15, 2004.

1.  Allow Life Safety objects to advertise supported mode.
2.  Add Unsilence Options to the LifeSafetyOperation Service.
3.  Specify the relationship between the Event_Type and Event_Parameter

properties.

4.  Add a new Accumulator Object Type.
5.  Add a new Pulse Converter Object Type.
6.  Standardize event notification priorities.
7.  Define Abort reason when insufficient segments are available.
8.  Add new Error Codes and specify usage.

1

4

Addendum d to ANSI/ASHRAE 135-2001
Approved by the ASHRAE Standards Committee October 5, 2003; by the ASHRAE
Board  of  Directors  January  29,  2004;  and  by  the  American  National  Standards
Institute February 15, 2004.

1.  Add clauses describing BACnet-EIB/KNX mapping.

1508

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

1

1

4

4

5

ANSI/ASHRAE 135-2004
A consolidated version of the standard that incorporates all of the known errata and
Addenda a, b, c, and d to ANSI/ASHRAE 135-2001.

ANSI/ASHRAE 135-2004 (reprinted October, 2005)
This reprinted version incorporated all errata known as of September 30, 2005.

Addendum a to ANSI/ASHRAE 135-2004
Approved by the ASHRAE Standards Committee October 3, 2004; by the ASHRAE
Board  of  Directors  February  10,  2005;  and  by  the  American  National  Standards
Institute February 10, 2005.

1.  Revise Life Safety Point and Life Safety Zone objects to modify their

behavior when placed out of service.

1

5

Addendum c to ANSI/ASHRAE 135-2004
Approved  by  the  ASHRAE  Standards  Committee  September  29,  2006  and  by  the
ASHRAE  Board  of  Directors  September  29,  2006;  and  by  the  American  National
Standards Institute October 2, 2006.

1.  Add BACnet/WS Web Services Interface.

1

5

Addendum d to ANSI/ASHRAE 135-2004
Approved by the ASHRAE Standards Committee June 24, 2006, and by the ASHRAE
Board of Directors June 29, 2006; and by the American National Standards Institute
June 30, 2006.

1.  Add a new Structured View object type.
2.  Allow acknowledgment of unseen TO_OFFNORMAL event notification.
3.  Relax the Private Transfer and Text Message BIBB requirements.
4.  Exclude LIFE_SAFETY and BUFFER_READY notifications from the

Alarm Notifications BIBBs.

5.  Establish the minimum requirements for a BACnet device with an

application layer.

6.  Remove the requirement for the DM-DOB-A BIBB from the B-OWS and

B-BC device profiles.

7.  Relax mandated values for APDU timeouts and retries when configurable,

and change default values.

8.  Fix EventCount handling error in MS/TP Manager Node State Machine.
9.  Permit routers to use a local network number in Device_Address_Binding.
10.  Identify conditionally writable properties.
11.  Specify Error returns for the AcknowledgeAlarm service.

1

6

Addendum e to ANSI/ASHRAE 135-2004
Approved by the ASHRAE Standards Committee January 27, 2007, by the ASHRAE
Board of Directors March 25, 2007; and by the American National Standards Institute
March 26, 2007.

1.  Add a new Load Control object type.

ANSI/ASHRAE Standard 135-2024

1509

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

6

Addendum f to ANSI/ASHRAE 135-2004
Approved  by  the  ASHRAE  Standards  Committee  January  27,  2007,  and  by  the
ASHRAE  Board  of  Directors  March  25,  2007,  and  by  the  American  National
Standards Institute March 26, 2007.

1.  Add new Access Door object type.

1

1

6

7

Amendment 1 to EN ISO 16484-5 2007
This  amendment  to  the  ISO  standard  contains  the  same  technical  content  as  the
cumulative changes in Addenda  a, c, d, e, and  f to ANSI/ASHRAE Standard 135-
2004.

Addendum b to ANSI/ASHRAE 135-2004
Approved by the ASHRAE Standards Committee October 12, 2008, by the ASHRAE
Board  of  Directors  October  24,  2008,  and  by  the  American  National  Standards
Institute October 27, 2008.

1.  Add a new Event Log object type.
2.  Add a new Global Group object type. (Removed after third public review.)
3.  Add a new Trend Log Multiple object type.
4.  Harmonize the Trend Log object with the new Event Log and Trend Log

Multiple objects.

5.  Define a means for a device to provide a notification that it has restarted.
6.  Define a means to configure a device to periodically send time

synchronization messages.

7.  Extend the number of character sets supported. (Removed after first public

review.)

8.  Enable devices other than alarm recipients to acknowledge alarms.
9.  Allow MS/TP BACnet Data Expecting Reply frames to be broadcast.
10.  Revise the Clause 5 state machines to handle slow servers. (Removed after

second public review.)

11.  Add new Error Codes and specify usage.
12.  Add new Reliability enumeration to objects with a Reliability property.

1

7

Addendum m to ANSI/ASHRAE 135-2004
Approved by the ASHRAE Standards Committee October 12, 2008, by the ASHRAE
Board  of  Directors  October  24,  2008,  and  by  the  American  National  Standards
Institute October 27, 2008.

1.  Resolve Foreign Device registration grace period and remaining time

ambiguities.
Improve Clause 5 FillWindow segment timeout constraints.

2.
3.  Clarify  the  Priority  Filter  parameter  in  the  GetEventEnrollment  service

request.

4.  Allow alarms to be re-acknowledged successfully.
5.  Add requirements to Alarm and Event BIBBs.
6.  Remove B-BC requirements for BIBBs without use cases.
7.  Clarify that a device may support only the ReinitializeDevice restart choices.
8.  Clarify DeviceCommunicationControl and ReinitializeDevice interactions.
9.  Define "object."
10.  Add a Deadband property to the Loop object.
11.  Correct  the  TO_FAULT  conditions  in  the  Life  Safety  objects'  Reliability

properties.

12.  Clarify the Trend Log's acquisition of Status_Flags.

1510

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

1

1

7

7

8

ANSI/ASHRAE 135-2008
A consolidated version of the standard that incorporates all of the known errata and
Addenda a, b, c, d, e, f, and m to ANSI/ASHRAE 135-2004.

EN ISO 16484-5 2010
This  ISO  standard  contains  the  same  technical  content  as Version  1  Revision  7  of
ANSI/ASHRAE Standard 135-2008. It also includes all errata approved as of May 6,
2009.
Addendum q to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 24, 2009; by the ASHRAE
Board  of  Directors  January  28,  2009;  and  by  the  American  National  Standards
Institute January 29, 2009.

1.  Allow unicast I-Ams.
2.  Define virtual addressing for data links with MAC addresses longer than 6

octets.

3.  Define the use of ZigBee as a BACnet data link layer.

1

9

Addendum j to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Add a new Access Point object type.
2.  Add a new Access Zone object type.
3.  Add a new Access User object type.
4.  Add a new Access Rights object type.
5.  Add a new Access Credential object type.
6.  Add a new Credential Data Input object type.
7.  Add a new ACCESS_EVENT event algorithm.
8.  Add a new ANNEX P BACnet encoding rules for authentication factor

values

1

9

Addendum l to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Add new workstation BIBBs and profiles.

1

9

Addendum o to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Accommodate remote operator access and NAT in Annex J BACnet/IP.

ANSI/ASHRAE Standard 135-2024

1511

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

9

Addendum r to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Clarify transitions in FLOATING_LIMIT and OUT_OF_RANGE events.
2.  Clarify router action when a network is marked as temporarily unreachable.
3.  Clarify the destination MAC used when replying to a broadcast DER

frame.

4.  Clarify the handling of write priorities greater than 16.
5.  Clarify LogDatum presentation.

1

9

Addendum s to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Clarify the circumstances that cause the File object's Archive property to be

set to TRUE or FALSE.

2.  Require support for COV subscriptions of at least 8 hours' lifetime.

1

9

Addendum v to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 20, 2009; by the ASHRAE
Board of Directors June 24, 2009; and by the American National Standards Institute
June 25, 2009.

1.  Fix the MS/TP TokenCount Value.
2.  Clarify "Supported".
3.  Remove NM-CE-A from Device Profiles.

1

10

Addendum h to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Change Device_Busy to Busy and apply to the Command Object type.
2.  Prevent overflow and underflow in Pulse_Converter object's Count

property.

3.  Add context tags to Clause 21 production BACnetPropertyStates.
4.  Add new BACnetEngineering Units.
5.  Define COV notification service Error returns.
6.  Remove non-support for automatic cancellation of COV subscriptions.
7.
8.  Add even and odd day support in Dates.

[This section was removed from this addendum]

1

10

Addendum k to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Add support for UTF-8.
2.  Change JIS Reference.

1512

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

10

Addendum n to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Add support for long Backup and Restore preparation times.

1

10

Addendum t to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Add XML data formats.

1

10

Addendum u to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Clarify the use of BACnet-Reject-PDUs.
2.  Add error code UNSUPPORTED_OBJECT_TYPE for CreateObject

service.

3.  Add new Abort and Error codes.
4.  Specify proper Errors when attempting access to the Log_Buffer property.

1

10

Addendum w to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Add more primitive value objects.

1

10

Addendum x to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Fix the Criteria for COV for Load Control.
2.  Clarify Trend Log Time Stamp.
3.  Clarify ReadRange on Lists.
4.  Clarify Results of Using Special Property Identifiers.

1

10

Addendum y to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 23, 2010; by the ASHRAE
Board  of  Directors  January  27,  2010;  and  by  the  American  National  Standards
Institute January 28, 2010.

1.  Specify Deployment Options for MS/TP.

ANSI/ASHRAE Standard 135-2024

1513

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

11

Addendum g to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 26, 2010; by the ASHRAE
Board of Directors June 30, 2010; and by the American National Standards Institute
July 1, 2010.

1.  Update BACnet Network Security

1

11

Addendum p to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee June 26, 2010; by the ASHRAE
Board of Directors June 30, 2010; and by the American National Standards Institute
July 1, 2010.

1.  Add a new Global Group object type.

1

11

Addendum z to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee  June 26, 2010; by the ASHRAE
Board of Directors June 30, 2010; and by the American National Standards Institute
July 1, 2010.

1.  Add Event_Message_Texts.
2.  Add UnconfirmedEventNotification to Automated Trend Retrieval BIBBs.
3.  Modify MS/TP State Machine to Ignore Data Not For Us
4.  Add New Engineering Units
5.  Add Duplicate Segment Detection

1

12

Addendum ab to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 29, 2011; by the ASHRAE
Board  of  Directors  February  2,  2011;  and  by  the  American  National  Standards
Institute February 3, 2011.

1.  Add More Standard Baud Rates for MS/TP

1

12

Addendum ac to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 29, 2011; by the ASHRAE
Board  of  Directors  February  2,  2011;  and  by  the  American  National  Standards
Institute February 3, 2011.

1.  Clarify the Usage of Dates and Times.

1

12

Addendum ag to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 29, 2011; by the ASHRAE
Board  of  Directors  February  2,  2011;  and  by  the  American  National  Standards
Institute February 3, 2011.

1.  Prevent BBMD Broadcast Storms.
2.  Align BIBBs for Automated Trend Retrieval.

1

12

Addendum ah to ANSI/ASHRAE 135-2008
Approved by the ASHRAE Standards Committee January 29, 2011; by the ASHRAE
Board  of  Directors  February  2,  2011;  and  by  the  American  National  Standards
Institute March 3, 2011.

1.  Remove ReadPropertyConditional.

1514

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

1

1

12

12

13

ANSI/ASHRAE 135-2010
A consolidated version of the standard that incorporates all of the known errata and
Addenda  g,  h,  j,  k,  l,  n,  o,  p,  q,  r,  s,  t,  u,  v,  w,  x,  y,  z,  ab,  ac,  ag,  and  ah  to
ANSI/ASHRAE 135-2008.

EN ISO 16484-5 2012
This ISO standard contains the same technical content as Version 1 Revision 12 of
ANSI/ASHRAE Standard 135-2010.
Addendum ad to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee June 25, 2011; by the ASHRAE
Board of Directors June 29, 2011; and by the American National Standards Institute
June 30, 2011.

1.  Provide Examples of Encoding Tag Numbers Greater than 14
2.  Allow Feedback_Value to be used to calculate Elapsed_Active_Time
3.  Add READ_ACCESS_DENIED condition to ReadProperty and

ReadPropertyMultiple

4.  Remove Unqualified Frame Reference in USE_TOKEN
5.  Align the Loop Object's Out_Of_Service Behavior with Other Objects
6.  Add DM-DDB-A to the Device Profile B-AAC
7.  Clarify Requirements for BBMDs
8.  Restrict BBMD Foreign Device Forwarding
9.  Restrict ReadRange 'Count' to Integer16

1

13

Addendum ae to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee June 25, 2011; by the ASHRAE
Board of Directors June 29, 2011; and by the American National Standards Institute
June 30, 2011.

1.  Add a "Too large" error condition to the ERROR authentication encoding
2.  Simplify the Initialization of Negative and Positive Access Rules
3.  Replace Master_Exemption Property of the Access Credential Object Type
4.  Add Fault Enumeration to Door_Status in Access Door Object Type
5.  Clarify  the  behavior  of  Door_Unlock_Delay_Time  and  Present_Value  of

Access Door

ANSI/ASHRAE Standard 135-2024

1515

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

13

Addendum af to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee June 25, 2011; by the ASHRAE
Board of Directors June 29, 2011; and by the American National Standards Institute
June 30, 2011.

1.  Remove Annex C and Annex D
2.  Clarify Optionality of Properties Related to Intrinsic Event Reporting
3.  Clarify Optionality of Properties Related to Change of Value Reporting
4.  Ensure that Pulse_Rate and Limit_Monitoring_Interval are Always Together
5.  Clarify when Priority_Array and Relinquish_Default are allowed to be Present
6.  Clarify when Segmentation Related Properties are Allowed to be Present
7.  Clarify when Virtual Terminal Related Properties are Allowed to be Present
8.  Clarify when Time Sync Interval Properties are Allowed to be Present
9.  Clarify when Backup and Restore Properties are Allowed to be Present
10.  Clarify  when  the  Active_COV_Subscriptions  Property  is  Allowed  to  be

Present

11.  Clarify when the Subordinate Proxy Properties are Allowed to be Present
12.  Clarify when the Restart Related Properties are Allowed to be Present
13.  Clarify  when  the  Log_DeviceObjectProperty  Property  is  Allowed  to  be

Present

14.  Clarify when the Clock Aligning Properties are Allowed to be Present
15.  Clarify when the Occupancy Counting Properties are Allowed to be Present
16.  Add the Ability to Configure Event Message Text
17.  Add an Event Detection Enable / Disable Property
18.  Add the Ability to Dynamically Suppress Event Detection
19.  Add  the  Ability  to  Specify  a  Different  Time  Delay  for  TO_NORMAL

Transitions

20.  Add the Ability to Inhibit the Evaluation of Fault Conditions
21.  Separate the Detection of Fault Conditions from Intrinsic Reporting
22.  Ensure that Event Notifications are not Ignored due to Character Set Issues
23.  Make the Event Reporting Property Descriptions Consistent
24.  Identify the Property in each Object that is Monitored by Intrinsic Reporting
25.  Change the Description of the Reliability Property
26.  Improve Fault Detection in Event Enrollment Objects
27.  Add the Ability for some Objects Types to Send Only Fault Notifications
28.  Add a Notification Forwarder Object Type
29.  Reduce the Requirements on Notification-Servers
30.  Add an Alert Enrollment Object Type
31.  Improve the Specification of Event Reporting

1

14

Addendum aa to ANSI/ASHRAE 135-2010
Approved  by  the  ASHRAE  Standards  Committee  June  23,  2012;  by  the  ASHRAE
Board of Directors June 27, 2012; and by the  American National Standards Institute
July 26, 2012.

1.  Add Channel Object Type
2.  Add WriteGroup Service

1516

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

14

Addendum ao to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee October 2, 2012; by the ASHRAE
Board of Directors October 26, 2012; and by the American National Standards Institute
October 27, 2012.

1.  Update ReadRange Example
2.  Add Present Value Range to Value Objects
3.  Clarify Reject-Message-To-Network reason #3 DNET
4.  Prevent Reliance on Static Router Bindings
5.  Add Property_List Property

1

14

Addendum ak to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee June 23, 2012; by the ASHRAE
Board of Directors June 27, 2012; and by the American National Standards Institute
June 28, 2012.

1.  Specify Address Range Requirements
2.  Specify 'abort-reason' Values
3.  Add Serial_Number Property

1

14

Addendum i to ANSI/ASHRAE 135-2010
Approved by the ASHRAE Standards Committee October 2, 2012; by the ASHRAE
Board of Directors October 26, 2012; and by the American National Standards Institute
October 27, 2012.

1.  Add Lighting Output Type

1

1

14

15

ANSI/ASHRAE 135-2012
A consolidated version of the standard that incorporates all of the known errata and
Addenda i, aa, ad, ae, af, ak, and ao to ANSI/ASHRAE 135-2010.

Addendum ar to ANSI/ASHRAE 135-2012
Approved by the ASHRAE Standards Committee January 26, 2013; by the ASHRAE
Board of Directors January 30, 2013; and by the American National Standards Institute
January 30, 2013.

1.  Add New Engineering Units
2.  Clarify Coercion Requirements
3.  Specify SubscribeCOVProperty Error Codes
4.  Add Subordinate Proxy BIBBs
5.  Allow Unicast I-Have Messages
6.  Require Both Time Sync Services for Time Distributors

1

16

Addendum an to ANSI/ASHRAE 135-2012
Approved  by  the  ASHRAE  Standards  Committee  June  28,  2014;  by  the  ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute July
3, 2014.

1.  Add Extended Length MS/TP Frames
2.  Add Procedure for Determining Maximum Conveyable APDU

ANSI/ASHRAE Standard 135-2024

1517

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

16

Addendum at to ANSI/ASHRAE 135-2012
Approved  by  the  ASHRAE  Standards  Committee  June  28,  2014;  by  the  ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute July
3, 2014.

1.  Add Interface_Value Property

1

16

Addendum au to ANSI/ASHRAE 135-2012
Approved  by  the  ASHRAE  Standards  Committee  June  28,  2014;  by  the  ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute July
3, 2014.

1.  Clarify Authentication Factor Value Encoding Rules
2.  Clarify Coercion Support Requirements

1

16

Addendum av to ANSI/ASHRAE 135-2012
Approved  by  the  ASHRAE  Standards  Committee  June  28,  2014;  by  the  ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute July
3, 2014.

1.  Deprecate Execution of GetAlarmSummary
2.  Deprecate Execution of GetEnrollmentSummary

1

16

Addendum aw to ANSI/ASHRAE 135-2012
Approved  by  the  ASHRAE  Standards  Committee  June  28,  2014;  by  the  ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute July
3, 2014.

1.  Extend the CHANGE-OF-STATE Event Algorithm for All Discrete Types
2.  Add a New Event Algorithm CHANGE_OF_DISCRETE_VALUE
3.  Add a New Fault Algorithm FAULT_OUT_OF_RANGE
4.  Extend the Loop Object Type to Support Specific Low and High Error Limits
5.  Add the Ability to Report Faults to Date and Time Related Value Objects
6.  Add the Ability to Report  Faults to  the Command, Device and Notification

Class Objects

1

16

Addendum ax to ANSI/ASHRAE 135-2012
Approved by the ASHRAE Standards Committee June 28, 2014; by the ASHRAE
Board of Directors July 2, 2014; and by the American  National Standards Institute
July 3, 2014.

1.  Remove Incorrect Recipient_List Requirement to be Non-empty
2.  Section Removed
3.  Extend the Allowable BACnetPropertyStates Enumeration Range
4.  Specifically Disallow Duplicate Time Entries in Schedules
5.  Non-BBMD Responses to BBMD BVLL Requests

1518

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

16

Addendum az to ANSI/ASHRAE 135-2012
Approved by the ASHRAE Standards Committee June 28, 2014; by the ASHRAE
Board of Directors July 2, 2014; and by the American National Standards Institute
July 3, 2014.

1.  Add Binary Lighting Output Object Type
2.  Setting Non-zero Values to Change_Of_State_Count and

Elapsed_Active_Time

1

17

Addendum ai to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  December  30,  2014;  and  by  the  American  National
Standards Institute on December 31, 2014.

1.  Add Network Port Object Type
2.  Changes to Annex J for the Network Port Object
3.  Changes to 135-2012al for the Network Port Object

1

17

Addendum al to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  December  30,  2014;  and  by  the  American  National
Standards Institute on December 31, 2014.

1.  Specify Best Practices for Gateway Design
2.  Add new BIBBS and Devices Profiles

1

17

Addendum as to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  December  30,  2014;  and  by  the  American  National
Standards Institute on December 31, 2014.

1.  Add Value Source Information

1

17

Addendum ay to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  December  30,  2014;  and  by  the  American  National
Standards Institute on December 31, 2014.

1.  Add a Timer Object Type
2.  Correct  Expiry_Time  property  name  to  Expiration_Time  in  the  Access

Credential Object

1

18

Addendum aj to ANSI/ASHRAE 135-2012
Approved by ASHRAE on February 29, 2016; and by the American National Standards
Institute on March 1, 2016.

1.  Add support for IPv6
2.  Add an additional method for VMAC determination

1

18

Addendum aq to ANSI/ASHRAE 135-2012
Approved by ASHRAE on February 29, 2016; and by the American National Standards
Institute on March 1, 2016.

1.  Add Elevator Object Types
2.  Add COV Property Multiple Services
3.  Add a New Fault Algorithm FAULT_LISTED

ANSI/ASHRAE Standard 135-2024

1519

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

18

Addendum bf to ANSI/ASHRAE 135-2012
Approved by ASHRAE on Februray 29, 2016; and by the American National Standards
Institute on March 1, 2016.

1.  Advanced Network Configuration
2.  BVLL Responses for non-BBMD Devices

1

18

Addendum bg to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  February  29,  2016;  and  by  the  American  National
Standards Institute on March 1, 2016.

1.  Add engineering units
2.  Harmonize the message text handling for all alarm services
3.  Ensure  Alert  Enrollment  objects  do  not  send  notifications  which  require

acknowledgment

4.  Allow selection of the Nth last day of the month in a BACnetWeekNDay
5.  Remove initiation of GetEnrollmentSummary from AE-AS-A
6.  Ensure UTC_Offset is configurable
7.  Clarify ReadRange
8.  Clarify the effect of changing Buffer_Size
9.  Stop  MS/TP  nodes  from  sending  POLL_FOR_  MANAGER  frames  to

themselves

10.  Improve the Clause 12 preamble
11.  Fix the Notification_Class property of the Notification Class object

1

18

Addendum bh to ANSI/ASHRAE 135-2012
Approved  by  ASHRAE  on  February  29,  2016;  and  by  the  American  National
Standards Institute on March 1, 2016.

1.  Correct Application State Machine Failover
2.

Increase Segmentation Window Size for MS/TP

1

19

Addendum am to ANSI/ASHRAE 135-2012
Approved by ASHRAE on April 29, 2016; and by the American National Standards
Institute on April 29, 2016.

1.  Extend  BACnet/WS  with  RESTful  services  for  complex  data  types  and

subscriptions

2.  Extract data model from Annex Q into separate common model
3.  Rework Annex Q to be an XML syntax for the common model
4.  Add a JSON syntax for the common model
5.  Deprecate Annex N SOAP services and add a migration guide
6.  Change Clause 21 identifiers to use a consistent format

1

19

Addendum ba to ANSI/ASHRAE 135-2012
Approved by ASHRAE on April 29, 2016; and by the American National Standards
Institute on April 29, 2016.

1.  Add CSML Descriptions of BACnet Devices
2.  Add Semantic Tags to All Objects,
3.  Extend Structured View Object to Contain Semantic Information
4.  Change Clause 21 identifiers to use a consistent format
5.  Add Data Revisioning Capabilities to CSML

1520

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

19

Addendum bc to ANSI/ASHRAE 135-2012
Approved by ASHRAE on April 29, 2016; and by the American National Standards
Institute on April 29, 2016.

1.  Extend BIBBs for Primitive Value Objects
2.  Add New BIBBs for Event Enrollment and Subscription
3.  Amend B-AWS Related BIBBs for Revised Event Reporting
4.  Add Life Safety BIBBs and Device Profiles
5.  Add Physical Access Control BIBBs and Device Profiles
6.  Add an All-Domain Advanced Workstation Profile

1

1

19

20

ANSI/ASHRAE 135-2016
A consolidated version of the standard that incorporates all of the known errata and
Addenda ai, aj, al, am, an, aq, ar, as, at, au, av, aw, ax, ay, az, ba, bc, bf, bg, and bh
to ANSI/ASHRAE 135-2012.

Addendum bd to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Add Staging Object Type

1

20

Addendum be to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Add Lighting BIBBs and Device Profiles

1

20

Addendum bi to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Add Audit Reporting.
2.  Change DeviceCommunicationControl Service for Audit Reporting.
3.  Modify Logging Objects to Allow for Extremely Large Logs.

1

20

Addendum bk to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Expand the reserved range of BACnetPropertyIdentifier

1

20

Addendum bl to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Clarify Result(-) response for failed WritePropertyMultiple requests.
2.  Clarify ReadPropertyMultiple response on OPTIONAL when empty.
3.  Clarify Out Of_Service.

ANSI/ASHRAE Standard 135-2024

1521

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

20

Addendum bm to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Reduce allowed range for Usage Timeout.
2.  Specify design choices for MS/TP devices.
3.  Handle unwanted MS/TP frames in IDLE state.

1

20

Addendum bn to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Make SCHED BIBBs consistent on supported datatypes, and add Boolean.
2.  Clarify COV and COVP related BIBBs.
3.  Clock is required for support of AE-ACK-A.

1

20

Addendum bp to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Make rules for POST consistent with rules for PUT
2.  Make 'type' consistent at all levels and introduce 'effectiveType'
3.  Fully specify the behavior of "includes"
4.  Remove the path syntax from the 'select' query parameter
5.  Resolve  conflicting  statements  about  configuring  external  authorization

servers

6.  Remove incorrect table for callback formats
7.  Allow plain text POSTs for primitive data
8.  Allow extended error numbers
9.  Add new error numbers
10.  Add formal definition for JSON equivalent to XML's <CSML>
11.  Specify 'name' safety check for setting data
12.  Specify how to evaluate relative paths for collections of links
13.  Allow proprietary categories for the 'metadata' query parameter

1

20

Addendum bq to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE on June 15, 2018; and by the American National Standards
Institute on June 15, 2018.

1.  Fix the Absentee_Limit property of the Access Credential object type.
2.  Ensure that the denied or granted access event is generated last.

1522

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

21

Addendum br to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE and by the American National Standards Institute on August
26, 2019.

1.  Add new engineering units.
2.  Add mandate to accept writes of Null to non-commandable properties.
3.  Add intrinsic fault reporting to Lighting Output object type.
4.  Deprecate Time form of timestamps.
5.  Clarify the Multi-state object types when Number_Of_States shrinks.
6.  Fix  the  language  for  event  type  and  message  text  parameters  of  event

notifications.

7.  Clarify the object instance 4194303.
8.  Wildcard instance for Network Port objects in ReadPropertyMultiple requests
9.  Clarify the timestamp of trend log and trend log multiple log records.

1

21

Addendum bs to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE and by the American National Standards Institute on August
26, 2019.

1.  Add Elevator BIBBs and Device Profiles

1

21

Addendum bt to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE and by the American National Standards Institute on August
26, 2019.

1.  Add  re-alert  transitions  to  the  CHANGE_OF_LIFE_SAFETY  event

algorithm.

2.  Add specific error codes for LifeSafetyOperation error situations.
3.  Add support for elevator based occupant evacuation (OEO) to the life safety

objects.

1

21

Addendum bu to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE and by the American National Standards Institute on August
26, 2019.

Introduce BACnetARRAY of BACnetLIST collection property data type.

1.
2.  Clarifications on character and value encoding issues.
3.  Clarify transmission of unconfirmed COV notifications.
4.  Clarify logging of event notifications.
5.  Clarify recording of status events in log buffers.
6.  Clarify the Event Enrollment object reliability-evaluation.
7.  Clarify the Global Group object reliability-evaluation.

1

21

Addendum bw to ANSI/ASHRAE Standard 135-2016
Approved by ASHRAE and by the American National Standards Institute on August
26, 2019.

1.  Add Time Series Data Exchange File Format.

ANSI/ASHRAE Standard 135-2024

1523

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

22

Addendum bj to ANSI/ASHRAE Standard 135-2016
Approved  by  ASHRAE  and  by  the  American  National  Standards  Institute  on
November 18, 2019.

1.
Introduce BACnet Secure Connect Data link Layer Option
2.
Introduce BACnet/SC in the Application and Network Layer Specifications
3.  Add new Annex AB for the BACnet Secure Connect Data link Layer Option
4.  Add a Device_UUID Property to the Device Object
5.  Extend APDU Encoding for Large APDU Sizes
6.  New Error Codes for BACnet/SC
7.
8.  Define Extended 6-Octet VMAC

Interoperability Specification Extensions for BACnet/SC

1

22

Addendum by to ANSI/ASHRAE Standard 135-2016
Approved  by  ASHRAE  and  by  the  American  National  Standards  Institute  on
November 18, 2019.

1.  Remove Clause 24, Network Security.

1

22

Addendum bz to ANSI/ASHRAE Standard 135-2016
Approved  by  ASHRAE  and  by  the  American  National  Standards  Institute  on
November 18, 2019.

1.  Add Who-Am-I and You-Are Services.

1

1

22

23

ANSI/ASHRAE 135-2020
A consolidated version of the standard that incorporates all of the known errata and
Addenda bd, be, bi, bj, bk, bl, bm, bn, bp, bq, br, bs, bt, bu, bw, by, and bz to
ANSI/ASHRAE 135-2016.

Addendum cd to ANSI/ASHRAE 135-2020
Approved by ASHRAE and the American National Standards Institute on August 31,
2021.

1.  TLS V1.3 Cipher Suite Application Profile for BACnet/SC

1

24

Addendum bv to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE on January 21, 2022; and by the American National Standards
Institute on January 21, 2022.

1.  Add new property Write_Every_Scheduled_Action to the Schedule object
2.  Fix XML namespace

1

24

Addendum ca to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE on January 21, 2022; and by the American National Standards
Institute on January 21, 2022.

1.  Add new Color object type.
2.  Add new Color Temperature object type.
3.  Add color-reference properties to LO and BLO object types.
4.  Add high/low trim to LO object type.
5.  BIBB Changes to Support Additional Object Types.

1524

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

24

Addendum cc to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE on January 21, 2022; and by the American National Standards
Institute on January 21, 2022.

1.  Update the Network Port Object and Add BACnet/SC Configuration Support.
2.  Modifications to Annex AB.
3.  Add a Procedure to Replace BACnet/SC Certificates.
4.  Add Network Port Object Configuration BIBBs.

1

24

Addendum ce to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE on January 21, 2022; and by the American National Standards
Institute on January 21, 2022.

1.  MS/TP Language Replacement
2.  CSML Name Aliasing.
3.  Remove writableWhen and requiredWhen.

1

25

Addendum cf to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  November  30,  2022  and  by  the  American  National
Standards Institute November 30, 2022.

1.  Formal Definition of the 'data_attributes' Parameter
2.  Redefinition of 'Must Understand' for data options
3.  Changes to segmentation to enforce data attribute consistency

1

26

Addendum ch to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE  on June 28, 2024; and by the American National Standards
Institute on June 28, 2024.

1.  Changes to Clause 5 to correct segmentation errors.

1

26

Addendum ck to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE  on June 28, 2024; and by the American National Standards
Institute on June 28, 2024.

1.  Add missing formal definitions of ASN.1 datatypes

1

26

Addendum cn to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE  on June 28, 2024; and by the American National Standards
Institute on June 28, 2024.

1.  Clarify Engineering Units

1

26

Addendum cq to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE  on June 28, 2024; and by the American National Standards
Institute on June 28, 2024.

1.  Define a new “short form” for Array, List, and SequenceOf types
2.  Formally define the existing “short form” for primitives

ANSI/ASHRAE Standard 135-2024

1525

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

26

Addendum cs to ANSI/ASHRAE Standard 135-2020
Approved by ASHRAE  on June 28, 2024; and by the American National Standards
Institute on June 28, 2024.

1.  Certificate Authority Requirements Interchange Format

1

27

Addendum bx to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  September  30,  2024;  and  by  the  American  National
Standards Institute on September 30, 2024.

1.  Add Device Address Proxy functions

1

27

Addendum ci to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  September  30,  2024;  and  by  the  American  National
Standards Institute on September 30, 2024.

1.  Changes to Clause 12 to add

OPTIONAL_FUNCTIONALITY_NOT_SUPPORTED

2.  Clarify optionally supported command procedure
3.  Clarify Schedule Object requirements
4.  Clarify INVALID_ARRAY_SIZE
5.  Clarify Accumulator Object Scale Datatype
6.  Clarify BVLC-Result in BACnet/SC
7.  Relax DS-COV-A and DS-COVP-A
8.  Add Time Series Exchange Format BIBBs
9.  Clarify use of UNSUPPORTED_OBJECT_TYPE

1

28

Addendum cj to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  September  30,  2024;  and  by  the  American  National
Standards Institute on September 30, 2024.

1.  Add a method for restoring luminaire levels
2.  Add a method for toggling the Binary Lighting Output Object
3.  Clarify terminology for Current Command Priority

1

28

Addendum co to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  September  30,  2024;  and  by  the  American  National
Standards Institute on September 30, 2024.

1.  Clarify Reliability-Evaluation
2.  Event and Fault Parameter Consistency

1

29

Addendum cp to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  November  29,  2024;  and  by  the  American  National
Standards Institute on November 29, 2024.

1.  Addition of Authentication and Authorization
2.  BACnet/SC Changes to Support Authentication and Authorization
3.  Device Object Properties to support Authentication and Authorization
4.  Data Structures to support Authentication and Authorization
5.  Error Codes to support Authentication and Authorization
6.  PICS statements to support Authentication and Authorization capabilities.
7.  New definitions for Authentication and Authorization
8.  New BIBBs and Profiles for Authentication and Authorization
9.  Examples for Authentication and Authorization

1526

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

HISTORY OF REVISIONS

1

30

Addendum cm to ANSI/ASHRAE Standard 135-2020
Approved  by  ASHRAE  on  December  31,  2024;  and  by  the  American  National
Standards Institute on December 31, 2024.

1.  BACnet Energy Services Interface

1

30

ANSI/ASHRAE 135-2024
A consolidated version of the standard that incorporates all of the known errata and
Addenda  bv,  bx,  ca,  cc,  cd,  ce,  cf,  ch,  ci,  cj,  ck,  cm,  cn,  co,  cp,  cq,  and  cs    to
ANSI/ASHRAE 135-2020.

NA = Not Applicable because the Protocol_Revision property was first defined in Addendum b to

ANSI/ASHRAE 135-1995.

ANSI/ASHRAE Standard 135-2024

1527

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.POLICY STATEMENT DEFINING ASHRAE’S CONCERN
FOR THE ENVIRONMENTAL IMPACT OF ITS ACTIVITIES

ASHRAE  is  concerned  with  the  impact  of  its  members’  activities  on  both  the  indoor  and  outdoor  environment.
ASHRAE’s members will strive to minimize any possible deleterious effect on the indoor and outdoor environment of
the  systems  and  components  in  their  responsibility  while  maximizing  the  beneficial  effects  these  systems  provide,
consistent with accepted Standards and the practical state of the art.

ASHRAE’s  short-range  goal  is  to  ensure  that  the  systems  and  components  within  its  scope  do  not  impact  the
indoor and outdoor environment to a greater extent than specified by the Standards and Guidelines as established by
itself and other responsible bodies.

As an ongoing goal, ASHRAE will, through its Standards Committee and extensive Technical Committee structure,
continue  to  generate  up-to-date  Standards  and  Guidelines  where  appropriate  and  adopt,  recommend,  and  promote
those new and revised Standards developed by other responsible organizations.

Through  its  Handbook,  appropriate  chapters  will  contain  up-to-date  Standards  and  design  considerations  as  the

material is systematically revised.

ASHRAE will take the lead with respect to dissemination of environmental information of its primary interest and
will seek out and disseminate information from other responsible organizations that is pertinent, as guides to updating
Standards and Guidelines.

The  effects  of  the  design  and  selection  of  equipment  and  systems  will  be  considered  within  the  scope  of  the

system’s intended use and expected misuse. The disposal of hazardous materials, if any, will also be considered.

ASHRAE’s primary concern for environmental impact will be at the site where equipment within ASHRAE’s scope
operates.  However,  energy  source  selection  and  the  possible  environmental  impact  due  to  the  energy  source  and
energy  transportation  will  be  considered  where  possible.  Recommendations  concerning  energy  source  selection
should be made by its members.

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.ASHRAE · 180 Technology Parkway · Peachtree Corners, GA 30092 · www.ashrae.org

About ASHRAE

Founded in 1894, ASHRAE is a global professional society committed to serve humanity by advancing the arts and
sciences of heating, ventilation, air conditioning, refrigeration, and their allied fields.

As  an  industry  leader  in  research,  standards  writing,  publishing,  certification,  and  continuing  education,  ASHRAE
and its members are dedicated to promoting a healthy and sustainable built environment for all, through strategic
partnerships with organizations in the HVAC&R community and across related industries.

To  stay  current  with  this  and  other  ASHRAE  Standards  and  Guidelines,  visit  www.ashrae.org/standards,  and
connect on LinkedIn, Facebook, Twitter, and YouTube.

Visit the ASHRAE Bookstore

ASHRAE offers its Standards and Guidelines in print, as immediately downloadable PDFs, and via ASHRAE Digital
Collections,  which  provides  online  access  with  automatic  updates  as  well  as  historical  versions  of  publications.
Selected Standards and Guidelines are also offered in redline versions that indicate the changes made between the
active  Standard  or  Guideline  and  its  previous  edition.  For  more  information,  visit  the  Standards  and  Guidelines
section of the ASHRAE Bookstore at www.ashrae.org/bookstore.

IMPORTANT NOTICES ABOUT THIS STANDARD

To ensure that you have all of the approved addenda, errata, and interpretations for this
Standard, visit www.ashrae.org/standards to download them free of charge.

Addenda, errata, and interpretations for ASHRAE Standards and Guidelines are no
longer distributed with copies of the Standards and Guidelines. ASHRAE provides
these addenda, errata, and interpretations only in electronic form to promote
more sustainable use of resources.

Product code: 86950

12/24

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.
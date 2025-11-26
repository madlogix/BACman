ANNEX L - DESCRIPTIONS AND PROFILES OF STANDARDIZED BACnet DEVICES (NORMATIVE)

•  No requirements

Device and Network Management

•  Ability to respond to queries about its status
•  Ability to respond to requests for information about any of its objects
•  Ability to respond to communication control messages
•  Ability to synchronize its internal clock upon request
•  Ability to perform re-initialization upon request

L.13.3 BACnet Elevator Monitor (B-EM)

A B-EM device performs monitoring of an elevator control system. It supports presentation of its elevator objects by another
device, but is not required to support modifications of any of its elevator objects.

Data Sharing

•  Ability to contain elevator objects.
•  Ability to provide the values of any of its BACnet objects

Alarm and Event Management

•  Generation of alarm / event notifications of internal objects and the ability to direct notifications to recipients.
•  Maintain a list of unacknowledged alarms / events
•  Notifying other recipients that the acknowledgment has been received

Scheduling

•  No requirements

Trending

•  No requirements

Device and Network Management

•  Ability to respond to queries about its status
•  Ability to respond to requests for information about any of its objects
•  Ability to respond to communication control messages

L.14 Authentication and Authorization Profiles

The following table indicates which BIBBs shall be supported by the device types of this family, for each interoperability area.

Data Sharing
B-AS
DS-RP-B

Scheduling

B-AS

  Alarm & Event Management

B-AS

  Trending
B-AS

Device & Network Management

  Authentication & Authorization

B-AS
DM-DDB-B
DM-DOB-B
DM-DCC-B

B-AS
AA-AS-B

L.14.1 BACnet Authorization Server (B-AS)

The B-AS is an authorization server that issues access tokens to authorization clients or their helpers. The server has a persistent
database of access policies configured to meet the installation’s needs.  The server has a User Interface suitable for configuring
the authorization policies for the tokens that it issues. Support for all optional policy fields is required. See Clause 17.

ANSI/ASHRAE Standard 135-2024

1221

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

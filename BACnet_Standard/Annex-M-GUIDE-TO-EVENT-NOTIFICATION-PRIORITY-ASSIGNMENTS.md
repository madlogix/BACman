ANNEX M - GUIDE TO EVENT NOTIFICATION PRIORITY ASSIGNMENTS (INFORMATIVE)

(This annex is not part of this standard but is included for informative purposes only)

The  Alarm  and  Event  Priorities  and  Network  Priorities  defined  in  Clause  13.2.5.4  broadly  categorize  the  alarm  and  event
notification priorities. This annex provides examples of various alarms and events that could be assigned into these categories.

Table M-1 extends Table 13-6 by adding semantic meaning to the priority classifications. The subsequent narrative details the
classifications and provides examples of various alarm and event priorities in an interoperable system.

Table M-1. Message Groups Priorities

Message Group

Life Safety

Priority Range
00 - 31

Network Priority

Life Safety Message

Property Safety

32 - 63

Life Safety Message

Brief Description

Notifications related to an immediate threat to
life, safety or health such as fire detection or
armed robbery
Notifications related as an immediate threat to
property such as forced entry

Supervisory

64 - 95

Critical Equipment Message  Notifications related to improper operation,

Trouble

96 - 127

128 - 191

Miscellaneous
Higher Priority
Alarm and Events

Miscellaneous
Lower Priority
Alarm and Events

monitoring failure (particularly of Life Safety or
Property Safety monitoring), or monetary loss
Critical Equipment Message  Notifications related to communication failure
(particularly of Life Safety or Property Safety
equipment)
Higher-level notifications related to occupant
discomfort, normal operation, normal
monitoring, or return to normal

Urgent Message

192 - 255

Normal Message

Lower-level notification related to occupant
discomfort, normal operation, normal
monitoring, or return to normal.

M.1 Life Safety Message Group (0 - 31)

This message group includes any event report related to an immediate threat to life, safety or health. Examples include fire
detection, armed robbery and medical emergency.

M.1.1 Life Safety Message Group Examples

Criteria for membership in a particular life safety message group vary from jurisdiction to jurisdiction. The examples below
are intended to clarify the intent of the grouping and are not meant to be prescriptive.

Event

Reliable Fire Alarms

Life Safety Process Alarms

Description/Examples

Fire alarm events produced by reliable fire alarm detection devices. Examples might
include smoke detectors and heat detectors.

A process or equipment alarm that indicates an immediate threat to life, safety or health
belongs at this priority. Examples might include carbon monoxide or explosive vapor
detection and toxic chemical release.

Fire Alarms Requiring Verification

Fire alarm events requiring verification report. Examples might include pull stations
and alarmed fire exit doors. This category is separated from reliable fire alarm because
of the potential for false alarms caused by vandals or environmental contamination.

Medical Alarms

Immediate  threats  to  life  or  health  due  to  medical  emergencies.  Examples  might
include heart attack or stroke alarm and falls with injuries.

Hold Up And Duress Alarms

Potential threats to life, safety or health due to criminal activity belong at this priority.
Examples might include armed robbery, kidnapping, and bomb threats.

1222

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

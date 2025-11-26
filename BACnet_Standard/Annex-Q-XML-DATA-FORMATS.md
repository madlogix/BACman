ANNEX Q - XML DATA FORMATS (NORMATIVE)

The <object> identifier is in the form "<type>,<instance>" where <type> is either a decimal number or exactly equal to the
Clause 21 identifier text of BACnetObjectType, and <instance> is a decimal number.

The  <property>  identifier  is  either  a  decimal  number  or  exactly  equal  to  the  Clause  21  identifier  text  of
BACnetPropertyIdentifier. If it  is  omitted,  it  defaults  to  "present-value"  except  for  BACnet  File  objects,  where  absence  of
<property> refers to the entire content of the file accessed with Stream Access.

The <index> is the decimal number for the index of an array property.

ANSI/ASHRAE Standard 135-2024

1277

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

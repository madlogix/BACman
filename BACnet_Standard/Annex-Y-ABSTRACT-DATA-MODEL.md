ANNEX Y - ABSTRACT DATA MODEL (NORMATIVE)

If  this  metadata  is  absent,  then  the  data  is  not  intended  to be  accessed  as  a  BACnet  property.  Once  assigned  a  value by  a
definition, it cannot be changed by an instance or a derived type definition.

The following example declares that the Present_Value of an Analog Output object is accessible with property identifier 85.

<Definitions>
      <Object name="0-AnalogOutputObject">
            …
            <Real name="present-value" propertyIdentifier="85" … />
            …
      </Object >
</Definitions>

Y.21.7 'objectType'

This optional metadata, of type String, indicates the type name of the Enumerated type that shall be used for the object type
identifier  portion  of  the  ObjectIdentifier  and  ObjectIdentifierPattern  base  types.  This  type  shall  be  an  extension  of
"0-BACnetObjectType".

ANSI/ASHRAE Standard 135-2024

1439

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

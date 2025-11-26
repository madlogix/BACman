ANNEX X - EXTENDED DISCOVERY OF DEVICES, PROFILES, AND VIEWS (NORMATIVE)

BACnet Device Object:

Profile_Name = "555-BCU200-2.0"          (unchanged)
Profile_Location = "bacnet://.this/file,22"       (unchanged)
Deployed_Profile_Location = "http://bws.customer.example.com/deployed/chillerplant.xdd"

BACnet File Object (in same device):

Object_Identifier = (file,22)
Content=
{ describes only the profiles and views programmed into the single device }

Http File: http://bws.customer.example.com/deployed/chillerplant.xdd

Contents of zipped file "ashrae-csml.xml" =
<?xml version="1.0" encoding="UTF-8"?>
<CSML xmlns="http://bacnet.org/csml/1.2">
    <Object name="chwplant" displayName="Chiller Plant" >
       ... subordinates arranging multiple controller's objects and views into a chiller plant ...
    </Object>
    ... more views ...
</CSML>

X.5 PICS Declarations

To enable devices to declare their capabilities in a machine readable format, a reserved section of the device's xdd file is defined
to  hold  PICS  information  in  XML.  The  PICS  information  shall  be  placed  in  a  <Composition>  element  directly  under  the
<CSML> element with a 'name' attribute equal to ".pics."  The CSML type of this element shall be one of the standard PICS
types defined by ASHRAE that is for a protocol revision that is greater than or equal to the Protocol_Revision of the device.

For example, the xdd file referenced from the Device object's Profile_Location might contain an "ashrae-csml.xml" file that
might contain:

<?xml version="1.0" encoding="UTF-8"?>
<CSML xmlns="http://bacnet.org/csml/1.2">
    ...
    <Composition name=".pics" type="0-PICS" > <!-- example type only, refer to separate definition -->
        <!-- example members only, actual type is defined separately from this standard -->
        <String name="vendor-name" value="Controls-R-Us"/>
        <String name="product-name" value="Building Controller Mark III"/>
        <String name="product-description" value="A really great thing"/>
        <Unsigned name="vendor-identifier" value="555"/>
        ... more ...
     </Composition>
    ...
</CSML>

1388

ANSI/ASHRAE Standard 135-2024

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

ANNEX V - MIGRATION FROM SOAP SERVICES (INFORMATIVE)

POST /.multi
...
<Composition>
    <List name="values">
        <Real name="1" via="/path/one" value="75.5"/>
        <Unsigned name="2" via="/path/two" value="100"/>
    </List>
<Composition>

V.1.9 getHistoryPeriodic Service

This service  retrieved a  list of localizable plain text values for periodic trend values, each value on a separate line. This is
replaced by using the 'historyPeriodic' function. The SOAP "start", "interval", "count", and "resampleMethod" are replaced by
the function parameters "start", "period", "count", and "method", respectively. For example:

GET /path/to/data/historyPeriodic(start=2011-12-03T00:00:00Z,period=3600,count=24,method=average)

V.1.10 getDefaultLocale Service

This service returned the default locale, or empty string if localization is not supported. It is replaced by reading /.info/.default-
locale. For example:

GET /.info/.default-locale?alt=plain

V.1.11 getSupportedLocales Service

This  service  returned  the  (possibly  empty)  list  of  supported  locales,  each  on  a  separate  line.  It  is  replaced  by  reading
/.info/.supported-locales, with the exception that the locales are returned in a semicolon-separated concatenation rather than on
separate lines. For example:

GET /.info/.supported-locales?alt=plain

V.2 Service Options

The SOAP services took "service options" that are not represented by HTTP query parameters. The following table represents
the mapping between the two.

V.2.1 readback

There is no REST equivalent to the SOAP "readback" option.

V.2.2 errorString, errorPrefix

The REST query parameters "error-string" and "error-prefix" serve the equivalent function as these SOAP service options.

V.2.3 locale, writeSingleLocale

The REST query parameter "locale" serves a very similar function to the SOAP options "locale" and "writeSingleLocale". See
Clause W.17

V.2.4 canonical, precision

The REST services support multiple locales for string values but do not support localizing representation of numbers and dates.
Therefore, the "canonical" and "precision" SOAP query parameters have no equivalent and clients will have to convert from
the always-canonical number and date formats to a localized format for display, if desired.

V.2.5 noEmptyArrays

The  "noEmptyArrays"  SOAP  option  was  created  to  work around  a  defect  in  a  common  VisualBasic  library  that  could  not
handle empty arrays in the response. This is no longer needed and has no equivalent in the REST services.

ANSI/ASHRAE Standard 135-2024

1299

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.

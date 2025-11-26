ANSI/ASHRAE Standard 135-2024
(Supersedes ANSI/ASHRAE Standard 135-2020)

A Data Communication
Protocol for
Building Automation
and Control Networks

See  the  History  of  Revisions  at  the  end  of  this  standard  for  approval  dates  by  the  ASHRAE  Standards  Committee,  the
ASHRAE Board of Directors, and the American National Standards Institute.

This Standard is under continuous maintenance by a Standing Standard Project Committee (SSPC) for which the Standards
Committee has established a documented program for regular publication of addenda or revisions, including procedures for
timely, documented, consensus action on requests for change to any part of the Standard. Instructions for how to submit a
change can be found on the ASHRAE® website (www.ashrae.org/continuous-maintenance).

The  latest  edition  of  an  ASHRAE  Standard  may  be  purchased  from  the  ASHRAE  website  (www.ashrae.org)  or  from
ASHRAE  Customer  Service,  180  Technology  Parkway,  Peachtree  Corners,  GA  30092.  E-mail:  orders@ashrae.org.  Fax:
678-539-2129.  Telephone:  404-636-8400  (worldwide),  or  toll  free  1-800-527-4723  (for  orders  in  US  and  Canada).  For
reprint permission, go to www.ashrae.org/permissions.

© 2024 ASHRAE                  ISSN 1041-2336

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.ASHRAE Standing Standard Project Committee 135
Cognizant TC: 1.4, Control Theory and Applications
SPLS Liaison: Paul Lindahl

Coleman L. Brumley, Jr.,* Chair
Scott Ziegenfus, Vice Chair
Salvatore Cataldi,* Secretary
Nathaniel Benes*
Jayson Bursill
Steven T. Bushby*
James F. Butler
Tyler Cove

Brandon M. DuPrey*
Richard A. Farmer
David Fisher
Bernhard Isler
Stephen Karg
Thomas Kurowski*
Andrew McMillan
Shahid Naeem

Dattatray J. Pawar
Frank V. Neher*
Michael Osborne*
Scott Reed
Marcelo Richter da Silva
David Ritter*
David Robin*
Frank Schubert

Steven C. Sill*
Ted Sunderland
Takeji Toyoda, Jr.
Lori Tribble
Klaus B. Waechter
Grant N. Wichenko*
Christoph Zeller

* Denotes members of voting status when the document was approved for publication

ASHRAE STANDARDS COMMITTEE 2024–2025

Douglas D. Fick, Chair
Adrienne G. Thomle, Vice Chair
Hoy R. Bohanon, Jr.
Kelley P. Cramm
Abdel K. Darwich
Drake H. Erbe
Patricia Graef
William M. Healy

Jaap Hogeling
Jennifer A. Isenbeck

Satish N. Iyengar
Phillip A. Johnson
Paul A. Lindahl, Jr.
Julie Majurin
Lawrence C. Markel
Margaret M. Mathison

Kenneth A. Monroe
Daniel H. Nall
Philip J. Naughton
Kathleen Owen

Gwelen Paliaga
Karl L. Peterman
Justin M. Prosser
Christopher J. Seeton

Paolo M. Tronville
Douglas K. Tucker
William F. Walter
David P. Yuill
Susanna S. Hanson, BOD ExO
Wade H. Conlan, CO

Ryan Shanley, Senior Manager of Standards

SPECIAL NOTE
This American National Standard (ANS) is a national voluntary consensus Standard developed under the auspices of ASHRAE. Consensus is defined
by  the  American  National  Standards  Institute  (ANSI),  of  which  ASHRAE  is  a  member  and  which  has  approved  this  Standard  as  an  ANS,  as
“substantial agreement reached by directly and materially affected interest categories. This signifies the concurrence of more than a simple majority,
but not necessarily unanimity. Consensus requires that all views and objections be considered, and that an effort be made toward their resolution.”
Compliance with this Standard is voluntary until and unless a legal jurisdiction makes compliance mandatory through legislation.

ASHRAE obtains consensus through participation of its national and international members, associated societies, and public review.
ASHRAE  Standards  are  prepared  by  a  Project  Committee  appointed  specifically  for  the  purpose  of  writing  the  Standard.  The  Project
Committee Chair and Vice-Chair must be members of ASHRAE; while other committee members may or may not be ASHRAE members, all
must be technically qualified in the subject area of the Standard. Every effort is made to balance the concerned interests on all Project Committees.

The Senior Manager of Standards of ASHRAE should be contacted for

a. interpretation of the contents of this Standard,
b. participation in the next review of the Standard,
c. offering constructive criticism for improving the Standard, or
d. permission to reprint portions of the Standard.

DISCLAIMER
ASHRAE uses its best efforts to promulgate Standards and Guidelines for the benefit of the public in light of available information and accepted
industry practices. However, ASHRAE does not guarantee, certify, or assure the safety or performance of any products, components, or systems
tested, installed, or operated in accordance with ASHRAE’s Standards or Guidelines or that any tests conducted under its Standards or Guidelines
will be nonhazardous or free from risk.

ASHRAE INDUSTRIAL ADVERTISING POLICY ON STANDARDS
ASHRAE Standards and Guidelines are established to assist industry and the public by offering a uniform method of testing for rating purposes, by
suggesting safe practices in designing and installing equipment, by providing proper definitions of this equipment, and by providing other information
that may serve to guide the industry. The creation of ASHRAE Standards and Guidelines is determined by the need for them, and conformance
to them is completely voluntary.

In referring to this Standard or Guideline and in marking of equipment and in advertising, no claim shall be made, either stated or implied,

that the product has been approved by ASHRAE.

Copyrighted material licensed to Conrad Ross on 2025-11-13 for licensee's use only. All rights reserved. No further reproduction or distribution is permitted. Distributed by Accuris for ASHRAE, www.accuristech.com.Contents

6

5

4.1
4.2
4.3

3.1
3.2
3.3

5.1
5.2
5.3
5.4
5.5
5.6

6.1
6.2
6.3
6.4
6.5
6.6
6.7

 CONTENTS
FOREWORD ............................................................................................................................................................................. 10
PURPOSE .......................................................................................................................................................................... 12
1
2
SCOPE ............................................................................................................................................................................... 12
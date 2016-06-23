# viet calendar
REST service to convert lunar vietnamese date to solar date & vice vesa.

### API
 * vietnamcalendar-giapha.rhcloud.com/index.html?raml=api/lunar.raml

### Tools in use
Maven, Vertx, Guice, Protocol Buffer, RAML
 * for maven to run on Windows, need to modify protocExecutable in pom.xml to point to Protocol Buffer compiler.
### Vertx Web
MainVerticle receive every request and transfer to other handler/ worker verticle, in different ways :)
 * /lunar?dd will convert request to Protoco Buffer message DateMonthYear and transfer to LunarWorkerVerticle
 	because I don't like JsonObject goes everywhere internal application. Use Protoco Buffer make app still polyglot :), better than vertx java converter. I strong belive nodejs verticle could handle Protoco Buffer message.
 * /lunar/{ddMMyyyy} use handle

 * /check_vietnam_holiday use java 8 lambda

### Raml API schema
 * output is json - no schema as NoSQL, but I think it need schema like the way my old wsdl file do.
 * so I use raml file for api description. I happy to write it by hand rather than machine generate wsdl.
 * the file is lunar.raml

### Bussiness Logic
 * de.unileipzig.informatik.VietCalendar for convert lunar date <-> solar date. Copy from  http://www.informatik.uni-leipzig.de/~duc/amlich/
 * VietCalendarService is a wrapper, calculate java LocalDate instead of int values (dd, MM, yyyy, timeZone).
 	contains a pre calculate leap month for years (need to convert solar date to lunar)
 * VietnamNationalHolidayService for the holiday only :), many TODO things goes here.

### Unit Test
 * Service Junit
 * Vertx verticle
 * RAML Schema validation
 * these tests do not cover enought but it take my times to pass all. I still stuck on ZonedDateTime & have to hardcode timezone to 7.0 (viet nam GMT time)

### WRK Performance Test
 * for short, faster than Spring Boot, slower than pojo Servlet (I think)

### Run from IDE
Vertx app have its main method, need to set it up to run MainVerticle
 * eclipse
 * intelij
  Run -> Edit Configuration -> Default -> Application -> + -> Application:
                               	Main class: io.vertx.core.Launcher
                               	Program argument: run io.github.amlich.calendar.MainVerticle

### Deploy to openshift
 * openshift support vertx 2, so I use an DIY cartridge [Vertx 3 Reference] (https://github.com/vert-x3/vertx-openshift-diy-quickstart)
 * To use openshift ssh & git to push to openshift. need to [install & setup rhc](Https://developers.openshift.com/en/managing-client-tools.html)
   
## Final TODO

Version 2 will able to check if input date is holiday. Which is good for check viet nam stock market is open or not.

Viet Nam goverment manual decide which time in year is in Tet holiday. In calendar, there are 4 days in Tet. Infact it is a whole week. Goverment will make a day off so people will have 9 holidays span. (2 satuday 4 Tet holiday 2 sunday plus a day off).
I need a screen to input this information for each year. 

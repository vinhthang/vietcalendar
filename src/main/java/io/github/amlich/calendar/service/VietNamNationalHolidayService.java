package io.github.amlich.calendar.service;

import io.vertx.core.logging.Logger;
import io.vertx.core.logging.LoggerFactory;

import javax.annotation.PostConstruct;
import javax.inject.Inject;
import javax.inject.Singleton;
import java.time.DayOfWeek;
import java.time.LocalDate;
import java.time.MonthDay;
import java.util.ArrayList;
import java.util.List;

/**
 * happy holyday
 */
@Singleton
public class VietNamNationalHolidayService {
    private static Logger logger = LoggerFactory.getLogger(VietNamNationalHolidayService.class);

    private List<MonthDay> solarHolidayList;
    private List<MonthDay> lunarHolidayList;

    private final VietCalendarService calendarService;

    @Inject
    public VietNamNationalHolidayService(VietCalendarService calendarService) {
        this.calendarService = calendarService;
        //@PostConstruct not work :(
        this.init();
    }
    //Guice do not support JSR-250
    @PostConstruct
    public void init() {
        solarHolidayList = new ArrayList<>();
        //Independency Day
        solarHolidayList.add(MonthDay.of(9, 2));
        //Giai phong mien nam
        solarHolidayList.add(MonthDay.of(4, 30));
        //world labor day
        solarHolidayList.add(MonthDay.of(5, 1));
        //new year
        solarHolidayList.add(MonthDay.of(1, 1));

        lunarHolidayList = new ArrayList<>();
        //King Hung birthday!
        lunarHolidayList.add(MonthDay.of(3, 10));
        //30 Tet
        lunarHolidayList.add(MonthDay.of(1, 30));
        //mung 1 Tet
        lunarHolidayList.add(MonthDay.of(1, 1));
        //mung 2 Tet
        lunarHolidayList.add(MonthDay.of(1, 2));
        //mung 3 Tet
        lunarHolidayList.add(MonthDay.of(1, 2));

        logger.info("VietNam National Holiday List inited.");

    }

    public boolean isVietNamHoliday(final LocalDate date) {
        return isSolarHoliday(date) || isInLunarHoliday(date);
    }

    /**
     * check if date is lunar holiday
     * @param date
     * @return
     */
    public boolean isInLunarHoliday(final LocalDate date) {
        return lunarHolidayList.stream()
                .map(monthDay -> calendarService.toSolar(monthDay.atYear(date.getYear())))
                .filter(lunarDate->lunarDate.equals(date))
                .findAny()
                .isPresent();
    }

    public boolean isSolarHoliday(final LocalDate date) {
        return isSaturdayOrSunday(date) || isInSolarHoliday(MonthDay.of(date.getMonth(), date.getDayOfMonth()));
    }

    private boolean isInSolarHoliday(MonthDay dateMonth) {
        return solarHolidayList.contains(dateMonth);
    }

    public boolean isSaturdayOrSunday(LocalDate date) {
        return date.getDayOfWeek() == DayOfWeek.SATURDAY || date.getDayOfWeek() == DayOfWeek.SUNDAY;
    }


}

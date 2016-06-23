package io.github.amlich.calendar.service;

import de.unileipzig.informatik.VietCalendar;
import rx.Observable;

import java.time.LocalDate;
import java.time.YearMonth;
import java.time.ZonedDateTime;
import java.util.HashSet;
import java.util.Set;

/**
 * wrapper for VietCalendar
 */

public class VietCalendarService {
    //hold pre-calculated leap month from years
    private Set<YearMonth> leapYearMonth;

    public VietCalendarService() {
        leapYearMonth = new HashSet<>();
        for (long i = 0; i < 99_000; i++) {
            YearMonth ym = YearMonth.of(-5000, 1).plusMonths(i);
            if (isLeap(ym.atDay(1))) {
                leapYearMonth.add(ym);
            }
        }
    }

    public Observable<Boolean> isLeapRx(int jdDate) {
        return Observable.just(jdDate)
                .map(jd -> VietCalendar.jdToDate(jd))
                .map(solar -> VietCalendar.convertSolar2Lunar(solar[0], solar[1], solar[2], 7d))
                .map(lunar -> lunar[3] == 1);
    }

    public Observable<Boolean> isLeapRx(LocalDate lunarDate) {
        return Observable.just(lunarDate)
                .map(date -> VietCalendar.jdFromDate(date.getDayOfMonth(), date.getMonthValue(), date.getYear()))
                .flatMap(jd -> isLeapRx(jd));
    }

    public LocalDate toLunar(LocalDate date) {
        return toLunar(date, 7d);
    }

    public Observable<LocalDate> toLunarRx(LocalDate date) {
        return Observable.just(toLunar(date));
    }

    public LocalDate toLunar(ZonedDateTime zonedDateTime) {
        LocalDate date = zonedDateTime.toLocalDate();

        return toLunar(date);
    }

    public LocalDate toLunar(LocalDate date, double timeZone) {
        int[] values = VietCalendar.convertSolar2Lunar(
                date.getDayOfMonth(), date.getMonthValue(), date.getYear(), timeZone);

        return LocalDate.of(values[2], values[1], values[0]);
    }

    public LocalDate toSolar(LocalDate date, double timeZone) {
        int leap = 0;
        if (leapYearMonth.contains(YearMonth.of(date.getYear(), date.getMonthValue()))) leap = 1;

        int[] values = VietCalendar.convertLunar2Solar(
                date.getDayOfMonth(), date.getMonthValue(), date.getYear(), leap, timeZone);

        return LocalDate.of(values[2], values[1], values[0]);
    }

    public LocalDate toSolar(ZonedDateTime zonedDateTime) {
        LocalDate date = zonedDateTime.toLocalDate();
        return toSolar(date);
    }

    public LocalDate toSolar(LocalDate date) {
        return toSolar(date, 7d);
    }

    public Boolean isLeap(Integer jdDate) {
        int[] s = VietCalendar.jdToDate(jdDate);
        int[] l = VietCalendar.convertSolar2Lunar(s[0], s[1], s[2], 7d);

        return l[3] == 1;
    }

    public Boolean isLeap(LocalDate date) {
        return isLeap(VietCalendar.jdFromDate(date.getDayOfMonth(), date.getMonthValue(), date.getYear()));
    }
}

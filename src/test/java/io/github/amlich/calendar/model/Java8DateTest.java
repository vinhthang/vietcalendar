package io.github.amlich.calendar.model;

import org.junit.Test;

import java.time.Month;
import java.time.MonthDay;
import java.time.YearMonth;

public class Java8DateTest {
    @Test
    public void test () {
        MonthDay md = MonthDay.of(Month.JANUARY, 10);
        YearMonth ym = YearMonth.of(2016, Month.APRIL);

    }
}

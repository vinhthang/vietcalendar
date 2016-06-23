package io.github.amlich.calendar.service;

import org.junit.Assert;
import org.junit.Before;
import org.junit.Ignore;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

import java.time.LocalDate;
import java.time.MonthDay;

@RunWith(JUnit4.class)
public class VietnamNationalHolidayTest {
    private VietNamNationalHolidayService service;

    @Before
    public void init() {

        VietCalendarService calendarService = new VietCalendarService();
        service = new VietNamNationalHolidayService(calendarService);
        service.init();
    }

    @Test
    public void testIsSaturdayOrSunday() {
        Assert.assertTrue(service.isSaturdayOrSunday(LocalDate.of(2015, 9, 12)));
        Assert.assertTrue(service.isSaturdayOrSunday(LocalDate.of(2015, 9, 13)));
        Assert.assertTrue(service.isSaturdayOrSunday(LocalDate.of(2011, 5, 1)));

        Assert.assertFalse(service.isSaturdayOrSunday(MonthDay.of(5, 1).atYear(2015)));
        Assert.assertTrue(service.isSaturdayOrSunday(MonthDay.of(5, 1).atYear(2011)));
    }

    @Test
    public void test30thang4nam2015() {
        Assert.assertTrue(service.isVietNamHoliday(LocalDate.of(2015, 4, 30)));
    }

    @Test
    public void test30thang4nam2011() {
        Assert.assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 4, 30)));
    }

    @Test
    @Ignore
    public void test02thang5nam2011() {
        Assert.assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 5, 2)));
    }
    //TODO study more for compensatory. this should be holiday
    @Test
    @Ignore
    public void test03thang5nam2011() {
        Assert.assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 5, 3)));
    }

    @Test
    public void test04thang5nam2011() {
        Assert.assertFalse(service.isVietNamHoliday(LocalDate.of(2011, 5, 4)));
    }

    @Test
    public void test04thang5nam2015() {
        Assert.assertFalse(service.isVietNamHoliday(LocalDate.of(2015, 5, 4)));
    }

    @Test
    public void test03thang5nam2015() {
        Assert.assertTrue(service.isVietNamHoliday(LocalDate.of(2015, 5, 3)));
    }
}

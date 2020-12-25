package io.github.amlich.calendar.service;

import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.LocalDate;
import java.time.MonthDay;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

public class VietnamNationalHolidayTest {
    private VietNamNationalHolidayService service;

    @BeforeEach
    public void init() {

        VietCalendarService calendarService = new VietCalendarService();
        service = new VietNamNationalHolidayService(calendarService);
        service.init();
    }

    @Test
    public void testIsSaturdayOrSunday() {
        assertTrue(service.isSaturdayOrSunday(LocalDate.of(2015, 9, 12)));
        assertTrue(service.isSaturdayOrSunday(LocalDate.of(2015, 9, 13)));
        assertTrue(service.isSaturdayOrSunday(LocalDate.of(2011, 5, 1)));

        assertFalse(service.isSaturdayOrSunday(MonthDay.of(5, 1).atYear(2015)));
        assertTrue(service.isSaturdayOrSunday(MonthDay.of(5, 1).atYear(2011)));
    }

    @Test
    public void test30thang4nam2015() {
        assertTrue(service.isVietNamHoliday(LocalDate.of(2015, 4, 30)));
    }

    @Test
    public void test30thang4nam2011() {
        assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 4, 30)));
    }

    @Test
    public void test02thang5nam2011() {
        assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 5, 2)));
    }

    //TODO study more for compensatory. this should be holiday
    @Test
    public void test03thang5nam2011() {
        assertTrue(service.isVietNamHoliday(LocalDate.of(2011, 5, 3)));
    }

    @Test
    public void test04thang5nam2011() {
        assertFalse(service.isVietNamHoliday(LocalDate.of(2011, 5, 4)));
    }

    @Test
    public void test04thang5nam2015() {
        assertFalse(service.isVietNamHoliday(LocalDate.of(2015, 5, 4)));
    }

    @Test
    public void test03thang5nam2015() {
        assertTrue(service.isVietNamHoliday(LocalDate.of(2015, 5, 3)));
    }
}

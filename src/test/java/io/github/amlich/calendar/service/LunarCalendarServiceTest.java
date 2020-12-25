package io.github.amlich.calendar.service;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import rx.Observable;
import rx.observers.TestSubscriber;

import javax.inject.Inject;
import java.time.*;
import java.util.Calendar;
import java.util.GregorianCalendar;
import java.util.TimeZone;
import java.util.stream.LongStream;

import static org.junit.jupiter.api.Assertions.*;

public class LunarCalendarServiceTest {
    @Inject
    private VietCalendarService service;

    @BeforeEach
    public void init() {
        service = new VietCalendarService();
    }

    @Test
    public void testToLunar() {
        LocalDate lunarDate = service.toLunar(LocalDate.of(2015, 9, 12));
        assertEquals(LocalDate.of(2015, 7, 30), lunarDate);

        lunarDate = service.toLunar(LocalDate.of(2001, 1, 1));
        assertEquals(LocalDate.of(2000, 12, 7), lunarDate);
    }

    @Test
    public void toLunarRx() {
        TestSubscriber<LocalDate> testSubscriber = new TestSubscriber<>();
        service.toLunarRx(LocalDate.of(2016, 02, 22)).subscribe(testSubscriber);
        testSubscriber.assertNoErrors();
        testSubscriber.assertValue(LocalDate.of(2016, 1, 15));
    }

    @Test
    public void toLunar2() {
        LocalDate lunarDate = service.toLunar(LocalDate.of(2015, 12, 12));
        assertEquals(LocalDate.of(2015, 11, 02), lunarDate);

        lunarDate = service.toLunar(LocalDate.of(2015, 12, 13));
        assertEquals(LocalDate.of(2015, 11, 03), lunarDate);

        lunarDate = service.toLunar(LocalDate.of(2015, 12, 14));
        assertEquals(LocalDate.of(2015, 11, 04), lunarDate);
    }

    @Test
    public void toSolar() {
        //leaf year.
        LocalDate solarDate = service.toSolar(LocalDate.of(2004, 2, 11));
        assertEquals(LocalDate.of(2004, 3, 31), solarDate);
    }

    @Test
    public void testLeap() {
        assertFalse(service.isSolarLeap(2005));
        //2004 is leaf year.
        assertTrue(service.isSolarLeap(2004));
    }

    @Test
    public void toSolarZone() {
        LocalDate lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-7")));
        assertEquals(LocalDate.of(2004, 3, 31), lunarDate);

        lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-10")));
        assertEquals(LocalDate.of(2004, 3, 31), lunarDate);

        lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-14")));
        assertEquals(LocalDate.of(2004, 3, 31), lunarDate);
    }

    @Test
    public void testTimeZone() {
        TimeZone timeZone = TimeZone.getTimeZone("Etc/GMT+7");
        Calendar c = new GregorianCalendar();

        c.setTimeZone(timeZone);

        assertEquals(-7, c.get(Calendar.HOUR_OF_DAY) - Calendar.getInstance(TimeZone.getTimeZone("Etc/GMT+0")).get(Calendar.HOUR_OF_DAY));

    }

    @Test
    public void testDateTimeZone() {
        ZonedDateTime date = ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 1, 1), ZoneId.of("Etc/GMT-14"));
        assertEquals(LocalDate.of(2004, 2, 11), date.toLocalDate());

        ZonedDateTime utc0 = ZonedDateTime.now(ZoneId.of("UTC+0"));

        assertEquals(LocalTime.now().getHour(), utc0.getHour() + 7);
    }

    @Test
    public void testDateTimeZone_LocalDate() {
        ZonedDateTime utc = ZonedDateTime.now(ZoneId.of("UTC-13"));
        ZonedDateTime utc13 = ZonedDateTime.now(ZoneId.of("UTC+13"));

        assertNotEquals(utc.toLocalDate(), utc13.toLocalDate());
    }
//
//    @Test
//    public void writePreCalculatedLeapMonth() {
//        Observable.from(() -> IntStream.rangeClosed(0, 1_000_000).iterator())
//                .filter(jdDate -> service.isLeap(jdDate))
//                .subscribe(System.out::println);
//    }

    @Test
    public void testLeapMonth() {
        Observable.from(() -> LongStream.rangeClosed(0, 1_000).iterator())
                .map(i -> YearMonth.of(0, 1).plusMonths(i))
                .filter(ym -> service.isSolarLeap(ym))
                .subscribe(System.out::println);
    }
}

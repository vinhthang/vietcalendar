package io.github.amlich.calendar.service;

import de.unileipzig.informatik.VietCalendar;
import org.junit.Assert;
import org.junit.Before;
import org.junit.Ignore;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;
import rx.Observable;
import rx.observers.TestSubscriber;

import javax.inject.Inject;
import java.time.*;
import java.time.temporal.TemporalAdjusters;
import java.util.Calendar;
import java.util.GregorianCalendar;
import java.util.TimeZone;
import java.util.stream.IntStream;
import java.util.stream.LongStream;

@RunWith(JUnit4.class)
public class LunarCalendarServiceTest {
    @Inject
    private VietCalendarService service;

    @Before
    public void init() {
        service = new VietCalendarService();
    }

    @Test
    public void testToLunar() {
        LocalDate lunarDate = service.toLunar(LocalDate.of(2015, 9, 12));
        Assert.assertEquals(LocalDate.of(2015, 7, 30), lunarDate);

        lunarDate = service.toLunar(LocalDate.of(2001, 1, 1));
        Assert.assertEquals(LocalDate.of(2000, 12, 7), lunarDate);
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
        Assert.assertEquals(LocalDate.of(2015, 11, 02), lunarDate);

         lunarDate = service.toLunar(LocalDate.of(2015, 12, 13));
         Assert.assertEquals(LocalDate.of(2015, 11, 03), lunarDate);

         lunarDate = service.toLunar(LocalDate.of(2015, 12, 14));
         Assert.assertEquals(LocalDate.of(2015, 11, 04), lunarDate);
    }
    @Test
    public void toSolar() {
        //leaf year.
        LocalDate solarDate = service.toSolar(LocalDate.of(2004, 2, 11));
        Assert.assertEquals(LocalDate.of(2004, 3, 31), solarDate);
    }

    @Test
    public void toSolarZone() {
        LocalDate lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-7")));
        Assert.assertEquals(LocalDate.of(2004, 3, 31), lunarDate);

        lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-10")));
        Assert.assertEquals(LocalDate.of(2004, 3, 31), lunarDate);

        lunarDate = service.toSolar(ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 12, 10), ZoneId.of("Etc/GMT-14")));
        Assert.assertEquals(LocalDate.of(2004, 3, 31), lunarDate);
    }
    @Test
    @Ignore
    public void testTimeZone() {
        TimeZone timeZone = TimeZone.getTimeZone("Etc/GMT+7");
        Calendar c = new GregorianCalendar();

        c.setTimeZone(timeZone);

        Assert.assertEquals(-7, c.get(Calendar.HOUR_OF_DAY) - Calendar.getInstance(TimeZone.getTimeZone("Etc/GMT+0")).get(Calendar.HOUR_OF_DAY) );

    }
    @Test
    public void testDateTimeZone() {
        ZonedDateTime date = ZonedDateTime.of(LocalDateTime.of(2004, 2, 11, 1, 1), ZoneId.of("Etc/GMT-14"));
        Assert.assertEquals(LocalDate.of(2004, 2, 11), date.toLocalDate());

        ZonedDateTime utc0 = ZonedDateTime.now(ZoneId.of("UTC+0"));

        Assert.assertEquals(LocalTime.now().getHour(), utc0.getHour() + 7);
    }
    @Test
    public void testDateTimeZone_LocalDate() {
        ZonedDateTime utc = ZonedDateTime.now(ZoneId.of("UTC-13"));
        ZonedDateTime utc13 = ZonedDateTime.now(ZoneId.of("UTC+13"));

        Assert.assertNotEquals(utc.toLocalDate(), utc13.toLocalDate());
    }
    @Test
    public void writePreCalculatedLeapMonth() {
        System.out.println(VietCalendar.jdToDate(0));
        Observable.from(() -> IntStream.rangeClosed(0, 1_000_000).iterator())
                .filter(jdDate -> service.isLeap(jdDate))
                .subscribe(System.out::println);
    }
    @Test
    @Ignore
    public void testLeapMonth() {
        Observable.from(() -> LongStream.rangeClosed(0, 1_000).iterator())
                .map(i -> YearMonth.of(0, 1).plusMonths(i))
                .filter(ym -> service.isLeap(ym.atDay(1)))
                .subscribe(System.out::println);
    }
}

package io.github.amlich.calendar;

import io.vertx.core.buffer.Buffer;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayInputStream;
import java.time.LocalDate;
import java.time.Month;
import java.time.temporal.TemporalAdjusters;
import java.util.Optional;
import java.util.TimeZone;

public class TestEnv {

    @Test
    public void testSystemVariable() {
        System.out.println(System.getenv("OPENSHIFT_DYI_PORT"));
        System.out.println(System.getenv("DOCKER_HOST"));

    }

    @Test
    public void testBufferToInputStream() {
        Buffer b = Buffer.buffer();
        b.appendByte((byte) 10);

        b.getBytes();

        ByteArrayInputStream inputStream = new ByteArrayInputStream(b.getBytes());
    }

    public static long getStartTimeInMillis(java.time.Year year, Optional<Month> monthOpt) {
        Month month = monthOpt.orElse(Month.JANUARY);

        if (year != null) {
            return LocalDate.of(year.getValue(), month, 1)
                    .with(TemporalAdjusters.firstDayOfMonth())
                    .atStartOfDay()
                    .atZone(TimeZone.getDefault().toZoneId())
                    .toInstant()
                    .toEpochMilli();
        }

        return 0;
    }
}

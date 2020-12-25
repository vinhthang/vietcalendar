package io.github.amlich.calendar;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

public class ProtocolBufferMessageTest {
    @Test
    public void testDayMonth() throws Exception {
        Message.DayMonth dayMonth = Message.DayMonth.newBuilder().setDay(10).setMonth(12).build();

        assertNotNull(dayMonth);

        Message.DayMonth dayMonth2 = Message.DayMonth.parseFrom(dayMonth.toByteArray());

        assertEquals(dayMonth, dayMonth2);
    }
}

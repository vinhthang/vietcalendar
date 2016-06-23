package io.github.amlich.calendar;

import com.google.protobuf.InvalidProtocolBufferException;
import org.junit.Assert;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

@RunWith(JUnit4.class)
public class ProtocolBufferMessageTest {
    @Test
    public void testDayMonth () throws Exception {
        Message.DayMonth dayMonth = Message.DayMonth.newBuilder().setDay(10).setMonth(12).build();

        Assert.assertNotNull(dayMonth);

        Message.DayMonth dayMonth2 = Message.DayMonth.parseFrom(dayMonth.toByteArray());

        Assert.assertEquals(dayMonth, dayMonth2);
    }
}

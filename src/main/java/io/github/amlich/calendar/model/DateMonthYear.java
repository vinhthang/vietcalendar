package io.github.amlich.calendar.model;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.protobuf.InvalidProtocolBufferException;
import io.github.amlich.calendar.AmLichException;
import io.github.amlich.calendar.Message;

import java.time.*;
import java.util.Optional;

public class DateMonthYear {
    private final Message.DayMonthYear message;

    public DateMonthYear(int dd, int MM, int yyyy) {
        this(dd, MM, yyyy, 7.0);
    }

    public DateMonthYear(int dd, int MM, int yyyy, double timeZone) {
        this.message = Message.DayMonthYear.newBuilder()
                .setDay(dd)
                .setMonth(MM)
                .setYear(yyyy)
                .setTimeZone(timeZone).build();
        try {
            this.toLocalDate();
        } catch (DateTimeException | NumberFormatException e) {
            throw new AmLichException("Cannot build date: " + this);
        }
    }

    public DateMonthYear(byte[] message) {
        try {
            this.message = Message.DayMonthYear.newBuilder().mergeFrom(message).build();
        } catch (InvalidProtocolBufferException e) {
            throw new AmLichException("cannot build object from protobuff message: " + new String(message));
        }
    }

    public DateMonthYear(String dd, String mm, String yyyy) {
        this(Integer.parseInt(dd), Integer.parseInt(mm), Integer.parseInt(yyyy));
    }

    public DateMonthYear(LocalDate localDate) {
        this(localDate.getDayOfMonth(), localDate.getMonthValue(), localDate.getYear());
    }

    public DateMonthYear(String dd, String mm, String yyyy, Optional<String> timeZone) {
        this(Integer.parseInt(dd), Integer.parseInt(mm), Integer.parseInt(yyyy), Double.parseDouble(timeZone.orElse("7.0")));
    }

    @Override
    public String toString() {
        return "DateMonthYear{" +
                "dd='" + getDd() + '\'' +
                ", MM='" + getMM() + '\'' +
                ", yyyy='" + getYyyy() + '\'' +
                '}';


    }

    @JsonIgnore
    public byte[] getBytes() {
        return message.toByteArray();
    }

    public int getDd() {
        return message.getDay();
    }

    @JsonProperty("MM")
    public int getMM() {
        return message.getMonth();
    }

    public int getYyyy() {
        return message.getYear();
    }

    @JsonIgnore
    public double getTimeZone() {
        return message.getTimeZone();
    }

    public LocalDate toLocalDate() {
        return LocalDate.of(message.getYear(), message.getMonth(), message.getDay());
    }

    public ZonedDateTime toZonedDateTime() {
        String zoneId = "UTC";
        int timeZone = (int) getTimeZone();
        if (timeZone > 0) {
            zoneId += "+" + timeZone;
        } else {
            zoneId += timeZone;
        }

        return ZonedDateTime.of(toLocalDate(), LocalTime.now(), ZoneId.of(zoneId));

    }
}

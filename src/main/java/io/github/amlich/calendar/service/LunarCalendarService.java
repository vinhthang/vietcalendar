package io.github.amlich.calendar.service;

import io.github.amlich.calendar.AmLichException;
import io.github.amlich.calendar.model.DateMonthYear;
import io.vertx.core.json.JsonObject;

import javax.inject.Inject;
import javax.inject.Singleton;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.HashMap;
import java.util.Map;

@Singleton
public class LunarCalendarService {
    private final DateTimeFormatter ddMMyyyy;

    private final VietCalendarService vietCalendarService = new VietCalendarService();
    private final VietNamNationalHolidayService vietnamNationalHoliday;

    @Inject
    public LunarCalendarService(VietNamNationalHolidayService vietnamNationalHoliday) {
        ddMMyyyy = DateTimeFormatter.ofPattern("ddMMyyyy");
        this.vietnamNationalHoliday = vietnamNationalHoliday;
    }

    public JsonObject toLunar(String dd, String mm, String yyyy) {
        if (mm.length() == 1) mm = "0" + mm;
        if (dd.length() == 1) dd = "0" + dd;
        String date = dd + mm + yyyy;

        return toLunar(date);
    }


    public JsonObject toLunar(LocalDate polarDate) {
        LocalDate lunarDate = vietCalendarService.toLunar(polarDate);

        Map<String, Object> result = new HashMap<>();

        result.put("dd", lunarDate.getDayOfMonth());
        result.put("MM", lunarDate.getMonthValue());
        result.put("yyyy", lunarDate.getYear());

        JsonObject obj = new JsonObject(result);

        return obj;
    }

    public Boolean isVietNamHoliday(String dd, String month, String yyyy) {
        if (month.length() == 1) month = "0" + month;
        if (dd.length() == 1) dd = "0" + dd;

        LocalDate polarDate = LocalDate.parse(dd + month + yyyy, ddMMyyyy);

        return vietnamNationalHoliday.isVietNamHoliday(polarDate);
    }

    public JsonObject toLunar(String date) {
        try {
            LocalDate polarDate = LocalDate.parse(date, ddMMyyyy);
            return toLunar(polarDate);
        } catch (DateTimeParseException se) {
            throw new AmLichException(se.getMessage());
        }
    }

    public DateMonthYear toLunar(DateMonthYear obj) {
        return new DateMonthYear(vietCalendarService.toLunar(obj.toZonedDateTime()));
    }

    public Boolean isVietNamHoliday(DateMonthYear dateMonthYear) {
        return vietnamNationalHoliday.isVietNamHoliday(dateMonthYear.toZonedDateTime().toLocalDate());
    }
}

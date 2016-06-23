package io.github.amlich.calendar.endpoint;

import io.github.amlich.calendar.model.DateMonthYear;
import io.github.amlich.calendar.service.LunarCalendarService;
import io.vertx.core.Handler;
import io.vertx.core.json.Json;
import io.vertx.ext.web.RoutingContext;

import javax.inject.Inject;
import javax.inject.Singleton;
import java.time.LocalDate;
import java.util.Optional;

/**
 * handle end point /check_vietnam_holiday
 */
@Singleton
public class VietNamHolidayEndPoint implements Handler<RoutingContext> {
    private LunarCalendarService service;

    @Inject
    public VietNamHolidayEndPoint(LunarCalendarService service) {
        this.service = service;
    }

    @Override
    public void handle(RoutingContext context) {
        String dd = context.request().getParam("dd");
        String MM = context.request().getParam("MM");
        String yyyy = context.request().getParam("yyyy");

        DateMonthYear dateMonthYear = null;
        try {
            dateMonthYear = new DateMonthYear(dd, MM, yyyy, Optional.empty());
        } catch (Exception e) {
            dateMonthYear = new DateMonthYear(LocalDate.now());
        }

        context.response()
                .putHeader("content-type", "application/json; charset=utf-8")
                .end(Json.encodePrettily(service.isVietNamHoliday(dateMonthYear)));
    }

}

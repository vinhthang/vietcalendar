package io.github.amlich.calendar.verticle;

import io.github.amlich.calendar.model.DateMonthYear;
import io.github.amlich.calendar.service.LunarCalendarService;
import io.vertx.core.AbstractVerticle;
import io.vertx.core.Promise;
import io.vertx.core.eventbus.MessageConsumer;
import lombok.extern.slf4j.Slf4j;

import javax.inject.Inject;

@Slf4j
public class SolarDateToLunarWorkerVerticle extends AbstractVerticle {
    public static final String CONSUMER = "solar.to.lunar.worker";
    private LunarCalendarService service;

    @Inject
    public SolarDateToLunarWorkerVerticle(LunarCalendarService service) {
        this.service = service;
    }

    @Override
    public void start(Promise<Void> startFuture) throws Exception {
        MessageConsumer<byte[]> consumer = vertx.eventBus().consumer(CONSUMER);
        consumer.handler(message -> {
            DateMonthYear obj = new DateMonthYear(message.body());
            DateMonthYear result = service.toLunar(obj);
            message.reply(result.getBytes());
        });
        log.info("Solar Date To Lunar Date Service Worker Verticle Started.");
    }

    @Override
    public void stop(Promise<Void> stopFuture) throws Exception {
        log.info("Solar Date To Lunar Date Service Worker Verticle Stopped.");
    }
}

package io.github.amlich.calendar.verticle;

import io.github.amlich.calendar.model.DateMonthYear;
import io.github.amlich.calendar.service.LunarCalendarService;
import io.vertx.core.AbstractVerticle;
import io.vertx.core.Future;
import io.vertx.core.eventbus.MessageConsumer;
import io.vertx.core.logging.Logger;
import io.vertx.core.logging.LoggerFactory;

import javax.inject.Inject;

public class SolarDateToLunarWorkerVerticle extends AbstractVerticle {
    private static final Logger logger = LoggerFactory.getLogger(SolarDateToLunarWorkerVerticle.class);
    public static final String CONSUMER = "solar.to.lunar.worker";
    private LunarCalendarService service;

    @Inject
    public SolarDateToLunarWorkerVerticle(LunarCalendarService service) {
        this.service = service;
    }

    @Override
    public void start(Future<Void> startFuture) throws Exception {
        MessageConsumer<byte[]> consumer = vertx.eventBus().consumer(CONSUMER);
        consumer.handler(message -> {
            DateMonthYear obj = new DateMonthYear(message.body());
            DateMonthYear result = service.toLunar(obj);
            message.reply(result.getBytes());
        });
        logger.info("Solar Date To Lunar Date Service Worker Verticle Started.");
    }

    @Override
    public void stop(Future<Void> stopFuture) throws Exception {
        logger.info("Solar Date To Lunar Date Service Worker Verticle Stopped.");
    }
}

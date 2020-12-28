package io.github.amlich.calendar.verticle;

import com.google.inject.Guice;
import io.github.amlich.calendar.LunarCalendarGuiceModule;
import io.github.amlich.calendar.endpoint.VietNamHolidayEndPoint;
import io.github.amlich.calendar.model.DateMonthYear;
import io.github.amlich.calendar.service.LunarCalendarService;
import io.github.amlich.calendar.service.VietCalendarService;
import io.github.amlich.calendar.service.VietNamNationalHolidayService;
import io.vertx.core.AbstractVerticle;
import io.vertx.core.DeploymentOptions;
import io.vertx.core.Promise;
import io.vertx.core.http.HttpServerRequest;
import io.vertx.core.json.Json;
import io.vertx.ext.web.Router;
import io.vertx.ext.web.RoutingContext;
import io.vertx.ext.web.handler.BodyHandler;
import io.vertx.ext.web.handler.StaticHandler;
import lombok.extern.slf4j.Slf4j;

import javax.inject.Inject;
import java.time.LocalDate;
import java.util.Optional;

@Slf4j
public class AmLichVerticle extends AbstractVerticle {
    private final LunarCalendarService service;
    private final VietNamHolidayEndPoint vietnamHolidayEndPoint;
    private String deploymentID;

    @Inject
    public AmLichVerticle() {
        VietCalendarService calendarService = new VietCalendarService();
        VietNamNationalHolidayService vietnamNationalHoliday = new VietNamNationalHolidayService(calendarService);
        this.service = new LunarCalendarService(vietnamNationalHoliday);
        this.vietnamHolidayEndPoint = new VietNamHolidayEndPoint(service);
    }

    @Override
    public void start(Promise<Void> startFuture) {
//        Guice.createInjector(new LunarCalendarGuiceModule(vertx)).injectMembers(this);

        DeploymentOptions options = new DeploymentOptions();
        options.setWorker(true);
        //TODO don't know why not log/print to console
        vertx.deployVerticle(new SolarDateToLunarWorkerVerticle(service), options, res -> {
            if (res.succeeded()) {
                deploymentID = res.result();
                log.info("Lunar Worker Started. with ID:" + deploymentID);
            } else {
                log.error("Deploy Lunar Worker Failed: " + res.result());
                System.exit(-1);
            }
        });

        Router router = Router.router(vertx);

        router.get("/lunar").handler(ctx -> {
            HttpServerRequest request = ctx.request();
            request.pause();

            String dd = ctx.request().getParam("dd");
            String MM = ctx.request().getParam("MM");
            String yyyy = ctx.request().getParam("yyyy");
            String timeZone = ctx.request().getParam("timeZone");

            DateMonthYear message = new DateMonthYear(dd, MM, yyyy, Optional.ofNullable(timeZone));

            log.debug("got lunar date: " + message);

            ctx.vertx().eventBus().request(SolarDateToLunarWorkerVerticle.CONSUMER, message.getBytes(), (ar) -> {
                request.resume();
                DateMonthYear result = new DateMonthYear((byte[])ar.result().body());
                ctx.response()
                        .putHeader("content-type", "application/json; charset=utf-8")
                        .end(Json.encodePrettily(result));
            });
        }).failureHandler(this::defaultFailureHandler);

        router.get("/vietnam-holiday")
                .handler(vietnamHolidayEndPoint::handle)
                .failureHandler(this::defaultFailureHandler);

        router.get("/lunar/:ddMMyyyy").handler(r -> {
            r.response()
                    .putHeader("content-type", "application/json; charset=utf-8")
                    .end(service.toLunar(r.request().getParam("ddMMyyyy")).encodePrettily());
        }).failureHandler(this::defaultFailureHandler);

        router.route("/").handler(this::home);

        router.route().handler(BodyHandler.create());
        router.route().handler(StaticHandler.create());

        String httpAddress = System.getProperty("http.address");
        if (httpAddress == null) httpAddress = "localhost";
        Integer httpPort = Integer.getInteger(System.getProperty("http.port"), 8080);

        log.info(String.format("listen to http address: %s port: %s", httpAddress, httpPort));
        vertx.createHttpServer()
                .requestHandler(router::handle)
                .listen(httpPort, httpAddress, result -> {
                    if (result.succeeded()) {
                        startFuture.complete();
                    } else {
                        startFuture.fail(result.cause());
                    }
                });
    }

    @Override
    public void stop(Promise<Void> stopFuture) {
        log.info("Main Verticle stopped!");
        //NEED this for unit test to stop or hang forever
        stopFuture.complete();
    }

    private void home(RoutingContext routingContext) {
        routingContext
                .response()
                .putHeader("content-type", "application/json; charset=utf-8")
                .end(service.toLunar(LocalDate.now()).encodePrettily());
    }

    private void defaultFailureHandler(RoutingContext rc) {
        rc.response()
                .putHeader("content-type", "application/json; charset=utf-8")
                .setStatusCode(400)
                .end(rc.failure().getMessage());
    }
}

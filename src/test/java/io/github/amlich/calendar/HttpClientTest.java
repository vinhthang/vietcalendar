package io.github.amlich.calendar;

import io.vertx.core.Vertx;
import io.vertx.core.http.HttpClient;
import io.vertx.rxjava.ext.unit.Async;
import io.vertx.rxjava.ext.unit.TestContext;
import org.junit.jupiter.api.Test;

public class HttpClientTest {
    @Test
    public void test(TestContext context) {
        Vertx vertx = Vertx.vertx();
        Async async = context.async();
        HttpClient client = vertx.createHttpClient();
//        client.getNow(80, "vnexpress.net", "/", res -> {
//            res.bodyHandler(body -> {
////                System.out.println(body.toString());
//                context.asyncAssertSuccess();
//                async.complete();
//                client.close();
//            });
//        });
    }
}

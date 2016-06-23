package io.github.amlich.calendar;

import io.vertx.core.Vertx;
import io.vertx.core.http.HttpClient;
import io.vertx.ext.unit.Async;
import io.vertx.ext.unit.TestContext;
import io.vertx.ext.unit.junit.VertxUnitRunner;
import org.junit.Ignore;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(VertxUnitRunner.class)

public class HttpClientTest {
    @Ignore
    @Test
    public void test(TestContext context) {
        Vertx vertx = Vertx.vertx();
        Async async = context.async();
        HttpClient client = vertx.createHttpClient();
        client.getNow(80, "vnexpress.net" , "/", res -> {
            res.bodyHandler(body -> {
//                System.out.println(body.toString());
                context.asyncAssertSuccess();
                async.complete();
                client.close();
            });
        });
    }
}

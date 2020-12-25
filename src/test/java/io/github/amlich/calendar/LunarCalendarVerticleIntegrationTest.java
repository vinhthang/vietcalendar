//package io.github.amlich.calendar;
//
//import io.github.amlich.calendar.verticle.AmLichVerticle;
//import io.vertx.core.Vertx;
//import io.vertx.core.http.HttpClient;
//
//public class LunarCalendarVerticleIntegrationTest {
//    private Vertx vertx;
//
//    @Before
//    public void setUp(TestContext context) {
//        vertx = Vertx.vertx();
//
//        vertx.deployVerticle(AmLichVerticle.class.getName(), context.asyncAssertSuccess());
//    }
//
//    @After
//    public void tearDown(TestContext context) {
//        vertx.close(context.asyncAssertSuccess());
//    }
//
//    @Test
//    public void testEndPointSlash(TestContext context) {
//        final Async async = context.async();
//
//        vertx.createHttpClient().getNow(8080, "localhost", "/lunar?dd=1&MM=2&yyyy=1981",
//                response -> {
//                    response.handler(body -> {
//                        context.assertTrue(body.toString().contains("{"));
//                        async.complete();
//                    });
//                });
//    }
//
//    @Test
//    public void testEndPointLunar(TestContext context) {
//        final Async async = context.async();
//        HttpClient client = vertx.createHttpClient();
//
//        client.getNow(8080, "localhost", "/lunar/15092015",
//                response -> {
//                    response.bodyHandler(body -> {
//                        context.assertTrue(body.toString().contains("2015"));
//                        async.complete();
//                    });
//                });
//    }
//
//    @Test(timeout = 1000)
//    public void testEndPointLunar2(TestContext context) {
//        final Async async = context.async();
//        final HttpClient client = vertx.createHttpClient();
//
//        client.getNow(8080, "localhost", "/lunar?dd=15&MM=09&yyyy=2015",
//                response -> {
//                    response.handler(body -> {
//                        context.assertTrue(body.toString().contains("2015"));
//                        async.complete();
//                    });
//                });
//
//        client.getNow(8080, "localhost", "/lunar?dd=15&MM=09&yyyy=2015&timeZone=7",
//                response -> {
//                    response.handler(body -> {
//                        context.assertTrue(body.toString().contains("2015"));
//                        async.complete();
//                    });
//                });
//    }
//
//    @Test
//    public void testFailureHandler(TestContext context) {
//        final Async async = context.async();
//        final HttpClient client = vertx.createHttpClient();
//
//        client.getNow(8080, "localhost", "/lunar?dd=xxx",
//                response -> {
//                    response.handler(body -> {
//                        context.assertEquals(400, response.statusCode());
//                        async.complete();
//                    });
//                });
//
//    }
//}

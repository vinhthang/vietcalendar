//package io.github.amlich.calendar;
//
//import guru.nidi.ramltester.RamlDefinition;
//import guru.nidi.ramltester.RamlLoaders;
//import guru.nidi.ramltester.jaxrs.CheckingWebTarget;
//import guru.nidi.ramltester.junit.RamlMatchers;
//import io.github.amlich.calendar.verticle.AmLichVerticle;
//import io.vertx.core.Vertx;
//import io.vertx.ext.unit.TestContext;
//import io.vertx.ext.unit.junit.VertxUnitRunner;
//import org.jboss.resteasy.client.jaxrs.ResteasyClient;
//import org.jboss.resteasy.client.jaxrs.ResteasyClientBuilder;
//import org.junit.Assert;
//import org.junit.Before;
//import org.junit.BeforeClass;
//import org.junit.Test;
//import org.junit.runner.RunWith;
//
//@RunWith(VertxUnitRunner.class)
//public class LunarEndpointAPI_IntegrationTest {
//    private static final RamlDefinition api = RamlLoaders.fromClasspath()
//            .load("/webroot/api/lunar.raml")
//            .assumingBaseUri("http://localhost:8080/");
//
//    private ResteasyClient client = new ResteasyClientBuilder().build();
//    private CheckingWebTarget checking;
//
//    @BeforeClass
//    public static void bootApp(TestContext context) {
//        Vertx.vertx().deployVerticle(AmLichVerticle.class.getName(),
//                context.asyncAssertSuccess());
//    }
//
//    @Before
//    public void createTarget() {
//        checking = api.createWebTarget(client.target("http://localhost:8080"));
//    }
//
//    @Test
//    public void testLunarEndpoint() {
//        checking.path("/lunar")
//                .queryParam("dd", 10)
//                .queryParam("MM", 5)
//                .queryParam("yyyy", 2015)
//                .request().get();
//        Assert.assertThat(checking.getLastReport(), RamlMatchers.hasNoViolations());
//    }
//}

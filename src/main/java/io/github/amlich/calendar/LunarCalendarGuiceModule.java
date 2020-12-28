package io.github.amlich.calendar;

import com.google.inject.AbstractModule;
import com.google.inject.Provides;
import com.google.inject.Singleton;
import com.google.inject.name.Names;
import io.vertx.core.Context;
import io.vertx.core.Vertx;
import io.vertx.core.eventbus.EventBus;
import io.vertx.core.json.JsonObject;

import java.util.Properties;

public class LunarCalendarGuiceModule extends AbstractModule {
    private final Vertx vertx;
    private final Context context;

    public LunarCalendarGuiceModule(Vertx vertx) {
        this.vertx = vertx;
        this.context = vertx.getOrCreateContext();
    }

    @Override
    protected void configure() {
        bind(EventBus.class).toInstance(vertx.eventBus());
        Names.bindProperties(binder(), extractToProperties(context.config()));
    }

    private Properties extractToProperties(JsonObject config) {
        Properties properties = new Properties();
        config
                .getMap()
                .keySet()
                .stream()
                .forEach((String key) -> properties.setProperty(key, config.getValue(key).toString()));
        return properties;
    }

}

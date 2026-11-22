//! Criterion microbenchmarks for hot-path calendar, holiday, parsing, and kernel dispatch.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

use vietcalendar_rs::contracts::astrology::{
    AstrologyQuery, AstrologyService, ChartSystem, Gender,
};
use vietcalendar_rs::contracts::calendar::CalendarService;
use vietcalendar_rs::contracts::canchi::CanChiSolarService;
use vietcalendar_rs::contracts::holidays::HolidayService;
use vietcalendar_rs::contracts::recurrence::{
    Day30Policy, LeapMonthPolicy, LunarRecurrenceService, RecurrenceFrequency, RecurrencePreset,
    RecurrenceRequest,
};
use vietcalendar_rs::contracts::retail::RetailCalendarService;
use vietcalendar_rs::contracts::trach_nhat::{AuspiciousSearchQuery, Occasion, TrachNhatService};
use vietcalendar_rs::kernel::Kernel;
use vietcalendar_rs::plugins::astrology_vn::AstrologyEngine;
use vietcalendar_rs::plugins::calendar_vn::layout::LunarSpan;
use vietcalendar_rs::plugins::calendar_vn::{VietnamCalendarEngine, VietnamCalendarPlugin};
use vietcalendar_rs::plugins::canchi_solar_vn::CanChiSolarEngine;
use vietcalendar_rs::plugins::holidays_vn::loader::parse_year_schedule_yaml;
use vietcalendar_rs::plugins::holidays_vn::ShareableHolidayEngine;
use vietcalendar_rs::plugins::http_calendar::parse_ymd_ascii;
use vietcalendar_rs::plugins::recurrence_vn::LunarRecurrenceEngine;
use vietcalendar_rs::plugins::retail_vn::loader::parse_retail_yaml;
use vietcalendar_rs::plugins::retail_vn::RetailCalendarEngine;
use vietcalendar_rs::plugins::trach_nhat_vn::TrachNhatEngine;

fn bench_calendar_and_kernel(c: &mut Criterion) {
    let cal_engine = VietnamCalendarEngine::new(4096);
    let solar_date = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
    // Warm up LunarSpan cache.
    let _ = cal_engine.to_lunar(solar_date, 7.0);
    let _ = cal_engine.to_solar(1, 1, 2024, false, 7.0);

    c.bench_function("calendar_to_lunar_warm", |b| {
        b.iter(|| cal_engine.to_lunar(black_box(solar_date), black_box(7.0)))
    });

    c.bench_function("calendar_to_solar_warm", |b| {
        b.iter(|| cal_engine.to_solar(black_box(1), black_box(1), black_box(2024), false, 7.0))
    });

    c.bench_function("calendar_lunar_span_cold", |b| {
        b.iter(|| LunarSpan::compute(black_box(2024), black_box(7.0)))
    });

    let yaml_2026 = include_str!("../config/holidays/2026.yml");
    let sched_2026 = parse_year_schedule_yaml(yaml_2026, "2026.yml").unwrap();
    let hol_engine =
        ShareableHolidayEngine::new(HashMap::from([(2026, Arc::new(sched_2026))]), 256);
    let check_date = NaiveDate::from_ymd_opt(2026, 8, 22).unwrap();
    let _ = hol_engine.is_holiday(check_date);

    c.bench_function("holiday_is_holiday_warm", |b| {
        b.iter(|| hol_engine.is_holiday(black_box(check_date)))
    });

    c.bench_function("http_parse_ymd_ascii", |b| {
        b.iter(|| parse_ymd_ascii(black_box("2026-08-22")))
    });

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let kernel = rt.block_on(async {
        let k = Kernel::new();
        let _ = k
            .mount("calendar_vn", Arc::new(VietnamCalendarPlugin), json!({}))
            .await;
        k
    });

    c.bench_function("kernel_wait_free_inject", |b| {
        b.iter(|| kernel.inject::<dyn CalendarService>())
    });

    // Niche Lunar plugins microbenchmarks
    let cal_arc: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(4096));
    let canchi_arc: Arc<dyn CanChiSolarService> =
        Arc::new(CanChiSolarEngine::new(Arc::clone(&cal_arc)));
    let _ = canchi_arc.solar_terms_for_year(2024, 7.0);

    c.bench_function("canchi_full_breakdown", |b| {
        b.iter(|| canchi_arc.full_can_chi(black_box(solar_date), black_box(12), black_box(0), 7.0))
    });

    c.bench_function("solar_terms_2024_cached", |b| {
        b.iter(|| canchi_arc.solar_terms_for_year(black_box(2024), 7.0))
    });

    let astro_engine = AstrologyEngine::new(Arc::clone(&cal_arc), Arc::clone(&canchi_arc));
    let astro_query = AstrologyQuery {
        date: "1990-05-15".to_string(),
        hour: 10,
        minute: 0,
        gender: Gender::Male,
        system: ChartSystem::Both,
        is_lunar: false,
        is_leap_month: false,
        time_zone: 7.0,
    };
    c.bench_function("astrology_bazi_and_tuvi", |b| {
        b.iter(|| astro_engine.generate_chart(black_box(&astro_query)))
    });

    let tn_engine = TrachNhatEngine::new(Arc::clone(&cal_arc), Arc::clone(&canchi_arc));
    let tn_query = AuspiciousSearchQuery {
        start_date: "2024-02-10".to_string(),
        end_date: "2024-03-10".to_string(),
        occasion: Occasion::Wedding,
        owner_birth_year: Some(1990),
        partner_birth_year: None,
        min_score: None,
        limit: Some(10),
        time_zone: 7.0,
    };
    c.bench_function("trach_nhat_search_30d", |b| {
        b.iter(|| tn_engine.find_auspicious_dates(black_box(&tn_query)))
    });

    let rec_engine = LunarRecurrenceEngine::new(Arc::clone(&cal_arc));
    let rec_10yr = RecurrenceRequest {
        title: Some("Rằm & Mùng 1".to_string()),
        description: None,
        frequency: None,
        preset: Some(RecurrencePreset::NewAndFullMoon),
        lunar_day: None,
        lunar_month: None,
        start_date: "2020-01-01".to_string(),
        end_date: "2029-12-31".to_string(),
        day30_policy: Day30Policy::LastDayOfMonth,
        leap_month_policy: LeapMonthPolicy::IncludeLeap,
        time_zone: 7.0,
        reminder_minutes: Some(720),
    };
    c.bench_function("recurrence_10yr_expand", |b| {
        b.iter(|| rec_engine.expand_occurrences(black_box(&rec_10yr)))
    });

    let rec_1yr = RecurrenceRequest {
        frequency: Some(RecurrenceFrequency::Monthly),
        preset: None,
        lunar_day: Some(15),
        end_date: "2020-12-31".to_string(),
        ..rec_10yr.clone()
    };
    c.bench_function("recurrence_ics_render", |b| {
        b.iter(|| rec_engine.export_ics(black_box(&rec_1yr)))
    });

    let retail_yaml = include_str!("../config/retail_events.yml");
    let retail_cat = parse_retail_yaml(retail_yaml, "retail_events.yml").unwrap();
    let retail_engine = RetailCalendarEngine::new(Arc::clone(&cal_arc), retail_cat);
    c.bench_function("retail_2025_year_lookup", |b| {
        b.iter(|| retail_engine.retail_events_for_year(black_box(2025), None, 7.0))
    });

    rt.block_on(kernel.shutdown());
    let _ = Path::new("config");
}

criterion_group!(benches, bench_calendar_and_kernel);
criterion_main!(benches);

//! BaZi (Four Pillars / Tứ Trụ) chart builder.

use chrono::{Datelike, NaiveDate};

use crate::contracts::astrology::{
    BaziChart, BaziPillar, FiveElementBalance, Gender, HiddenStemEntry, LuckCycle,
};
use crate::contracts::canchi::{BilingualLabel, CanChiBreakdown, CanChiPair, CanChiSolarService};
use crate::contracts::models::CalendarError;
use crate::plugins::canchi_solar_vn::canchi::{element_label, stem_element_index, stem_label};
use crate::plugins::canchi_solar_vn::solar_terms::local_to_unix_seconds;

const TEN_GODS_SAME_POL: &[(&str, &str, &str)] = &[
    ("ty_kien", "Tỷ Kiên", "Companion (Bi Jian)"),
    ("thuc_than", "Thực Thần", "Eating God (Shi Shen)"),
    ("thien_tai", "Thiên Tài", "Indirect Wealth (Pian Cai)"),
    ("that_sat", "Thất Sát", "Seven Killings (Qi Sha)"),
    ("thien_an", "Thiên Ấn", "Indirect Resource (Pian Yin)"),
];

const TEN_GODS_OPP_POL: &[(&str, &str, &str)] = &[
    ("kiep_tai", "Kiếp Tài", "Rob Wealth (Jie Cai)"),
    ("thuong_quan", "Thương Quan", "Hurting Officer (Shang Guan)"),
    ("chinh_tai", "Chính Tài", "Direct Wealth (Zheng Cai)"),
    ("chinh_quan", "Chính Quan", "Direct Officer (Zheng Guan)"),
    ("chinh_an", "Chính Ấn", "Direct Resource (Zheng Yin)"),
];

const LIFE_STAGES: &[(&str, &str, &str)] = &[
    ("truong_sinh", "Trường Sinh", "Growth (Chang Sheng)"),
    ("moc_duc", "Mộc Dục", "Bath (Mu Yu)"),
    ("quan_doi", "Quan Đới", "Crown Belt (Guan Dai)"),
    ("lam_quan", "Lâm Quan", "Coming of Age (Lin Guan)"),
    ("de_vuong", "Đế Vượng", "Prosperity Peak (Di Wang)"),
    ("suy", "Suy", "Decline (Shuai)"),
    ("benh", "Bệnh", "Sickness (Bing)"),
    ("tu", "Tử", "Death (Si)"),
    ("mo", "Mộ", "Tomb / Storage (Mu)"),
    ("tuyet", "Tuyệt", "Extinction (Jue)"),
    ("thai", "Thai", "Conception (Tai)"),
    ("duong", "Dưỡng", "Nurturing (Yang)"),
];

// Hidden stems: (stem_index, weight_pct)
const HIDDEN_STEMS: &[&[(usize, u8)]] = &[
    &[(9, 100)],                  // 0 Ty: Quy
    &[(5, 60), (9, 30), (7, 10)], // 1 Suu: Ky, Quy, Tan
    &[(0, 60), (2, 30), (4, 10)], // 2 Dan: Giap, Binh, Mau
    &[(1, 100)],                  // 3 Mao: At
    &[(4, 60), (1, 30), (9, 10)], // 4 Thin: Mau, At, Quy
    &[(2, 60), (6, 30), (4, 10)], // 5 Ty_ran: Binh, Canh, Mau
    &[(3, 70), (5, 30)],          // 6 Ngo: Dinh, Ky
    &[(5, 60), (3, 30), (1, 10)], // 7 Mui: Ky, Dinh, At
    &[(6, 60), (8, 30), (4, 10)], // 8 Than: Canh, Nham, Mau
    &[(7, 100)],                  // 9 Dau: Tan
    &[(4, 60), (7, 30), (3, 10)], // 10 Tuat: Mau, Tan, Dinh
    &[(8, 60), (0, 40)],          // 11 Hoi: Nham, Giap
];

// Branch main Five Element index (0=Kim, 1=Moc, 2=Thuy, 3=Hoa, 4=Tho)
const BRANCH_ELEMENT: &[usize] = &[2, 4, 1, 1, 4, 3, 3, 4, 0, 0, 4, 2];

// Generating cycle index from Can index: 0=Wood, 1=Fire, 2=Earth, 3=Metal, 4=Water.
#[inline]
fn gen_cycle_idx(stem_idx: usize) -> i32 {
    (stem_idx as i32 % 10) / 2
}

pub(crate) fn compute_ten_god(day_master_stem: usize, target_stem: usize) -> BilingualLabel {
    let dm = day_master_stem % 10;
    let tg = target_stem % 10;
    let diff = (gen_cycle_idx(tg) - gen_cycle_idx(dm)).rem_euclid(5) as usize;
    let same_pol = (dm % 2) == (tg % 2);
    let table = if same_pol {
        TEN_GODS_SAME_POL
    } else {
        TEN_GODS_OPP_POL
    };
    table.get(diff).map_or_else(
        || BilingualLabel::new("ty_kien", "Tỷ Kiên", "Companion"),
        |(id, vi, en)| BilingualLabel::new(*id, *vi, *en),
    )
}

fn day_master_life_stage(dm_stem: usize, branch_idx: usize) -> BilingualLabel {
    let dm = dm_stem % 10;
    let b = branch_idx % 12;
    let starts: [usize; 10] = [11, 6, 2, 9, 2, 9, 5, 0, 8, 3];
    let start = starts.get(dm).copied().unwrap_or(11);
    let offset = if dm.is_multiple_of(2) {
        (b as i32 - start as i32).rem_euclid(12) as usize
    } else {
        (start as i32 - b as i32).rem_euclid(12) as usize
    };
    LIFE_STAGES.get(offset).map_or_else(
        || BilingualLabel::new("truong_sinh", "Trường Sinh", "Growth"),
        |(id, vi, en)| BilingualLabel::new(*id, *vi, *en),
    )
}

fn build_pillar(
    slot: BilingualLabel,
    pair: CanChiPair,
    day_master_stem: usize,
    is_day_pillar: bool,
) -> BaziPillar {
    let ten_god_stem = if is_day_pillar {
        BilingualLabel::new("nhat_chu", "Nhật Chủ", "Day Master")
    } else {
        compute_ten_god(day_master_stem, usize::from(pair.stem_index))
    };

    let branch = usize::from(pair.branch_index) % 12;
    let hidden_defs = HIDDEN_STEMS.get(branch).copied().unwrap_or(&[]);
    let mut hidden_stems = Vec::with_capacity(hidden_defs.len());
    for &(h_stem, weight_pct) in hidden_defs {
        hidden_stems.push(HiddenStemEntry {
            stem: stem_label(h_stem),
            element: element_label(stem_element_index(h_stem)),
            ten_god: compute_ten_god(day_master_stem, h_stem),
            weight_pct,
        });
    }

    let life_stage = day_master_life_stage(day_master_stem, branch);
    BaziPillar {
        pillar: slot,
        can_chi: pair,
        ten_god_stem,
        hidden_stems,
        life_stage,
    }
}

fn tally_elements(pillars: &[&CanChiPair]) -> FiveElementBalance {
    let mut counts = [0u8; 5]; // 0=Kim, 1=Moc, 2=Thuy, 3=Hoa, 4=Tho
    for pair in pillars {
        let s_elem = stem_element_index(usize::from(pair.stem_index));
        if let Some(c) = counts.get_mut(s_elem) {
            *c = c.saturating_add(1);
        }
        let b_elem = BRANCH_ELEMENT
            .get(usize::from(pair.branch_index) % 12)
            .copied()
            .unwrap_or(0);
        if let Some(c) = counts.get_mut(b_elem) {
            *c = c.saturating_add(1);
        }
    }

    let mut dom_idx = 0usize;
    let mut weak_idx = 0usize;
    for (idx, &val) in counts.iter().enumerate() {
        if val > counts.get(dom_idx).copied().unwrap_or(0) {
            dom_idx = idx;
        }
        if val < counts.get(weak_idx).copied().unwrap_or(u8::MAX) {
            weak_idx = idx;
        }
    }

    let [metal, wood, water, fire, earth] = counts;
    FiveElementBalance {
        wood,
        fire,
        earth,
        metal,
        water,
        dominant: element_label(dom_idx),
        weakest: element_label(weak_idx),
    }
}

fn compute_start_luck_age(
    canchi: &dyn CanChiSolarService,
    solar_date: NaiveDate,
    breakdown: &CanChiBreakdown,
    forward: bool,
) -> Result<u8, CalendarError> {
    let birth_ts = local_to_unix_seconds(
        solar_date,
        breakdown.hour,
        breakdown.minute,
        breakdown.time_zone,
    )?;
    let mut major_terms = Vec::with_capacity(72);
    for yr in [
        solar_date.year() - 1,
        solar_date.year(),
        solar_date.year() + 1,
    ] {
        for term in canchi.solar_terms_for_year(yr, breakdown.time_zone)? {
            if term.is_major_tiet {
                major_terms.push(term.unix_seconds);
            }
        }
    }

    let delta_secs = if forward {
        major_terms
            .iter()
            .copied()
            .find(|&ts| ts >= birth_ts)
            .map_or(86_400 * 3, |ts| (ts - birth_ts).abs())
    } else {
        major_terms
            .iter()
            .copied()
            .rev()
            .find(|&ts| ts <= birth_ts)
            .map_or(86_400 * 3, |ts| (birth_ts - ts).abs())
    };
    let age = ((delta_secs as f64 / 86_400.0) / 3.0)
        .round()
        .clamp(1.0, 10.0) as u8;
    Ok(age)
}

/// Builds a complete BaZi chart from a Can Chi breakdown, solar date, and gender.
///
/// # Errors
/// Returns [`CalendarError`] when solar-term distance resolution fails bounds checks.
pub(crate) fn build_bazi_chart(
    canchi: &dyn CanChiSolarService,
    solar_date: NaiveDate,
    gender: Gender,
    breakdown: &CanChiBreakdown,
) -> Result<BaziChart, CalendarError> {
    let dm = usize::from(breakdown.day_can_chi.stem_index) % 10;
    let dm_label = stem_label(dm);
    let dm_element = element_label(stem_element_index(dm));

    let year_is_yang = breakdown.bazi_year_can_chi.stem_index.is_multiple_of(2);
    let polarity = if year_is_yang {
        BilingualLabel::new("yang", "Dương", "Yang")
    } else {
        BilingualLabel::new("yin", "Âm", "Yin")
    };
    let forward =
        (year_is_yang && gender == Gender::Male) || (!year_is_yang && gender == Gender::Female);

    let year_pillar = build_pillar(
        BilingualLabel::new("year", "Trụ Năm", "Year Pillar"),
        breakdown.bazi_year_can_chi.clone(),
        dm,
        false,
    );
    let month_pillar = build_pillar(
        BilingualLabel::new("month", "Trụ Tháng", "Month Pillar"),
        breakdown.bazi_month_can_chi.clone(),
        dm,
        false,
    );
    let day_pillar = build_pillar(
        BilingualLabel::new("day", "Trụ Ngày", "Day Pillar"),
        breakdown.day_can_chi.clone(),
        dm,
        true,
    );
    let hour_pillar = build_pillar(
        BilingualLabel::new("hour", "Trụ Giờ", "Hour Pillar"),
        breakdown.hour_can_chi.clone(),
        dm,
        false,
    );

    let element_balance = tally_elements(&[
        &year_pillar.can_chi,
        &month_pillar.can_chi,
        &day_pillar.can_chi,
        &hour_pillar.can_chi,
    ]);

    let start_age = compute_start_luck_age(canchi, solar_date, breakdown, forward)?;
    let step: i32 = if forward { 1 } else { -1 };
    let month_sexagenary = i32::from(month_pillar.can_chi.sexagenary_index);

    let mut luck_cycles = Vec::with_capacity(8);
    for idx in 1..=8u8 {
        let cycle_sexagenary = (month_sexagenary + step * i32::from(idx)).rem_euclid(60) as u8;
        let cc = canchi.pair_from_index(cycle_sexagenary);
        let ten_god_stem = compute_ten_god(dm, usize::from(cc.stem_index));
        let s_age = start_age + (idx - 1) * 10;
        let e_age = s_age + 9;
        let start_year = solar_date.year() + i32::from(s_age);
        luck_cycles.push(LuckCycle {
            index: idx,
            start_age: s_age,
            end_age: e_age,
            start_year,
            can_chi: cc,
            ten_god_stem,
        });
    }

    Ok(BaziChart {
        day_master: dm_label,
        day_master_element: dm_element,
        yin_yang_polarity: polarity,
        forward_luck_direction: forward,
        start_luck_age_years: start_age,
        year_pillar,
        month_pillar,
        day_pillar,
        hour_pillar,
        element_balance,
        luck_cycles,
    })
}

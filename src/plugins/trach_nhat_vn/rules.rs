//! Trạch Nhật rules: Hoàng Đạo, 12 Trực, 28 Tú, taboos, age conflicts, and scoring.

use crate::contracts::canchi::{BilingualLabel, CanChiBreakdown, CanChiSolarService};
use crate::contracts::models::CalendarError;
use crate::contracts::trach_nhat::{
    AgeConflictCheck, HourFortune, Occasion, TabooAlert, TrachNhatDayAnalysis,
};
use crate::plugins::canchi_solar_vn::canchi::branch_label;

const HOUR_RANGES: &[&str] = &[
    "23:00-01:00",
    "01:00-03:00",
    "03:00-05:00",
    "05:00-07:00",
    "07:00-09:00",
    "09:00-11:00",
    "11:00-13:00",
    "13:00-15:00",
    "15:00-17:00",
    "17:00-19:00",
    "19:00-21:00",
    "21:00-23:00",
];

// 12-spirit Hoang Dao / Hac Dao wheel starting at Thanh Long.
const TWELVE_SPIRITS: &[(&str, &str, &str, bool)] = &[
    ("thanh_long", "Thanh Long", "Azure Dragon", true),
    ("minh_duong", "Minh Đường", "Bright Hall", true),
    ("thien_hinh", "Thiên Hình", "Heavenly Penalty", false),
    ("chu_tuoc", "Chu Tước", "Vermilion Bird", false),
    ("kim_quy", "Kim Quỹ", "Golden Coffer", true),
    ("bao_quang", "Bảo Quang", "Precious Light", true),
    ("bach_ho", "Bạch Hổ", "White Tiger", false),
    ("ngoc_duong", "Ngọc Đường", "Jade Hall", true),
    ("thien_lao", "Thiên Lao", "Heavenly Prison", false),
    ("huyen_vu", "Huyền Vũ", "Black Tortoise", false),
    ("tu_menh", "Tư Mệnh", "Destiny Commander", true),
    ("cau_tran", "Câu Trận", "Grappling Hook", false),
];

// 12 Kien Tru Truc: (id, vi, en, quality, wedding_ok, earth_ok, commerce_ok)
const TWELVE_TRUC: &[(&str, &str, &str, &str)] = &[
    ("kien", "Trực Kiến", "Establish (Jian)", "neutral"),
    ("tru", "Trực Trừ", "Remove (Chu)", "auspicious"),
    ("man", "Trực Mãn", "Fullness (Man)", "auspicious"),
    ("binh", "Trực Bình", "Balance (Ping)", "neutral"),
    ("dinh", "Trực Định", "Stability (Ding)", "auspicious"),
    ("chap", "Trực Chấp", "Hold (Zhi)", "neutral"),
    ("pha", "Trực Phá", "Break (Po)", "inauspicious"),
    ("nguy", "Trực Nguy", "Danger (Wei)", "inauspicious"),
    ("thanh", "Trực Thành", "Success (Cheng)", "auspicious"),
    ("thu", "Trực Thu", "Harvest (Shou)", "neutral"),
    ("khai", "Trực Khai", "Open (Kai)", "auspicious"),
    ("be", "Trực Bế", "Close (Bi)", "inauspicious"),
];

// 28 Lunar Mansions (Nhi Thap Bat Tu): 0 = Giac .. 23 = Lieu .. 27 = Chan.
const TWENTY_EIGHT_TU: &[(&str, &str, &str, bool)] = &[
    (
        "giac",
        "Sao Giác (Giác Mộc Giao)",
        "Horn (Wood Dragon)",
        true,
    ),
    (
        "cang",
        "Sao Cang (Cang Kim Long)",
        "Neck (Metal Dragon)",
        false,
    ),
    ("de", "Sao Đê (Đê Thổ Lạc)", "Root (Earth Badger)", false),
    (
        "phong",
        "Sao Phòng (Phòng Nhật Thố)",
        "Room (Sun Hare)",
        true,
    ),
    ("tam", "Sao Tâm (Tâm Nguyệt Hồ)", "Heart (Moon Fox)", false),
    ("vi", "Sao Vĩ (Vĩ Hỏa Hổ)", "Tail (Fire Tiger)", true),
    (
        "co",
        "Sao Cơ (Cơ Thủy Báo)",
        "Winnowing Basket (Water Leopard)",
        true,
    ),
    (
        "dau_star",
        "Sao Đẩu (Đẩu Mộc Giải)",
        "Dipper (Wood Unicorn)",
        true,
    ),
    ("nguu", "Sao Ngưu (Ngưu Kim Ngưu)", "Ox (Metal Ox)", false),
    ("nu", "Sao Nữ (Nữ Thổ Bức)", "Girl (Earth Bat)", false),
    ("hu", "Sao Hư (Hư Nhật Thử)", "Emptiness (Sun Rat)", false),
    (
        "nguy_star",
        "Sao Nguy (Nguy Nguyệt Yến)",
        "Rooftop (Moon Swallow)",
        false,
    ),
    (
        "that",
        "Sao Thất (Thất Hỏa Trư)",
        "Encampment (Fire Boar)",
        true,
    ),
    (
        "bich",
        "Sao Bích (Bích Thủy Du)",
        "Wall (Water Porcupine)",
        true,
    ),
    (
        "khue",
        "Sao Khuê (Khuê Mộc Lang)",
        "Legs (Wood Wolf)",
        false,
    ),
    ("lau", "Sao Lâu (Lâu Kim Cẩu)", "Bond (Metal Dog)", true),
    (
        "vi_star",
        "Sao Vị (Vị Thổ Trĩ)",
        "Stomach (Earth Pheasant)",
        true,
    ),
    (
        "mao_star",
        "Sao Mão (Mão Nhật Kê)",
        "Hairy Head (Sun Rooster)",
        false,
    ),
    ("tat", "Sao Tất (Tất Nguyệt Ô)", "Net (Moon Crow)", true),
    (
        "chuy",
        "Sao Chủy (Chủy Hỏa Hầu)",
        "Turtle Beak (Fire Monkey)",
        false,
    ),
    (
        "sam",
        "Sao Sâm (Sâm Thủy Viên)",
        "Three Stars (Water Ape)",
        true,
    ),
    ("tinh", "Sao Tỉnh (Tỉnh Mộc Hãn)", "Well (Wood Tapir)", true),
    (
        "quy_star",
        "Sao Quỷ (Quỷ Kim Dương)",
        "Ghost (Metal Sheep)",
        false,
    ),
    (
        "lieu",
        "Sao Liễu (Liễu Thổ Chương)",
        "Willow (Earth Deer)",
        false,
    ),
    (
        "tinh_star",
        "Sao Tinh (Tinh Nhật Mã)",
        "Star (Sun Horse)",
        false,
    ),
    (
        "truong",
        "Sao Trương (Trương Nguyệt Lộc)",
        "Extended Net (Moon Deer)",
        true,
    ),
    ("duc", "Sao Dực (Dực Hỏa Xà)", "Wings (Fire Serpent)", false),
    (
        "chan",
        "Sao Chẩn (Chẩn Thủy Dẫn)",
        "Chariot (Water Worm)",
        true,
    ),
];

const DUONG_CONG_KY_NHAT: &[(i32, i32)] = &[
    (1, 13),
    (2, 11),
    (3, 9),
    (4, 7),
    (5, 5),
    (6, 3),
    (7, 1),
    (7, 29),
    (8, 27),
    (9, 25),
    (10, 23),
    (11, 21),
    (12, 19),
];

// Lunar month (1..=12) -> forbidden branch index for Sat Chu & Tho Tu.
const SAT_CHU_BRANCH: &[usize] = &[5, 0, 7, 3, 8, 10, 1, 11, 6, 9, 2, 4];
const THO_TU_BRANCH: &[usize] = &[10, 4, 11, 5, 0, 6, 1, 7, 2, 8, 3, 9];

const HOANG_OC_STAGES: &[(&str, &str, &str, bool)] = &[
    ("nhat_cat", "Nhất Cát", "First Auspicious (Optimal)", false),
    ("nhi_nghi", "Nhì Nghi", "Second Proper (Prosperous)", false),
    (
        "tam_dia_sat",
        "Tam Địa Sát",
        "Third Earthly Sha (Avoid)",
        true,
    ),
    (
        "tu_tan_tai",
        "Tứ Tấn Tài",
        "Fourth Wealth Growth (Good)",
        false,
    ),
    (
        "ngu_tho_tu",
        "Ngũ Thọ Tử",
        "Fifth Longevity Clash (Avoid)",
        true,
    ),
    (
        "luc_hoang_oc",
        "Lục Hoang Ốc",
        "Sixth Desolate House (Avoid)",
        true,
    ),
];

#[inline]
fn thanh_long_start(ref_branch: usize) -> usize {
    match ref_branch % 12 {
        2 | 8 => 0,  // Dan / Than -> Ty
        3 | 9 => 2,  // Mao / Dau -> Dan
        4 | 10 => 4, // Thin / Tuat -> Thin
        5 | 11 => 6, // Ty_ran / Hoi -> Ngo
        0 | 6 => 8,  // Ty / Ngo -> Than
        _ => 10,     // Suu / Mui -> Tuat
    }
}

fn spirit_at(start_branch: usize, target_branch: usize) -> (BilingualLabel, bool) {
    let offset = (target_branch as i32 - start_branch as i32).rem_euclid(12) as usize;
    TWELVE_SPIRITS.get(offset).map_or_else(
        || {
            (
                BilingualLabel::new("thanh_long", "Thanh Long", "Azure Dragon"),
                true,
            )
        },
        |&(id, vi, en, good)| (BilingualLabel::new(id, vi, en), good),
    )
}

fn eval_age(
    canchi: &dyn CanChiSolarService,
    birth_year: i32,
    target_lunar_year: i32,
    day_branch_idx: usize,
) -> Result<AgeConflictCheck, CalendarError> {
    if birth_year <= 0 {
        return Err(CalendarError::InvalidSolarDate(format!(
            "birth_year must be positive (got {birth_year})"
        )));
    }
    let birth_cc = canchi.year_can_chi(birth_year);
    let target_yr_cc = canchi.year_can_chi(target_lunar_year);
    let b_branch = usize::from(birth_cc.branch_index) % 12;
    let t_branch = usize::from(target_yr_cc.branch_index) % 12;

    let lunar_age = target_lunar_year - birth_year + 1;
    let effective_age = lunar_age.max(1);

    let tam_tai = match b_branch % 4 {
        0 => matches!(t_branch, 2..=4),      // Than-Ty-Thin -> Dan, Mao, Thin
        2 => matches!(t_branch, 8..=10),     // Dan-Ngo-Tuat -> Than, Dau, Tuat
        1 => matches!(t_branch, 11 | 0 | 1), // Ty_ran-Dau-Suu -> Hoi, Ty, Suu
        _ => matches!(t_branch, 5..=7),      // Hoi-Mao-Mui -> Ty_ran, Ngo, Mui
    };

    let (kim_lau, kim_lau_type) = match effective_age % 9 {
        1 => (
            true,
            Some(BilingualLabel::new(
                "kim_lau_than",
                "Kim Lâu Thân",
                "Affects Self",
            )),
        ),
        3 => (
            true,
            Some(BilingualLabel::new(
                "kim_lau_the",
                "Kim Lâu Thê",
                "Affects Spouse",
            )),
        ),
        6 => (
            true,
            Some(BilingualLabel::new(
                "kim_lau_tu",
                "Kim Lâu Tử",
                "Affects Children",
            )),
        ),
        8 => (
            true,
            Some(BilingualLabel::new(
                "kim_lau_suc",
                "Kim Lâu Súc",
                "Affects Assets/Livestock",
            )),
        ),
        _ => (false, None),
    };

    let tens = (effective_age / 10).max(1);
    let ones = effective_age % 10;
    let ho_idx = ((tens - 1 + ones).rem_euclid(6)) as usize;
    let (hoang_oc, hoang_oc_is_bad) = HOANG_OC_STAGES.get(ho_idx).map_or_else(
        || {
            (
                BilingualLabel::new("nhat_cat", "Nhất Cát", "Optimal"),
                false,
            )
        },
        |&(id, vi, en, bad)| (BilingualLabel::new(id, vi, en), bad),
    );

    let xung_tuoi_day = (b_branch + 6) % 12 == day_branch_idx % 12;

    let mut notes = Vec::new();
    if tam_tai {
        notes.push(format!(
            "{birth_year} faces Tam Tai during target lunar year {target_lunar_year}"
        ));
    }
    if let Some(ref kl) = kim_lau_type {
        notes.push(format!("Lunar age {effective_age} falls in {}", kl.name_vi));
    }
    if hoang_oc_is_bad {
        notes.push(format!(
            "Lunar age {effective_age} hits unfavorable {}",
            hoang_oc.name_vi
        ));
    }
    if xung_tuoi_day {
        notes.push("Birth year branch directly clashes (Lục Xung) with day branch".to_string());
    }

    Ok(AgeConflictCheck {
        birth_year,
        year_can_chi: birth_cc,
        lunar_age,
        tam_tai,
        kim_lau,
        kim_lau_type,
        hoang_oc,
        hoang_oc_is_bad,
        xung_tuoi_day,
        notes,
    })
}

/// Evaluates all Vietnamese Trạch Nhật rules for a precomputed `CanChiBreakdown` and `jd`.
///
/// # Errors
/// Returns [`CalendarError`] when any birth year is non-positive or Can Chi bounds fail.
pub(crate) fn evaluate_day(
    canchi: &dyn CanChiSolarService,
    jd: i32,
    breakdown: &CanChiBreakdown,
    occasion: Occasion,
    owner_birth_year: Option<i32>,
    partner_birth_year: Option<i32>,
) -> Result<TrachNhatDayAnalysis, CalendarError> {
    let day_branch = usize::from(breakdown.day_can_chi.branch_index) % 12;
    let bazi_month_branch = usize::from(breakdown.bazi_month_can_chi.branch_index) % 12;

    // 1. Day Hoang Dao
    let day_start = thanh_long_start(bazi_month_branch);
    let (day_deity, is_hoang_dao_day) = spirit_at(day_start, day_branch);

    // 2. 12 Truc
    let truc_idx = (day_branch as i32 - bazi_month_branch as i32).rem_euclid(12) as usize;
    let (truc, truc_quality) = TWELVE_TRUC.get(truc_idx).map_or_else(
        || {
            (
                BilingualLabel::new("kien", "Trực Kiến", "Establish"),
                "neutral".to_string(),
            )
        },
        |&(id, vi, en, q)| (BilingualLabel::new(id, vi, en), q.to_string()),
    );

    // 3. 28 Tu
    let tu_idx = (jd + 4).rem_euclid(28) as usize;
    let (tu_star, tu_is_auspicious) = TWENTY_EIGHT_TU.get(tu_idx).map_or_else(
        || (BilingualLabel::new("giac", "Sao Giác", "Horn"), true),
        |&(id, vi, en, good)| (BilingualLabel::new(id, vi, en), good),
    );

    // 4. 12 Hour Fortunes (Ty..Hoi using Ngu Thu Don on non-shifted day stem)
    let hour_start = thanh_long_start(day_branch);
    let mut hours = Vec::with_capacity(12);
    for h_idx in 0..12usize {
        let test_hour = if h_idx == 0 { 0u8 } else { (h_idx * 2) as u8 };
        let (hour_cc, _) = crate::plugins::canchi_solar_vn::canchi::pair_for_hour(
            breakdown.day_can_chi.stem_index,
            test_hour,
        )?;
        let (deity, is_hd) = spirit_at(hour_start, h_idx);
        hours.push(HourFortune {
            branch_index: h_idx as u8,
            branch: branch_label(h_idx),
            can_chi: hour_cc,
            time_range: HOUR_RANGES
                .get(h_idx)
                .copied()
                .unwrap_or("00:00-02:00")
                .to_string(),
            deity,
            is_hoang_dao: is_hd,
        });
    }

    // 5. Major Lunar Taboos
    let lunar = &breakdown.lunar;
    let mut taboos = Vec::new();
    if matches!(lunar.day, 3 | 7 | 13 | 18 | 22 | 27) {
        taboos.push(TabooAlert {
            id: "tam_nuong".to_string(),
            name_vi: "Ngày Tam Nương".to_string(),
            name_en: "Tam Nuong Taboo Day".to_string(),
            severity: "major".to_string(),
            reason: format!(
                "Lunar day {} belongs to Tam Nương [3,7,13,18,22,27]",
                lunar.day
            ),
        });
    }
    if matches!(lunar.day, 5 | 14 | 23) {
        taboos.push(TabooAlert {
            id: "nguyet_ky".to_string(),
            name_vi: "Ngày Nguyệt Kỵ".to_string(),
            name_en: "Nguyet Ky Taboo Day".to_string(),
            severity: "major".to_string(),
            reason: format!("Lunar day {} belongs to Nguyệt Kỵ [5,14,23]", lunar.day),
        });
    }
    if DUONG_CONG_KY_NHAT.contains(&(lunar.month, lunar.day)) {
        taboos.push(TabooAlert {
            id: "duong_cong_ky_nhat".to_string(),
            name_vi: "Dương Công Kỵ Nhật".to_string(),
            name_en: "Yang Gong Inauspicious Day".to_string(),
            severity: "major".to_string(),
            reason: format!(
                "Lunar date {:02}/{:02} matches Dương Công Kỵ Nhật",
                lunar.day, lunar.month
            ),
        });
    }
    if let Some(&sc_branch) = lunar
        .month
        .checked_sub(1)
        .and_then(|m| SAT_CHU_BRANCH.get(m as usize))
    {
        if sc_branch == day_branch {
            taboos.push(TabooAlert {
                id: "sat_chu".to_string(),
                name_vi: "Ngày Sát Chủ".to_string(),
                name_en: "Sat Chu Taboo Day".to_string(),
                severity: "major".to_string(),
                reason: format!(
                    "Day branch clashes as Sát Chủ for lunar month {}",
                    lunar.month
                ),
            });
        }
    }
    if let Some(&tt_branch) = lunar
        .month
        .checked_sub(1)
        .and_then(|m| THO_TU_BRANCH.get(m as usize))
    {
        if tt_branch == day_branch {
            taboos.push(TabooAlert {
                id: "tho_tu".to_string(),
                name_vi: "Ngày Thọ Tử".to_string(),
                name_en: "Tho Tu Taboo Day".to_string(),
                severity: "major".to_string(),
                reason: format!("Day branch falls on Thọ Tử for lunar month {}", lunar.month),
            });
        }
    }

    // 6. Personal Age Checks
    let mut age_checks = Vec::new();
    if let Some(by) = owner_birth_year {
        age_checks.push(eval_age(canchi, by, lunar.year, day_branch)?);
    }
    if let Some(py) = partner_birth_year {
        age_checks.push(eval_age(canchi, py, lunar.year, day_branch)?);
    }

    // 7. Deterministic Scoring
    let mut score: i32 = 55;
    let mut reasons = Vec::new();

    if is_hoang_dao_day {
        score += 20;
        reasons.push(format!("+20 Hoàng Đạo day ({})", day_deity.name_vi));
    } else {
        score -= 10;
        reasons.push(format!("-10 Hắc Đạo day ({})", day_deity.name_vi));
    }

    match truc_quality.as_str() {
        "auspicious" => {
            score += 15;
            reasons.push(format!("+15 Favorable {}", truc.name_vi));
        }
        "inauspicious" => {
            score -= 18;
            reasons.push(format!("-18 Unfavorable {}", truc.name_vi));
        }
        _ => reasons.push(format!("+0 Neutral {}", truc.name_vi)),
    }

    if tu_is_auspicious {
        score += 10;
        reasons.push(format!("+10 Auspicious {}", tu_star.name_vi));
    } else {
        score -= 8;
        reasons.push(format!("-8 Inauspicious {}", tu_star.name_vi));
    }

    // Occasion-specific Truc affinities
    match occasion {
        Occasion::Wedding if matches!(truc.id.as_str(), "thanh" | "dinh" | "man") => {
            score += 6;
            reasons.push("+6 Truc aligns well with Wedding".to_string());
        }
        Occasion::Groundbreaking
            if matches!(truc.id.as_str(), "dinh" | "thanh" | "khai" | "kien") =>
        {
            score += 6;
            reasons.push("+6 Truc aligns well with Groundbreaking".to_string());
        }
        Occasion::ContractSigning | Occasion::GrandOpening
            if matches!(truc.id.as_str(), "khai" | "thanh" | "man") =>
        {
            score += 6;
            reasons.push("+6 Truc aligns well with Commerce/Contracts".to_string());
        }
        Occasion::MovingIn if matches!(truc.id.as_str(), "khai" | "thanh" | "dinh" | "man") => {
            score += 6;
            reasons.push("+6 Truc aligns well with Moving In".to_string());
        }
        _ => {}
    }

    for taboo in &taboos {
        score -= 18;
        reasons.push(format!("-18 Taboo: {}", taboo.name_vi));
    }

    for age in &age_checks {
        if age.xung_tuoi_day {
            score -= 22;
            reasons.push(format!(
                "-22 Day branch clashes with birth year {}",
                age.birth_year
            ));
        }
        if matches!(
            occasion,
            Occasion::Groundbreaking | Occasion::MovingIn | Occasion::Wedding
        ) {
            if age.tam_tai {
                score -= 10;
                reasons.push(format!(
                    "-10 Tam Tai year for birth year {}",
                    age.birth_year
                ));
            }
            if age.kim_lau {
                score -= 10;
                reasons.push(format!("-10 Kim Lâu age for birth year {}", age.birth_year));
            }
            if matches!(occasion, Occasion::Groundbreaking | Occasion::MovingIn)
                && age.hoang_oc_is_bad
            {
                score -= 8;
                reasons.push(format!(
                    "-8 Hoang Ốc {} for {}",
                    age.hoang_oc.name_vi, age.birth_year
                ));
            }
        }
    }

    let final_score = score.clamp(0, 100);
    let rating = match final_score {
        80..=100 => BilingualLabel::new("rat_tot", "Rất tốt (Đại Cát)", "Highly Auspicious"),
        60..=79 => BilingualLabel::new("tot", "Tốt (Cát)", "Auspicious"),
        40..=59 => BilingualLabel::new("trung_binh", "Trung bình", "Moderate / Neutral"),
        _ => BilingualLabel::new("nen_tranh", "Nên tránh (Hung)", "Inauspicious / Avoid"),
    };

    Ok(TrachNhatDayAnalysis {
        solar_date: breakdown.solar_date.clone(),
        lunar: breakdown.lunar.clone(),
        can_chi: breakdown.clone(),
        occasion,
        day_deity,
        is_hoang_dao_day,
        truc,
        truc_quality,
        tu_star,
        tu_is_auspicious,
        hours,
        taboos,
        age_checks,
        score: final_score,
        rating,
        reasons,
    })
}

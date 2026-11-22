//! Can Chi sexagenary cycle, Ngũ Hổ Độn, Ngũ Thử Độn, and 30 Nạp Âm calculations.

use crate::contracts::canchi::{BilingualLabel, CanChiPair};
use crate::contracts::models::CalendarError;

struct StaticLabel {
    id: &'static str,
    vi: &'static str,
    en: &'static str,
    elem_idx: usize,
}

const ELEMENTS: &[(&str, &str, &str)] = &[
    ("kim", "Kim", "Metal"),
    ("moc", "Mộc", "Wood"),
    ("thuy", "Thủy", "Water"),
    ("hoa", "Hỏa", "Fire"),
    ("tho", "Thổ", "Earth"),
];

const STEMS: &[StaticLabel] = &[
    StaticLabel {
        id: "giap",
        vi: "Giáp",
        en: "Jia (Yang Wood)",
        elem_idx: 1,
    },
    StaticLabel {
        id: "at",
        vi: "Ất",
        en: "Yi (Yin Wood)",
        elem_idx: 1,
    },
    StaticLabel {
        id: "binh",
        vi: "Bính",
        en: "Bing (Yang Fire)",
        elem_idx: 3,
    },
    StaticLabel {
        id: "dinh",
        vi: "Đinh",
        en: "Ding (Yin Fire)",
        elem_idx: 3,
    },
    StaticLabel {
        id: "mau",
        vi: "Mậu",
        en: "Wu (Yang Earth)",
        elem_idx: 4,
    },
    StaticLabel {
        id: "ky",
        vi: "Kỷ",
        en: "Ji (Yin Earth)",
        elem_idx: 4,
    },
    StaticLabel {
        id: "canh",
        vi: "Canh",
        en: "Geng (Yang Metal)",
        elem_idx: 0,
    },
    StaticLabel {
        id: "tan",
        vi: "Tân",
        en: "Xin (Yin Metal)",
        elem_idx: 0,
    },
    StaticLabel {
        id: "nham",
        vi: "Nhâm",
        en: "Ren (Yang Water)",
        elem_idx: 2,
    },
    StaticLabel {
        id: "quy",
        vi: "Quý",
        en: "Gui (Yin Water)",
        elem_idx: 2,
    },
];

const BRANCHES: &[StaticLabel] = &[
    StaticLabel {
        id: "ty",
        vi: "Tý",
        en: "Rat",
        elem_idx: 2,
    },
    StaticLabel {
        id: "suu",
        vi: "Sửu",
        en: "Ox",
        elem_idx: 4,
    },
    StaticLabel {
        id: "dan",
        vi: "Dần",
        en: "Tiger",
        elem_idx: 1,
    },
    StaticLabel {
        id: "mao",
        vi: "Mão",
        en: "Rabbit",
        elem_idx: 1,
    },
    StaticLabel {
        id: "thin",
        vi: "Thìn",
        en: "Dragon",
        elem_idx: 4,
    },
    StaticLabel {
        id: "ty_ran",
        vi: "Tỵ",
        en: "Snake",
        elem_idx: 3,
    },
    StaticLabel {
        id: "ngo",
        vi: "Ngọ",
        en: "Horse",
        elem_idx: 3,
    },
    StaticLabel {
        id: "mui",
        vi: "Mùi",
        en: "Goat",
        elem_idx: 4,
    },
    StaticLabel {
        id: "than",
        vi: "Thân",
        en: "Monkey",
        elem_idx: 0,
    },
    StaticLabel {
        id: "dau",
        vi: "Dậu",
        en: "Rooster",
        elem_idx: 0,
    },
    StaticLabel {
        id: "tuat",
        vi: "Tuất",
        en: "Dog",
        elem_idx: 4,
    },
    StaticLabel {
        id: "hoi",
        vi: "Hợi",
        en: "Pig",
        elem_idx: 2,
    },
];

const BRANCH_EN_ANIMALS: &[&str] = &[
    "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey", "Rooster", "Dog",
    "Pig",
];

const STEM_EN_PREFIX: &[&str] = &[
    "Yang Wood",
    "Yin Wood",
    "Yang Fire",
    "Yin Fire",
    "Yang Earth",
    "Yin Earth",
    "Yang Metal",
    "Yin Metal",
    "Yang Water",
    "Yin Water",
];

// Branch slugs without _ran suffix for combined ASCII id compatibility (e.g. at_ty, quy_ty).
const BRANCH_COMBINED_ID: &[&str] = &[
    "ty", "suu", "dan", "mao", "thin", "ty", "ngo", "mui", "than", "dau", "tuat", "hoi",
];

const NAP_AM: &[StaticLabel] = &[
    StaticLabel {
        id: "hai_trung_kim",
        vi: "Hải Trung Kim",
        en: "Gold in the Sea",
        elem_idx: 0,
    },
    StaticLabel {
        id: "lu_trung_hoa",
        vi: "Lư Trung Hỏa",
        en: "Fire in the Furnace",
        elem_idx: 3,
    },
    StaticLabel {
        id: "dai_lam_moc",
        vi: "Đại Lâm Mộc",
        en: "Great Forest Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "lo_bang_tho",
        vi: "Lộ Bàng Thổ",
        en: "Roadside Earth",
        elem_idx: 4,
    },
    StaticLabel {
        id: "kiem_phong_kim",
        vi: "Kiếm Phong Kim",
        en: "Sword-Edge Metal",
        elem_idx: 0,
    },
    StaticLabel {
        id: "son_dau_hoa",
        vi: "Sơn Đầu Hỏa",
        en: "Mountain-Top Fire",
        elem_idx: 3,
    },
    StaticLabel {
        id: "gian_ha_thuy",
        vi: "Giản Hạ Thủy",
        en: "Stream Water",
        elem_idx: 2,
    },
    StaticLabel {
        id: "thanh_dau_tho",
        vi: "Thành Đầu Thổ",
        en: "City-Wall Earth",
        elem_idx: 4,
    },
    StaticLabel {
        id: "bach_lap_kim",
        vi: "Bạch Lạp Kim",
        en: "White Wax Metal",
        elem_idx: 0,
    },
    StaticLabel {
        id: "duong_lieu_moc",
        vi: "Dương Liễu Mộc",
        en: "Willow Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "tuyen_trung_thuy",
        vi: "Tuyền Trung Thủy",
        en: "Spring Water",
        elem_idx: 2,
    },
    StaticLabel {
        id: "oc_thuong_tho",
        vi: "Ốc Thượng Thổ",
        en: "Rooftop Earth",
        elem_idx: 4,
    },
    StaticLabel {
        id: "tich_lich_hoa",
        vi: "Tích Lịch Hỏa",
        en: "Thunderbolt Fire",
        elem_idx: 3,
    },
    StaticLabel {
        id: "tung_bach_moc",
        vi: "Tùng Bách Mộc",
        en: "Pine and Cypress Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "truong_luu_thuy",
        vi: "Trường Lưu Thủy",
        en: "Long Flowing Water",
        elem_idx: 2,
    },
    StaticLabel {
        id: "sa_trung_kim",
        vi: "Sa Trung Kim",
        en: "Gold in Sand",
        elem_idx: 0,
    },
    StaticLabel {
        id: "son_ha_hoa",
        vi: "Sơn Hạ Hỏa",
        en: "Foothill Fire",
        elem_idx: 3,
    },
    StaticLabel {
        id: "binh_dia_moc",
        vi: "Bình Địa Mộc",
        en: "Flatland Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "bich_thuong_tho",
        vi: "Bích Thượng Thổ",
        en: "Wall Earth",
        elem_idx: 4,
    },
    StaticLabel {
        id: "kim_bach_kim",
        vi: "Kim Bạch Kim",
        en: "Gold Leaf Metal",
        elem_idx: 0,
    },
    StaticLabel {
        id: "phu_dang_hoa",
        vi: "Phú Đăng Hỏa",
        en: "Lantern Flame Fire",
        elem_idx: 3,
    },
    StaticLabel {
        id: "thien_ha_thuy",
        vi: "Thiên Hà Thủy",
        en: "Milky Way Water",
        elem_idx: 2,
    },
    StaticLabel {
        id: "dai_trach_tho",
        vi: "Đại Trạch Thổ",
        en: "Great Marsh Earth",
        elem_idx: 4,
    },
    StaticLabel {
        id: "thoa_xuyen_kim",
        vi: "Thoa Xuyến Kim",
        en: "Hairpin Gold",
        elem_idx: 0,
    },
    StaticLabel {
        id: "tang_do_moc",
        vi: "Tang Đố Mộc",
        en: "Mulberry Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "dai_khe_thuy",
        vi: "Đại Khê Thủy",
        en: "Great Stream Water",
        elem_idx: 2,
    },
    StaticLabel {
        id: "sa_trung_tho",
        vi: "Sa Trung Thổ",
        en: "Earth in Sand",
        elem_idx: 4,
    },
    StaticLabel {
        id: "thien_thuong_hoa",
        vi: "Thiên Thượng Hỏa",
        en: "Heavenly Fire",
        elem_idx: 3,
    },
    StaticLabel {
        id: "thach_luu_moc",
        vi: "Thạch Lựu Mộc",
        en: "Pomegranate Wood",
        elem_idx: 1,
    },
    StaticLabel {
        id: "dai_hai_thuy",
        vi: "Đại Hải Thủy",
        en: "Great Ocean Water",
        elem_idx: 2,
    },
];

/// Returns bilingual Five Element label from canonical element index (`0..5`).
pub(crate) fn element_label(elem_idx: usize) -> BilingualLabel {
    ELEMENTS.get(elem_idx % 5).map_or_else(
        || BilingualLabel::new("kim", "Kim", "Metal"),
        |(id, vi, en)| BilingualLabel::new(*id, *vi, *en),
    )
}

/// Returns bilingual Heavenly Stem label (`0..10`).
pub(crate) fn stem_label(stem_idx: usize) -> BilingualLabel {
    STEMS.get(stem_idx).map_or_else(
        || BilingualLabel::new("giap", "Giáp", "Jia (Yang Wood)"),
        |s| BilingualLabel::new(s.id, s.vi, s.en),
    )
}

/// Returns the Five Element index (`0..5`) of a Heavenly Stem (`0..10`).
pub(crate) fn stem_element_index(stem_idx: usize) -> usize {
    STEMS.get(stem_idx).map_or(1, |s| s.elem_idx)
}

/// Returns bilingual Earthly Branch label (`0..12`).
pub(crate) fn branch_label(branch_idx: usize) -> BilingualLabel {
    BRANCHES.get(branch_idx).map_or_else(
        || BilingualLabel::new("ty", "Tý", "Rat"),
        |b| BilingualLabel::new(b.id, b.vi, b.en),
    )
}

/// Builds a [`CanChiPair`] from normalized `(stem_index 0..10, branch_index 0..12)`.
pub(crate) fn pair_from_stem_branch(stem_index: usize, branch_index: usize) -> CanChiPair {
    let s = stem_index % 10;
    let b = branch_index % 12;
    let sexagenary = if s % 2 == b % 2 {
        (s as i32 * 6 - b as i32 * 5).rem_euclid(60) as usize
    } else {
        s
    };
    build_pair(s, b, sexagenary)
}

fn build_pair(s: usize, b: usize, sexagenary: usize) -> CanChiPair {
    let fallback_stem = &StaticLabel {
        id: "giap",
        vi: "Giáp",
        en: "Jia (Yang Wood)",
        elem_idx: 1,
    };
    let fallback_branch = &StaticLabel {
        id: "ty",
        vi: "Tý",
        en: "Rat",
        elem_idx: 2,
    };
    let fallback_nap = &StaticLabel {
        id: "hai_trung_kim",
        vi: "Hải Trung Kim",
        en: "Gold in the Sea",
        elem_idx: 0,
    };
    let stem_meta = STEMS
        .get(s)
        .or_else(|| STEMS.first())
        .unwrap_or(fallback_stem);
    let branch_meta = BRANCHES
        .get(b)
        .or_else(|| BRANCHES.first())
        .unwrap_or(fallback_branch);
    let branch_slug = BRANCH_COMBINED_ID.get(b).copied().unwrap_or("ty");
    let en_prefix = STEM_EN_PREFIX.get(s).copied().unwrap_or("Yang Wood");
    let en_animal = BRANCH_EN_ANIMALS.get(b).copied().unwrap_or("Rat");
    let nap_meta = NAP_AM
        .get((sexagenary / 2) % 30)
        .or_else(|| NAP_AM.first())
        .unwrap_or(fallback_nap);

    CanChiPair {
        stem_index: s as u8,
        branch_index: b as u8,
        sexagenary_index: sexagenary as u8,
        id: format!("{}_{}", stem_meta.id, branch_slug),
        name_vi: format!("{} {}", stem_meta.vi, branch_meta.vi),
        name_en: format!("{en_prefix} {en_animal}"),
        stem: BilingualLabel::new(stem_meta.id, stem_meta.vi, stem_meta.en),
        branch: BilingualLabel::new(branch_meta.id, branch_meta.vi, branch_meta.en),
        stem_element: element_label(stem_meta.elem_idx),
        nap_am: BilingualLabel::new(nap_meta.id, nap_meta.vi, nap_meta.en),
        nap_am_element: element_label(nap_meta.elem_idx),
    }
}

/// Returns a validated [`CanChiPair`] from sexagenary index (`0..60`).
pub(crate) fn pair_from_sexagenary_index(index: usize) -> CanChiPair {
    let idx = index % 60;
    build_pair(idx % 10, idx % 12, idx)
}

/// Computes day Can Chi pair from integer Julian Day Number (`2000-01-01` = `2451545` = `Mậu Ngọ`).
pub(crate) fn pair_from_jd(jd: i32) -> CanChiPair {
    let idx = (jd + 49).rem_euclid(60) as usize;
    pair_from_sexagenary_index(idx)
}

/// Computes year Can Chi pair (`1984 = Giáp Tý`, `2024 = Giáp Thìn`).
pub(crate) fn pair_for_year(year: i32) -> CanChiPair {
    let idx = (year - 4).rem_euclid(60) as usize;
    pair_from_sexagenary_index(idx)
}

/// Computes month Can Chi pair via *Ngũ Hổ Độn* (`month` in `1..=12`).
///
/// # Errors
/// Returns [`CalendarError::InvalidLunarMonth`] when `month` is outside `1..=12`.
pub(crate) fn pair_for_month(year_stem_index: u8, month: i32) -> Result<CanChiPair, CalendarError> {
    if !(1..=12).contains(&month) {
        return Err(CalendarError::InvalidLunarMonth(month));
    }
    let y_stem = usize::from(year_stem_index);
    let month_offset = (month - 1) as usize;
    let stem = y_stem * 2 + 2 + month_offset;
    let branch = 2 + month_offset;
    Ok(pair_from_stem_branch(stem, branch))
}

/// Computes hour Can Chi pair via *Ngũ Thử Độn* and reports `23:00` Giờ Tý day-stem rollover.
///
/// # Errors
/// Returns [`CalendarError::InvalidSolarDate`] when `hour >= 24`.
pub(crate) fn pair_for_hour(
    day_stem_index: u8,
    hour: u8,
) -> Result<(CanChiPair, bool), CalendarError> {
    if hour >= 24 {
        return Err(CalendarError::InvalidSolarDate(format!(
            "hour {hour} is out of valid 0..=23 range"
        )));
    }
    let hour_shifted = hour == 23;
    let eff_day_stem = if hour_shifted {
        usize::from(day_stem_index) + 1
    } else {
        usize::from(day_stem_index)
    };
    let hour_branch = usize::from(hour.div_ceil(2)) % 12;
    let hour_stem = eff_day_stem * 2 + hour_branch;
    Ok((pair_from_stem_branch(hour_stem, hour_branch), hour_shifted))
}

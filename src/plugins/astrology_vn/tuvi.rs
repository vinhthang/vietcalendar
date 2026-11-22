//! Vietnamese Tử Vi (Nam Phái Core) 12-Palace star chart builder.

use std::collections::HashMap;

use crate::contracts::astrology::{Gender, TuHoaMapping, TuViChart, TuViPalace, TuViStar};
use crate::contracts::canchi::{BilingualLabel, CanChiBreakdown, CanChiSolarService};
use crate::contracts::models::CalendarError;
use crate::plugins::canchi_solar_vn::canchi::{branch_label, pair_from_stem_branch};

const PALACE_NAMES: &[(&str, &str, &str)] = &[
    ("menh", "Mệnh", "Life (Ming)"),
    ("phu_mau", "Phụ Mẫu", "Parents"),
    ("phuc_duc", "Phúc Đức", "Fortune & Virtue"),
    ("dien_trach", "Điền Trạch", "Property"),
    ("quan_loc", "Quan Lộc", "Career"),
    ("no_boc", "Nô Bộc", "Friends & Peers"),
    ("thien_di", "Thiên Di", "Travel"),
    ("tat_ach", "Tật Ách", "Health"),
    ("tai_bach", "Tài Bạch", "Wealth"),
    ("tu_tuc", "Tử Tức", "Children"),
    ("phu_the", "Phu Thê", "Spouse"),
    ("huynh_de", "Huynh Đệ", "Siblings"),
];

const CHINH_TINH_NAMES: &[(&str, &str, &str)] = &[
    ("tu_vi", "Tử Vi", "Emperor Star (Zi Wei)"),
    ("thien_co", "Thiên Cơ", "Heavenly Secret"),
    ("thai_duong", "Thái Dương", "The Sun"),
    ("vu_khuc", "Vũ Khúc", "Finance & Martial"),
    ("thien_dong", "Thiên Đồng", "Childlike Fortune"),
    ("liem_trinh", "Liêm Trinh", "Judge / Integrity"),
    ("thien_phu", "Thiên Phủ", "Southern Treasury"),
    ("thai_am", "Thái Âm", "The Moon"),
    ("tham_lang", "Tham Lang", "Greedy Wolf"),
    ("cu_mon", "Cự Môn", "Dark Gate / Speech"),
    ("thien_tuong", "Thiên Tướng", "Grand Seal Minister"),
    ("thien_luong", "Thiên Lương", "Heavenly Pillar"),
    ("that_sat", "Thất Sát", "Seven Killings"),
    ("pha_quan", "Phá Quân", "Army Breaker"),
];

const THAI_TUE_RING: &[(&str, &str, &str)] = &[
    ("thai_tue", "Thái Tuế", "Grand Duke"),
    ("thieu_duong", "Thiếu Dương", "Lesser Yang"),
    ("tang_mon", "Tang Môn", "Mourning Gate"),
    ("thieu_am", "Thiếu Âm", "Lesser Yin"),
    ("quan_phu_tt", "Quan Phù", "Official Charm"),
    ("tu_phu", "Tử Phù", "Death Charm"),
    ("tue_pha", "Tuế Phá", "Year Breaker"),
    ("long_duc", "Long Đức", "Dragon Virtue"),
    ("bach_ho", "Bạch Hổ", "White Tiger"),
    ("phuc_duc_tt", "Phúc Đức", "Virtue Blessing"),
    ("dieu_khach", "Điếu Khách", "Mourning Guest"),
    ("truc_phu", "Trực Phù", "Direct Charm"),
];

const TRUONG_SINH_RING: &[(&str, &str, &str)] = &[
    ("truong_sinh", "Trường Sinh", "Longevity Birth"),
    ("moc_duc", "Mộc Dục", "Lustral Bath"),
    ("quan_doi", "Quan Đới", "Maturity Crown"),
    ("lam_quan", "Lâm Quan", "Official Office"),
    ("de_vuong", "Đế Vượng", "Imperial Zenith"),
    ("suy", "Suy", "Slow Decline"),
    ("benh", "Bệnh", "Ailment"),
    ("tu_tv", "Tử", "Dormancy"),
    ("mo_tv", "Mộ", "Vault Treasury"),
    ("tuyet", "Tuyệt", "Void Transition"),
    ("thai", "Thai", "Germination"),
    ("duong", "Dưỡng", "Nurturing"),
];

// Brightness table (14 stars x 12 branches: Ty..Hoi), condensed Nam Phai standard levels.
const CHINH_TINH_BRIGHTNESS: &[&[&str]] = &[
    // tu_vi
    &["B", "Đ", "M", "V", "V", "M", "M", "Đ", "V", "B", "V", "B"],
    // thien_co
    &["Đ", "H", "V", "M", "V", "B", "M", "H", "Đ", "V", "Đ", "B"],
    // thai_duong
    &["H", "H", "V", "M", "M", "M", "M", "Đ", "B", "H", "H", "H"],
    // vu_khuc
    &["V", "M", "Đ", "H", "M", "B", "V", "M", "Đ", "B", "M", "B"],
    // thien_dong
    &["M", "H", "Đ", "M", "B", "M", "H", "B", "V", "H", "H", "M"],
    // liem_trinh
    &["B", "Đ", "M", "B", "V", "H", "B", "Đ", "M", "B", "V", "H"],
    // thien_phu
    &["M", "M", "M", "B", "V", "Đ", "V", "M", "M", "B", "V", "Đ"],
    // thai_am
    &["M", "Đ", "H", "H", "H", "H", "H", "B", "V", "M", "M", "M"],
    // tham_lang
    &["V", "M", "B", "Đ", "M", "H", "V", "M", "B", "Đ", "M", "H"],
    // cu_mon
    &["M", "Đ", "M", "M", "B", "B", "V", "H", "V", "M", "H", "V"],
    // thien_tuong
    &["V", "Đ", "M", "H", "Đ", "B", "V", "Đ", "M", "H", "B", "B"],
    // thien_luong
    &["M", "V", "M", "M", "V", "H", "M", "V", "H", "B", "H", "H"],
    // that_sat
    &["M", "Đ", "M", "H", "B", "B", "M", "Đ", "M", "B", "V", "B"],
    // pha_quan
    &["M", "V", "H", "H", "Đ", "B", "M", "Đ", "H", "B", "Đ", "B"],
];

// Tu Hoa per stem: (loc, quyen, khoa, ky) star IDs
const TU_HOA_BY_STEM: &[(&str, &str, &str, &str)] = &[
    ("liem_trinh", "pha_quan", "vu_khuc", "thai_duong"), // Giap
    ("thien_co", "thien_luong", "tu_vi", "thai_am"),     // At
    ("thien_dong", "thien_co", "van_xuong", "liem_trinh"), // Binh
    ("thai_am", "thien_dong", "thien_co", "cu_mon"),     // Dinh
    ("tham_lang", "thai_am", "huu_bat", "thien_co"),     // Mau
    ("vu_khuc", "tham_lang", "thien_luong", "van_khuc"), // Ky
    ("thai_duong", "vu_khuc", "thai_am", "thien_dong"),  // Canh
    ("cu_mon", "thai_duong", "van_khuc", "van_xuong"),   // Tan
    ("thien_luong", "tu_vi", "ta_phu", "vu_khuc"),       // Nham
    ("pha_quan", "cu_mon", "thai_am", "tham_lang"),      // Quy
];

const TU_HOA_LABELS: &[(&str, &str, &str)] = &[
    ("hoa_loc", "Hóa Lộc", "Auspicious Wealth (Hua Lu)"),
    ("hoa_quyen", "Hóa Quyền", "Authority Power (Hua Quan)"),
    ("hoa_khoa", "Hóa Khoa", "Academic Grace (Hua Ke)"),
    ("hoa_ky", "Hóa Kỵ", "Obstruction Mist (Hua Ji)"),
];

fn cuc_from_elem_id(elem_id: &str) -> (u8, BilingualLabel) {
    match elem_id {
        "thuy" => (
            2,
            BilingualLabel::new("thuy_nhi_cuc", "Thủy Nhị Cục", "Water 2nd Config"),
        ),
        "moc" => (
            3,
            BilingualLabel::new("moc_tam_cuc", "Mộc Tam Cục", "Wood 3rd Config"),
        ),
        "kim" => (
            4,
            BilingualLabel::new("kim_tu_cuc", "Kim Tứ Cục", "Metal 4th Config"),
        ),
        "tho" => (
            5,
            BilingualLabel::new("tho_ngu_cuc", "Thổ Ngũ Cục", "Earth 5th Config"),
        ),
        _ => (
            6,
            BilingualLabel::new("hoa_luc_cuc", "Hỏa Lục Cục", "Fire 6th Config"),
        ),
    }
}

fn compute_tu_vi_branch(cuc_num: u8, lunar_day: i32) -> usize {
    let cuc = i32::from(cuc_num);
    let rem = lunar_day % cuc;
    if rem == 0 {
        let q = lunar_day / cuc;
        (2 + q - 1).rem_euclid(12) as usize
    } else {
        let k = cuc - rem;
        let q = (lunar_day + k) / cuc;
        let base = 2 + q - 1;
        if k % 2 == 1 {
            (base - k).rem_euclid(12) as usize
        } else {
            (base + k).rem_euclid(12) as usize
        }
    }
}

struct StarEntry<'a> {
    id: &'a str,
    vi: &'a str,
    en: &'a str,
    category: &'a str,
    brightness: Option<&'a str>,
}

fn push_star(
    buckets: &mut [Vec<TuViStar>; 12],
    branch: usize,
    star: StarEntry<'_>,
    tu_hoa_map: &HashMap<&str, BilingualLabel>,
) {
    if let Some(bucket) = buckets.get_mut(branch % 12) {
        bucket.push(TuViStar {
            id: star.id.to_string(),
            name_vi: star.vi.to_string(),
            name_en: star.en.to_string(),
            category: star.category.to_string(),
            brightness: star.brightness.map(str::to_string),
            tu_hoa: tu_hoa_map.get(star.id).cloned(),
        });
    }
}

/// Builds a complete Nam Phái Core Tử Vi chart across 12 palaces.
///
/// # Errors
/// Returns [`CalendarError`] when lunar day/month values fall outside valid 1..=30 / 1..=12 bounds.
pub(crate) fn build_tuvi_chart(
    _canchi: &dyn CanChiSolarService,
    gender: Gender,
    breakdown: &CanChiBreakdown,
) -> Result<TuViChart, CalendarError> {
    let lunar = &breakdown.lunar;
    if !(1..=12).contains(&lunar.month) {
        return Err(CalendarError::InvalidLunarMonth(lunar.month));
    }
    if !(1..=30).contains(&lunar.day) {
        return Err(CalendarError::InvalidLunarDay {
            day: lunar.day,
            month: lunar.month,
            year: lunar.year,
            max_days: 30,
        });
    }

    let (tuvi_month, leap_shifted) = if lunar.is_leap && lunar.day > 15 {
        let next_m = if lunar.month == 12 {
            1
        } else {
            lunar.month + 1
        };
        (next_m, true)
    } else {
        (lunar.month, false)
    };

    let hb = usize::from(breakdown.hour_can_chi.branch_index) % 12;
    let ys = usize::from(breakdown.lunar_year_can_chi.stem_index) % 10;
    let yb = usize::from(breakdown.lunar_year_can_chi.branch_index) % 12;

    let menh = (2 + (tuvi_month - 1) - hb as i32).rem_euclid(12) as usize;
    let than = (2 + (tuvi_month - 1) + hb as i32).rem_euclid(12) as usize;

    let dan_stem = ((ys % 5) * 2 + 2) % 10;
    let menh_offset = (menh as i32 - 2).rem_euclid(12) as usize;
    let menh_palace_cc = pair_from_stem_branch((dan_stem + menh_offset) % 10, menh);
    let (cuc_number, cuc) = cuc_from_elem_id(&menh_palace_cc.nap_am_element.id);

    let yang_year = ys % 2 == 0;
    let forward = (yang_year && gender == Gender::Male) || (!yang_year && gender == Gender::Female);

    let th_tuple = TU_HOA_BY_STEM.get(ys).copied().unwrap_or((
        "liem_trinh",
        "pha_quan",
        "vu_khuc",
        "thai_duong",
    ));
    let th_targets = [th_tuple.0, th_tuple.1, th_tuple.2, th_tuple.3];
    let mut tu_hoa_map: HashMap<&str, BilingualLabel> = HashMap::with_capacity(4);
    let mut tu_hoa_summary = Vec::with_capacity(4);
    for (idx, &star_id) in th_targets.iter().enumerate() {
        if let Some((tid, tvi, ten)) = TU_HOA_LABELS.get(idx) {
            let lbl = BilingualLabel::new(*tid, *tvi, *ten);
            tu_hoa_map.insert(star_id, lbl.clone());
            tu_hoa_summary.push(TuHoaMapping {
                transformation: lbl,
                star: BilingualLabel::new(star_id, star_id, star_id),
            });
        }
    }

    let mut buckets: [Vec<TuViStar>; 12] = Default::default();

    // 14 Chinh Tinh
    let tu_vi = compute_tu_vi_branch(cuc_number, lunar.day);
    let thien_phu = (4 - tu_vi as i32).rem_euclid(12) as usize;
    let chinh_positions = [
        tu_vi,
        (tu_vi as i32 - 1).rem_euclid(12) as usize,
        (tu_vi as i32 - 3).rem_euclid(12) as usize,
        (tu_vi as i32 - 4).rem_euclid(12) as usize,
        (tu_vi as i32 - 5).rem_euclid(12) as usize,
        (tu_vi as i32 - 8).rem_euclid(12) as usize,
        thien_phu,
        (thien_phu + 1) % 12,
        (thien_phu + 2) % 12,
        (thien_phu + 3) % 12,
        (thien_phu + 4) % 12,
        (thien_phu + 5) % 12,
        (thien_phu + 6) % 12,
        (thien_phu + 10) % 12,
    ];

    let mut star_title_by_id: HashMap<&str, (&str, &str)> = HashMap::new();
    for (i, &pos) in chinh_positions.iter().enumerate() {
        if let Some(&(sid, svi, sen)) = CHINH_TINH_NAMES.get(i) {
            let bright = CHINH_TINH_BRIGHTNESS
                .get(i)
                .and_then(|row| row.get(pos).copied());
            star_title_by_id.insert(sid, (svi, sen));
            push_star(
                &mut buckets,
                pos,
                StarEntry {
                    id: sid,
                    vi: svi,
                    en: sen,
                    category: "chinh_tinh",
                    brightness: bright,
                },
                &tu_hoa_map,
            );
        }
    }

    // Luc Cat (Ta Phu, Huu Bat, Van Xuong, Van Khuc, Thien Khoi, Thien Viet)
    let ta_phu = (4 + (tuvi_month - 1)).rem_euclid(12) as usize;
    let huu_bat = (10 - (tuvi_month - 1)).rem_euclid(12) as usize;
    let van_xuong = (10 - hb as i32).rem_euclid(12) as usize;
    let van_khuc = (4 + hb) % 12;
    let khoi_viet: [(usize, usize); 10] = [
        (1, 7),
        (0, 8),
        (11, 9),
        (11, 9),
        (1, 7),
        (0, 8),
        (1, 7),
        (6, 2),
        (3, 5),
        (3, 5),
    ];
    let (khoi, viet) = khoi_viet.get(ys).copied().unwrap_or((1, 7));

    let aux_stars = [
        (ta_phu, "ta_phu", "Tả Phù", "Left Assistant", "luc_cat"),
        (huu_bat, "huu_bat", "Hữu Bật", "Right Assistant", "luc_cat"),
        (
            van_xuong,
            "van_xuong",
            "Văn Xương",
            "Literary Star",
            "luc_cat",
        ),
        (van_khuc, "van_khuc", "Văn Khúc", "Oratory Star", "luc_cat"),
        (khoi, "thien_khoi", "Thiên Khôi", "Noble Scholar", "luc_cat"),
        (viet, "thien_viet", "Thiên Việt", "Noble Helper", "luc_cat"),
    ];
    for (pos, sid, svi, sen, cat) in aux_stars {
        star_title_by_id.insert(sid, (svi, sen));
        push_star(
            &mut buckets,
            pos,
            StarEntry {
                id: sid,
                vi: svi,
                en: sen,
                category: cat,
                brightness: None,
            },
            &tu_hoa_map,
        );
    }

    // Patch bilingual star names inside tu_hoa_summary now that aux & chinh_tinh names are known.
    for entry in &mut tu_hoa_summary {
        if let Some(&(vi, en)) = star_title_by_id.get(entry.star.id.as_str()) {
            entry.star.name_vi = vi.to_string();
            entry.star.name_en = en.to_string();
        }
    }

    // Loc Ton, Kinh Duong, Da La & Luc Sat
    let loc_ton_table: [usize; 10] = [2, 3, 5, 6, 5, 6, 8, 9, 11, 0];
    let loc_ton = loc_ton_table.get(ys).copied().unwrap_or(2);
    let kinh_duong = (loc_ton + 1) % 12;
    let da_la = (loc_ton + 11) % 12;
    let dia_khong = (11 - hb as i32).rem_euclid(12) as usize;
    let dia_kiep = (11 + hb) % 12;
    let (hoa_start, linh_start) = match yb {
        2 | 6 | 10 => (1usize, 3usize),
        8 | 0 | 4 => (2, 10),
        5 | 9 | 1 => (3, 10),
        _ => (9, 10),
    };
    let hoa_tinh = (hoa_start + hb) % 12;
    let linh_tinh = (linh_start + hb) % 12;

    // Additional auspicious/key minor stars: Thien Ma, Dao Hoa, Hong Loan, Thien Hy, Co Than, Qua Tu
    let thien_ma = match yb {
        2 | 6 | 10 => 8,
        8 | 0 | 4 => 2,
        5 | 9 | 1 => 11,
        _ => 5,
    };
    let dao_hoa = match yb {
        2 | 6 | 10 => 3,
        8 | 0 | 4 => 9,
        5 | 9 | 1 => 6,
        _ => 0,
    };
    let hong_loan = (3 - yb as i32).rem_euclid(12) as usize;
    let thien_hy = (hong_loan + 6) % 12;
    let (co_than, qua_tu) = match yb {
        11 | 0 | 1 => (2, 10),
        2..=4 => (5, 1),
        5..=7 => (8, 4),
        _ => (11, 7),
    };

    let extra_stars = [
        (
            loc_ton,
            "loc_ton",
            "Lộc Tồn",
            "Wealth Preserver",
            "phu_tinh",
        ),
        (
            kinh_duong,
            "kinh_duong",
            "Kình Dương",
            "Ram Blade",
            "luc_sat",
        ),
        (da_la, "da_la", "Đà La", "Humpback Obstacle", "luc_sat"),
        (
            dia_khong,
            "dia_khong",
            "Địa Không",
            "Earthly Void",
            "luc_sat",
        ),
        (
            dia_kiep,
            "dia_kiep",
            "Địa Kiếp",
            "Earthly Calamity",
            "luc_sat",
        ),
        (hoa_tinh, "hoa_tinh", "Hỏa Tinh", "Fire Star", "luc_sat"),
        (linh_tinh, "linh_tinh", "Linh Tinh", "Bell Star", "luc_sat"),
        (thien_ma, "thien_ma", "Thiên Mã", "Sky Horse", "phu_tinh"),
        (dao_hoa, "dao_hoa", "Đào Hoa", "Peach Blossom", "phu_tinh"),
        (
            hong_loan,
            "hong_loan",
            "Hồng Loan",
            "Red Phoenix",
            "phu_tinh",
        ),
        (thien_hy, "thien_hy", "Thiên Hỷ", "Sky Joy", "phu_tinh"),
        (co_than, "co_than", "Cô Thần", "Solitary Star", "phu_tinh"),
        (qua_tu, "qua_tu", "Quả Tú", "Widow Star", "phu_tinh"),
    ];
    for (pos, sid, svi, sen, cat) in extra_stars {
        push_star(
            &mut buckets,
            pos,
            StarEntry {
                id: sid,
                vi: svi,
                en: sen,
                category: cat,
                brightness: None,
            },
            &tu_hoa_map,
        );
    }

    // 12 Vong Thai Tue (clockwise from yb)
    for (i, &(sid, svi, sen)) in THAI_TUE_RING.iter().enumerate() {
        push_star(
            &mut buckets,
            (yb + i) % 12,
            StarEntry {
                id: sid,
                vi: svi,
                en: sen,
                category: "thai_tue",
                brightness: None,
            },
            &tu_hoa_map,
        );
    }

    // 12 Vong Truong Sinh (by Cuc & forward direction)
    let ts_start = match cuc_number {
        2 | 5 => 8usize,
        3 => 11,
        4 => 5,
        _ => 2,
    };
    for (i, &(sid, svi, sen)) in TRUONG_SINH_RING.iter().enumerate() {
        let pos = if forward {
            (ts_start + i) % 12
        } else {
            (ts_start as i32 - i as i32).rem_euclid(12) as usize
        };
        push_star(
            &mut buckets,
            pos,
            StarEntry {
                id: sid,
                vi: svi,
                en: sen,
                category: "truong_sinh",
                brightness: None,
            },
            &tu_hoa_map,
        );
    }

    // Tuan & Triet void branch pairs
    let tuan_base = (yb as i32 - ys as i32).rem_euclid(12) as u8;
    let tuan_branches = [(tuan_base + 10) % 12, (tuan_base + 11) % 12];
    let triet_table: [[u8; 2]; 5] = [[8, 9], [6, 7], [4, 5], [2, 3], [0, 1]];
    let triet_branches = triet_table.get(ys % 5).copied().unwrap_or([8, 9]);

    let mut palaces = Vec::with_capacity(12);
    for b in 0..12usize {
        let role_idx = (b as i32 - menh as i32).rem_euclid(12) as usize;
        let palace_lbl = PALACE_NAMES.get(role_idx).map_or_else(
            || BilingualLabel::new("menh", "Mệnh", "Life"),
            |(id, vi, en)| BilingualLabel::new(*id, *vi, *en),
        );
        let decade_step = if forward {
            role_idx as u8
        } else {
            ((menh as i32 - b as i32).rem_euclid(12)) as u8
        };
        let d_start = cuc_number + decade_step * 10;
        let d_end = d_start + 9;

        let b_offset = (b as i32 - 2).rem_euclid(12) as usize;
        let palace_cc = pair_from_stem_branch((dan_stem + b_offset) % 10, b);
        let stars = buckets.get(b).cloned().unwrap_or_default();

        palaces.push(TuViPalace {
            branch_index: b as u8,
            branch: branch_label(b),
            palace: palace_lbl,
            is_menh: b == menh,
            is_than: b == than,
            major_decade_range: (d_start, d_end),
            palace_can_chi: palace_cc,
            has_tuan: tuan_branches.contains(&(b as u8)),
            has_triet: triet_branches.contains(&(b as u8)),
            stars,
        });
    }

    Ok(TuViChart {
        cuc,
        cuc_number,
        menh_branch_index: menh as u8,
        than_branch_index: than as u8,
        menh_palace_can_chi: menh_palace_cc,
        tu_vi_branch_index: tu_vi as u8,
        thien_phu_branch_index: thien_phu as u8,
        tu_hoa_summary,
        tuan_branches,
        triet_branches,
        leap_month_shifted_for_tuvi: leap_shifted,
        forward_decade_direction: forward,
        palaces,
    })
}

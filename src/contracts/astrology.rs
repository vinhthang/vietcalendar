//! Service contract and DTOs for Vietnamese Tử Vi (Nam Phái Core) and BaZi (Four Pillars).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::contracts::canchi::{BilingualLabel, CanChiBreakdown, CanChiPair};
use crate::contracts::models::{CalendarError, LunarDate};

/// Astrology chart system selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChartSystem {
    /// Four Pillars of Destiny (*Bát Tự / Tứ Trụ*).
    Bazi,
    /// Vietnamese Tử Vi Đẩu Số (Nam Phái Core).
    Tuvi,
    /// Generate both BaZi and Tử Vi charts together.
    Both,
}

/// Subject biological/traditional polarity used for Luck Cycle & Đại Hạn direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    /// Male (*Nam*).
    Male,
    /// Female (*Nữ*).
    Female,
}

/// Birth chart query accepted by HTTP, MCP, and [`AstrologyService`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AstrologyQuery {
    /// Birth date formatted as `YYYY-MM-DD`.
    pub date: String,
    /// Local birth hour (`0..=23`).
    pub hour: u8,
    /// Local birth minute (`0..=59`).
    #[serde(default)]
    pub minute: u8,
    /// Birth gender (`"male"` or `"female"`).
    pub gender: Gender,
    /// Chart engine (`"bazi"`, `"tuvi"`, or `"both"`).
    #[serde(default = "default_chart_system")]
    pub system: ChartSystem,
    /// Set `true` if `date` expresses a lunar date rather than Gregorian solar date.
    #[serde(default)]
    pub is_lunar: bool,
    /// Set `true` only when `is_lunar = true` and `date` falls in a leap month (*tháng nhuận*).
    #[serde(default)]
    pub is_leap_month: bool,
    /// Reference timezone offset from UTC (defaults to Vietnam UTC+7).
    #[serde(default = "default_vn_timezone")]
    pub time_zone: f64,
}

fn default_chart_system() -> ChartSystem {
    ChartSystem::Both
}

fn default_vn_timezone() -> f64 {
    7.0
}

/// Hidden Heavenly Stem (*Tàng Can*) contained inside an Earthly Branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HiddenStemEntry {
    /// Heavenly Stem metadata.
    pub stem: BilingualLabel,
    /// Stem Five Element.
    pub element: BilingualLabel,
    /// Ten God relation (*Thập Thần*) relative to the BaZi Day Master.
    pub ten_god: BilingualLabel,
    /// Traditional percentage weight (`100`, `70`/`30`, or `60`/`30`/`10`).
    pub weight_pct: u8,
}

/// One BaZi pillar with Ten God stem relation, Hidden Stems, and 12 Life Stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BaziPillar {
    /// Pillar slot label (`year`, `month`, `day`, `hour`).
    pub pillar: BilingualLabel,
    /// Stem-Branch pair for this pillar.
    pub can_chi: CanChiPair,
    /// Ten God (*Thập Thần*) of this pillar's Stem relative to Day Master (`"Nhật Chủ"` for day).
    pub ten_god_stem: BilingualLabel,
    /// Hidden Stems stored within this pillar's Branch.
    pub hidden_stems: Vec<HiddenStemEntry>,
    /// 12 Life Stage (*Vòng Trường Sinh*) of the Day Master at this Branch.
    pub life_stage: BilingualLabel,
}

/// Weighted Five Elements (*Ngũ Hành*) count across the Four Pillars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FiveElementBalance {
    /// Wood (*Mộc*) score count.
    pub wood: u8,
    /// Fire (*Hỏa*) score count.
    pub fire: u8,
    /// Earth (*Thổ*) score count.
    pub earth: u8,
    /// Metal (*Kim*) score count.
    pub metal: u8,
    /// Water (*Thủy*) score count.
    pub water: u8,
    /// Strongest element in the nativity.
    pub dominant: BilingualLabel,
    /// Weakest element in the nativity.
    pub weakest: BilingualLabel,
}

/// One 10-year BaZi Luck Cycle (*Đại Vận*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LuckCycle {
    /// Sequence index (`1..=8`).
    pub index: u8,
    /// Starting nominal age.
    pub start_age: u8,
    /// Ending nominal age.
    pub end_age: u8,
    /// Starting Gregorian year.
    pub start_year: i32,
    /// Stem-Branch pillar governing the decade.
    pub can_chi: CanChiPair,
    /// Ten God of the Luck Cycle Stem relative to Day Master.
    pub ten_god_stem: BilingualLabel,
}

/// Complete BaZi (*Tứ Trụ*) birth chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BaziChart {
    /// Day Master Heavenly Stem (*Nhật Chủ*).
    pub day_master: BilingualLabel,
    /// Day Master Five Element.
    pub day_master_element: BilingualLabel,
    /// Birth year stem yin/yang polarity (`yang` / `yin`).
    pub yin_yang_polarity: BilingualLabel,
    /// `true` when Đại Vận progresses forward through the 60-cycle; `false` when backward.
    pub forward_luck_direction: bool,
    /// Approximate age when first Đại Vận begins (derived from solar-term distance).
    pub start_luck_age_years: u8,
    /// Year pillar (*Trụ Năm*, anchored at *Lập Xuân*).
    pub year_pillar: BaziPillar,
    /// Month pillar (*Trụ Tháng*, anchored at 12 *Tiết*).
    pub month_pillar: BaziPillar,
    /// Day pillar (*Trụ Ngày*).
    pub day_pillar: BaziPillar,
    /// Hour pillar (*Trụ Giờ*).
    pub hour_pillar: BaziPillar,
    /// Stems & Branches elemental distribution.
    pub element_balance: FiveElementBalance,
    /// Eight consecutive 10-year Đại Vận luck cycles.
    pub luck_cycles: Vec<LuckCycle>,
}

/// Single Tử Vi star placement entry inside a palace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TuViStar {
    /// Canonical ASCII star identifier.
    pub id: String,
    /// Vietnamese star name with diacritics.
    pub name_vi: String,
    /// English star name/gloss.
    pub name_en: String,
    /// Star class (`chinh_tinh`, `luc_cat`, `luc_sat`, `phu_tinh`, `tu_hoa`, `thai_tue`, `truong_sinh`).
    pub category: String,
    /// Brightness rating (`"M"` Miếu, `"V"` Vượng, `"D"` Đắc, `"B"` Bình, `"H"` Hãm) for major stars.
    pub brightness: Option<String>,
    /// Attached Tứ Hóa transformation (`Lộc`, `Quyền`, `Khoa`, `Kỵ`), if transformed.
    pub tu_hoa: Option<BilingualLabel>,
}

/// One of the 12 Palaces (*12 Cung*) on a Tử Vi chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TuViPalace {
    /// Fixed Earthly Branch ring position (`0 = Tý .. 11 = Hợi`).
    pub branch_index: u8,
    /// Fixed Earthly Branch of the palace house.
    pub branch: BilingualLabel,
    /// Assigned Palace role (*Mệnh*, *Phụ Mẫu*, *Phúc Đức*, ..., *Huynh Đệ*).
    pub palace: BilingualLabel,
    /// `true` if this palace hosts *Cung Mệnh*.
    pub is_menh: bool,
    /// `true` if this palace hosts *Cung Thân*.
    pub is_than: bool,
    /// Inclusive 10-year *Đại Hạn* age range for this palace `(start_age, end_age)`.
    pub major_decade_range: (u8, u8),
    /// Heavenly Stem + Earthly Branch (*Ngũ Hổ Độn* palace Can) attached to this house.
    pub palace_can_chi: CanChiPair,
    /// `true` if *Tuần Không* spans this palace.
    pub has_tuan: bool,
    /// `true` if *Triệt Lộ Không* spans this palace.
    pub has_triet: bool,
    /// All major, auxiliary, ring, and Tứ Hóa stars seated in this palace.
    pub stars: Vec<TuViStar>,
}

/// Tứ Hóa star transformation mapping summary for the birth year stem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TuHoaMapping {
    /// Transformation label (`Lộc`, `Quyền`, `Khoa`, `Kỵ`).
    pub transformation: BilingualLabel,
    /// Target star receiving the transformation.
    pub star: BilingualLabel,
}

/// Complete Vietnamese Tử Vi (Nam Phái Core) chart across 12 Palaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TuViChart {
    /// Elemental Cục (*Thủy Nhị Cục*, *Mộc Tam Cục*, *Kim Tứ Cục*, *Thổ Ngũ Cục*, *Hỏa Lục Cục*).
    pub cuc: BilingualLabel,
    /// Numeric Cục divisor (`2`, `3`, `4`, `5`, or `6`).
    pub cuc_number: u8,
    /// Earthly Branch index hosting *Cung Mệnh* (`0..12`).
    pub menh_branch_index: u8,
    /// Earthly Branch index hosting *Cung Thân* (`0..12`).
    pub than_branch_index: u8,
    /// Stem-Branch of *Cung Mệnh*.
    pub menh_palace_can_chi: CanChiPair,
    /// Earthly Branch index of *Tử Vi* (`0..12`).
    pub tu_vi_branch_index: u8,
    /// Earthly Branch index of *Thiên Phủ* (`0..12`).
    pub thien_phu_branch_index: u8,
    /// Year-stem *Tứ Hóa* mappings (`Hóa Lộc`, `Hóa Quyền`, `Hóa Khoa`, `Hóa Kỵ`).
    pub tu_hoa_summary: Vec<TuHoaMapping>,
    /// Two Earthly Branch indices affected by *Tuần*.
    pub tuan_branches: [u8; 2],
    /// Two Earthly Branch indices affected by *Triệt*.
    pub triet_branches: [u8; 2],
    /// `true` if a leap-month day 16–30 rolled into the next lunar month under Nam Phái rules.
    pub leap_month_shifted_for_tuvi: bool,
    /// `true` when 10-year Đại Hạn progresses clockwise; `false` when counterclockwise.
    pub forward_decade_direction: bool,
    /// Ordered 12 Palaces from `Tý (0)` through `Hợi (11)`.
    pub palaces: Vec<TuViPalace>,
}

/// Combined response returning deterministic BaZi and/or Tử Vi chart calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AstrologyChartResponse {
    /// Resolved Gregorian birth date `YYYY-MM-DD`.
    pub solar_date: String,
    /// Resolved Vietnamese lunar birth date.
    #[schema(value_type = crate::contracts::canchi::LunarDateSchema)]
    pub lunar: LunarDate,
    /// Birth hour (`0..=23`).
    pub hour: u8,
    /// Birth minute (`0..=59`).
    pub minute: u8,
    /// Subject gender.
    pub gender: Gender,
    /// Requested chart system.
    pub system: ChartSystem,
    /// Full underlying Can Chi & active Tiết Khí breakdown.
    pub can_chi: CanChiBreakdown,
    /// BaZi chart (present when `system` is `bazi` or `both`).
    pub bazi: Option<BaziChart>,
    /// Tử Vi chart (present when `system` is `tuvi` or `both`).
    pub tuvi: Option<TuViChart>,
}

/// Cordis service contract for deterministic Vietnamese Tử Vi and BaZi generation.
pub trait AstrologyService: Send + Sync + 'static {
    /// Generate a full BaZi and/or Tử Vi chart from verified Gregorian or lunar birth parameters.
    ///
    /// # Errors
    /// Returns [`CalendarError`] when the input date, hour, minute, lunar day/leap status, or timezone is invalid.
    fn generate_chart(
        &self,
        query: &AstrologyQuery,
    ) -> Result<AstrologyChartResponse, CalendarError>;
}

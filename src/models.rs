use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct DateMonthYear {
    pub dd: i32,
    #[serde(alias = "mm", alias = "MM")]
    pub mm: i32,
    pub yyyy: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leap: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct LunarDate {
    pub day: i32,
    pub month: i32,
    pub year: i32,
    pub is_leap: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_month_year_serde() {
        // Deserializing with uppercase MM
        let json_data = r#"{"dd": 15, "MM": 8, "yyyy": 2024}"#;
        let parsed: DateMonthYear = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            parsed,
            DateMonthYear {
                dd: 15,
                mm: 8,
                yyyy: 2024,
                leap: None
            }
        );

        // Deserializing with lowercase mm and leap
        let json_data_leap = r#"{"dd": 15, "mm": 8, "yyyy": 2024, "leap": true}"#;
        let parsed_leap: DateMonthYear = serde_json::from_str(json_data_leap).unwrap();
        assert_eq!(
            parsed_leap,
            DateMonthYear {
                dd: 15,
                mm: 8,
                yyyy: 2024,
                leap: Some(true)
            }
        );

        // Serializing omits leap when None
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(!serialized.contains("leap"));
    }

    #[test]
    fn test_lunar_date_serde() {
        let lunar = LunarDate {
            day: 1,
            month: 1,
            year: 2024,
            is_leap: false,
        };
        let serialized = serde_json::to_string(&lunar).unwrap();
        let deserialized: LunarDate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(lunar, deserialized);
    }
}

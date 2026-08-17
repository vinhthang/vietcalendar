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


use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
#[allow(non_snake_case)]
pub struct DateMonthYear {
    pub dd: i32,
    pub MM: i32,
    pub yyyy: i32,
}

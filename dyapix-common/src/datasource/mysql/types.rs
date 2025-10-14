use chrono::{DateTime, Utc};

/// Data source record from database
#[derive(sqlx::FromRow)]
pub struct DyapixDs {
    pub id: i64,
    pub key: String,
    pub ds_type: String,
    pub ds_json: String,
    pub prev_ds_json: Option<String>,
    pub ds_status: String,
    pub operation_type: String,
    pub is_deleted: bool,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}


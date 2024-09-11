use serde::Deserialize;

use crate::librus::client::{LibrusClient, LibrusResult};

#[derive(Deserialize)]
pub struct User {
    #[serde(rename = "Id")]
    pub id: i32,

    #[serde(rename = "FirstName")]
    pub first_name: Option<String>,

    #[serde(rename = "LastName")]
    pub last_name: Option<String>,
}

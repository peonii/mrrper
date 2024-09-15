use serde::Deserialize;

use crate::librus::client::{LibrusClient, LibrusResult};

#[derive(Deserialize)]
pub struct SchoolNoticeResponse {
    #[serde(rename = "SchoolNotices")]
    notices: Vec<SchoolNotice>,
}

#[derive(Deserialize)]
pub struct SchoolNotice {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "Subject")]
    pub title: String,
    #[serde(alias = "CreationDate")]
    pub created_at: String,
}

impl<'a> LibrusClient<'a> {
    pub async fn fetch_notices(&mut self) -> LibrusResult<Vec<SchoolNotice>> {
        Ok(self
            .request::<SchoolNoticeResponse>("https://api.librus.pl/3.0/SchoolNotices/")
            .await?
            .notices)
    }
}

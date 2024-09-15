use std::{borrow::Cow, sync::Arc};

use reqwest::header::{HeaderMap, InvalidHeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
pub struct SynergiaAccount {
    pub id: i32,
    #[serde(rename = "accessToken")]
    pub access_token: String,
    pub login: String,
}

#[derive(Serialize, Deserialize)]
pub struct SynergiaAccountsWrapper {
    #[serde(rename = "accounts")]
    pub inner: Vec<SynergiaAccount>,
}

#[derive(Serialize, Deserialize)]
pub struct LibrusCredentials<'a> {
    pub email: Cow<'a, str>,
    pub password: Cow<'a, str>,
}

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";

pub struct LibrusClient<'a> {
    http: reqwest::Client,
    credentials: Option<LibrusCredentials<'a>>,

    token: Option<Cow<'a, str>>,
}

#[derive(Error, Debug)]
pub enum LibrusError {
    #[error("Failed to request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to parse header")]
    InvalidHeaderError(#[from] InvalidHeaderValue),
    #[error("Failed to authenticate")]
    AuthenticationError,
}
pub type LibrusResult<T> = Result<T, LibrusError>;

impl<'a> LibrusClient<'a> {
    pub fn new() -> LibrusResult<Self> {
        Ok(Self {
            http: reqwest::Client::builder().cookie_store(true).build()?,
            credentials: None,
            token: None,
        })
    }

    pub fn with_credentials(mut self, credentials: LibrusCredentials<'a>) -> Self {
        self.credentials = Some(credentials);
        self
    }

    async fn fetch_csrf(&self) -> LibrusResult<String> {
        let text = self
            .http
            .get("https://portal.librus.pl/")
            .send()
            .await?
            .text()
            .await?;

        let re = regex::Regex::new(r#"<meta name="csrf-token" content="(.*)">"#)
            .expect("Regex should be valid");

        Ok(re
            .captures(&text)
            .ok_or(LibrusError::AuthenticationError)?
            .get(1)
            .ok_or(LibrusError::AuthenticationError)?
            .as_str()
            .to_owned())
    }

    pub async fn login(&mut self) -> LibrusResult<()> {
        if self.credentials.is_none() {
            return Err(LibrusError::AuthenticationError);
        }

        let csrf = self.fetch_csrf().await?;
        let mut headers = HeaderMap::new();

        headers.insert("X-CSRF-TOKEN", csrf.parse()?);
        headers.insert("User-Agent", USER_AGENT.parse()?);
        headers.insert("Content-Type", "application/json".parse()?);

        let response_cookies = self
            .http
            .post("https://portal.librus.pl/konto-librus/login/action")
            .headers(headers)
            .json(self.credentials.as_ref().expect("Already checked"))
            .send()
            .await?;

        if !response_cookies.status().is_success() {
            return Err(LibrusError::AuthenticationError);
        }

        let response = self
            .http
            .get("https://portal.librus.pl/api/v3/SynergiaAccounts")
            .send()
            .await?;

        let accounts = response.json::<SynergiaAccountsWrapper>().await?;

        self.token = accounts
            .inner
            .first()
            .map(|a| a.access_token.clone().into());

        Ok(())
    }

    pub async fn request<T: serde::de::DeserializeOwned + Sized>(
        &mut self,
        url: &str,
    ) -> LibrusResult<T> {
        if self.token.is_none() {
            return Err(LibrusError::AuthenticationError);
        }

        let mut headers = HeaderMap::new();

        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token.as_ref().expect("oops")).parse()?,
        );
        headers.insert("User-Agent", USER_AGENT.parse()?);
        headers.insert("gzip", "true".parse()?);

        let req = self.http.get(url).headers(headers).send().await?;

        if !req.status().is_success() {
            self.login().await?;
        }

        let resp = req.json::<T>().await?;

        Ok(resp)
    }
}

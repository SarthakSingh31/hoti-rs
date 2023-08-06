use reqwest::header::{HeaderMap, AUTHORIZATION};

pub mod text_to_speech;

#[derive(Clone)]
pub struct Client(reqwest::Client);

impl Client {
    pub fn from_env() -> anyhow::Result<Self> {
        let project = std::env::var("GOOGLE_PROJECT")?;
        let bearer_token = std::env::var("GOOGLE_BEARER_TOKEN")?;

        let mut headers = HeaderMap::default();
        headers.insert("x-goog-user-project", project.parse()?);
        headers.insert(AUTHORIZATION, format!("Bearer {bearer_token}").parse()?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client(client))
    }

    pub fn remake_with_bearer_token(&mut self, token: String) -> anyhow::Result<()> {
        let token = token.trim();
        println!("{token}");
        let project = std::env::var("GOOGLE_PROJECT")?;

        let mut headers = HeaderMap::default();
        headers.insert("x-goog-user-project", project.parse()?);
        headers.insert(AUTHORIZATION, format!("Bearer {token}").parse()?);

        self.0 = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(())
    }
}

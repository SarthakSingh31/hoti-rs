#![feature(async_fn_in_trait)]

use async_openai::config::OpenAIConfig;
use reqwest_middleware::ClientWithMiddleware;

pub mod gcloud;
pub mod scp;
pub mod video_gen;

pub trait ContentSource {
    type ContentIter: Iterator<Item = Self>;

    async fn dialogue(
        &mut self,
        openai: &async_openai::Client<OpenAIConfig>,
        reqwest: ClientWithMiddleware,
    ) -> anyhow::Result<String>;
    async fn image_description(
        &mut self,
        openai: &async_openai::Client<OpenAIConfig>,
        reqwest: ClientWithMiddleware,
    ) -> anyhow::Result<String>;

    fn iter() -> anyhow::Result<Self::ContentIter>;
}

use std::time::Duration;

use async_openai::{
    config::OpenAIConfig,
    types::{CreateImageRequest, ImageData},
};
use base64::Engine;

use super::ui::{ImageHandle, Node, UiUpdater, VideoUI};

pub struct ImageManager {
    images: Vec<(u32, ImageHandle)>,
}

impl ImageManager {
    pub async fn new(
        prompt: String,
        openai: &async_openai::Client<OpenAIConfig>,
        frame_rate: u32,
        duration: Duration,
        ui: &mut VideoUI,
    ) -> anyhow::Result<Self> {
        let mut n = ((duration - Duration::from_secs(5)).as_secs_f64() / 5.0) as u8;
        println!("Generating {n} images");

        let frame_step =
            ((duration - Duration::from_secs(5)).as_secs_f64() / n as f64) as u32 * frame_rate;

        let img_gen = async_openai::Images::new(openai);
        let mut resps = Vec::default();

        while n != 0 {
            let resp = img_gen
                .create(CreateImageRequest {
                    prompt: prompt.clone(),
                    n: Some(n.min(10)),
                    size: Some(async_openai::types::ImageSize::S1024x1024),
                    response_format: Some(async_openai::types::ResponseFormat::B64Json),
                    user: None,
                })
                .await?;

            n -= n.min(10);

            resps.push(resp);
        }

        let images = resps
            .into_iter()
            .map(|resp| resp.data)
            .flat_map(|images| {
                images.into_iter().map(|img| {
                    let ImageData::B64Json(data) = img.as_ref() else {
                    panic!("Got response in wrong format");
                };

                    let data = base64::prelude::BASE64_STANDARD
                        .decode(data.as_bytes())
                        .unwrap();

                    image::load_from_memory(&data).unwrap().to_rgba8()
                })
            })
            .enumerate()
            .map(|(i, img)| (5 * frame_rate + frame_step * i as u32, ui.add(img)))
            .collect();

        Ok(ImageManager { images })
    }
}

impl UiUpdater for ImageManager {
    fn update(&mut self, frame_idx: u32, ui: &mut VideoUI) {
        if let Some((_, new_img)) = self.images.iter().find(|(frame, _)| *frame == frame_idx) {
            if let Node::Image(img) = &mut ui.children[1].node {
                *img = *new_img;
            }
        }
    }
}

#![feature(async_fn_in_trait)]

use std::fs;

use hoti_rs::gcloud;
use hoti_rs::scp::SCP;
use hoti_rs::video_gen;
use hoti_rs::{gcloud::text_to_speech::EnString, ContentSource};
use reqwest_middleware::ClientBuilder;
use taffy::{
    prelude::{Rect, Size},
    style::{Dimension, LengthPercentageAuto, Style},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().expect(".env file is missing!");

    let openai = async_openai::Client::new();

    let retry_policy =
        reqwest_retry::policies::ExponentialBackoff::builder().build_with_max_retries(5);
    let reqwest = ClientBuilder::new(reqwest::Client::new())
        .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
            retry_policy,
        ))
        .build();

    let mut client = hoti_rs::gcloud::Client::from_env()?;

    for (idx, mut scp) in SCP::iter()?.enumerate().skip(102) {
        let start = std::time::Instant::now();

        println!("Idx: {idx} - Generating: {}", scp.name());

        let title = scp.title(reqwest.clone()).await.unwrap_or("Unknown".into());
        let classification = scp.classification(reqwest.clone()).await?;

        println!("Title: {title}");
        println!("Classification: {classification:?}");

        let dialogue = scp.dialogue(&openai, reqwest.clone()).await?;
        let mut image_description = scp.image_description(&openai, reqwest.clone()).await?;

        println!("Generating Audio For Dialogue:\n{dialogue}");
        println!("Image Description: {:#?}", image_description);

        let mut path = std::env::temp_dir();
        path.push(format!("{}-output.mp3", scp.name()));

        let contents = gcloud::text_to_speech::SynthesisPayload::synthesize(
            &mut client,
            EnString(dialogue.clone()),
        )
        .await;
        fs::write(&path, contents.clone())?;

        // let contents = fs::read(&path).unwrap();

        let mut video = video_gen::VideoFrameIter::new(
            glam::UVec2 { x: 1080, y: 1920 },
            60,
            video_gen::Mp3::new(contents.clone()).duration(),
        );

        let font = rusttype::Font::try_from_vec(
            include_bytes!("/usr/share/fonts/noto/NotoSansMono-ExtraBold.ttf").to_vec(),
        )
        .unwrap();

        let scp_logo = video
            .ui
            .add(image::open("assets/SCP.png").unwrap().to_rgba8());
        video.ui.children = vec![
            video_gen::ui::StyledNode {
                node: video_gen::ui::Node::Container(vec![
                    video_gen::ui::StyledNode {
                        node: video_gen::ui::Node::TextCentered {
                            text: scp.name().into(),
                            font: font.clone(),
                            scale: rusttype::Scale { x: 120.0, y: 120.0 },
                            line_height: 120,
                            color: [255, 255, 255, 255].into(),
                        },
                        style: Style {
                            size: Size {
                                width: Dimension::Auto,
                                height: Dimension::Points(120.0),
                            },
                            ..Default::default()
                        },
                    },
                    video_gen::ui::StyledNode {
                        node: video_gen::ui::Node::TextCentered {
                            text: title.to_ascii_uppercase(),
                            font: font.clone(),
                            scale: rusttype::Scale { x: 120.0, y: 120.0 },
                            line_height: 120,
                            color: [255, 255, 255, 255].into(),
                        },
                        style: Style {
                            size: Size {
                                width: Dimension::Auto,
                                height: Dimension::Points(120.0),
                            },
                            ..Default::default()
                        },
                    },
                ]),
                style: Style {
                    flex_direction: taffy::style::FlexDirection::Column,
                    size: Size {
                        width: Dimension::Auto,
                        height: Dimension::Auto,
                    },
                    margin: Rect {
                        left: LengthPercentageAuto::Points(0.0),
                        right: LengthPercentageAuto::Points(0.0),
                        top: LengthPercentageAuto::Points(100.0),
                        bottom: LengthPercentageAuto::Points(100.0),
                    },
                    ..Default::default()
                },
            },
            video_gen::ui::StyledNode {
                node: video_gen::ui::Node::Image(scp_logo),
                style: Style {
                    size: Size {
                        width: Dimension::Points(800.0),
                        height: Dimension::Points(800.0),
                    },
                    margin: Rect {
                        left: LengthPercentageAuto::Auto,
                        right: LengthPercentageAuto::Auto,
                        top: LengthPercentageAuto::Points(0.0),
                        bottom: LengthPercentageAuto::Auto,
                    },
                    ..Default::default()
                },
            },
            classification.ui(font.clone(), &mut video.ui),
            video_gen::ui::StyledNode {
                node: video_gen::ui::Node::TextCentered {
                    text: String::default(),
                    font: font,
                    scale: rusttype::Scale { x: 60.0, y: 60.0 },
                    line_height: 80,
                    color: [255, 255, 255, 255].into(),
                },
                style: Style {
                    size: Size {
                        width: Dimension::Auto,
                        height: Dimension::Points(420.0),
                    },
                    margin: Rect {
                        left: LengthPercentageAuto::Points(100.0),
                        right: LengthPercentageAuto::Points(100.0),
                        top: LengthPercentageAuto::Points(0.0),
                        bottom: LengthPercentageAuto::Points(0.0),
                    },
                    ..Default::default()
                },
            },
        ];
        video.ui.background_color = [24, 24, 24, 255].into();

        let sub_mgr = video_gen::subtitle::SubtitleManager::new(dialogue, video.total_frames());

        println!("Fetching images for the video for: {}", scp.name());

        let mut attempt = 0;
        let img_mgr = loop {
            if attempt > 10 {
                panic!("Failed to fetch images.")
            }

            attempt += 1;

            match video_gen::image_manager::ImageManager::new(
                image_description,
                &openai,
                video.frame_rate(),
                video.duration(),
                &mut video.ui,
            )
            .await
            {
                Ok(img_mgr) => break img_mgr,
                Err(err) => {
                    println!("\nError Generating Images: {err:?}");
                    image_description = scp.image_description(&openai, reqwest.clone()).await?;
                    println!("Trying to use new description: {}", image_description);
                }
            }
        };

        video.updaters.push(Box::new(sub_mgr));
        video.updaters.push(Box::new(img_mgr));

        println!("Starting to encode the video for: {}", scp.name());

        video
            .encode_h264(
                path.to_str().unwrap(),
                format!("{}.mp4", scp.name()).as_str(),
            )
            .await;

        println!(
            "Made video for {} and it took {:?}",
            scp.name(),
            start.elapsed()
        );
        println!("----------------------------------------------------------\n");
    }

    Ok(())
}

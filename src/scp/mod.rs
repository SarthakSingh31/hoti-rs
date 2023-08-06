use std::{cmp::Ordering, collections::HashMap, fs, path::Path};

use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, CreateChatCompletionRequest, Role},
    Chat,
};
use image::RgbaImage;
use markup5ever::interface::TreeSink;
use reqwest_middleware::ClientWithMiddleware;
use rusttype::Font;
use scraper::{ElementRef, Html, Selector};
use serde::de::Visitor;
use taffy::{
    prelude::{Rect, Size},
    style::{AlignContent, Dimension, FlexWrap, LengthPercentageAuto, Style},
};

use crate::{
    video_gen::ui::{StyledNode, VideoUI},
    ContentSource,
};

#[derive(Debug)]
pub enum SCPSeries {
    Series1,
    Series2,
    Series3,
    Series4,
    Series5,
    Series6,
    Series7,
    Series8,
    Joke,
    Explained,
    International,
    Archived,
    Decommissioned,
}

impl<'de> serde::Deserialize<'de> for SCPSeries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SCPEnumVisitor;

        impl<'de> Visitor<'de> for SCPEnumVisitor {
            type Value = SCPSeries;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "one of [
    \"series-1\",
    \"series-2\",
    \"series-3\",
    \"series-4\",
    \"series-5\",
    \"series-6\",
    \"series-7\",
    \"series-8\",
    \"joke\",
    \"explained\",
    \"international\",
    \"archived\",
    \"decommissioned\",
]",
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "series-1" => Ok(SCPSeries::Series1),
                    "series-2" => Ok(SCPSeries::Series2),
                    "series-3" => Ok(SCPSeries::Series3),
                    "series-4" => Ok(SCPSeries::Series4),
                    "series-5" => Ok(SCPSeries::Series5),
                    "series-6" => Ok(SCPSeries::Series6),
                    "series-7" => Ok(SCPSeries::Series7),
                    "series-8" => Ok(SCPSeries::Series8),
                    "joke" => Ok(SCPSeries::Joke),
                    "explained" => Ok(SCPSeries::Explained),
                    "international" => Ok(SCPSeries::International),
                    "archived" => Ok(SCPSeries::Archived),
                    "decommissioned" => Ok(SCPSeries::Decommissioned),
                    _ => Err(E::custom(format!("Not a valid variant: {}", v))),
                }
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    b"series-1" => Ok(SCPSeries::Series1),
                    b"series-2" => Ok(SCPSeries::Series2),
                    b"series-3" => Ok(SCPSeries::Series3),
                    b"series-4" => Ok(SCPSeries::Series4),
                    b"series-5" => Ok(SCPSeries::Series5),
                    b"series-6" => Ok(SCPSeries::Series6),
                    b"series-7" => Ok(SCPSeries::Series7),
                    b"series-8" => Ok(SCPSeries::Series8),
                    b"joke" => Ok(SCPSeries::Joke),
                    b"explained" => Ok(SCPSeries::Explained),
                    b"international" => Ok(SCPSeries::International),
                    b"archived" => Ok(SCPSeries::Archived),
                    b"decommissioned" => Ok(SCPSeries::Decommissioned),
                    _ => Err(E::custom(format!("Not a valid variant: {:?}", v))),
                }
            }
        }

        deserializer.deserialize_identifier(SCPEnumVisitor)
    }
}

impl SCPSeries {
    pub fn url(&self) -> String {
        const BASE_URL: &str = "https://scp-wiki.wikidot.com";

        match self {
            SCPSeries::Series1 => format!("{BASE_URL}/scp-series"),
            SCPSeries::Series2 => format!("{BASE_URL}/scp-series-2"),
            SCPSeries::Series3 => format!("{BASE_URL}/scp-series-3"),
            SCPSeries::Series4 => format!("{BASE_URL}/scp-series-4"),
            SCPSeries::Series5 => format!("{BASE_URL}/scp-series-5"),
            SCPSeries::Series6 => format!("{BASE_URL}/scp-series-6"),
            SCPSeries::Series7 => format!("{BASE_URL}/scp-series-7"),
            SCPSeries::Series8 => format!("{BASE_URL}/scp-series-8"),
            SCPSeries::Joke => format!("{BASE_URL}/joke-scps"),
            SCPSeries::Explained => format!("{BASE_URL}/scp-ex"),
            SCPSeries::International => format!("{BASE_URL}/scp-international"),
            SCPSeries::Archived => format!("{BASE_URL}/archived-scps"),
            SCPSeries::Decommissioned => format!("{BASE_URL}/archived:decommissioned-scps"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SCPItem {
    series: SCPSeries,
    scp: String,
    url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct SCPIndex(HashMap<String, SCPItem>);

impl SCPIndex {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    pub fn sorted_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.0.keys().map(|k| k.clone()).collect();

        keys.sort_by(|a, b| {
            let mut a = a.split('-');
            let mut b = b.split('-');

            loop {
                match (a.next(), b.next()) {
                    (None, None) => break Ordering::Equal,
                    (None, Some(_)) => break Ordering::Less,
                    (Some(_), None) => break Ordering::Greater,
                    (Some(a), Some(b)) => match (a.parse::<u16>(), b.parse::<u16>()) {
                        (Ok(a), Ok(b)) => match a.cmp(&b) {
                            Ordering::Equal => continue,
                            order => break order,
                        },
                        (Ok(_), Err(_)) => break Ordering::Less,
                        (Err(_), Ok(_)) => break Ordering::Greater,
                        (Err(_), Err(_)) => match a.cmp(&b) {
                            Ordering::Equal => continue,
                            order => break order,
                        },
                    },
                }
            }
        });

        keys
    }
}

pub struct SCP {
    name: String,
    series: SCPSeries,
    url: String,
    article: Option<String>,
}

impl SCP {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn title(&self, reqwest: ClientWithMiddleware) -> Option<String> {
        let body = Html::parse_document(
            &reqwest
                .get(&self.series.url())
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
        );

        let elm = body
            .select(
                &Selector::parse(&format!("a[href=\"/{}\"]", self.name.to_ascii_lowercase()))
                    .unwrap(),
            )
            .next()?;

        let title = ElementRef::wrap(elm.parent().unwrap())
            .unwrap()
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split("-")
            .last()
            .unwrap()
            .trim()
            .to_owned();

        Some(title)
    }

    pub async fn classification(
        &mut self,
        reqwest: ClientWithMiddleware,
    ) -> anyhow::Result<Classification> {
        let article = self.article(reqwest).await?.clone();

        let mut end = 500;

        let article = loop {
            match article.get(..end) {
                Some(article) => break article,
                None => end += 1,
            }
        };

        Ok(Classification::from_article(article))
    }

    pub async fn article(&mut self, reqwest: ClientWithMiddleware) -> anyhow::Result<String> {
        if let Some(article) = &self.article {
            Ok(article.clone())
        } else {
            let mut body = Html::parse_document(
                &reqwest
                    .get(&self.url)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap(),
            );

            for script_tag in body
                .select(&Selector::parse("script").unwrap())
                .map(|elm| elm.id())
                .collect::<Vec<_>>()
            {
                body.remove_from_parent(&script_tag);
            }

            for license_tag in body
                .select(&Selector::parse(".licensebox").unwrap())
                .map(|elm| elm.id())
                .collect::<Vec<_>>()
            {
                body.remove_from_parent(&license_tag);
            }

            for footer_tag in body
                .select(&Selector::parse(".footer-wikiwalk-nav").unwrap())
                .map(|elm| elm.id())
                .collect::<Vec<_>>()
            {
                body.remove_from_parent(&footer_tag);
            }

            for collection_tag in body
                .select(&Selector::parse(".collection").unwrap())
                .map(|elm| elm.id())
                .collect::<Vec<_>>()
            {
                body.remove_from_parent(&collection_tag);
            }

            if let Some(link) = body
                .select(&Selector::parse("a").unwrap())
                .filter_map(|elm| elm.value().attr("href"))
                .find(|link| link.contains("offset/1"))
            {
                let data = reqwest
                    .get(link)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                body = Html::parse_document(&data);
            }

            let root = body
                .select(&Selector::parse("#page-content").unwrap())
                .next()
                .unwrap();

            let full_article = root
                .text()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .enumerate()
                .filter_map(|(i, s)| if i < 5 { None } else { Some(s) })
                .collect::<Vec<_>>()
                .join("\n");

            let start = full_article.find("SCP-").unwrap();
            let full_article = full_article[start.checked_sub(100).unwrap_or(0)..].to_string();

            let full_article = if full_article.len() > 65000 {
                full_article[0..65000].into()
            } else {
                full_article
            };

            self.article = Some(full_article);

            Ok(self.article.as_ref().unwrap().clone())
        }
    }
}

impl ContentSource for SCP {
    type ContentIter = SCPIter;

    async fn dialogue(
        &mut self,
        openai: &async_openai::Client<OpenAIConfig>,
        reqwest: ClientWithMiddleware,
    ) -> anyhow::Result<String> {
        let article = self.article(reqwest).await?.clone();

        let messages = vec![
            ChatCompletionRequestMessage {
                role: Role::User,
                content: format!("Here is a fragment of {}'s information page:\n```\n{article}\n```", self.name),
                name: None,
            },
            ChatCompletionRequestMessage {
                role: Role::User,
                content: format!("Generate a summary of {} based on the information provided above. The summary should be a paragraph. Start the paragraph with its object classification, then go on to describe the SCP. Then talk about its containment procedures. Do not use the █ character.", self.name),
                name: None,
            },
        ];

        let resp = Chat::new(openai)
            .create(CreateChatCompletionRequest {
                model: "gpt-3.5-turbo-16k".into(),
                messages,
                temperature: None,
                top_p: None,
                n: None,
                stream: None,
                stop: None,
                max_tokens: None,
                presence_penalty: None,
                frequency_penalty: None,
                logit_bias: None,
                user: None,
            })
            .await?;

        assert!(resp.choices.len() == 1);
        assert!(resp.choices[0].message.role == Role::Assistant);

        Ok(resp.choices[0].message.content.clone())
    }

    async fn image_description(
        &mut self,
        openai: &async_openai::Client<OpenAIConfig>,
        reqwest: ClientWithMiddleware,
    ) -> anyhow::Result<String> {
        let article = self.article(reqwest).await?.clone();

        let resp = Chat::new(openai)
            .create(CreateChatCompletionRequest {
                model: "gpt-3.5-turbo-16k".into(),
                messages: vec![
                    ChatCompletionRequestMessage {
                        role: Role::User,
                        content: format!("Here is a fragment of {}'s information page:\n```\n{article}\n```", self.name),
                        name: None,
                    },
                    ChatCompletionRequestMessage {
                        role: Role::User,
                        content: format!("Visually describe {} based on the information provided above. Do not use the █ character. Do not mention anything outside of the visual description. Try to be as concise as possible.", self.name),
                        name: None,
                    }
                ],
                temperature: None,
                top_p: None,
                n: None,
                stream: None,
                stop: None,
                max_tokens: None,
                presence_penalty: None,
                frequency_penalty: None,
                logit_bias: None,
                user: None,
            })
            .await?;

        assert!(resp.choices.len() == 1);
        assert!(resp.choices[0].message.role == Role::Assistant);

        Ok(resp.choices[0]
            .message
            .content
            .clone()
            .replace("memetic", "███████")
            .replace("bodily fluids", "****** fluids")
            .replace("living humans", "****** humans")
            .replace("trauma", "******")
            .replace("necrosis", "********")
            .replace("gangrene", "********")
            .replace("orifices", "********")
            .replace("oral", "mouth's"))
    }

    fn iter() -> anyhow::Result<Self::ContentIter> {
        let index = SCPIndex::from_file("src/scp/index.json")?;

        Ok(SCPIter {
            ordered_keys: index.sorted_keys().into_iter(),
            index,
        })
    }
}

pub struct SCPIter {
    ordered_keys: std::vec::IntoIter<String>,
    index: SCPIndex,
}

impl Iterator for SCPIter {
    type Item = SCP;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.ordered_keys.next()?;
        let item = self.index.0.remove(&key)?;

        Some(SCP {
            name: item.scp,
            series: item.series,
            url: item.url,
            article: None,
        })
    }
}

#[derive(Debug, proc_macros::FromArticle, proc_macros::AsText)]
pub enum ContainmentClass {
    Safe,
    Euclid,
    Keter,
    Neutralized,
    Pending,
    Explained,
    Esoteric,
}

impl Into<RgbaImage> for &ContainmentClass {
    fn into(self) -> RgbaImage {
        match self {
            ContainmentClass::Safe => image::open("assets/containment/Safe.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Euclid => image::open("assets/containment/Euclid.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Keter => image::open("assets/containment/Keter.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Neutralized => image::open("assets/containment/Neutralized.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Pending => image::open("assets/containment/Pending.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Explained => image::open("assets/containment/Explained.png")
                .unwrap()
                .to_rgba8(),
            ContainmentClass::Esoteric => image::open("assets/containment/Esoteric.png")
                .unwrap()
                .to_rgba8(),
        }
    }
}

impl Into<String> for &ContainmentClass {
    fn into(self) -> String {
        self.as_text().into()
    }
}

#[derive(Debug, proc_macros::FromArticle, proc_macros::AsText)]
pub enum SecondaryClass {
    Apollyon,
    Archon,
    Cernunnos,
    Decommissioned,
    Hiemal,
    Tiamat,
    Ticonderoga,
    Thaumiel,
    Uncontained,
}

impl Into<RgbaImage> for &SecondaryClass {
    fn into(self) -> RgbaImage {
        match self {
            SecondaryClass::Apollyon => image::open("assets/secondary/Apollyon.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Archon => image::open("assets/secondary/Archon.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Cernunnos => image::open("assets/secondary/Cernunnos.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Decommissioned => image::open("assets/secondary/Decommissioned.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Hiemal => image::open("assets/secondary/Hiemal.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Tiamat => image::open("assets/secondary/Tiamat.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Ticonderoga => image::open("assets/secondary/Ticonderoga.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Thaumiel => image::open("assets/secondary/Thaumiel.png")
                .unwrap()
                .to_rgba8(),
            SecondaryClass::Uncontained => image::open("assets/secondary/Uncontained.png")
                .unwrap()
                .to_rgba8(),
        }
    }
}

impl Into<String> for &SecondaryClass {
    fn into(self) -> String {
        self.as_text().into()
    }
}

#[derive(Debug, proc_macros::FromArticle, proc_macros::AsText)]
pub enum DisruptionClass {
    Dark,
    Vlam,
    Keneq,
    Ekhi,
    Amida,
}

impl Into<RgbaImage> for &DisruptionClass {
    fn into(self) -> RgbaImage {
        match self {
            DisruptionClass::Dark => image::open("assets/disruption/Dark.png")
                .unwrap()
                .to_rgba8(),
            DisruptionClass::Vlam => image::open("assets/disruption/Vlam.png")
                .unwrap()
                .to_rgba8(),
            DisruptionClass::Keneq => image::open("assets/disruption/Keneq.png")
                .unwrap()
                .to_rgba8(),
            DisruptionClass::Ekhi => image::open("assets/disruption/Ekhi.png")
                .unwrap()
                .to_rgba8(),
            DisruptionClass::Amida => image::open("assets/disruption/Amida.png")
                .unwrap()
                .to_rgba8(),
        }
    }
}

impl Into<String> for &DisruptionClass {
    fn into(self) -> String {
        self.as_text().into()
    }
}

#[derive(Debug, proc_macros::FromArticle, proc_macros::AsText)]
pub enum RiskClass {
    Notice,
    Caution,
    Warning,
    Danger,
    Critical,
}

impl Into<RgbaImage> for &RiskClass {
    fn into(self) -> RgbaImage {
        match self {
            RiskClass::Notice => image::open("assets/risk/Notice.png").unwrap().to_rgba8(),
            RiskClass::Caution => image::open("assets/risk/Caution.png").unwrap().to_rgba8(),
            RiskClass::Warning => image::open("assets/risk/Warning.png").unwrap().to_rgba8(),
            RiskClass::Danger => image::open("assets/risk/Danger.png").unwrap().to_rgba8(),
            RiskClass::Critical => image::open("assets/risk/Critical.png").unwrap().to_rgba8(),
        }
    }
}

impl Into<String> for &RiskClass {
    fn into(self) -> String {
        self.as_text().into()
    }
}

#[derive(Debug)]
pub struct Classification {
    pub containment: Option<ContainmentClass>,
    pub secondary: Option<SecondaryClass>,
    pub disruption: Option<DisruptionClass>,
    pub risk: Option<RiskClass>,
}

impl Classification {
    fn from_article(s: &str) -> Self {
        Classification {
            containment: ContainmentClass::from_article(s),
            secondary: SecondaryClass::from_article(s),
            disruption: DisruptionClass::from_article(s),
            risk: RiskClass::from_article(s),
        }
    }

    pub fn ui(&self, font: Font<'static>, ui: &mut VideoUI) -> StyledNode {
        let mut nodes = Vec::default();

        const TAG_WIDTH: f32 = 450.0;
        const ICON_TEXT_SIZE: f32 = 60.0;

        fn add_ui<T: Into<RgbaImage> + Into<String> + Copy>(
            font: &Font<'static>,
            ui: &mut VideoUI,
            nodes: &mut Vec<StyledNode>,
            class: Option<T>,
        ) {
            let (img, text) = if let Some(class) = class {
                (class.into(), class.into())
            } else {
                (
                    image::open("assets/containment/Pending.png")
                        .unwrap()
                        .to_rgba8(),
                    "???".to_owned(),
                )
            };

            let handle = ui.add(img);

            nodes.push(StyledNode {
                node: crate::video_gen::ui::Node::Container(vec![
                    StyledNode {
                        node: crate::video_gen::ui::Node::Image(handle),
                        style: Style {
                            size: Size {
                                width: Dimension::Points(ICON_TEXT_SIZE),
                                height: Dimension::Auto,
                            },
                            ..Default::default()
                        },
                    },
                    StyledNode {
                        node: crate::video_gen::ui::Node::TextCentered {
                            text,
                            font: font.clone(),
                            scale: rusttype::Scale {
                                x: ICON_TEXT_SIZE,
                                y: ICON_TEXT_SIZE,
                            },
                            line_height: ICON_TEXT_SIZE as u32,
                            color: [255; 4].into(),
                        },
                        style: Style {
                            size: Size {
                                width: Dimension::Auto,
                                height: Dimension::Points(ICON_TEXT_SIZE),
                            },
                            margin: Rect {
                                left: LengthPercentageAuto::Points(20.0),
                                right: LengthPercentageAuto::Points(20.0),
                                top: LengthPercentageAuto::Auto,
                                bottom: LengthPercentageAuto::Auto,
                            },
                            ..Default::default()
                        },
                    },
                ]),
                style: Style {
                    size: Size {
                        width: Dimension::Points(TAG_WIDTH),
                        height: Dimension::Points(ICON_TEXT_SIZE),
                    },
                    margin: Rect {
                        left: LengthPercentageAuto::Points(0.0),
                        right: LengthPercentageAuto::Points(0.0),
                        top: LengthPercentageAuto::Points(0.0),
                        bottom: LengthPercentageAuto::Points(50.0),
                    },
                    ..Default::default()
                },
            });
        }

        add_ui(&font, ui, &mut nodes, self.containment.as_ref());
        add_ui(&font, ui, &mut nodes, self.secondary.as_ref());
        add_ui(&font, ui, &mut nodes, self.disruption.as_ref());
        add_ui(&font, ui, &mut nodes, self.risk.as_ref());

        StyledNode {
            node: crate::video_gen::ui::Node::Container(nodes),
            style: Style {
                size: Size {
                    width: Dimension::Auto,
                    height: Dimension::Points(200.0),
                },
                margin: Rect {
                    left: LengthPercentageAuto::Points(120.0),
                    right: LengthPercentageAuto::Points(0.0),
                    top: LengthPercentageAuto::Points(0.0),
                    bottom: LengthPercentageAuto::Auto,
                },
                align_content: Some(AlignContent::Start),
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
        }
    }
}

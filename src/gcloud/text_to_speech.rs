use std::marker::PhantomData;

use base64::Engine;
use tokio::process::Command;

use super::Client;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SynthesisInput {
    Text(String),
    Ssml(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSelectionParams<'s> {
    language_code: &'s str,
    name: &'s str,
    ssml_gender: SsmlVoiceGender,
    custom_voice: Option<CustomVoiceParams<'s>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SsmlVoiceGender {
    SsmlVoiceGenderUnspecified,
    Male,
    Female,
    Neutral,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomVoiceParams<'s> {
    model: &'s str,
    reported_usage: ReportedUsage,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportedUsage {
    ReportedUsageUnspecified,
    Realtime,
    Offline,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    audio_encoding: AudioEncoding,
    speaking_rate: f64,
    pitch: f64,
    volume_gain_db: f64,
    sample_rate_hertz: u64,
    // effects_profile_id: [&'s str; N],
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AudioEncoding {
    AudioEncodingUnspecified,
    Linear16,
    Mp3,
    Mp3_64Kbps,
    OggOpus,
    Mulaw,
    Alaw,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisPayload<'s, L: Language> {
    input: SynthesisInput,
    #[serde(borrow)]
    voice: VoiceSelectionParams<'s>,
    audio_config: AudioConfig,
    #[serde(skip)]
    _phantom: PhantomData<fn() -> L>,
}

impl<'s, L: Language> SynthesisPayload<'s, L> {
    pub const URL: &str = "https://texttospeech.googleapis.com/v1beta1/text:synthesize";

    pub fn from_text(text: String) -> SynthesisPayload<'static, L> {
        SynthesisPayload {
            input: SynthesisInput::Text(text),
            voice: L::VOICE,
            audio_config: L::AUDIO,
            _phantom: PhantomData::default(),
        }
    }

    pub async fn synthesize(client: &mut Client, text: L) -> Vec<u8> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Response {
            audio_content: String,
        }

        let text = text.inner_string();
        let mut parts: Vec<String> = Vec::default();

        for part in text.split(".") {
            if let Some(last) = parts.last_mut() {
                if last.len() + part.len() < 1000 {
                    last.push_str(". ");
                    last.push_str(part);
                } else {
                    parts.push(part.to_owned());
                }
            } else {
                parts.push(part.to_owned());
            }
        }

        let mut audio_content = Vec::default();

        for part in parts {
            let payload = Self::from_text(part);
            let mut attempt = 0;

            let response = loop {
                if attempt > 10 {
                    panic!("Failed to create a valid text-to-speech client");
                }

                let response = client
                    .0
                    .post(Self::URL)
                    .json(&payload)
                    .send()
                    .await
                    .unwrap()
                    .json::<Response>()
                    .await;

                attempt += 1;

                match response {
                    Ok(response) => break response,
                    Err(err) => {
                        println!("Got error while trying to do text-to-speech: {err:?}");

                        let output = Command::new("gcloud")
                            .arg("auth")
                            .arg("print-access-token")
                            .output()
                            .await
                            .unwrap();
                        client
                            .remake_with_bearer_token(String::from_utf8(output.stdout).unwrap())
                            .unwrap();
                    }
                }
            };

            let mut output: Vec<u8> = (0..response.audio_content.len()).map(|_| 0).collect();
            base64::prelude::BASE64_STANDARD
                .decode_slice(&response.audio_content, &mut output)
                .unwrap();

            audio_content.extend(output);
        }

        audio_content
    }
}

pub trait Language {
    const VOICE: VoiceSelectionParams<'static>;
    const AUDIO: AudioConfig;

    fn inner_string(self) -> String;
}

pub struct EnString(pub String);

impl Language for EnString {
    const VOICE: VoiceSelectionParams<'static> = VoiceSelectionParams {
        language_code: "en-US",
        name: "en-US-Studio-M",
        ssml_gender: SsmlVoiceGender::Male,
        custom_voice: None,
    };

    const AUDIO: AudioConfig = AudioConfig {
        audio_encoding: AudioEncoding::Mp3,
        speaking_rate: 1.2,
        pitch: 0.0,
        volume_gain_db: 0.0,
        sample_rate_hertz: 24000,
    };

    fn inner_string(self) -> String {
        self.0
    }
}

pub struct HiString(pub String);

impl Language for HiString {
    const VOICE: VoiceSelectionParams<'static> = VoiceSelectionParams {
        language_code: "hi-IN",
        name: "hi-IN-Neural2-B",
        ssml_gender: SsmlVoiceGender::Male,
        custom_voice: None,
    };

    const AUDIO: AudioConfig = AudioConfig {
        audio_encoding: AudioEncoding::Mp3,
        speaking_rate: 1.0,
        pitch: 0.0,
        volume_gain_db: 0.0,
        sample_rate_hertz: 24000,
    };

    fn inner_string(self) -> String {
        self.0
    }
}

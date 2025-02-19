use std::error::Error;
use openai_api_rs::v1::api::{OpenAIClient, OpenAIClientBuilder};
use openai_api_rs::v1::audio::AudioSpeechRequest;

pub struct TTS {
    client: OpenAIClient,
    model: String,
    voice: String
}
impl TTS {
    pub fn new(api_key: String, model: String, voice: String) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client: OpenAIClientBuilder::new().with_api_key(api_key).build()?,
            model,
            voice
        })
    }

    pub async fn convert(self, text: String, out_file_path: String) -> Result<bool, String> {
        let req = AudioSpeechRequest::new(
            self.model.to_string(),
            text,
            self.voice.clone(),
            out_file_path
        );
        self.client.audio_speech(req)
            .await
            .map(|res| res.result)
            .map_err(|e| e.to_string())
    }
}

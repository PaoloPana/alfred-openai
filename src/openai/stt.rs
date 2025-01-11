use std::error::Error;
use openai_api_rs::v1::api::{OpenAIClient, OpenAIClientBuilder};
use openai_api_rs::v1::audio::AudioTranscriptionRequest;

pub struct STT {
    client: OpenAIClient,
    model: String,
    language: Option<String>,
}

impl STT {
    pub fn new(api_key: String, model: String, language: Option<String>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client: OpenAIClientBuilder::new().with_api_key(api_key).build()?,
            model,
            language
        })
    }

    pub async fn send_stt_file(self, filename: String) -> Result<String, Box<dyn Error>> {
        let mut request = AudioTranscriptionRequest::new(filename, self.model);
        if let Some(language) = self.language {
            request = request.language(language);
        }
        Ok(
            self.client
                .audio_transcription(request)
                .await?
                .text
        )
    }

    pub async fn convert(self, filename: String) -> Result<String, Box<dyn Error>> {
        self.send_stt_file(filename).await
    }
}

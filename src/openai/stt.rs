use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::audio::AudioTranscriptionRequest;

pub struct STT {
    client: OpenAIClient,
    model: String,
}

impl STT {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: OpenAIClient::new(api_key),
            model,
        }
    }

    pub async fn send_stt_file(self, filename: String) -> Result<String, Box<dyn std::error::Error>> {
        let request = AudioTranscriptionRequest::new(filename, self.model);
        Ok(
            self.client.audio_transcription(request)
                .await?
                .text
        )
    }

    pub async fn convert(self, filename: String) -> Result<String, Box<dyn std::error::Error>> {
        self.send_stt_file(filename).await
    }
}

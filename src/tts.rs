use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::audio::AudioSpeechRequest;

pub struct TTS {
    client: Client,
    model: String,
    voice: String
}
impl TTS {
    pub fn new(api_key: String, model: String, voice: String) -> TTS {
        TTS {
            client: Client::new(api_key),
            model,
            voice
        }
    }

    pub fn convert(self, text: String, out_file_path: String) -> Result<bool, String> {
        let req = AudioSpeechRequest::new(
            self.model.to_string(),
            text,
            self.voice.clone(),
            out_file_path
        );
        self.client.audio_speech(req)
            .map(|res| res.result)
            .map_err(|e| e.to_string())
    }
}

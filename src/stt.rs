use std::fs;
use reqwest::{header, multipart};
use serde::Deserialize;

pub struct STT {
    api_key: String,
    model: String
}

#[derive(Deserialize)]
struct STTRes {
    pub text: String
}
impl STT {
    pub fn new(api_key: String, model: String) -> STT {
        STT { api_key, model }
    }

    pub async fn send_stt_file(self, filename: String) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .build()?;

        let mut headers = header::HeaderMap::new();
        headers.insert("Authorization", ["Bearer ", self.api_key.as_str()].concat().parse().unwrap());

        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(fs::read(filename.clone())?).file_name(filename))
            .text("language", "it")
            .text("model", self.model.clone());

        let response: STTRes = client.request(reqwest::Method::POST, "https://api.openai.com/v1/audio/transcriptions")
            .headers(headers)
            .multipart(form)
            .send().await?
            .json().await?;
        Ok(response.text)
    }

    pub async fn convert(self, filename: String) -> Result<String, Box<dyn std::error::Error>> {
        self.send_stt_file(filename).await
    }
}

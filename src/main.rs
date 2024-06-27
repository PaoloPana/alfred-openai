mod chat;
mod stt;
mod tts;

use std::error::Error;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::log;
use alfred_rs::message::MessageType;
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::audio::{TTS_1, VOICE_ALLOY, WHISPER_1};
use openai_api_rs::v1::common::GPT3_5_TURBO;
use uuid::Uuid;
use crate::chat::Chat;
use crate::stt::STT;
use crate::tts::TTS;

const MODULE_NAME: &str = "openai";
const DEFAULT_GPT_MODEL: &str = GPT3_5_TURBO;
const STT_TOPIC: &str = "stt";
const DEFAULT_STT_MODEL: &str = WHISPER_1;
const TTS_TOPIC: &str = "tts";
const DEFAULT_TTS_MODEL: &str = TTS_1;
const DEFAULT_TTS_VOICE: &str = VOICE_ALLOY;

async fn get_chat_manager(module: &mut ServiceModule) -> Result<Chat, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let system_msg = module.config.get_module_value("system_msg".to_string())
        .unwrap_or("".to_string());
    let chat_model = module.config.get_module_value("chat_model".to_string())
        .unwrap_or(DEFAULT_GPT_MODEL.to_string());
    module.listen(MODULE_NAME.to_string()).await.expect(format!("Error during subscription to {MODULE_NAME}").as_str());
    Ok(Chat::new(openai_api_key, chat_model, system_msg))
}

fn get_stt(module: &mut ServiceModule) -> Result<Option<STT>, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let stt_model = module.config.get_module_value("stt_model".to_string())
        .unwrap_or(DEFAULT_STT_MODEL.to_string());
    Ok(Some(STT::new(openai_api_key, stt_model)))
}

fn get_tts(module: &mut ServiceModule) -> Result<Option<TTS>, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let tts_model = module.config.get_module_value("tts_model".to_string())
        .unwrap_or(DEFAULT_TTS_MODEL.to_string());
    let tts_voice = module.config.get_module_value("tts_voice".to_string())
        .unwrap_or(DEFAULT_TTS_VOICE.to_string());
    Ok(Some(TTS::new(openai_api_key, tts_model, tts_voice)))
}

async fn setup_stt(module: &mut ServiceModule) -> Result<(), Box<dyn Error>> {
    let is_stt_enable = module.config.get_module_value("enable_stt".to_string())
        .map(|s| s == "true" )
        .unwrap_or(false);
    if !is_stt_enable {
        return Ok(())
    }
    log::debug!("Loading STT...");
    module.listen(STT_TOPIC.to_string()).await
        .map_err(|e| e.into())
}

async fn setup_tts(module: &mut ServiceModule) -> Result<(), Box<dyn Error>> {
    let is_tts_enable = module.config.get_module_value("enable_tts".to_string())
        .map(|s| s == "true" )
        .unwrap_or(false);
    if !is_tts_enable {
        return Ok(())
    }
    log::debug!("Loading TTS...");
    module.listen(TTS_TOPIC.to_string()).await
        .map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut module = ServiceModule::new(MODULE_NAME.to_string()).await?;
    let mut chat_manager = get_chat_manager(&mut module).await?;
    setup_stt(&mut module).await?;
    setup_tts(&mut module).await?;

    loop {
        let (topic, mut message) = module.receive().await.unwrap();
        log::debug!("{}: {:?}", topic, message);
        match topic.as_str() {
            MODULE_NAME => {
                if message.message_type != MessageType::TEXT {
                    log::warn!("Message of type {} cannot be elaborated by {} topic", message.message_type, MODULE_NAME);
                    continue;
                }
                let response_text = chat_manager.generate_response(message.sender.clone(), message.text.clone());
                let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            },
            STT_TOPIC => {
                let stt_manager = get_stt(&mut module)?.expect("Error loading STT module");
                let response_text = stt_manager.convert(message.text.clone()).await.map_err(|e| e.to_string())?;
                let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            },
            TTS_TOPIC => {
                let tts_manager = get_tts(&mut module)?.expect("Error loading TTS module");
                let filename = format!("{}.mp3", Uuid::new_v4());
                tts_manager.convert(message.text.clone(), filename.clone())?;
                let (response_topic, response) = message.reply(filename, MessageType::AUDIO).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            }
            _ => {}
        }
    }
}

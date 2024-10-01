pub mod openai;

use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::log;
use alfred_rs::message::{Message, MessageType};
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::audio::{TTS_1, VOICE_ALLOY};
use uuid::Uuid;
use openai::tts::TTS;

const MODULE_NAME: &str = "openai_tts";
const TTS_TOPIC: &str = "tts";
const DEFAULT_TTS_MODEL: &str = TTS_1;
const DEFAULT_TTS_VOICE: &str = VOICE_ALLOY;
const TTS_STARTED_EVENT: &'static str = "tts_started";
const TTS_ENDED_EVENT: &'static str = "tts_ended";


fn get_tts(module: &mut ServiceModule) -> Result<Option<TTS>, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let tts_model = module.config.get_module_value("tts_model".to_string())
        .unwrap_or(DEFAULT_TTS_MODEL.to_string());
    let tts_voice = module.config.get_module_value("tts_voice".to_string())
        .unwrap_or(DEFAULT_TTS_VOICE.to_string());
    Ok(Some(TTS::new(openai_api_key, tts_model, tts_voice)))
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
    let config = Config::read(Some("openai".to_string()))?;
    let mut module = ServiceModule::new_with_custom_config(MODULE_NAME.to_string(), config).await?;
    setup_tts(&mut module).await?;

    loop {
        let (topic, message) = module.receive().await.unwrap();
        log::debug!("{}: {:?}", topic, message);
        match topic.as_str() {
            TTS_TOPIC => {
                module.send_event(MODULE_NAME.to_string(), TTS_STARTED_EVENT.to_string(), &Message::default()).await?;
                let tts_manager = get_tts(&mut module)?.expect("Error loading TTS module");
                let filename = format!("{}/{}.mp3", module.config.get_alfred_tmp_dir(), Uuid::new_v4());
                tts_manager.convert(message.text.clone(), filename.clone()).await?;
                module.send_event(MODULE_NAME.to_string(), TTS_ENDED_EVENT.to_string(), &Message::default()).await?;
                let (response_topic, response) = message.reply(filename, MessageType::AUDIO).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            }
            _ => {}
        }
    }
}

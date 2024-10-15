pub mod openai;

use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::{log, tokio};
use alfred_rs::message::{Message, MessageType};
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::audio::WHISPER_1;
use openai::stt::STT;

const MODULE_NAME: &str = "openai_stt";
const STT_TOPIC: &str = "stt";
const DEFAULT_STT_MODEL: &str = WHISPER_1;
const STT_STARTED_EVENT: &'static str = "stt_started";
const STT_ENDED_EVENT: &'static str = "stt_ended";

fn get_stt(module: &mut ServiceModule) -> Result<Option<STT>, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key")
        .ok_or("openai_api_key needed")?;
    let stt_model = module.config.get_module_value("stt_model")
        .unwrap_or(DEFAULT_STT_MODEL.to_string());
    Ok(Some(STT::new(openai_api_key, stt_model)))
}

async fn setup_stt(module: &mut ServiceModule) -> Result<(), Box<dyn Error>> {
    let is_stt_enable = module.config.get_module_value("enable_stt")
        .map(|s| s == "true" )
        .unwrap_or(false);
    if !is_stt_enable {
        return Ok(())
    }
    log::debug!("Loading STT...");
    module.listen(STT_TOPIC).await
        .map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some("openai"))?;
    let mut module = ServiceModule::new_with_custom_config(MODULE_NAME, config).await?;
    setup_stt(&mut module).await?;

    loop {
        let (topic, message) = module.receive().await.unwrap();
        log::debug!("{}: {:?}", topic, message);
        match topic.as_str() {
            STT_TOPIC => {
                let (response_text, response_type) = match message.message_type {
                    MessageType::AUDIO => {
                        module.send_event(MODULE_NAME, STT_STARTED_EVENT, &Message::default()).await?;
                        let stt_manager = get_stt(&mut module)?.expect("Error loading STT module");
                        let response_text = stt_manager.convert(message.text.clone()).await.map_err(|e| e.to_string())?;
                        module.send_event(MODULE_NAME, STT_ENDED_EVENT, &Message::default()).await?;
                        (response_text, MessageType::TEXT)
                    }
                    _ => {
                        (message.text.clone(), message.message_type)
                    }
                };
                let (response_topic, response) = message.reply(response_text, response_type).expect("Error on create response");
                module.send(&response_topic, &response).await.expect("Error on publish");
            },
            _ => {}
        }
    }
}

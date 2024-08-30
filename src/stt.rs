pub mod openai;

use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::log;
use alfred_rs::message::MessageType;
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::audio::{WHISPER_1};
use openai::stt::STT;

const MODULE_NAME: &str = "openai-stt";
const STT_TOPIC: &str = "stt";
const DEFAULT_STT_MODEL: &str = WHISPER_1;

fn get_stt(module: &mut ServiceModule) -> Result<Option<STT>, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let stt_model = module.config.get_module_value("stt_model".to_string())
        .unwrap_or(DEFAULT_STT_MODEL.to_string());
    Ok(Some(STT::new(openai_api_key, stt_model)))
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some("openai".to_string()))?;
    let mut module = ServiceModule::new_with_custom_config(MODULE_NAME.to_string(), config).await?;
    setup_stt(&mut module).await?;

    loop {
        let (topic, mut message) = module.receive().await.unwrap();
        log::debug!("{}: {:?}", topic, message);
        match topic.as_str() {
            STT_TOPIC => {
                let stt_manager = get_stt(&mut module)?.expect("Error loading STT module");
                let response_text = stt_manager.convert(message.text.clone()).await.map_err(|e| e.to_string())?;
                let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            },
            _ => {}
        }
    }
}

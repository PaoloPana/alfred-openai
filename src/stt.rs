pub mod openai;

use std::error::Error;
use alfred_core::config::Config;
use alfred_core::{log, tokio, ModuleDetailsBuilder};
use alfred_core::message::{Message, MessageType};
use alfred_core::AlfredModule;
use openai_api_rs::v1::audio::WHISPER_1;
use openai::stt::STT;

const MODULE_NAME: &str = "openai_stt";
const STT_TOPIC: &str = "stt";
const DEFAULT_STT_MODEL: &str = WHISPER_1;
const STT_STARTED_EVENT: &str = "stt_started";
const STT_ENDED_EVENT: &str = "stt_ended";

fn get_stt(module: &AlfredModule) -> Result<STT, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key")
        .ok_or("openai_api_key needed")?;
    let stt_model = module.config.get_module_value("stt_model")
        .unwrap_or_else(|| DEFAULT_STT_MODEL.to_string());
    let language = module.config.get_module_value("stt_language");
    STT::new(openai_api_key, stt_model, language)
}

async fn setup_stt(module: &mut AlfredModule) -> Result<(), Box<dyn Error>> {
    let is_stt_enable = module.config.get_module_value("enable_stt")
        .is_some_and(|s| s == "true" );
    if !is_stt_enable {
        return Ok(())
    }
    log::debug!("Loading STT...");
    module.listen(STT_TOPIC).await
        .map_err(Into::into)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some("openai"));
    let module_details = ModuleDetailsBuilder::new()
        .module_name(MODULE_NAME)
        .version(env!("CARGO_PKG_VERSION"))
        .config(Some(config))
        .build();
    let mut module = AlfredModule::new_with_details(module_details).await?;
    setup_stt(&mut module).await?;

    loop {
        let (topic, message) = module.receive().await?;
        log::debug!("{}: {:?}", topic, message);
        if topic == STT_TOPIC {
            let (response_text, response_type) = match message.message_type {
                MessageType::Audio => {
                    module.send_event(MODULE_NAME, STT_STARTED_EVENT, &Message::default()).await?;
                    let stt_manager = get_stt(&module)?;
                    let response_text = stt_manager.convert(message.text.clone()).await.map_err(|e| e.to_string())?;
                    module.send_event(MODULE_NAME, STT_ENDED_EVENT, &Message::default()).await?;
                    (response_text, MessageType::Text)
                }
                MessageType::Unknown | MessageType::Text | MessageType::Photo | MessageType::ModuleInfo => {
                    (message.text.clone(), message.message_type.clone())
                }
            };
            let (response_topic, response) = message.reply(response_text, response_type).expect("Error on create response");
            module.send(&response_topic, &response).await.expect("Error on publish");
        }
    }
}

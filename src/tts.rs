pub mod openai;

use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::{log, tokio, ModuleDetailsBuilder};
use alfred_rs::message::{Message, MessageType};
use alfred_rs::AlfredModule;
use openai_api_rs::v1::audio::{TTS_1, VOICE_ALLOY};
use uuid::Uuid;
use openai::tts::TTS;

const MODULE_NAME: &str = "openai_tts";
const TTS_TOPIC: &str = "tts";
const DEFAULT_TTS_MODEL: &str = TTS_1;
const DEFAULT_TTS_VOICE: &str = VOICE_ALLOY;
const TTS_STARTED_EVENT: &str = "tts_started";
const TTS_ENDED_EVENT: &str = "tts_ended";


fn get_tts(module: &AlfredModule) -> Result<TTS, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key")
        .ok_or("openai_api_key needed")?;
    let tts_model = module.config.get_module_value("tts_model")
        .unwrap_or_else(|| DEFAULT_TTS_MODEL.to_string());
    let tts_voice = module.config.get_module_value("tts_voice")
        .unwrap_or_else(|| DEFAULT_TTS_VOICE.to_string());
    TTS::new(openai_api_key, tts_model, tts_voice)
}

async fn setup_tts(module: &mut AlfredModule) -> Result<(), Box<dyn Error>> {
    let is_tts_enable = module.config.get_module_value("enable_tts")
        .is_some_and(|s| s == "true" );
    if !is_tts_enable {
        return Ok(())
    }
    log::debug!("Loading TTS...");
    module.listen(TTS_TOPIC).await
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
    setup_tts(&mut module).await?;

    loop {
        let (topic, message) = module.receive().await?;
        log::debug!("{}: {:?}", topic, message);
        if topic == TTS_TOPIC {
            module.send_event(MODULE_NAME, TTS_STARTED_EVENT, &Message::default()).await?;
            let tts_manager = get_tts(&module)?;
            let filename = format!("{}/{}.mp3", module.config.alfred.tmp_dir, Uuid::new_v4());
            tts_manager.convert(message.text.clone(), filename.clone()).await?;
            module.send_event(MODULE_NAME, TTS_ENDED_EVENT, &Message::default()).await?;
            let (response_topic, response) = message.reply(filename, MessageType::Audio).expect("Error on create response");
            module.send(&response_topic, &response).await.expect("Error on publish");
        }
    }
}

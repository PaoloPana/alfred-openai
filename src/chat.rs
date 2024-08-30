pub mod openai;

use openai::chat::Chat;
use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::log;
use alfred_rs::message::MessageType;
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::common::GPT4_O;

const MODULE_NAME: &str = "openai-chat";
const DEFAULT_GPT_MODEL: &str = GPT4_O;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some("openai".to_string()))?;
    let mut module = ServiceModule::new_with_custom_config(MODULE_NAME.to_string(), config).await?;
    let mut chat_manager = get_chat_manager(&mut module).await?;

    loop {
        let (topic, mut message) = module.receive().await.unwrap();
        log::debug!("{}: {:?}", topic, message);
        match topic.as_str() {
            MODULE_NAME => {
                if message.message_type != MessageType::TEXT {
                    log::warn!("Message of type {} cannot be elaborated by {} topic", message.message_type, MODULE_NAME);
                    continue;
                }
                let response_text = chat_manager.generate_response(message.sender.clone(), message.text.clone()).await;
                let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
                module.send(response_topic, &response).await.expect("Error on publish");
            },
            _ => {}
        }
    }
}

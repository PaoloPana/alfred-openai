mod chat;

use std::error::Error;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::log::debug;
use alfred_rs::message::MessageType;
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::common::GPT3_5_TURBO;
use crate::chat::Chat;

const MODULE_NAME: &str = "openai";

fn get_chat_manager(module: &ServiceModule) -> Result<Chat, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key".to_string())
        .ok_or("openai_api_key needed")?;
    let system_msg = module.config.get_module_value("system_msg".to_string())
        .unwrap_or("".to_string());
    let chat_model = module.config.get_module_value("chat_model".to_string())
        .unwrap_or(GPT3_5_TURBO.to_string());
    Ok(Chat::new(openai_api_key, chat_model, system_msg))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut module = ServiceModule::new(MODULE_NAME.to_string()).await?;
    module.listen(MODULE_NAME.to_string()).await.expect(format!("Error during subscription to {MODULE_NAME}").as_str());
    let mut chat_manager = get_chat_manager(&module)?;

    loop {
        let (topic, mut message) = module.receive().await.unwrap();
        //if alfred.manage_module_info_request(topic, MODULE_NAME.to_string()).await { continue }
        debug!("{}: {:?}", topic, message);
        let response_text = chat_manager.generate_response(message.sender.clone(), message.text.clone());
        let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
        module.send(response_topic, &response).await.expect("Error on publish");
    }
}

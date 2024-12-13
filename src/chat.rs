pub mod openai;

use openai::chat::Chat;
use std::error::Error;
use alfred_rs::config::Config;
use alfred_rs::{log, tokio};
use alfred_rs::log::warn;
use alfred_rs::message::{Message, MessageType};
use alfred_rs::AlfredModule;
use alfred_rs::connection::{MODULE_INFO_TOPIC_REQUEST, MODULE_INFO_TOPIC_RESPONSE};
use openai_api_rs::v1::common::GPT4_O;

const MODULE_NAME: &str = "openai_chat";
const INPUT_TOPIC: &str = "chat";
const INPUT_DEBUG_TOPIC: &str = "chat.debug";
const DEFAULT_GPT_MODEL: &str = GPT4_O;
const CHAT_STARTED_EVENT: &str = "chat_started";

const CHAT_ENDED_EVENT: &str = "chat_ended";

async fn get_chat_manager(module: &mut AlfredModule) -> Result<Chat, Box<dyn Error>> {
    let openai_api_key = module.config.get_module_value("openai_api_key")
        .ok_or("openai_api_key needed")?;
    let system_msg_intro = module.config.get_module_value("system_msg")
        .unwrap_or_default();
    let chat_model = module.config.get_module_value("chat_model")
        .unwrap_or_else(|| DEFAULT_GPT_MODEL.to_string());
    module.listen(INPUT_TOPIC).await?;
    module.listen(INPUT_DEBUG_TOPIC).await?;
    module.listen(MODULE_INFO_TOPIC_RESPONSE).await?;
    module.send(MODULE_INFO_TOPIC_REQUEST, &Message::default()).await?;
    Chat::new(openai_api_key, chat_model, system_msg_intro)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config = Config::read(Some("openai"));
    let mut module = AlfredModule::new_with_details(MODULE_NAME, env!("CARGO_PKG_VERSION"), Some(config), None).await?;
    let mut chat_manager = get_chat_manager(&mut module).await?;

    loop {
        if let Err(e) = chat_handler(&mut module, &mut chat_manager).await {
            warn!("Error while handling chat: {}", e);
        }
    }
}

async fn chat_handler(module: &mut AlfredModule, chat_manager: &mut Chat) -> Result<(), Box<dyn Error>> {
    let (topic, message) = module.receive().await?;
    log::debug!("{}: {:?}", topic, message);
    match topic.as_str() {
        INPUT_TOPIC => {
            if message.message_type != MessageType::Text {
                return Err(format!("Message of type {} cannot be elaborated by {} topic", message.message_type, MODULE_NAME))?;
            }
            module.send_event(MODULE_NAME, CHAT_STARTED_EVENT, &Message::default()).await?;
            let response_text = chat_manager.generate_response(message.sender.clone(), message.text.clone()).await?;
            module.send_event(MODULE_NAME, CHAT_ENDED_EVENT, &Message::default()).await?;
            let (response_topic, response) = message.reply(response_text, MessageType::Text).expect("Error on create response");
            module.send(&response_topic, &response).await.expect("Error on publish");
            Ok(())
        },
        INPUT_DEBUG_TOPIC => {
            let text = chat_manager.get_capabilities().iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<String>>()
                .join(", ");
            let (resp_topic, resp_message) = message.reply(text, MessageType::Text).expect("Error on create message");
            module.send(&resp_topic, &resp_message).await.expect("Error on publish");
            Ok(())
        },
        MODULE_INFO_TOPIC_RESPONSE => {
            message.params.iter().for_each(|(cap, msg)| {
                log::debug!("Adding capability {cap} ({msg})");
                chat_manager.update_capability(cap, msg);
            });
            Ok(())
        },
        _ => {
            Err(format!("Topic {topic} unknown"))?
        }
    }
}

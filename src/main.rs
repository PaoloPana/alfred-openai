use std::collections::HashMap;
use alfred_rs::connection::{Receiver, Sender};
use alfred_rs::error::Error;
use alfred_rs::message::MessageType;
use alfred_rs::service_module::ServiceModule;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion;
use openai_api_rs::v1::chat_completion::{ChatCompletionMessage, ChatCompletionRequest};

const MODULE_NAME: &str = "openai";
const GPT_MODEL: &str = openai_api_rs::v1::common::GPT3_5_TURBO;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let mut module = ServiceModule::new(MODULE_NAME.to_string()).await?;
    let openai_token = module.config.get_module_value("openai_token".to_string())
        .expect("OPENAI_TOKEN needed");
    let system_msg = module.config.get_module_value("system_msg".to_string())
        .unwrap_or("".to_string());
    module.listen("openai".to_string()).await.expect("Error during subscription to echo");
    let mut users_history: HashMap<String, Vec<ChatCompletionMessage>> = HashMap::new();
    let client = Client::new(openai_token);

    loop {
        let (_, mut message) = module.receive().await.unwrap();
        //if alfred.manage_module_info_request(topic, MODULE_NAME.to_string()).await { continue }
        println!("{:?}", message);
        let response_text = generate_response(&mut users_history, message.sender.clone(), message.text.clone(), &client, system_msg.clone());
        let (response_topic, response) = message.reply(response_text, MessageType::TEXT).expect("Error on create response");
        module.send(response_topic, &response).await.expect("Error on publish");
    }
}

fn generate_response(users_history: &mut HashMap<String, Vec<ChatCompletionMessage>>, user: String, text: String, client: &Client, system_msg: String) -> String {
    if !users_history.contains_key(&user.to_string()) {
        users_history.insert(user.clone().to_string(), vec![generate_system_msg(system_msg)]);
    }
    let history: &mut Vec<ChatCompletionMessage> = users_history.get_mut(&user.clone()).unwrap();

    history.push(ChatCompletionMessage {
        role: chat_completion::MessageRole::user,
        content: chat_completion::Content::Text(String::from(text.clone())),
        name: None,
    });
    let req = ChatCompletionRequest::new(GPT_MODEL.to_string(), history.to_vec());
    let result = client.chat_completion(req).unwrap();
    let response_text = <Option<String> as Clone>::clone(&result.choices[0].message.content).expect("No message received");
    println!("Content: {:?}", response_text);
    history.push(ChatCompletionMessage {
        role: chat_completion::MessageRole::assistant,
        content: chat_completion::Content::Text(String::from(response_text.clone())),
        name: None,
    });

    response_text
}

fn generate_system_msg(system_msg: String) -> ChatCompletionMessage {
    ChatCompletionMessage {
        role: chat_completion::MessageRole::system,
        content: chat_completion::Content::Text(system_msg),
        name: None,
    }
}

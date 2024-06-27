use std::collections::HashMap;
use alfred_rs::log::debug;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion;
use openai_api_rs::v1::chat_completion::{ChatCompletionMessage, ChatCompletionRequest};

pub struct Chat {
    users_history: HashMap<String, Vec<ChatCompletionMessage>>,
    client: Client,
    chat_model: String,
    system_msg: String
}
impl Chat {
    pub fn new(api_key: String, chat_model: String, system_msg: String) -> Chat {
        Chat {
            users_history: HashMap::new(),
            client: Client::new(api_key),
            chat_model,
            system_msg
        }
    }

    pub fn generate_response(&mut self, user: String, text: String) -> String {
        if !self.users_history.contains_key(&user.to_string()) {
            self.users_history.insert(user.clone().to_string(), vec![generate_system_msg(self.system_msg.clone())]);
        }
        let history: &mut Vec<ChatCompletionMessage> = self.users_history.get_mut(&user.clone()).unwrap();

        history.push(ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(String::from(text.clone())),
            name: None,
        });
        let req = ChatCompletionRequest::new(self.chat_model.clone(), history.to_vec());
        let result = self.client.chat_completion(req).unwrap();
        let response_text = result.choices.get(0).unwrap().message.content.clone().expect("No message received");
        debug!("Content: {:?}", response_text);
        history.push(ChatCompletionMessage {
            role: chat_completion::MessageRole::assistant,
            content: chat_completion::Content::Text(String::from(response_text.clone())),
            name: None,
        });

        response_text
    }


}

fn generate_system_msg(system_msg: String) -> ChatCompletionMessage {
    ChatCompletionMessage {
        role: chat_completion::MessageRole::system,
        content: chat_completion::Content::Text(system_msg),
        name: None,
    }
}

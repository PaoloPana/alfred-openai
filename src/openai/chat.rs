use std::collections::HashMap;
use std::error::Error;
use alfred_rs::log::debug;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion;
use openai_api_rs::v1::chat_completion::{ChatCompletionMessage, ChatCompletionRequest};

pub struct Chat {
    users_history: HashMap<String, Vec<ChatCompletionMessage>>,
    client: OpenAIClient,
    chat_model: String,
    system_msg: String
}
impl Chat {
    pub fn new(api_key: String, chat_model: String, system_msg: String) -> Self {
        Self {
            users_history: HashMap::new(),
            client: OpenAIClient::new(api_key),
            chat_model,
            system_msg
        }
    }

    pub async fn generate_response(&mut self, user: String, text: String) -> Result<String, Box<dyn Error>> {
        if !self.users_history.contains_key(&user.to_string()) {
            self.users_history.insert(user.clone(), vec![generate_system_msg(self.system_msg.clone())]);
        }
        let history = self.users_history.get_mut(&user.clone()).ok_or("User not found")?;

        history.push(ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(text.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
        let req = ChatCompletionRequest::new(self.chat_model.clone(), history.clone());
        let result = self.client.chat_completion(req).await?;
        let response_text = result.choices.first()
            .ok_or("choices array not found in OpenAI response")?
            .message.content.clone()
            .ok_or("No message received")?;
        debug!("Content: {:?}", response_text);
        history.push(ChatCompletionMessage {
            role: chat_completion::MessageRole::assistant,
            content: chat_completion::Content::Text(response_text.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        Ok(response_text)
    }


}

const fn generate_system_msg(system_msg: String) -> ChatCompletionMessage {
    ChatCompletionMessage {
        role: chat_completion::MessageRole::system,
        content: chat_completion::Content::Text(system_msg),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

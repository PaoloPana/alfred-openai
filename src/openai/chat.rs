use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use alfred_core::log::debug;
use openai_api_rs::v1::api::{OpenAIClient, OpenAIClientBuilder};
use openai_api_rs::v1::chat_completion;
use openai_api_rs::v1::chat_completion::{ChatCompletionMessage, ChatCompletionRequest};

pub struct SystemMsg {
    intro: String,
    capabilities: HashMap<String, String>,
}
impl SystemMsg {
    pub fn new(intro: String) -> Self {
        Self { intro, capabilities: HashMap::new() }
    }
    pub fn update_capability(&mut self, cap: &str, msg: &str) {
        self.capabilities.insert(cap.to_string(), msg.to_string());
    }

    fn map_capability(cap: &String, msg: &String) -> String {
        format!("{{ \"cap\": \"{cap}\", \"msg\": \"{msg}\" }}")
    }
}

impl Display for SystemMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n```\n[{}]\n```",
            self.intro,
            self.capabilities.iter().map(|(k, v)| Self::map_capability(k, v)).collect::<Vec<_>>().join(",\n")
        )
    }
}

pub struct Chat {
    users_history: HashMap<String, Vec<ChatCompletionMessage>>,
    client: OpenAIClient,
    chat_model: String,
    system_msg: SystemMsg
}
impl Chat {
    pub fn new(api_key: String, chat_model: String, system_msg_intro: String) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            users_history: HashMap::new(),
            client: OpenAIClientBuilder::new().with_api_key(api_key).build()?,
            chat_model,
            system_msg: SystemMsg::new(system_msg_intro)
        })
    }

    pub fn update_capability(&mut self, capability: &str, message: &str) {
        self.system_msg.update_capability(capability, message);
    }

    pub const fn get_capabilities(&self) -> &HashMap<String, String> {
        &self.system_msg.capabilities
    }

    pub async fn generate_response(&mut self, user: String, text: String) -> Result<String, Box<dyn Error>> {
        if !self.users_history.contains_key(&user.to_string()) {
            self.users_history.insert(user.clone(), vec![]);
        }
        let history = self.users_history.get_mut(&user.clone()).ok_or("User not found")?;

        history.push(ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(text.clone()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
        let mut messages = vec![generate_system_msg(self.system_msg.to_string())];
        messages.append(&mut history.clone());
        let req = ChatCompletionRequest::new(
            self.chat_model.clone(),
            messages
        );
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

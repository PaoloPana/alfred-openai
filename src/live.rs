pub mod openai;

use std::error::Error;
use std::time::Duration;
use alfred_core::config::Config;
use alfred_core::connection::Connection;
use alfred_core::{log, tokio, ModuleDetailsBuilder};
use alfred_core::message::{Message, MessageType};
use alfred_core::tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use alfred_core::AlfredModule;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openai::live::{LiveCommand, LiveConfig, LiveEvent};
use uuid::Uuid;

const MODULE_NAME: &str = "openai_live";
const LIVE_TOPIC: &str = "live";
/// What the user is saying, as it is recognised.
const TRANSCRIPT_EVENT: &str = "transcript";
/// What the model is saying.
const RESPONSE_EVENT: &str = "response";
/// The model handed the turn over: the text is the user's request, to be answered.
const DELEGATION_EVENT: &str = "delegation";
/// The model's speech, to be played.
const AUDIO_EVENT: &str = "audio";
const DEFAULT_VOICE: &str = "marin";
const DEFAULT_SAMPLE_RATE: u32 = 16_000;
const DEFAULT_QUIET_TIMEOUT_SECS: u64 = 8;

fn get_live_config(module: &AlfredModule) -> Result<LiveConfig, Box<dyn Error>> {
    let api_key = module.config.get_module_value("openai_api_key")
        .ok_or("openai_api_key needed")?;
    let instructions = module.config.get_module_value("live_instructions")
        .ok_or("live_instructions needed")?;
    let voice = module.config.get_module_value("live_voice")
        .unwrap_or_else(|| DEFAULT_VOICE.to_string());
    let sample_rate = module.config.get_module_value("live_sample_rate")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SAMPLE_RATE);
    let quiet_timeout = module.config.get_module_value("live_quiet_timeout")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_QUIET_TIMEOUT_SECS);
    Ok(LiveConfig {
        api_key,
        voice,
        sample_rate,
        instructions,
        quiet_timeout: Duration::from_secs(quiet_timeout),
    })
}

async fn setup_live(module: &mut AlfredModule) -> Result<(), Box<dyn Error>> {
    let is_live_enable = module.config.get_module_value("enable_live")
        .is_some_and(|value| value == "true");
    if !is_live_enable {
        return Ok(());
    }
    log::debug!("Loading Live...");
    module.listen(LIVE_TOPIC).await
        .map_err(Into::into)
}

fn start_session(module: &AlfredModule) -> Result<UnboundedSender<LiveCommand>, Box<dyn Error>> {
    let config = get_live_config(module)?;
    let sample_rate = config.sample_rate;
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        if let Err(err) = openai::live::run(config, command_rx, &event_tx).await {
            log::error!("Live session failed: {err}");
        }
    });
    tokio::spawn(publish_events(module.connection.clone(), event_rx, sample_rate));
    Ok(command_tx)
}

/// Publishes everything the session produces, then closes the audio stream so the
/// player knows the speech is over.
async fn publish_events(connection: Connection, mut events: UnboundedReceiver<LiveEvent>, sample_rate: u32) {
    let stream_id = format!("{MODULE_NAME}-{}", Uuid::new_v4());
    let mut transcript_sequence = 0;
    let mut response_sequence = 0;
    let mut audio_sequence = 0;

    while let Some(event) = events.recv().await {
        let (event_name, message) = match event {
            LiveEvent::UserTranscript(delta) => {
                transcript_sequence += 1;
                (TRANSCRIPT_EVENT, text_chunk(delta, &stream_id, transcript_sequence - 1))
            }
            LiveEvent::AssistantTranscript(delta) => {
                response_sequence += 1;
                (RESPONSE_EVENT, text_chunk(delta, &stream_id, response_sequence - 1))
            }
            LiveEvent::Audio(pcm) => {
                audio_sequence += 1;
                (AUDIO_EVENT, audio_chunk(&pcm, &stream_id, audio_sequence - 1, sample_rate, false))
            }
            LiveEvent::Delegation(question) => {
                (DELEGATION_EVENT, Message { text: question, message_type: MessageType::Text, ..Message::default() })
            }
        };
        if let Err(err) = connection.send_event(MODULE_NAME, event_name, &message).await {
            log::error!("Cannot publish {event_name}: {err}");
        }
    }

    let end_of_audio = audio_chunk(&[], &stream_id, audio_sequence, sample_rate, true);
    if let Err(err) = connection.send_event(MODULE_NAME, AUDIO_EVENT, &end_of_audio).await {
        log::error!("Cannot publish the end of {AUDIO_EVENT}: {err}");
    }
}

fn text_chunk(text: String, stream_id: &str, sequence: u32) -> Message {
    Message {
        text,
        message_type: MessageType::StreamText,
        stream_id: stream_id.to_string(),
        sequence,
        ..Message::default()
    }
}

fn audio_chunk(pcm: &[u8], stream_id: &str, sequence: u32, sample_rate: u32, is_final: bool) -> Message {
    let mut message = Message {
        text: BASE64.encode(pcm),
        message_type: MessageType::StreamAudio,
        stream_id: stream_id.to_string(),
        sequence,
        is_final,
        ..Message::default()
    };
    message.params.insert("sample_rate".to_string(), sample_rate.to_string());
    message.params.insert("channels".to_string(), "1".to_string());
    message
}

fn handle_message(module: &AlfredModule, session: &mut Option<UnboundedSender<LiveCommand>>, message: &Message) -> Result<(), Box<dyn Error>> {
    match message.message_type {
        MessageType::StreamAudio => {
            let live = match session.take() {
                Some(live) if !live.is_closed() => live,
                _ => start_session(module)?,
            };
            if !message.text.is_empty() {
                live.send(LiveCommand::Audio(BASE64.decode(&message.text)?))
                    .map_err(|_| "Live session is gone")?;
            }
            if message.is_final {
                live.send(LiveCommand::InputEnded).map_err(|_| "Live session is gone")?;
            }
            *session = Some(live);
            Ok(())
        }
        MessageType::Text => {
            let Some(live) = session.as_ref().filter(|live| !live.is_closed()) else {
                return Err("No Live session is waiting for an answer".into());
            };
            live.send(LiveCommand::Answer(message.text.clone()))
                .map_err(|_| "Live session is gone")?;
            Ok(())
        }
        MessageType::Unknown | MessageType::Audio | MessageType::Photo | MessageType::StreamText
        | MessageType::StreamPhoto | MessageType::ModuleInfo => {
            Err(format!("Message of type {} cannot be elaborated by {MODULE_NAME}", message.message_type).into())
        }
    }
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
    setup_live(&mut module).await?;
    let mut session = None;

    loop {
        let (topic, message) = module.receive().await?;
        log::debug!("{topic}: {message:?}");
        if topic == LIVE_TOPIC {
            if let Err(err) = handle_message(&module, &mut session, &message) {
                log::warn!("Cannot handle message: {err}");
            }
        }
    }
}

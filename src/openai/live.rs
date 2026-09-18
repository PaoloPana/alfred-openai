use std::error::Error;
use std::time::{Duration, Instant};
use alfred_core::log::{debug, warn};
use alfred_core::tokio;
use alfred_core::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use oai_rt_rs::live::{
    AudioConfig, AudioFormat, AudioOutput, ClientEvent, Command, DelegationConfig, Field,
    LiveClient, Nullable, ServerEvent, ServerFrame, SessionConfig, Voice,
};

/// One context append is capped at 500 tokens, so long answers go in several pieces.
const MAX_APPEND_CHARS: usize = 800;
/// Only ticks the loop, so the quiet timeout is checked while no event arrives.
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const CLOSE_DEADLINE: Duration = Duration::from_secs(5);

pub struct LiveConfig {
    pub api_key: String,
    pub voice: String,
    pub sample_rate: u32,
    pub instructions: String,
    /// A session is billed per second while it stays open, silence included, so it is
    /// closed this long after the microphone stopped and the answer went quiet.
    pub quiet_timeout: Duration,
}

pub enum LiveCommand {
    /// Raw PCM16 mono microphone audio.
    Audio(Vec<u8>),
    /// The answer to the pending delegation, for the model to say aloud.
    Answer(String),
    /// No more microphone audio will follow.
    InputEnded,
}

pub enum LiveEvent {
    UserTranscript(String),
    AssistantTranscript(String),
    /// Raw PCM16 mono audio to play.
    Audio(Vec<u8>),
    /// The model handed the turn over; carries what the user asked for.
    Delegation(String),
}

/// Runs one Live session until the conversation goes quiet or the commands stop:
/// microphone audio goes in, transcripts, delegations and speech come out.
pub async fn run(
    config: LiveConfig,
    mut commands: UnboundedReceiver<LiveCommand>,
    events: &UnboundedSender<LiveEvent>,
) -> Result<(), Box<dyn Error>> {
    let client = LiveClient::new(&config.api_key)?;
    let session = SessionConfig {
        audio: Some(AudioConfig {
            format: Some(AudioFormat::Pcm { rate: config.sample_rate }),
            output: Some(AudioOutput { voice: Some(Voice::Named(config.voice)) }),
        }),
        delegation: Field::Value(DelegationConfig::Client),
        instructions: Field::Value(config.instructions),
        ..SessionConfig::default()
    };
    let mut connection = client.connect(session).await?;
    let sender = connection.sender();

    let mut question = String::new();
    let mut delegation_id = None;
    let mut input_ended = false;
    let mut last_output = Instant::now();

    loop {
        if input_ended && last_output.elapsed() > config.quiet_timeout {
            debug!("Live session went quiet, closing it");
            break;
        }
        tokio::select! {
            command = commands.recv() => {
                match command {
                    Some(LiveCommand::Audio(pcm)) => sender.send_audio(&pcm).await?,
                    Some(LiveCommand::Answer(answer)) => {
                        for piece in split_append(&answer) {
                            sender.send(ClientEvent::new(Command::CommentaryAppend {
                                content: piece,
                                delegation_id: Nullable(delegation_id.clone()),
                            })).await?;
                        }
                    }
                    Some(LiveCommand::InputEnded) => {
                        input_ended = true;
                        last_output = Instant::now();
                    }
                    None => break,
                }
            }
            frame = tokio::time::timeout(POLL_INTERVAL, connection.next_event()) => {
                let Ok(frame) = frame else { continue };
                let Some(ServerFrame { event, raw, .. }) = frame? else { break };
                match event {
                    ServerEvent::InputTranscriptDelta { delta, .. } => {
                        question.push_str(&delta);
                        send_event(events, LiveEvent::UserTranscript(delta));
                    }
                    ServerEvent::OutputTranscriptDelta { delta, .. } => {
                        last_output = Instant::now();
                        send_event(events, LiveEvent::AssistantTranscript(delta));
                    }
                    ServerEvent::OutputAudioDelta { delta, .. } => {
                        last_output = Instant::now();
                        send_event(events, LiveEvent::Audio(BASE64.decode(delta)?));
                    }
                    // the delegation carries no text: what was asked comes from the transcript
                    ServerEvent::DelegationCreated { delegation, .. } => {
                        delegation_id = Some(delegation.id);
                        send_event(events, LiveEvent::Delegation(question.trim().to_string()));
                        question.clear();
                    }
                    ServerEvent::Error { .. } => warn!("Live session error: {raw}"),
                    ServerEvent::Closed { .. } => break,
                    ServerEvent::Started { .. } | ServerEvent::Updated { .. }
                    | ServerEvent::InputAudioMuted { .. } | ServerEvent::InputAudioUnmuted { .. }
                    | ServerEvent::InstructionsAppended { .. } | ServerEvent::ThinkingAppended { .. }
                    | ServerEvent::CommentaryAppended { .. } | ServerEvent::InputAudio { .. }
                    | ServerEvent::Response { .. } | ServerEvent::UsageUpdated { .. }
                    | ServerEvent::Info { .. } | ServerEvent::DtmfReceived { .. }
                    | ServerEvent::DtmfSend { .. } | ServerEvent::Ringing { .. }
                    | ServerEvent::Answered { .. } | ServerEvent::TransportFailed { .. }
                    | ServerEvent::Unknown => {}
                }
            }
        }
    }

    connection.close(CLOSE_DEADLINE, |_| Ok(())).await?;
    Ok(())
}

fn send_event(events: &UnboundedSender<LiveEvent>, event: LiveEvent) {
    if events.send(event).is_err() {
        warn!("Nobody is listening to the Live session events");
    }
}

/// Splits an answer at whitespace into pieces small enough for one context append.
fn split_append(answer: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut piece = String::new();
    for word in answer.split_whitespace() {
        if !piece.is_empty() && piece.len() + 1 + word.len() > MAX_APPEND_CHARS {
            pieces.push(std::mem::take(&mut piece));
        }
        if !piece.is_empty() {
            piece.push(' ');
        }
        piece.push_str(word);
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::{split_append, MAX_APPEND_CHARS};

    #[test]
    fn short_answer_stays_in_one_piece() {
        let pieces = split_append("Luce accesa, ci sono ventidue gradi.");
        assert_eq!(pieces, vec!["Luce accesa, ci sono ventidue gradi."]);
    }

    #[test]
    fn long_answer_is_split_at_whitespace() {
        let answer = "parola ".repeat(300);
        let pieces = split_append(&answer);
        assert!(pieces.len() > 1);
        assert!(pieces.iter().all(|piece| piece.len() <= MAX_APPEND_CHARS));
        assert_eq!(pieces.join(" "), answer.trim());
    }

    #[test]
    fn empty_answer_produces_nothing() {
        assert!(split_append("   ").is_empty());
    }
}

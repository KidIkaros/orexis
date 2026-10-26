//! The turn loop: a use case over core + ports. No adapter, no runtime.

use orexis_core::{derive_messages, hash_messages, Action, CodecId, SessionEvent, FORMAT_VERSION};
use orexis_ports::{EventSink, PortResult, Provider, SessionAppend, SessionRead, ToolExecutor};

pub struct TurnDeps<'a> {
    pub provider: &'a dyn Provider,
    pub tools: &'a dyn ToolExecutor,
    pub session: &'a dyn SessionAppend,
    pub reader: &'a dyn SessionRead,
    pub sink: &'a dyn EventSink,
    pub session_id: &'a str,
    pub prompt: &'a str,
    pub max_steps: usize,
}

pub struct TurnOutcome {
    pub steps: usize,
    pub done: bool,
}

pub async fn run_turn(deps: TurnDeps<'_>) -> PortResult<TurnOutcome> {
    let TurnDeps {
        provider,
        tools,
        session,
        reader,
        sink,
        session_id,
        prompt,
        max_steps,
    } = deps;

    append(
        session,
        sink,
        SessionEvent::SessionStart {
            session_id: session_id.to_string(),
            format_version: FORMAT_VERSION,
        },
    )
    .await?;
    append(
        session,
        sink,
        SessionEvent::UserMessage {
            text: prompt.to_string(),
        },
    )
    .await?;

    let mut done = false;
    let mut steps = 0;

    for step in 0..max_steps {
        steps = step + 1;

        let events = reader
            .events()
            .await?
            .into_iter()
            .map(|stored| stored.event)
            .collect::<Vec<_>>();
        let messages = derive_messages(&events);
        append(
            session,
            sink,
            SessionEvent::ModelRequest {
                profile_id: "fake".to_string(),
                request_hash: hash_messages(&messages),
            },
        )
        .await?;

        let actions = provider.complete(&messages, &[], step).await;
        let text = actions
            .iter()
            .filter_map(|action| match action {
                Action::Say { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        append(
            session,
            sink,
            SessionEvent::ModelResponse {
                text,
                actions: actions.clone(),
            },
        )
        .await?;

        for action in actions {
            match action {
                Action::CallTool { name, args } => {
                    append(
                        session,
                        sink,
                        SessionEvent::ToolCall {
                            name: name.clone(),
                            args: args.clone(),
                        },
                    )
                    .await?;
                    let observation = tools.execute(&name, &args).await;
                    append(
                        session,
                        sink,
                        SessionEvent::Observation {
                            name: observation.name,
                            output: observation.output,
                        },
                    )
                    .await?;
                }
                Action::Done { summary } => {
                    append(session, sink, SessionEvent::Done { summary }).await?;
                    done = true;
                    break;
                }
                Action::Say { .. } => {}
            }
        }

        if done {
            break;
        }
    }

    Ok(TurnOutcome { steps, done })
}

async fn append(
    session: &dyn SessionAppend,
    sink: &dyn EventSink,
    event: SessionEvent,
) -> PortResult<orexis_core::EventRef> {
    let event_ref = session.append(event.clone()).await?;
    sink.emit(&event);
    Ok(event_ref)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecCapabilities {
    pub native_tools: bool,
    pub xml_dialect: bool,
    pub grammar: bool,
}

/// ADR-0003: native → known XML dialect → grammar JSON → lenient.
pub fn select_codec(capabilities: CodecCapabilities, override_id: Option<CodecId>) -> CodecId {
    if let Some(id) = override_id {
        return id;
    }
    if capabilities.native_tools {
        CodecId::Native
    } else if capabilities.xml_dialect {
        CodecId::XmlManual
    } else if capabilities.grammar {
        CodecId::Json
    } else {
        CodecId::Lenient
    }
}

/// Replay check: every logged `ModelRequest` hash must equal the pure
/// derivation from the events that preceded it.
pub fn verify_replay(events: &[SessionEvent]) -> bool {
    for (index, event) in events.iter().enumerate() {
        if let SessionEvent::ModelRequest { request_hash, .. } = event {
            let messages = derive_messages(&events[..index]);
            if hash_messages(&messages) != *request_hash {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_verifies_logged_hashes() {
        let user = SessionEvent::UserMessage { text: "hi".into() };
        let messages = derive_messages(std::slice::from_ref(&user));
        let events = vec![
            SessionEvent::SessionStart {
                session_id: "s".into(),
                format_version: FORMAT_VERSION,
            },
            user,
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: hash_messages(&messages),
            },
        ];
        assert!(verify_replay(&events));
    }

    #[test]
    fn replay_detects_a_tampered_hash() {
        let events = vec![
            SessionEvent::UserMessage { text: "hi".into() },
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: "deadbeef".into(),
            },
        ];
        assert!(!verify_replay(&events));
    }

    #[test]
    fn codec_selection_follows_capabilities() {
        let native = CodecCapabilities {
            native_tools: true,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(native, None), CodecId::Native);

        let xml = CodecCapabilities {
            native_tools: false,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(xml, None), CodecId::XmlManual);

        let json = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: true,
        };
        assert_eq!(select_codec(json, None), CodecId::Json);

        let lenient = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: false,
        };
        assert_eq!(select_codec(lenient, None), CodecId::Lenient);
        assert_eq!(
            select_codec(native, Some(CodecId::Lenient)),
            CodecId::Lenient
        );
    }
}

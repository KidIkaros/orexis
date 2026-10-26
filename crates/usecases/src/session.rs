//! The append helper shared by the loop and action execution.

use orexis_core::{EventRef, SessionEvent};
use orexis_ports::{EventSink, PortResult, SessionAppend};

pub(crate) async fn append(
    session: &dyn SessionAppend,
    sink: &dyn EventSink,
    event: SessionEvent,
) -> PortResult<EventRef> {
    let event_ref = session.append(event.clone()).await?;
    sink.emit(&event, event_ref.seq);
    Ok(event_ref)
}

use super::*;

#[test]
fn a_single_data_event_is_decoded() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push("data: {\"jsonrpc\":\"2.0\"}\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "{\"jsonrpc\":\"2.0\"}");
    assert_eq!(events[0].event, None);
}

#[test]
fn a_chunk_boundary_inside_an_event_is_buffered() {
    let mut decoder = SseDecoder::new();
    assert!(decoder.push("data: {\"a\"").is_empty());
    let events = decoder.push(":1}\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "{\"a\":1}");
}

#[test]
fn several_events_in_one_chunk_are_all_decoded() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push("data: one\n\ndata: two\n\n");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].data, "one");
    assert_eq!(events[1].data, "two");
}

#[test]
fn multiple_data_lines_join_with_newlines() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push("data: first\ndata: second\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "first\nsecond");
}

#[test]
fn the_event_and_id_fields_are_captured() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push("event: message\nid: 42\ndata: body\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.as_deref(), Some("message"));
    assert_eq!(events[0].id.as_deref(), Some("42"));
}

#[test]
fn comments_are_ignored() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push(": keep-alive\ndata: body\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "body");
}

#[test]
fn carriage_returns_are_tolerated() {
    let mut decoder = SseDecoder::new();
    let events = decoder.push("data: body\r\n\r\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "body");
}

#[test]
fn a_stream_ending_without_a_blank_line_still_yields_its_last_event() {
    let mut decoder = SseDecoder::new();
    assert!(decoder.push("data: tail").is_empty());
    let events = decoder.finish();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "tail");
}

#[test]
fn a_blank_line_with_no_data_dispatches_nothing() {
    let mut decoder = SseDecoder::new();
    assert!(decoder.push("\n\n").is_empty());
}

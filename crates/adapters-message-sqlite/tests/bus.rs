//! The bus adapter against the contracts the fake encodes (M8-03).

use futures::executor::block_on;
use orexis_adapters_message_sqlite::SqliteBus;
use orexis_ports::MessageStore;

#[test]
fn the_bus_is_seq_ordered_and_filtered() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bus = SqliteBus::open(&dir.path().join("bus.db")).expect("open");
    block_on(bus.post("w1", None, "room hello")).expect("room");
    block_on(bus.post("w1", Some("w2"), "direct to w2")).expect("direct");

    let for_w2 = block_on(bus.read_since(0, Some("w2"))).expect("read");
    assert_eq!(for_w2.len(), 2);
    let for_w3 = block_on(bus.read_since(0, Some("w3"))).expect("read");
    assert_eq!(for_w3.len(), 1);
    assert_eq!(for_w3[0].text, "room hello");
    let all = block_on(bus.read_since(1, None)).expect("read");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].seq, 2);
}

#[test]
fn text_is_capped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bus = SqliteBus::open(&dir.path().join("bus.db")).expect("open");
    let long = "x".repeat(200_000);
    let message = block_on(bus.post("w1", None, &long)).expect("post");
    assert!(message.text.len() <= 64 * 1024, "capped");
}

#[test]
fn reopen_preserves_messages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bus.db");
    {
        let bus = SqliteBus::open(&path).expect("open");
        block_on(bus.post("w1", None, "kept")).expect("post");
    }
    let bus = SqliteBus::open(&path).expect("reopen");
    let messages = block_on(bus.read_since(0, None)).expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "kept");
}

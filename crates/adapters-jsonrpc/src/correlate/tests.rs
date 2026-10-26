use super::*;
use crate::message::RequestId;

#[test]
fn insert_then_take_round_trips() {
    let mut pending = Correlations::new();
    assert!(pending.insert(RequestId::Number(1), "call-a"));
    assert_eq!(pending.take(&RequestId::Number(1)), Some("call-a"));
    assert!(pending.is_empty());
}

#[test]
fn duplicate_insert_is_rejected_and_keeps_the_first() {
    let mut pending = Correlations::new();
    assert!(pending.insert(RequestId::Number(1), "first"));
    assert!(!pending.insert(RequestId::Number(1), "second"));
    assert_eq!(pending.take(&RequestId::Number(1)), Some("first"));
}

#[test]
fn number_and_string_ids_are_distinct_keys() {
    let mut pending = Correlations::new();
    assert!(pending.insert(RequestId::Number(1), "number"));
    assert!(pending.insert(RequestId::String("1".into()), "string"));
    assert_eq!(pending.take(&RequestId::Number(1)), Some("number"));
    assert_eq!(pending.take(&RequestId::String("1".into())), Some("string"));
}

#[test]
fn contains_and_len_report_pending_state() {
    let mut pending = Correlations::new();
    assert!(pending.is_empty());
    pending.insert(RequestId::Number(1), ());
    pending.insert(RequestId::Number(2), ());
    assert_eq!(pending.len(), 2);
    assert!(pending.contains(&RequestId::Number(2)));
    assert!(!pending.contains(&RequestId::Number(3)));
    pending.take(&RequestId::Number(2));
    assert_eq!(pending.len(), 1);
}

#[test]
fn take_of_an_unknown_id_is_none() {
    let mut pending = Correlations::<()>::new();
    assert_eq!(pending.take(&RequestId::Number(9)), None);
}

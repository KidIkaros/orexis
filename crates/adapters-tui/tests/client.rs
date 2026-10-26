//! Daemon client tests.

use std::io::{BufReader, Write};
use std::time::Duration;

use orexis_adapters_tui::DaemonClient;
use orexis_protocol::{ClientMsg, ServerMsg};

use orexis_core::SessionEvent;
use std::io::BufRead;
use std::os::unix::net::{UnixListener, UnixStream};

fn write(stream: &mut UnixStream, message: &ServerMsg) {
    let mut line = serde_json::to_string(message).expect("server msg");
    line.push('\n');
    stream.write_all(line.as_bytes()).expect("write");
    stream.flush().expect("flush");
}

#[test]
fn attach_and_receive_an_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("daemon.sock");
    let listener = UnixListener::bind(&path).expect("bind");
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut writer = stream;
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        let message: ClientMsg = serde_json::from_str(line.trim()).expect("client msg");
        assert!(matches!(message, ClientMsg::Attach { .. }));
        write(
            &mut writer,
            &ServerMsg::Attached {
                session_id: "s1".to_string(),
            },
        );
        write(
            &mut writer,
            &ServerMsg::Event {
                seq: 1,
                replay: true,
                event: SessionEvent::Done {
                    summary: "done".to_string(),
                },
            },
        );
    });

    let mut client = DaemonClient::connect(&path).expect("connect");
    assert_eq!(client.attach(None).expect("attach"), "s1");
    let mut received = None;
    for _ in 0..50 {
        if let Some(message) = client.try_recv().expect("recv") {
            received = Some(message);
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(received, Some(ServerMsg::Event { .. })));
    handle.join().expect("join");
}

#[test]
fn a_split_message_is_reassembled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("daemon.sock");
    let listener = UnixListener::bind(&path).expect("bind");
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut writer = stream;
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        let mut message = serde_json::to_string(&ServerMsg::Event {
            seq: 1,
            replay: true,
            event: SessionEvent::Done {
                summary: "done".to_string(),
            },
        })
        .expect("msg");
        message.push('\n');
        let split = message.len() / 2;
        writer
            .write_all(&message.as_bytes()[..split])
            .expect("write");
        writer.flush().expect("flush");
        std::thread::sleep(Duration::from_millis(100));
        writer
            .write_all(&message.as_bytes()[split..])
            .expect("write");
        writer.flush().expect("flush");
    });

    let mut client = DaemonClient::connect(&path).expect("connect");
    client
        .send(&ClientMsg::Attach { session: None })
        .expect("attach");
    let mut received = None;
    for _ in 0..50 {
        if let Some(message) = client.try_recv().expect("recv") {
            received = Some(message);
            break;
        }
    }
    assert!(matches!(received, Some(ServerMsg::Event { .. })));
    handle.join().expect("join");
}

#[test]
fn eof_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("daemon.sock");
    let listener = UnixListener::bind(&path).expect("bind");
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        drop(stream);
    });
    let mut client = DaemonClient::connect(&path).expect("connect");
    let error = client.try_recv().expect_err("eof");
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    handle.join().expect("join");
}

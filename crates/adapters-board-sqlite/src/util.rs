//! Shared helpers.

use orexis_ports::PortError;

pub(crate) fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

pub(crate) fn err(error: rusqlite::Error) -> PortError {
    PortError(error.to_string())
}

pub(crate) fn lock<T>(_: T) -> PortError {
    PortError("board lock poisoned".into())
}

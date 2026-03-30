use crate::components::websocket::FileTransferState;
use shared::events::{FileAbortPayload, FileAnnouncePayload, FileChunkPayload, FileRequestPayload};

pub fn handle_file_announce(
    payload: FileAnnouncePayload,
    file_transfer: &FileTransferState,
    my_username: &str,
    tx: &super::super::WsSender,
) {
    file_transfer.handle_file_announce(payload, my_username.to_string(), Some(tx.clone()));
}

pub fn handle_file_request(
    payload: FileRequestPayload,
    file_transfer: &FileTransferState,
    room_name: &str,
    my_username: &str,
    tx: &super::super::WsSender,
) {
    file_transfer.handle_file_request(
        payload,
        room_name.to_string(),
        my_username.to_string(),
        Some(tx.clone()),
    );
}

pub fn handle_file_chunk(
    payload: FileChunkPayload,
    file_transfer: &FileTransferState,
    room_name: &str,
    my_username: &str,
    tx: &super::super::WsSender,
) {
    file_transfer.handle_file_chunk(
        payload,
        room_name.to_string(),
        my_username.to_string(),
        Some(tx.clone()),
    );
}

pub fn handle_file_abort(
    payload: FileAbortPayload,
    file_transfer: &FileTransferState,
    my_username: &str,
) {
    file_transfer.handle_file_abort(payload, my_username);
}

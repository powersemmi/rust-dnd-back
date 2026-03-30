use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use shared::events::{RoomState, SyncSnapshotPackedStatePayload, SyncSnapshotPayload};
use std::io::{Read, Write};

const SNAPSHOT_CODEC_VERSION: u8 = 1;
const SNAPSHOT_COMPRESSION: &str = "gzip";

#[derive(Clone, Default)]
pub struct SnapshotCodec;

impl SnapshotCodec {
    pub fn new() -> Self {
        Self
    }

    pub fn encode_payload(
        &self,
        _room_name: &str,
        state: &RoomState,
    ) -> Result<SyncSnapshotPayload, String> {
        let state_bytes = serde_json::to_vec(state)
            .map_err(|error| format!("failed to serialize snapshot state: {error}"))?;
        let compressed_state = gzip_compress(&state_bytes)?;

        Ok(SyncSnapshotPayload {
            version: state.version,
            packed_state: SyncSnapshotPackedStatePayload {
                codec_version: SNAPSHOT_CODEC_VERSION,
                compression: SNAPSHOT_COMPRESSION.to_string(),
                payload_b64: BASE64.encode(compressed_state),
            },
        })
    }

    pub fn decode_payload(
        &self,
        _room_name: &str,
        payload: &SyncSnapshotPayload,
    ) -> Result<RoomState, String> {
        self.decode_packed_state(&payload.packed_state)
    }

    fn decode_packed_state(
        &self,
        packed_state: &SyncSnapshotPackedStatePayload,
    ) -> Result<RoomState, String> {
        if packed_state.codec_version != SNAPSHOT_CODEC_VERSION {
            return Err(format!(
                "unsupported snapshot codec version: {}",
                packed_state.codec_version
            ));
        }
        if packed_state.compression != SNAPSHOT_COMPRESSION {
            return Err(format!(
                "unsupported snapshot compression: {}",
                packed_state.compression
            ));
        }

        let compressed_state = BASE64
            .decode(&packed_state.payload_b64)
            .map_err(|error| format!("failed to decode snapshot payload: {error}"))?;

        let decompressed_state = gzip_decompress(&compressed_state)?;
        serde_json::from_slice(&decompressed_state)
            .map_err(|error| format!("failed to deserialize snapshot state: {error}"))
    }
}

fn gzip_compress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|error| format!("failed to gzip snapshot payload: {error}"))?;
    encoder
        .finish()
        .map_err(|error| format!("failed to finalize gzip payload: {error}"))
}

fn gzip_decompress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|error| format!("failed to ungzip snapshot payload: {error}"))?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::events::ChatMessagePayload;

    #[test]
    fn roundtrip_packed_snapshot_payload() {
        let codec = SnapshotCodec::new();
        let room_name = "room-42";
        let mut state = RoomState::default();
        for index in 0..20 {
            state.chat_history.push(ChatMessagePayload {
                payload: format!("hello-{index}"),
                username: "tester".to_string(),
                attachments: Vec::new(),
            });
        }
        state.commit_changes();

        let payload = codec
            .encode_payload(room_name, &state)
            .expect("failed to encode payload");

        let decoded = codec
            .decode_payload(room_name, &payload)
            .expect("failed to decode payload");

        let original_json = serde_json::to_string(&state).expect("failed to serialize");
        let decoded_json = serde_json::to_string(&decoded).expect("failed to serialize");
        assert_eq!(decoded_json, original_json);
    }

    #[test]
    fn rejects_invalid_payload_bytes() {
        let codec = SnapshotCodec::new();
        let state = RoomState::default();
        let mut payload = codec
            .encode_payload("room-a", &state)
            .expect("failed to encode payload");
        payload.packed_state.payload_b64 = "%%%".to_string();
        let decode_result = codec.decode_payload("room-a", &payload);
        assert!(decode_result.is_err());
    }
}

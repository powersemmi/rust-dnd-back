// Pure utility functions used by file_transfer logic.
// No signals, no state, no DOM: all functions are deterministic and testable.

use super::{MAX_FILE_SIZE_BYTES, SUPPORTED_MIME_TYPES};
#[cfg(test)]
use super::CHUNK_SIZE_BYTES;
use js_sys::{Array, Uint8Array};
use sha2::{Digest, Sha256};
use shared::events::{ChatMessagePayload, FileRef, Scene};
use std::collections::HashSet;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, File};

/// Collects unique file references from all scene backgrounds and tokens.
pub fn collect_scene_files(scenes: &[Scene]) -> Vec<FileRef> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    for scene in scenes {
        if let Some(background) = &scene.background
            && seen.insert(background.hash.clone())
        {
            files.push(background.clone());
        }

        for token in &scene.tokens {
            if seen.insert(token.image.hash.clone()) {
                files.push(token.image.clone());
            }
        }
    }

    files
}

/// Collects unique file references from chat message attachments.
pub fn collect_chat_files(messages: &[ChatMessagePayload]) -> Vec<FileRef> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    for message in messages {
        for attachment in &message.attachments {
            if seen.insert(attachment.hash.clone()) {
                files.push(attachment.clone());
            }
        }
    }

    files
}

/// Deterministically picks which responder should serve a file request.
/// Uses FNV-1a hashing for a stable but spread distribution.
pub fn deterministic_holder_index(requester: &str, hash: &str, responders_len: usize) -> usize {
    let mut state = 0xcbf29ce484222325u64;
    for byte in requester.as_bytes().iter().chain(hash.as_bytes()) {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    (state % responders_len as u64) as usize
}

/// Validates that a browser `File` is within size and MIME-type limits.
pub fn validate_browser_file(file: &File) -> Result<(), String> {
    let size = file.size() as u64;
    if size > MAX_FILE_SIZE_BYTES {
        return Err("File is larger than 50 MB".to_string());
    }
    let mime_type = file.type_();
    if !SUPPORTED_MIME_TYPES
        .iter()
        .any(|allowed| *allowed == mime_type)
    {
        return Err(format!("Unsupported MIME type: {mime_type}"));
    }
    Ok(())
}

/// Reads all bytes from a `Blob` via its `arrayBuffer()` promise.
pub async fn blob_to_bytes(blob: &Blob) -> Result<Vec<u8>, String> {
    let promise = blob.array_buffer();
    let buffer = JsFuture::from(promise)
        .await
        .map_err(|error| format!("Failed to await Blob bytes: {error:?}"))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

/// Assembles raw bytes into a typed `Blob`.
pub fn bytes_to_blob(bytes: &[u8], mime_type: &str) -> Result<Blob, String> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type(mime_type);
    Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|error| format!("Failed to create Blob: {error:?}"))
}

/// Computes a lowercase hex SHA-256 digest of the given bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Returns the maximum number of bytes in a chunk, for use in tests.
#[cfg(test)]
pub fn chunk_size() -> usize {
    CHUNK_SIZE_BYTES
}

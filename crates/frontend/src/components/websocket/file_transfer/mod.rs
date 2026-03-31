mod utils;
use utils::{
    blob_to_bytes, bytes_to_blob, collect_chat_files, collect_scene_files,
    deterministic_holder_index, sha256_hex, validate_browser_file,
};

use super::{OutboundPriority, WsSender, storage};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use shared::events::{
    ChatMessagePayload, ClientEvent, FileAbortPayload, FileAnnouncePayload, FileChunkPayload,
    FileRef, FileRequestPayload, RoomState, Scene,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use web_sys::{Blob, File};

const MAX_FILE_SIZE_BYTES: u64 = 50 * 1024 * 1024;
const CHUNK_SIZE_BYTES: usize = 48 * 1024;
const OUTGOING_CONCURRENCY_LIMIT: usize = 3;
const REQUEST_RETRY_DELAY_MS: u32 = 10_000;
pub const CHAT_FILE_INPUT_ACCEPT: &str = "image/png,image/jpeg,image/webp,image/gif,application/pdf,application/xml,text/xml,application/json,application/zip,application/x-zip-compressed";
const SUPPORTED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/gif",
    "application/pdf",
    "application/xml",
    "text/xml",
    "application/json",
    "application/zip",
    "application/x-zip-compressed",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileTransferStage {
    Requested,
    Receiving,
    Complete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTransferStatus {
    pub stage: FileTransferStage,
    pub received_chunks: u32,
    pub total_chunks: u32,
    pub detail: Option<String>,
}

impl FileTransferStatus {
    pub fn requested() -> Self {
        Self {
            stage: FileTransferStage::Requested,
            received_chunks: 0,
            total_chunks: 0,
            detail: None,
        }
    }

    pub fn receiving(received_chunks: u32, total_chunks: u32) -> Self {
        Self {
            stage: FileTransferStage::Receiving,
            received_chunks,
            total_chunks,
            detail: None,
        }
    }

    pub fn complete() -> Self {
        Self {
            stage: FileTransferStage::Complete,
            received_chunks: 1,
            total_chunks: 1,
            detail: None,
        }
    }

    pub fn failed(detail: impl Into<String>) -> Self {
        Self {
            stage: FileTransferStage::Failed,
            received_chunks: 0,
            total_chunks: 0,
            detail: Some(detail.into()),
        }
    }

    pub fn progress_percent(&self) -> u32 {
        match self.stage {
            FileTransferStage::Complete => 100,
            FileTransferStage::Receiving if self.total_chunks > 0 => {
                (self.received_chunks.saturating_mul(100) / self.total_chunks).min(99)
            }
            _ => 0,
        }
    }
}

#[derive(Clone)]
pub struct FileTransferState {
    pub file_urls: RwSignal<HashMap<String, String>>,
    pub transfer_statuses: RwSignal<HashMap<String, FileTransferStatus>>,
    announcers: Rc<RefCell<HashMap<String, BTreeSet<String>>>>,
    known_files: Rc<RefCell<HashMap<String, FileRef>>>,
    requested_at: Rc<RefCell<HashMap<String, f64>>>,
    announced_local_hashes: Rc<RefCell<HashSet<String>>>,
    incoming: Rc<RefCell<HashMap<String, IncomingTransfer>>>,
    outgoing_queue: Rc<RefCell<VecDeque<OutgoingTransferJob>>>,
    active_outgoing: Rc<Cell<usize>>,
}

// SAFETY: frontend runs in the single-threaded wasm environment.
unsafe impl Send for FileTransferState {}
unsafe impl Sync for FileTransferState {}

#[derive(Clone)]
struct OutgoingTransferJob {
    hash: String,
    requester: String,
}

struct IncomingTransfer {
    file: FileRef,
    total_chunks: u32,
    chunks: Vec<Option<Vec<u8>>>,
    received_chunks: u32,
}

impl FileTransferState {
    pub fn new() -> Self {
        Self {
            file_urls: RwSignal::new(HashMap::new()),
            transfer_statuses: RwSignal::new(HashMap::new()),
            announcers: Rc::new(RefCell::new(HashMap::new())),
            known_files: Rc::new(RefCell::new(HashMap::new())),
            requested_at: Rc::new(RefCell::new(HashMap::new())),
            announced_local_hashes: Rc::new(RefCell::new(HashSet::new())),
            incoming: Rc::new(RefCell::new(HashMap::new())),
            outgoing_queue: Rc::new(RefCell::new(VecDeque::new())),
            active_outgoing: Rc::new(Cell::new(0)),
        }
    }

    pub fn reset(&self) {
        let existing_urls = self
            .file_urls
            .get_untracked()
            .into_values()
            .collect::<Vec<_>>();
        storage::revoke_file_urls(existing_urls);
        self.file_urls.set(HashMap::new());
        self.transfer_statuses.set(HashMap::new());
        self.announcers.borrow_mut().clear();
        self.known_files.borrow_mut().clear();
        self.requested_at.borrow_mut().clear();
        self.announced_local_hashes.borrow_mut().clear();
        self.incoming.borrow_mut().clear();
        self.outgoing_queue.borrow_mut().clear();
        self.active_outgoing.set(0);
    }

    pub fn note_scenes(&self, scenes: &[Scene]) {
        let mut known_files = self.known_files.borrow_mut();
        for scene in scenes {
            if let Some(background) = &scene.background {
                known_files.insert(background.hash.clone(), background.clone());
            }
            for token in &scene.tokens {
                known_files.insert(token.image.hash.clone(), token.image.clone());
            }
        }
    }

    pub fn hydrate_local_files(&self, files: &[FileRef]) {
        for file in files {
            self.known_files
                .borrow_mut()
                .insert(file.hash.clone(), file.clone());

            if self.file_urls.get_untracked().contains_key(&file.hash) {
                continue;
            }

            let this = self.clone();
            let file = file.clone();
            spawn_local(async move {
                match storage::load_file(&file.hash).await {
                    Ok(Some(record)) => {
                        this.ensure_file_url_from_blob(&record.file.hash, &record.blob);
                        this.set_status(&record.file.hash, FileTransferStatus::complete());
                    }
                    Ok(None) => {}
                    Err(error) => {
                        log!(
                            "Failed to hydrate local file '{}' from IndexedDB: {}",
                            file.hash,
                            error
                        );
                    }
                }
            });
        }
    }

    pub fn reconcile_scene_files(
        &self,
        room_name: String,
        username: String,
        ws_sender: Option<WsSender>,
        scenes: Vec<Scene>,
    ) {
        self.note_scenes(&scenes);

        let files = collect_scene_files(&scenes);
        for file in files {
            let this = self.clone();
            let room_name = room_name.clone();
            let username = username.clone();
            let ws_sender = ws_sender.clone();
            spawn_local(async move {
                match storage::load_file(&file.hash).await {
                    Ok(Some(record)) => {
                        this.ensure_file_url_from_blob(&record.file.hash, &record.blob);
                        this.set_status(&record.file.hash, FileTransferStatus::complete());
                        this.announce_local_file(record.file, username, ws_sender, false);
                    }
                    Ok(None) => {
                        this.request_file_if_needed(file, username, ws_sender);
                    }
                    Err(error) => {
                        log!("Failed to reconcile local file '{}': {}", file.hash, error);
                        this.set_status(&file.hash, FileTransferStatus::failed(error));
                    }
                }
                let _ = room_name;
            });
        }
    }

    pub fn reannounce_scene_files(
        &self,
        room_name: String,
        username: String,
        ws_sender: Option<WsSender>,
        state: &RoomState,
    ) {
        self.note_scenes(&state.scenes);
        for file in collect_scene_files(&state.scenes) {
            let this = self.clone();
            let username = username.clone();
            let ws_sender = ws_sender.clone();
            let room_name = room_name.clone();
            spawn_local(async move {
                match storage::load_file(&file.hash).await {
                    Ok(Some(record)) => {
                        this.ensure_file_url_from_blob(&record.file.hash, &record.blob);
                        // Force-announce so newcomers learn about available files.
                        this.announce_local_file(record.file, username, ws_sender, true);
                    }
                    Ok(None) => {
                        // File is referenced by a scene token/background but not in local
                        // IndexedDB.  Request it from peers so we can serve future newcomers
                        // and so our own render stays current.
                        log!(
                            "Requesting missing scene file '{}' in room '{}'",
                            file.hash,
                            room_name
                        );
                        this.request_file_if_needed(file, username, ws_sender);
                    }
                    Err(error) => log!(
                        "Failed to reannounce local file '{}' in room '{}': {}",
                        file.hash,
                        room_name,
                        error
                    ),
                }
            });
        }
    }

    /// Loads chat attachment blobs from local IndexedDB and, when a file is missing,
    /// sends a `FILE_REQUEST` to peers so the attachment can be downloaded.
    ///
    /// `username` and `ws_sender` may be empty/`None` when called before the WebSocket
    /// connection is established (e.g. initial IndexedDB hydration); in that case missing
    /// files are silently skipped and will be re-requested once the connection is open.
    pub fn reconcile_chat_attachments(
        &self,
        messages: &[ChatMessagePayload],
        username: String,
        ws_sender: Option<WsSender>,
    ) {
        for file in collect_chat_files(messages) {
            self.known_files
                .borrow_mut()
                .insert(file.hash.clone(), file.clone());

            let has_url = self.file_urls.get_untracked().contains_key(&file.hash);
            let is_request_in_flight = self
                .transfer_statuses
                .get_untracked()
                .get(&file.hash)
                .is_some_and(|status| {
                    matches!(
                        status.stage,
                        FileTransferStage::Requested | FileTransferStage::Receiving
                    )
                });
            if has_url || is_request_in_flight {
                continue;
            }

            let this = self.clone();
            let username = username.clone();
            let ws_sender = ws_sender.clone();
            spawn_local(async move {
                match storage::load_file(&file.hash).await {
                    Ok(Some(record)) => {
                        this.ensure_file_url_from_blob(&record.file.hash, &record.blob);
                        this.set_status(&record.file.hash, FileTransferStatus::complete());
                    }
                    Ok(None) => {
                        // File not in local IndexedDB — request it from a peer.
                        this.request_file_if_needed(file, username, ws_sender);
                    }
                    Err(error) => {
                        log!(
                            "Failed to hydrate chat attachment '{}' from IndexedDB: {}",
                            file.hash,
                            error
                        );
                    }
                }
            });
        }
    }

    pub fn handle_file_announce(
        &self,
        payload: FileAnnouncePayload,
        username: String,
        ws_sender: Option<WsSender>,
    ) {
        self.known_files
            .borrow_mut()
            .insert(payload.file.hash.clone(), payload.file.clone());
        self.announcers
            .borrow_mut()
            .entry(payload.file.hash.clone())
            .or_default()
            .insert(payload.from.clone());

        if payload.from == username {
            return;
        }

        self.request_file_if_needed(payload.file, username, ws_sender);
    }

    pub fn handle_file_request(
        &self,
        payload: FileRequestPayload,
        room_name: String,
        username: String,
        ws_sender: Option<WsSender>,
    ) {
        if payload.requester == username {
            return;
        }

        let this = self.clone();
        spawn_local(async move {
            let file = match storage::load_file(&payload.hash).await {
                Ok(Some(record)) => record,
                Ok(None) => return,
                Err(error) => {
                    log!(
                        "Failed to check local file '{}' for request: {}",
                        payload.hash,
                        error
                    );
                    return;
                }
            };

            this.known_files
                .borrow_mut()
                .insert(file.file.hash.clone(), file.file.clone());
            this.announcers
                .borrow_mut()
                .entry(file.file.hash.clone())
                .or_default()
                .insert(username.clone());

            if !this.should_respond_to_request(&file.file.hash, &payload.requester, &username) {
                return;
            }

            this.outgoing_queue
                .borrow_mut()
                .push_back(OutgoingTransferJob {
                    hash: payload.hash,
                    requester: payload.requester,
                });
            this.pump_outgoing(room_name, username, ws_sender);
        });
    }

    pub fn handle_file_chunk(
        &self,
        payload: FileChunkPayload,
        room_name: String,
        username: String,
        ws_sender: Option<WsSender>,
    ) {
        if payload.requester != username {
            return;
        }

        let Some(file) = self.known_files.borrow().get(&payload.hash).cloned() else {
            self.set_status(
                &payload.hash,
                FileTransferStatus::failed("Missing file metadata for incoming transfer"),
            );
            return;
        };

        let chunk_bytes = match BASE64.decode(payload.data.as_bytes()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.set_status(
                    &payload.hash,
                    FileTransferStatus::failed(format!("Invalid base64 chunk: {error}")),
                );
                return;
            }
        };

        let Some(chunk_index) = usize::try_from(payload.chunk_index).ok() else {
            self.set_status(
                &payload.hash,
                FileTransferStatus::failed("Chunk index is too large"),
            );
            return;
        };
        let Some(total_chunks) = usize::try_from(payload.total_chunks).ok() else {
            self.set_status(
                &payload.hash,
                FileTransferStatus::failed("Total chunk count is too large"),
            );
            return;
        };

        if chunk_index >= total_chunks || total_chunks == 0 {
            self.set_status(
                &payload.hash,
                FileTransferStatus::failed("Chunk index is out of bounds"),
            );
            return;
        }

        let mut maybe_complete = None::<IncomingTransfer>;
        {
            let mut incoming = self.incoming.borrow_mut();
            let entry = incoming
                .entry(payload.hash.clone())
                .or_insert_with(|| IncomingTransfer {
                    file: file.clone(),
                    total_chunks: payload.total_chunks,
                    chunks: vec![None; total_chunks],
                    received_chunks: 0,
                });

            if entry.total_chunks != payload.total_chunks || entry.chunks.len() != total_chunks {
                incoming.remove(&payload.hash);
                self.set_status(
                    &payload.hash,
                    FileTransferStatus::failed("Chunk sequence changed mid-transfer"),
                );
                return;
            }

            if entry.chunks[chunk_index].is_none() {
                entry.chunks[chunk_index] = Some(chunk_bytes);
                entry.received_chunks += 1;
            }

            self.set_status(
                &payload.hash,
                FileTransferStatus::receiving(entry.received_chunks, entry.total_chunks),
            );

            if entry.received_chunks == entry.total_chunks {
                maybe_complete = incoming.remove(&payload.hash);
            }
        }

        if let Some(transfer) = maybe_complete {
            let this = self.clone();
            spawn_local(async move {
                if let Err(error) = this
                    .finalize_incoming_transfer(transfer, room_name, username, ws_sender)
                    .await
                {
                    this.set_status(&payload.hash, FileTransferStatus::failed(error));
                }
            });
        }
    }

    pub fn handle_file_abort(&self, payload: FileAbortPayload, username: &str) {
        if payload.requester != username {
            return;
        }

        self.incoming.borrow_mut().remove(&payload.hash);
        self.set_status(&payload.hash, FileTransferStatus::failed(payload.reason));
    }

    pub async fn import_browser_file(
        &self,
        browser_file: File,
        username: String,
        ws_sender: Option<WsSender>,
    ) -> Result<FileRef, String> {
        self.import_browser_file_with_announce(browser_file, username, ws_sender, true)
            .await
    }

    pub async fn import_browser_file_with_announce(
        &self,
        browser_file: File,
        username: String,
        ws_sender: Option<WsSender>,
        announce_immediately: bool,
    ) -> Result<FileRef, String> {
        validate_browser_file(&browser_file)?;

        let blob: Blob = browser_file
            .clone()
            .dyn_into::<Blob>()
            .map_err(|_| "Selected file is not a Blob".to_string())?;
        let bytes = blob_to_bytes(&blob).await?;
        let hash = sha256_hex(&bytes);
        let file_ref = FileRef {
            hash,
            mime_type: browser_file.type_(),
            file_name: browser_file.name(),
            size: bytes.len() as u64,
        };

        self.known_files
            .borrow_mut()
            .insert(file_ref.hash.clone(), file_ref.clone());

        match storage::load_file(&file_ref.hash).await? {
            Some(record) => self.ensure_file_url_from_blob(&record.file.hash, &record.blob),
            None => {
                storage::save_file(&storage::StoredFile {
                    file: file_ref.clone(),
                    blob: blob.clone(),
                })
                .await?;
                self.ensure_file_url_from_blob(&file_ref.hash, &blob);
            }
        }

        self.set_status(&file_ref.hash, FileTransferStatus::complete());
        if announce_immediately {
            self.announce_local_file(file_ref.clone(), username, ws_sender, false);
        }

        Ok(file_ref)
    }

    pub fn announce_local_files(
        &self,
        files: &[FileRef],
        username: String,
        ws_sender: Option<WsSender>,
    ) {
        for file in files {
            self.announce_local_file(file.clone(), username.clone(), ws_sender.clone(), false);
        }
    }

    pub fn request_file(&self, file: FileRef, username: String, ws_sender: Option<WsSender>) {
        self.request_file_if_needed(file, username, ws_sender);
    }

    fn announce_local_file(
        &self,
        file: FileRef,
        username: String,
        ws_sender: Option<WsSender>,
        force: bool,
    ) {
        let Some(sender) = ws_sender else {
            return;
        };

        if !force && self.announced_local_hashes.borrow().contains(&file.hash) {
            return;
        }

        self.announced_local_hashes
            .borrow_mut()
            .insert(file.hash.clone());
        self.announcers
            .borrow_mut()
            .entry(file.hash.clone())
            .or_default()
            .insert(username.clone());

        let event = ClientEvent::FileAnnounce(FileAnnouncePayload {
            file,
            from: username,
        });
        let _ = sender.try_send_event(event);
    }

    fn request_file_if_needed(&self, file: FileRef, username: String, ws_sender: Option<WsSender>) {
        self.known_files
            .borrow_mut()
            .insert(file.hash.clone(), file.clone());

        let Some(sender) = ws_sender else {
            return;
        };

        if matches!(
            self.transfer_statuses.get_untracked().get(&file.hash),
            Some(FileTransferStatus {
                stage: FileTransferStage::Receiving | FileTransferStage::Complete,
                ..
            })
        ) {
            return;
        }

        let now = js_sys::Date::now();
        if let Some(last_request_at) = self.requested_at.borrow().get(&file.hash)
            && now - *last_request_at < f64::from(REQUEST_RETRY_DELAY_MS)
        {
            return;
        }

        self.requested_at
            .borrow_mut()
            .insert(file.hash.clone(), now);
        self.set_status(&file.hash, FileTransferStatus::requested());

        let event = ClientEvent::FileRequest(FileRequestPayload {
            hash: file.hash.clone(),
            requester: username.clone(),
        });
        let _ = sender.try_send_event(event);

        let this = self.clone();
        spawn_local(async move {
            TimeoutFuture::new(REQUEST_RETRY_DELAY_MS).await;

            let last_request_at = this.requested_at.borrow().get(&file.hash).copied();
            if last_request_at != Some(now) {
                return;
            }

            match storage::file_exists(&file.hash).await {
                Ok(true) => {
                    this.requested_at.borrow_mut().remove(&file.hash);
                    this.set_status(&file.hash, FileTransferStatus::complete());
                }
                Ok(false) => {
                    this.requested_at.borrow_mut().remove(&file.hash);
                    this.request_file_if_needed(file, username, Some(sender));
                }
                Err(error) => {
                    this.requested_at.borrow_mut().remove(&file.hash);
                    this.set_status(&file.hash, FileTransferStatus::failed(error));
                }
            }
        });
    }

    fn should_respond_to_request(&self, hash: &str, requester: &str, username: &str) -> bool {
        let mut responders = self
            .announcers
            .borrow()
            .get(hash)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        if !responders.iter().any(|user| user == username) {
            responders.push(username.to_string());
        }

        responders.sort();
        responders.dedup();

        if responders.is_empty() {
            return false;
        }

        let selected_index = deterministic_holder_index(requester, hash, responders.len());
        responders
            .get(selected_index)
            .is_some_and(|selected| selected == username)
    }

    fn pump_outgoing(&self, room_name: String, username: String, ws_sender: Option<WsSender>) {
        while self.active_outgoing.get() < OUTGOING_CONCURRENCY_LIMIT {
            let Some(job) = self.outgoing_queue.borrow_mut().pop_front() else {
                return;
            };

            let this = self.clone();
            let room_name = room_name.clone();
            let username = username.clone();
            let ws_sender = ws_sender.clone();
            self.active_outgoing
                .set(self.active_outgoing.get().saturating_add(1));

            spawn_local(async move {
                if let Err(error) = this
                    .run_outgoing_transfer(
                        job,
                        room_name.clone(),
                        username.clone(),
                        ws_sender.clone(),
                    )
                    .await
                {
                    log!(
                        "Outgoing transfer failed in room '{}': {}",
                        room_name,
                        error
                    );
                }

                this.active_outgoing
                    .set(this.active_outgoing.get().saturating_sub(1));
                this.pump_outgoing(room_name, username, ws_sender);
            });
        }
    }

    async fn run_outgoing_transfer(
        &self,
        job: OutgoingTransferJob,
        _room_name: String,
        _username: String,
        ws_sender: Option<WsSender>,
    ) -> Result<(), String> {
        let Some(sender) = ws_sender else {
            return Ok(());
        };

        let record = storage::load_file(&job.hash)
            .await?
            .ok_or_else(|| format!("Local file '{}' disappeared before send", job.hash))?;
        let bytes = blob_to_bytes(&record.blob).await?;
        let total_chunks = bytes.len().div_ceil(CHUNK_SIZE_BYTES);
        let total_chunks = u32::try_from(total_chunks)
            .map_err(|_| "File produces too many chunks for protocol".to_string())?;

        for (chunk_index, chunk) in bytes.chunks(CHUNK_SIZE_BYTES).enumerate() {
            let chunk_index = u32::try_from(chunk_index)
                .map_err(|_| "Chunk index exceeds protocol range".to_string())?;
            let event = ClientEvent::FileChunk(FileChunkPayload {
                hash: record.file.hash.clone(),
                requester: job.requester.clone(),
                chunk_index,
                total_chunks,
                data: BASE64.encode(chunk),
            });

            let _ = sender.try_send_event_with_priority(event, OutboundPriority::Low);
        }

        Ok(())
    }

    async fn finalize_incoming_transfer(
        &self,
        transfer: IncomingTransfer,
        _room_name: String,
        username: String,
        ws_sender: Option<WsSender>,
    ) -> Result<(), String> {
        let mut bytes = Vec::with_capacity(transfer.file.size as usize);
        for chunk in transfer.chunks {
            let chunk = chunk.ok_or_else(|| "Incoming transfer is missing a chunk".to_string())?;
            bytes.extend_from_slice(&chunk);
        }

        let computed_hash = sha256_hex(&bytes);
        if computed_hash != transfer.file.hash {
            self.requested_at.borrow_mut().remove(&transfer.file.hash);
            self.send_abort(
                &transfer.file.hash,
                &username,
                ws_sender.clone(),
                "Hash mismatch",
            );
            self.request_file_if_needed(transfer.file, username, ws_sender);
            return Err("Hash mismatch after reassembly".to_string());
        }

        let blob = bytes_to_blob(&bytes, &transfer.file.mime_type)?;
        storage::save_file(&storage::StoredFile {
            file: transfer.file.clone(),
            blob: blob.clone(),
        })
        .await?;

        self.ensure_file_url_from_blob(&transfer.file.hash, &blob);
        self.requested_at.borrow_mut().remove(&transfer.file.hash);
        self.set_status(&transfer.file.hash, FileTransferStatus::complete());
        self.announce_local_file(transfer.file, username, ws_sender, false);

        Ok(())
    }

    fn ensure_file_url_from_blob(&self, hash: &str, blob: &Blob) {
        if self.file_urls.get_untracked().contains_key(hash) {
            return;
        }

        match web_sys::Url::create_object_url_with_blob(blob) {
            Ok(url) => {
                self.file_urls.update(|urls| {
                    if let Some(old_url) = urls.insert(hash.to_string(), url) {
                        storage::revoke_file_urls([old_url]);
                    }
                });
            }
            Err(error) => log!("Failed to create object URL for '{}': {:?}", hash, error),
        }
    }

    fn send_abort(&self, hash: &str, requester: &str, ws_sender: Option<WsSender>, reason: &str) {
        let Some(sender) = ws_sender else {
            return;
        };

        let event = ClientEvent::FileAbort(FileAbortPayload {
            hash: hash.to_string(),
            requester: requester.to_string(),
            reason: reason.to_string(),
        });
        let _ = sender.try_send_event(event);
    }

    fn set_status(&self, hash: &str, status: FileTransferStatus) {
        self.transfer_statuses.update(|statuses| {
            statuses.insert(hash.to_string(), status);
        });
    }
}


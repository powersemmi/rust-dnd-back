use js_sys::{Object, Reflect};
use leptos::task::spawn_local;
use log::warn;
use rexie::{ObjectStore, Rexie, TransactionMode};
use serde::{Deserialize, Serialize};
use shared::events::{FileRef, RoomState};
use web_sys::wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, Url};

const DATABASE_NAME: &str = "dnd_vtt";
const DATABASE_VERSION: u32 = 2;
const ROOM_STATES_STORE: &str = "room_states";
const FILES_STORE: &str = "files";
const TOKEN_LIBRARY_STORE: &str = "token_library";

type StorageResult<T> = Result<T, String>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StoredRoomState {
    pub room_name: String,
    pub state: RoomState,
}

#[derive(Clone, Debug)]
pub struct StoredFile {
    pub file: FileRef,
    pub blob: Blob,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct StoredTokenLibraryItem {
    pub key: String,
    pub room_name: String,
    pub id: String,
    pub name: String,
    pub image: FileRef,
    pub width_cells: u16,
    pub height_cells: u16,
}

async fn open_database() -> StorageResult<Rexie> {
    Rexie::builder(DATABASE_NAME)
        .version(DATABASE_VERSION)
        .add_object_store(ObjectStore::new(ROOM_STATES_STORE).key_path("room_name"))
        .add_object_store(ObjectStore::new(FILES_STORE).key_path("hash"))
        .add_object_store(ObjectStore::new(TOKEN_LIBRARY_STORE).key_path("key"))
        .build()
        .await
        .map_err(|error| format!("failed to open IndexedDB: {error:?}"))
}

async fn load_state_from_indexed_db(room_name: &str) -> StorageResult<Option<StoredRoomState>> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[ROOM_STATES_STORE], TransactionMode::ReadOnly)
        .map_err(|error| format!("failed to open read transaction: {error:?}"))?;
    let store = transaction
        .store(ROOM_STATES_STORE)
        .map_err(|error| format!("failed to open room_states store: {error:?}"))?;

    let value = store
        .get(JsValue::from_str(room_name))
        .await
        .map_err(|error| format!("failed to read room state from IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("read transaction failed: {error:?}"))?;

    match value {
        Some(value) => serde_wasm_bindgen::from_value::<StoredRoomState>(value)
            .map(Some)
            .map_err(|error| format!("failed to decode room state from IndexedDB: {error}")),
        None => Ok(None),
    }
}

pub async fn load_state(room_name: &str) -> StorageResult<Option<StoredRoomState>> {
    load_state_from_indexed_db(room_name).await
}

pub async fn save_state(room_name: &str, state: &RoomState) -> StorageResult<()> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[ROOM_STATES_STORE], TransactionMode::ReadWrite)
        .map_err(|error| format!("failed to open write transaction: {error:?}"))?;
    let store = transaction
        .store(ROOM_STATES_STORE)
        .map_err(|error| format!("failed to open room_states store: {error:?}"))?;

    let record = StoredRoomState {
        room_name: room_name.to_string(),
        state: state.clone(),
    };
    let value = serde_wasm_bindgen::to_value(&record)
        .map_err(|error| format!("failed to encode room state for IndexedDB: {error}"))?;

    store
        .put(&value, None)
        .await
        .map_err(|error| format!("failed to save room state to IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("write transaction failed: {error:?}"))?;

    Ok(())
}

pub fn token_library_key(room_name: &str, token_id: &str) -> String {
    format!("{room_name}:{token_id}")
}

fn sort_token_library_items(items: &mut [StoredTokenLibraryItem]) {
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub async fn load_token_library(room_name: &str) -> StorageResult<Vec<StoredTokenLibraryItem>> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[TOKEN_LIBRARY_STORE], TransactionMode::ReadOnly)
        .map_err(|error| format!("failed to open token library read transaction: {error:?}"))?;
    let store = transaction
        .store(TOKEN_LIBRARY_STORE)
        .map_err(|error| format!("failed to open token_library store: {error:?}"))?;

    let values = store
        .get_all(None, None)
        .await
        .map_err(|error| format!("failed to read token library from IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("token library read transaction failed: {error:?}"))?;

    let mut items = values
        .into_iter()
        .map(|value| {
            serde_wasm_bindgen::from_value::<StoredTokenLibraryItem>(value)
                .map_err(|error| format!("failed to decode token library item: {error}"))
        })
        .collect::<StorageResult<Vec<_>>>()?;
    items.retain(|item| item.room_name == room_name);
    sort_token_library_items(&mut items);
    Ok(items)
}

pub async fn save_token_library_item(item: &StoredTokenLibraryItem) -> StorageResult<()> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[TOKEN_LIBRARY_STORE], TransactionMode::ReadWrite)
        .map_err(|error| format!("failed to open token library write transaction: {error:?}"))?;
    let store = transaction
        .store(TOKEN_LIBRARY_STORE)
        .map_err(|error| format!("failed to open token_library store: {error:?}"))?;

    let value = serde_wasm_bindgen::to_value(item)
        .map_err(|error| format!("failed to encode token library item for IndexedDB: {error}"))?;

    store
        .put(&value, None)
        .await
        .map_err(|error| format!("failed to save token library item to IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("token library write transaction failed: {error:?}"))?;

    Ok(())
}

pub async fn delete_token_library_item(room_name: &str, token_id: &str) -> StorageResult<()> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[TOKEN_LIBRARY_STORE], TransactionMode::ReadWrite)
        .map_err(|error| format!("failed to open token library delete transaction: {error:?}"))?;
    let store = transaction
        .store(TOKEN_LIBRARY_STORE)
        .map_err(|error| format!("failed to open token_library store: {error:?}"))?;

    store
        .delete(JsValue::from_str(&token_library_key(room_name, token_id)))
        .await
        .map_err(|error| {
            format!("failed to delete token library item from IndexedDB: {error:?}")
        })?;

    transaction
        .done()
        .await
        .map_err(|error| format!("token library delete transaction failed: {error:?}"))?;

    Ok(())
}

pub async fn delete_state(room_name: &str) -> StorageResult<()> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[ROOM_STATES_STORE], TransactionMode::ReadWrite)
        .map_err(|error| format!("failed to open delete transaction: {error:?}"))?;
    let store = transaction
        .store(ROOM_STATES_STORE)
        .map_err(|error| format!("failed to open room_states store: {error:?}"))?;

    store
        .delete(JsValue::from_str(room_name))
        .await
        .map_err(|error| format!("failed to delete room state from IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("delete transaction failed: {error:?}"))?;

    Ok(())
}

pub async fn move_state(from_room: &str, to_room: &str) -> StorageResult<()> {
    if from_room == to_room {
        return Ok(());
    }

    let Some(record) = load_state(from_room).await? else {
        return Ok(());
    };

    save_state(to_room, &record.state).await?;
    delete_state(from_room).await
}

pub fn save_state_in_background(room_name: &str, state: &RoomState) {
    let room_name = room_name.to_string();
    let state = state.clone();

    spawn_local(async move {
        if let Err(error) = save_state(&room_name, &state).await {
            warn!(
                "Failed to persist room state for '{}' to IndexedDB: {}",
                room_name, error
            );
        }
    });
}

fn build_file_record_value(record: &StoredFile) -> StorageResult<JsValue> {
    let object = Object::new();

    Reflect::set(
        &object,
        &JsValue::from_str("hash"),
        &JsValue::from_str(&record.file.hash),
    )
    .map_err(|error| format!("failed to encode file hash: {error:?}"))?;
    Reflect::set(
        &object,
        &JsValue::from_str("mime_type"),
        &JsValue::from_str(&record.file.mime_type),
    )
    .map_err(|error| format!("failed to encode file mime_type: {error:?}"))?;
    Reflect::set(
        &object,
        &JsValue::from_str("file_name"),
        &JsValue::from_str(&record.file.file_name),
    )
    .map_err(|error| format!("failed to encode file file_name: {error:?}"))?;
    Reflect::set(
        &object,
        &JsValue::from_str("size"),
        &JsValue::from_f64(record.file.size as f64),
    )
    .map_err(|error| format!("failed to encode file size: {error:?}"))?;
    Reflect::set(&object, &JsValue::from_str("blob"), record.blob.as_ref())
        .map_err(|error| format!("failed to encode file blob: {error:?}"))?;

    Ok(object.into())
}

fn decode_string_field(value: &JsValue, field: &str) -> StorageResult<String> {
    Reflect::get(value, &JsValue::from_str(field))
        .map_err(|error| format!("failed to read file field '{field}': {error:?}"))?
        .as_string()
        .ok_or_else(|| format!("file field '{field}' is missing or not a string"))
}

fn decode_u64_field(value: &JsValue, field: &str) -> StorageResult<u64> {
    Reflect::get(value, &JsValue::from_str(field))
        .map_err(|error| format!("failed to read file field '{field}': {error:?}"))?
        .as_f64()
        .ok_or_else(|| format!("file field '{field}' is missing or not numeric"))
        .map(|number| number as u64)
}

fn decode_blob_field(value: &JsValue, field: &str) -> StorageResult<Blob> {
    Reflect::get(value, &JsValue::from_str(field))
        .map_err(|error| format!("failed to read file field '{field}': {error:?}"))?
        .dyn_into::<Blob>()
        .map_err(|_| format!("file field '{field}' is missing or not a Blob"))
}

fn decode_file_record(value: JsValue) -> StorageResult<StoredFile> {
    Ok(StoredFile {
        file: FileRef {
            hash: decode_string_field(&value, "hash")?,
            mime_type: decode_string_field(&value, "mime_type")?,
            file_name: decode_string_field(&value, "file_name")?,
            size: decode_u64_field(&value, "size")?,
        },
        blob: decode_blob_field(&value, "blob")?,
    })
}

pub async fn load_file(hash: &str) -> StorageResult<Option<StoredFile>> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[FILES_STORE], TransactionMode::ReadOnly)
        .map_err(|error| format!("failed to open file read transaction: {error:?}"))?;
    let store = transaction
        .store(FILES_STORE)
        .map_err(|error| format!("failed to open files store: {error:?}"))?;

    let value = store
        .get(JsValue::from_str(hash))
        .await
        .map_err(|error| format!("failed to read file from IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("file read transaction failed: {error:?}"))?;

    match value {
        Some(value) => decode_file_record(value).map(Some),
        None => Ok(None),
    }
}

pub async fn file_exists(hash: &str) -> StorageResult<bool> {
    load_file(hash).await.map(|record| record.is_some())
}

pub async fn save_file(record: &StoredFile) -> StorageResult<()> {
    let database = open_database().await?;
    let transaction = database
        .transaction(&[FILES_STORE], TransactionMode::ReadWrite)
        .map_err(|error| format!("failed to open file write transaction: {error:?}"))?;
    let store = transaction
        .store(FILES_STORE)
        .map_err(|error| format!("failed to open files store: {error:?}"))?;

    let value = build_file_record_value(record)?;

    store
        .put(&value, None)
        .await
        .map_err(|error| format!("failed to save file to IndexedDB: {error:?}"))?;

    transaction
        .done()
        .await
        .map_err(|error| format!("file write transaction failed: {error:?}"))?;

    Ok(())
}

pub fn revoke_file_urls(urls: impl IntoIterator<Item = String>) {
    for url in urls {
        let _ = Url::revoke_object_url(&url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_library_key_is_namespaced_by_room() {
        assert_eq!(token_library_key("room-a", "token-1"), "room-a:token-1");
    }

    #[test]
    fn token_library_items_are_sorted_case_insensitively() {
        let file = FileRef {
            hash: "hash".to_string(),
            mime_type: "image/png".to_string(),
            file_name: "token.png".to_string(),
            size: 42,
        };
        let mut items = vec![
            StoredTokenLibraryItem {
                key: "room:t2".to_string(),
                room_name: "room".to_string(),
                id: "t2".to_string(),
                name: "zebra".to_string(),
                image: file.clone(),
                width_cells: 1,
                height_cells: 1,
            },
            StoredTokenLibraryItem {
                key: "room:t1".to_string(),
                room_name: "room".to_string(),
                id: "t1".to_string(),
                name: "Alpha".to_string(),
                image: file,
                width_cells: 1,
                height_cells: 1,
            },
        ];

        sort_token_library_items(&mut items);

        assert_eq!(items[0].id, "t1");
        assert_eq!(items[1].id, "t2");
    }
}

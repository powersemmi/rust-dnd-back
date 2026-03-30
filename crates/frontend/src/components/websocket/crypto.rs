use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use sha2::{Digest, Sha256};
use shared::events::{
    ClientEvent, CryptoKeyAnnouncePayload, CryptoKeyWrapPayload, CryptoPayload,
    EncryptedPayloadKind,
};
use std::collections::{HashMap, HashSet};
use x25519_dalek::{PublicKey, StaticSecret};

const CRYPTO_VERSION: u8 = 1;
const KEY_WRAP_CONTEXT: &[u8] = b"dnd-vtt-key-wrap-v1";
const PAYLOAD_CONTEXT: &[u8] = b"dnd-vtt-payload-v1";
const MAX_PENDING_PER_KEY: usize = 64;

pub struct RoomCryptoState {
    room_name: String,
    my_username: String,
    static_secret: StaticSecret,
    public_key: PublicKey,
    peers: HashMap<String, PublicKey>,
    outbound_key_id: Option<String>,
    outbound_room_key: Option<[u8; 32]>,
    outbound_wrapped_for: HashSet<String>,
    inbound_keys: HashMap<String, [u8; 32]>,
    pending_payloads: HashMap<String, Vec<CryptoPayload>>,
}

impl RoomCryptoState {
    pub fn new(room_name: &str, my_username: &str) -> Self {
        let secret_bytes: [u8; 32] = rand::random();
        let static_secret = StaticSecret::from(secret_bytes);
        let public_key = PublicKey::from(&static_secret);

        Self {
            room_name: room_name.to_string(),
            my_username: my_username.to_string(),
            static_secret,
            public_key,
            peers: HashMap::new(),
            outbound_key_id: None,
            outbound_room_key: None,
            outbound_wrapped_for: HashSet::new(),
            inbound_keys: HashMap::new(),
            pending_payloads: HashMap::new(),
        }
    }

    pub fn key_announce_event(&self) -> ClientEvent {
        ClientEvent::CryptoKeyAnnounce(CryptoKeyAnnouncePayload {
            username: self.my_username.clone(),
            public_key_b64: BASE64.encode(self.public_key.as_bytes()),
        })
    }

    pub fn should_encrypt_event(event: &ClientEvent) -> bool {
        Self::kind_for_event(event).is_some()
    }

    pub fn handle_key_announce(
        &mut self,
        payload: &CryptoKeyAnnouncePayload,
    ) -> Result<Vec<ClientEvent>, String> {
        if payload.username == self.my_username {
            return Ok(Vec::new());
        }

        let peer_public = decode_public_key(&payload.public_key_b64)?;
        self.peers.insert(payload.username.clone(), peer_public);
        self.wrap_events_for_known_peers()
    }

    pub fn handle_key_wrap(
        &mut self,
        payload: &CryptoKeyWrapPayload,
    ) -> Result<Vec<ClientEvent>, String> {
        if payload.recipient_username != self.my_username {
            return Ok(Vec::new());
        }

        let sender_public = decode_public_key(&payload.sender_public_key_b64)?;
        let shared_secret = self.static_secret.diffie_hellman(&sender_public);
        let wrap_key = derive_wrap_key(
            &self.room_name,
            &payload.key_id,
            &payload.sender_username,
            &payload.recipient_username,
            shared_secret.as_bytes(),
        );
        let nonce = decode_nonce(&payload.nonce_b64)?;
        let wrapped = BASE64
            .decode(&payload.wrapped_key_b64)
            .map_err(|error| format!("failed to decode wrapped key payload: {error}"))?;
        let aad = key_wrap_aad(
            &self.room_name,
            &payload.key_id,
            &payload.sender_username,
            &payload.recipient_username,
        );

        let room_key_vec = decrypt_bytes(&wrap_key, nonce, &aad, &wrapped)
            .map_err(|error| format!("failed to decrypt wrapped room key: {error}"))?;
        let room_key: [u8; 32] = room_key_vec
            .as_slice()
            .try_into()
            .map_err(|_| "wrapped room key has invalid length".to_string())?;
        self.inbound_keys.insert(payload.key_id.clone(), room_key);

        self.decrypt_pending_for_key(&payload.key_id)
    }

    pub fn prepare_encrypted_events(
        &mut self,
        event: &ClientEvent,
    ) -> Result<Vec<ClientEvent>, String> {
        let kind =
            Self::kind_for_event(event).ok_or_else(|| "event is not encryptable".to_string())?;
        self.ensure_outbound_key();
        let key_id = self
            .outbound_key_id
            .clone()
            .ok_or_else(|| "missing outbound key id".to_string())?;
        let room_key = self
            .outbound_room_key
            .ok_or_else(|| "missing outbound room key".to_string())?;

        let mut events = self.wrap_events_for_known_peers()?;
        let plaintext = serde_json::to_vec(event)
            .map_err(|error| format!("failed to serialize encrypted event: {error}"))?;
        let nonce: [u8; 12] = rand::random();
        let aad = payload_aad(&self.room_name, &self.my_username, &key_id, &kind);
        let ciphertext = encrypt_bytes(&room_key, nonce, &aad, &plaintext)
            .map_err(|error| format!("failed to encrypt payload: {error}"))?;

        events.push(ClientEvent::CryptoPayload(CryptoPayload {
            version: CRYPTO_VERSION,
            key_id,
            sender_username: self.my_username.clone(),
            kind,
            nonce_b64: BASE64.encode(nonce),
            ciphertext_b64: BASE64.encode(ciphertext),
        }));

        Ok(events)
    }

    pub fn decrypt_payload(
        &mut self,
        payload: &CryptoPayload,
    ) -> Result<Option<ClientEvent>, String> {
        if payload.version != CRYPTO_VERSION {
            return Err(format!(
                "unsupported crypto payload version: {}",
                payload.version
            ));
        }

        let Some(room_key) = self.lookup_key(&payload.key_id) else {
            self.store_pending(payload);
            return Ok(None);
        };

        self.decrypt_payload_with_key(payload, room_key).map(Some)
    }

    fn lookup_key(&self, key_id: &str) -> Option<[u8; 32]> {
        if self.outbound_key_id.as_deref() == Some(key_id)
            && let Some(key) = self.outbound_room_key
        {
            return Some(key);
        }
        self.inbound_keys.get(key_id).copied()
    }

    fn decrypt_payload_with_key(
        &self,
        payload: &CryptoPayload,
        room_key: [u8; 32],
    ) -> Result<ClientEvent, String> {
        let nonce = decode_nonce(&payload.nonce_b64)?;
        let ciphertext = BASE64
            .decode(&payload.ciphertext_b64)
            .map_err(|error| format!("failed to decode encrypted payload: {error}"))?;
        let aad = payload_aad(
            &self.room_name,
            &payload.sender_username,
            &payload.key_id,
            &payload.kind,
        );
        let plaintext = decrypt_bytes(&room_key, nonce, &aad, &ciphertext)
            .map_err(|error| format!("failed to decrypt payload: {error}"))?;
        let event = serde_json::from_slice::<ClientEvent>(&plaintext)
            .map_err(|error| format!("failed to parse decrypted payload: {error}"))?;
        let Some(expected_kind) = Self::kind_for_event(&event) else {
            return Err("decrypted payload contains disallowed event type".to_string());
        };
        if expected_kind != payload.kind {
            return Err("decrypted payload kind mismatch".to_string());
        }
        Ok(event)
    }

    fn wrap_events_for_known_peers(&mut self) -> Result<Vec<ClientEvent>, String> {
        let Some(key_id) = self.outbound_key_id.clone() else {
            return Ok(Vec::new());
        };
        let Some(room_key) = self.outbound_room_key else {
            return Ok(Vec::new());
        };

        let peers = self
            .peers
            .iter()
            .map(|(username, public_key)| (username.clone(), *public_key))
            .collect::<Vec<_>>();
        let mut events = Vec::new();

        for (peer_username, peer_public) in peers {
            if self.outbound_wrapped_for.contains(&peer_username) {
                continue;
            }

            events.push(self.build_key_wrap_event(
                &key_id,
                room_key,
                &peer_username,
                peer_public,
            )?);
            self.outbound_wrapped_for.insert(peer_username);
        }

        Ok(events)
    }

    fn build_key_wrap_event(
        &self,
        key_id: &str,
        room_key: [u8; 32],
        peer_username: &str,
        peer_public: PublicKey,
    ) -> Result<ClientEvent, String> {
        let shared_secret = self.static_secret.diffie_hellman(&peer_public);
        let wrap_key = derive_wrap_key(
            &self.room_name,
            key_id,
            &self.my_username,
            peer_username,
            shared_secret.as_bytes(),
        );
        let nonce: [u8; 12] = rand::random();
        let aad = key_wrap_aad(&self.room_name, key_id, &self.my_username, peer_username);
        let ciphertext = encrypt_bytes(&wrap_key, nonce, &aad, &room_key)
            .map_err(|error| format!("failed to encrypt key wrap payload: {error}"))?;

        Ok(ClientEvent::CryptoKeyWrap(CryptoKeyWrapPayload {
            key_id: key_id.to_string(),
            sender_username: self.my_username.clone(),
            recipient_username: peer_username.to_string(),
            sender_public_key_b64: BASE64.encode(self.public_key.as_bytes()),
            nonce_b64: BASE64.encode(nonce),
            wrapped_key_b64: BASE64.encode(ciphertext),
        }))
    }

    fn ensure_outbound_key(&mut self) {
        if self.outbound_room_key.is_some() {
            return;
        }
        self.outbound_room_key = Some(rand::random());
        self.outbound_key_id = Some(uuid::Uuid::new_v4().to_string());
        self.outbound_wrapped_for.clear();
    }

    fn store_pending(&mut self, payload: &CryptoPayload) {
        let pending = self
            .pending_payloads
            .entry(payload.key_id.clone())
            .or_default();
        pending.push(payload.clone());
        if pending.len() > MAX_PENDING_PER_KEY {
            pending.drain(0..pending.len() - MAX_PENDING_PER_KEY);
        }
    }

    fn decrypt_pending_for_key(&mut self, key_id: &str) -> Result<Vec<ClientEvent>, String> {
        let Some(payloads) = self.pending_payloads.remove(key_id) else {
            return Ok(Vec::new());
        };
        let Some(room_key) = self.lookup_key(key_id) else {
            return Ok(Vec::new());
        };

        let mut decrypted = Vec::new();
        for payload in payloads {
            match self.decrypt_payload_with_key(&payload, room_key) {
                Ok(event) => decrypted.push(event),
                Err(error) => return Err(format!("failed to decrypt queued payload: {error}")),
            }
        }
        Ok(decrypted)
    }

    fn kind_for_event(event: &ClientEvent) -> Option<EncryptedPayloadKind> {
        match event {
            ClientEvent::ChatMessage(_) => Some(EncryptedPayloadKind::Chat),
            ClientEvent::NoteUpsert(_) | ClientEvent::NoteDelete(_) => {
                Some(EncryptedPayloadKind::Note)
            }
            ClientEvent::SyncSnapshot(_) => Some(EncryptedPayloadKind::Sync),
            ClientEvent::FileAnnounce(_)
            | ClientEvent::FileRequest(_)
            | ClientEvent::FileAbort(_) => Some(EncryptedPayloadKind::FileControl),
            ClientEvent::FileChunk(_) => Some(EncryptedPayloadKind::FileChunk),
            _ => None,
        }
    }
}

fn decode_public_key(public_key_b64: &str) -> Result<PublicKey, String> {
    let bytes = BASE64
        .decode(public_key_b64)
        .map_err(|error| format!("failed to decode public key: {error}"))?;
    let raw: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "public key has invalid length".to_string())?;
    Ok(PublicKey::from(raw))
}

fn decode_nonce(nonce_b64: &str) -> Result<[u8; 12], String> {
    let nonce_bytes = BASE64
        .decode(nonce_b64)
        .map_err(|error| format!("failed to decode nonce: {error}"))?;
    nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "nonce has invalid length".to_string())
}

fn encrypt_bytes(
    key: &[u8; 32],
    nonce: [u8; 12],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, chacha20poly1305::aead::Error> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher.encrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: plaintext,
            aad,
        },
    )
}

fn decrypt_bytes(
    key: &[u8; 32],
    nonce: [u8; 12],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, chacha20poly1305::aead::Error> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher.decrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: ciphertext,
            aad,
        },
    )
}

fn derive_wrap_key(
    room_name: &str,
    key_id: &str,
    sender_username: &str,
    recipient_username: &str,
    shared_secret: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(KEY_WRAP_CONTEXT);
    hasher.update(room_name.as_bytes());
    hasher.update([0]);
    hasher.update(key_id.as_bytes());
    hasher.update([0]);
    hasher.update(sender_username.as_bytes());
    hasher.update([0]);
    hasher.update(recipient_username.as_bytes());
    hasher.update([0]);
    hasher.update(shared_secret);
    hasher.finalize().into()
}

fn key_wrap_aad(
    room_name: &str,
    key_id: &str,
    sender_username: &str,
    recipient_username: &str,
) -> Vec<u8> {
    [
        room_name.as_bytes(),
        b"|",
        key_id.as_bytes(),
        b"|",
        sender_username.as_bytes(),
        b"|",
        recipient_username.as_bytes(),
    ]
    .concat()
}

fn payload_aad(
    room_name: &str,
    sender_username: &str,
    key_id: &str,
    kind: &EncryptedPayloadKind,
) -> Vec<u8> {
    let kind_tag = match kind {
        EncryptedPayloadKind::Chat => b"CHAT".as_slice(),
        EncryptedPayloadKind::Note => b"NOTE".as_slice(),
        EncryptedPayloadKind::Sync => b"SYNC".as_slice(),
        EncryptedPayloadKind::FileControl => b"FILE_CONTROL".as_slice(),
        EncryptedPayloadKind::FileChunk => b"FILE_CHUNK".as_slice(),
    };
    let mut hasher = Sha256::new();
    hasher.update(PAYLOAD_CONTEXT);
    hasher.update(room_name.as_bytes());
    hasher.update([0]);
    hasher.update(sender_username.as_bytes());
    hasher.update([0]);
    hasher.update(key_id.as_bytes());
    hasher.update([0]);
    hasher.update(kind_tag);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::events::{ChatMessagePayload, SyncSnapshotPackedStatePayload, SyncSnapshotPayload};

    #[test]
    fn key_wrap_then_payload_round_trip_between_clients() {
        let mut alice = RoomCryptoState::new("room-alpha", "alice");
        let mut bob = RoomCryptoState::new("room-alpha", "bob");

        let alice_announce = match alice.key_announce_event() {
            ClientEvent::CryptoKeyAnnounce(payload) => payload,
            _ => unreachable!(),
        };
        let bob_announce = match bob.key_announce_event() {
            ClientEvent::CryptoKeyAnnounce(payload) => payload,
            _ => unreachable!(),
        };

        bob.handle_key_announce(&alice_announce).unwrap();
        let _ = alice.handle_key_announce(&bob_announce).unwrap();

        let message = ClientEvent::ChatMessage(ChatMessagePayload {
            payload: "hello".to_string(),
            username: "alice".to_string(),
            attachments: Vec::new(),
        });

        let outbound = alice.prepare_encrypted_events(&message).unwrap();
        let mut decrypted = None;
        for event in outbound {
            match event {
                ClientEvent::CryptoKeyWrap(payload) => {
                    let pending = bob.handle_key_wrap(&payload).unwrap();
                    if let Some(event) = pending.into_iter().next() {
                        decrypted = Some(event);
                    }
                }
                ClientEvent::CryptoPayload(payload) => {
                    decrypted = bob.decrypt_payload(&payload).unwrap();
                }
                _ => {}
            }
        }

        match decrypted {
            Some(ClientEvent::ChatMessage(payload)) => {
                assert_eq!(payload.payload, "hello");
                assert_eq!(payload.username, "alice");
            }
            _ => panic!("expected decrypted chat message"),
        }
    }

    #[test]
    fn sync_snapshot_is_marked_as_encryptable() {
        let snapshot = ClientEvent::SyncSnapshot(SyncSnapshotPayload {
            version: 7,
            packed_state: SyncSnapshotPackedStatePayload {
                codec_version: 1,
                compression: "gzip".to_string(),
                payload_b64: "cGF5bG9hZA==".to_string(),
            },
        });
        assert!(RoomCryptoState::should_encrypt_event(&snapshot));
    }
}

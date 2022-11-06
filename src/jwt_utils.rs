use std::sync::Arc;
use actix_jwt_authc::*;
use dashmap::DashSet;
use futures::channel::{mpsc, mpsc::{channel, Sender}};
use futures::{SinkExt, Stream};
use jsonwebtoken::*;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use time::ext::*;
use tokio::sync::Mutex;

const JWT_SIGNING_ALGO: Algorithm = Algorithm::EdDSA;

pub struct JwtSigningKeys {
    pub encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtSigningKeys {
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let keypair = Ed25519KeyPair::from_pkcs8(doc.as_ref()).unwrap();
        let encoding_key = EncodingKey::from_ed_der(doc.as_ref());
        let decoding_key = DecodingKey::from_ed_der(keypair.public_key().as_ref());

        Ok(JwtSigningKeys {
            encoding_key,
            decoding_key,
        })
    }
}

#[derive(Clone)]
pub struct InvalidatedJWTStore {
    store: Arc<DashSet<JWT>>,
    tx: Arc<Mutex<Sender<InvalidatedTokensEvent>>>,
}

impl InvalidatedJWTStore {

    /// Returns a [InvalidatedJWTStore] with a Stream of [InvalidatedTokensEvent]s
    pub fn new_with_stream() -> (InvalidatedJWTStore, impl Stream<Item = InvalidatedTokensEvent>) {
        let invalidated = Arc::new(DashSet::new());
        let (tx, rx) = mpsc::channel(100);
        let tx_to_hold = Arc::new(Mutex::new(tx));
        (
            InvalidatedJWTStore {
                store: invalidated,
                tx: tx_to_hold,
            },
            rx,
        )
    }

    pub async fn add_to_invalidated(&self, authenticated: Authenticated<Claims>) {
        self.store.insert(authenticated.jwt.clone());
        let mut tx = self.tx.lock().await;
        if let Err(_e) = tx
            .send(InvalidatedTokensEvent::Add(authenticated.jwt))
            .await
        {
            #[cfg(feature = "tracing")]
            error!(error = ?_e, "Failed to send update on adding to invalidated")
        }
    }
}

pub fn get_jwt_ttl() -> JWTTtl {
    JWTTtl(time::Duration::hours(1))
}

#[derive(Clone, Copy)]
pub struct JWTTtl(pub time::Duration);

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

pub fn get_auth_middleware_settings(jwt_signing_keys: &JwtSigningKeys) -> AuthenticateMiddlewareSettings {
    AuthenticateMiddlewareSettings {
        jwt_decoding_key: jwt_signing_keys.decoding_key.clone(),
        jwt_authorization_header_prefixes: Some(vec!["Bearer".to_string()]),
        jwt_validator: Validation::new(JWT_SIGNING_ALGO),
    }
}

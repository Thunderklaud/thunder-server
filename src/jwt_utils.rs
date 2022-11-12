use std::sync::Arc;

use actix_jwt_authc::*;
use dashmap::DashSet;
use futures::channel::{mpsc, mpsc::Sender};
use futures::{SinkExt, Stream};
use hmac::{Hmac, Mac};
use jsonwebtoken::*;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use tokio::sync::Mutex;

pub const JWT_SIGNING_ALGO: Algorithm = Algorithm::HS512;
type HmacSha512 = Hmac<Sha512>;

pub struct JwtSigningKeys {
    pub encoding_key: EncodingKey, // encode and sign the jwt on login
    decoding_key: DecodingKey,     // check if the sign of an existing token is valid
}

impl JwtSigningKeys {
    pub fn parse(secret: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let encoding_key = EncodingKey::from_base64_secret(secret).unwrap();
        let decoding_key = DecodingKey::from_base64_secret(secret).unwrap();

        Ok(JwtSigningKeys {
            encoding_key,
            decoding_key,
        })
    }
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let mut randoms: [u8; 64] = [0; 64];
        let sr = SystemRandom::new();
        sr.fill(&mut randoms)
            .expect("failed to create random bytes for the jwt secret");

        let mac = HmacSha512::new_from_slice(&randoms).unwrap().to_owned();
        let secret = base64::encode(mac.finalize().into_bytes());
        println!("secret: {}", secret);

        JwtSigningKeys::parse(secret.as_str())
    }
}

#[derive(Clone)]
pub struct InvalidatedJWTStore {
    store: Arc<DashSet<JWT>>,
    tx: Arc<Mutex<Sender<InvalidatedTokensEvent>>>,
}

impl InvalidatedJWTStore {
    /// Returns a [InvalidatedJWTStore] with a Stream of [InvalidatedTokensEvent]s
    pub fn new_with_stream() -> (
        InvalidatedJWTStore,
        impl Stream<Item = InvalidatedTokensEvent>,
    ) {
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

    pub async fn add_to_invalidated(&self, authenticated: Authenticated<Claims>) -> bool {
        self.store.insert(authenticated.jwt.clone());
        let mut tx = self.tx.lock().await;
        if let Err(_e) = tx
            .send(InvalidatedTokensEvent::Add(authenticated.jwt))
            .await
        {
            #[cfg(feature = "tracing")]
            error!(error = ?_e, "Failed to send update on adding to invalidated");
            return false;
        }
        true
    }
}

pub fn get_jwt_ttl() -> JWTTtl {
    JWTTtl(time::Duration::hours(1))
}

#[derive(Clone, Copy)]
pub struct JWTTtl(pub time::Duration);

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub sub: String,
}

pub fn get_auth_middleware_settings(
    jwt_signing_keys: &JwtSigningKeys,
) -> AuthenticateMiddlewareSettings {
    AuthenticateMiddlewareSettings {
        jwt_decoding_key: jwt_signing_keys.decoding_key.clone(),
        jwt_authorization_header_prefixes: Some(vec!["Bearer".to_string()]),
        jwt_validator: Validation::new(JWT_SIGNING_ALGO),
    }
}

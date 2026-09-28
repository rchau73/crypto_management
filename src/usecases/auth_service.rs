use crate::domain::models::User;
use crate::domain::repository::{RefreshTokenRepo, UserRepo};
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;

pub const ACCESS_TOKEN_TTL_MINUTES: i64 = 15;
pub const REFRESH_TOKEN_TTL_DAYS: i64 = 14;

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    InvalidToken,
    Internal(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "invalid username or password"),
            AuthError::InvalidToken => write!(f, "invalid or expired token"),
            AuthError::Internal(msg) => write!(f, "internal auth error: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for AuthError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        AuthError::Internal(e.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String, // user id, as a string (JWT convention)
    pub role: String,
    pub exp: usize,
}

pub struct LoginResult {
    pub access_token: String,
    pub refresh_token: String,
    pub user: User,
}

/// Argon2id hash of a plaintext password. Never store or compare plaintext.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Internal(format!("failed to hash password: {e}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

fn issue_access_token(user_id: i64, role: &str, secret: &[u8]) -> Result<String, AuthError> {
    let exp = (Utc::now() + Duration::minutes(ACCESS_TOKEN_TTL_MINUTES)).timestamp() as usize;
    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AuthError::Internal(format!("failed to sign access token: {e}")))
}

/// Verifies an access token's signature and expiry, returning its claims.
pub fn verify_access_token(token: &str, secret: &[u8]) -> Result<AccessTokenClaims, AuthError> {
    decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AuthError::InvalidToken)
}

/// A fresh opaque refresh token (the raw value handed to the client) — never
/// stored directly, only its SHA-256 hash (see `hash_token`), so a DB leak
/// alone does not yield a usable token.
fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(64), |mut acc, b| {
        use std::fmt::Write;
        write!(acc, "{b:02x}").unwrap();
        acc
    })
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub struct AuthService {
    users: Arc<dyn UserRepo>,
    refresh_tokens: Arc<dyn RefreshTokenRepo>,
    jwt_secret: Arc<Vec<u8>>,
}

impl AuthService {
    pub fn new(
        users: Arc<dyn UserRepo>,
        refresh_tokens: Arc<dyn RefreshTokenRepo>,
        jwt_secret: Arc<Vec<u8>>,
    ) -> Self {
        Self {
            users,
            refresh_tokens,
            jwt_secret,
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResult, AuthError> {
        let user = self
            .users
            .find_user_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        let user_id = user
            .id
            .ok_or_else(|| AuthError::Internal("user row missing id".into()))?;
        let access_token = issue_access_token(user_id, &user.role, &self.jwt_secret)?;

        let refresh_token = generate_refresh_token();
        let expires_at = (Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS)).to_rfc3339();
        self.refresh_tokens
            .store_refresh_token(user_id, &hash_token(&refresh_token), &expires_at)
            .await?;

        Ok(LoginResult {
            access_token,
            refresh_token,
            user,
        })
    }

    /// Exchanges a still-valid refresh token for a new access token, without
    /// requiring the password again.
    pub async fn refresh(&self, refresh_token: &str) -> Result<String, AuthError> {
        let stored = self
            .refresh_tokens
            .find_refresh_token(&hash_token(refresh_token))
            .await?
            .ok_or(AuthError::InvalidToken)?;

        if stored.revoked_at.is_some() {
            return Err(AuthError::InvalidToken);
        }
        let expires_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&stored.expires_at)
            .map_err(|_| AuthError::InvalidToken)?
            .into();
        if expires_at < Utc::now() {
            return Err(AuthError::InvalidToken);
        }

        let user = self
            .users
            .find_user_by_id(stored.user_id)
            .await?
            .ok_or(AuthError::InvalidToken)?;
        let user_id = user
            .id
            .ok_or_else(|| AuthError::Internal("user row missing id".into()))?;
        issue_access_token(user_id, &user.role, &self.jwt_secret)
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        self.refresh_tokens
            .revoke_refresh_token(&hash_token(refresh_token))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_password_never_returns_the_plaintext() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert_ne!(hash, "correct horse battery staple");
        assert!(verify_password("correct horse battery staple", &hash));
    }

    #[test]
    fn verify_password_rejects_the_wrong_password() {
        let hash = hash_password("right-password").unwrap();
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn verify_password_rejects_a_malformed_hash_instead_of_panicking() {
        assert!(!verify_password("anything", "not-a-real-argon2-hash"));
    }

    #[test]
    fn two_hashes_of_the_same_password_are_not_equal() {
        // Argon2 salts each hash independently; this guards against a
        // regression that reuses a fixed salt.
        let a = hash_password("same-password").unwrap();
        let b = hash_password("same-password").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("same-password", &a));
        assert!(verify_password("same-password", &b));
    }

    #[test]
    fn access_token_round_trips_and_carries_role() {
        let secret = b"test-secret-at-least-this-long";
        let token = issue_access_token(42, "manager", secret).unwrap();
        let claims = verify_access_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.role, "manager");
    }

    #[test]
    fn access_token_verification_fails_with_the_wrong_secret() {
        let token = issue_access_token(1, "user", b"secret-a-of-sufficient-length").unwrap();
        let result = verify_access_token(&token, b"secret-b-of-sufficient-length");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn access_token_verification_fails_for_garbage_input() {
        let result = verify_access_token("not.a.jwt", b"secret-of-sufficient-length");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn refresh_tokens_are_long_random_and_never_stored_in_plaintext() {
        let a = generate_refresh_token();
        let b = generate_refresh_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 64); // 32 bytes, hex-encoded
        assert_ne!(hash_token(&a), a);
    }

    #[test]
    fn hash_token_is_deterministic() {
        let token = generate_refresh_token();
        assert_eq!(hash_token(&token), hash_token(&token));
    }
}

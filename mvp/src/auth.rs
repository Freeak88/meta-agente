#![allow(dead_code)]

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthResponse {
    pub account: Account,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub plan: Plan,
    pub credits: u64,
    pub max_agents: u32,
    pub max_workers: u32,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    AccountExists,
    AccountNotFound,
    InvalidPassword,
    InvalidToken,
    InvalidApiKey,
    HashError(String),
}

#[derive(Debug, Clone)]
struct StoredAccount {
    account: Account,
    password_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuthService {
    accounts: HashMap<String, StoredAccount>,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(jwt_secret: impl Into<String>) -> Self {
        Self {
            accounts: HashMap::new(),
            jwt_secret: jwt_secret.into(),
        }
    }

    pub fn register(&mut self, request: RegisterRequest) -> Result<AuthResponse, AuthError> {
        let email = request.email.trim().to_lowercase();
        if self.accounts.contains_key(&email) {
            return Err(AuthError::AccountExists);
        }

        let account = Account {
            id: format!("acct_{}", stable_id(&email)),
            email: email.clone(),
            plan: Plan::Free,
            credits: 1_000,
            max_agents: 3,
            max_workers: 1,
            api_key: format!("ak_{}", stable_id(&format!("api:{email}"))),
        };
        let password_hash = hash_password(&request.password)?;

        self.accounts.insert(
            email,
            StoredAccount {
                account: account.clone(),
                password_hash,
            },
        );

        Ok(AuthResponse {
            token: self.issue_jwt(&account)?,
            account,
        })
    }

    pub fn login(&self, request: LoginRequest) -> Result<AuthResponse, AuthError> {
        let email = request.email.trim().to_lowercase();
        let stored = self
            .accounts
            .get(&email)
            .ok_or(AuthError::AccountNotFound)?;

        verify_password(&request.password, &stored.password_hash)?;

        Ok(AuthResponse {
            token: self.issue_jwt(&stored.account)?,
            account: stored.account.clone(),
        })
    }

    pub fn validate_jwt(&self, token: &str) -> Result<Account, AuthError> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AuthError::InvalidToken)?;

        self.accounts
            .values()
            .find(|stored| stored.account.id == data.claims.sub)
            .map(|stored| stored.account.clone())
            .ok_or(AuthError::InvalidToken)
    }

    pub fn validate_api_key(&self, key: &str) -> Result<Account, AuthError> {
        self.accounts
            .values()
            .find(|stored| stored.account.api_key == key)
            .map(|stored| stored.account.clone())
            .ok_or(AuthError::InvalidApiKey)
    }

    fn issue_jwt(&self, account: &Account) -> Result<String, AuthError> {
        let exp = Utc::now() + Duration::hours(24);
        let claims = Claims {
            sub: account.id.clone(),
            email: account.email.clone(),
            exp: exp.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|err| AuthError::HashError(err.to_string()))
    }
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AuthError::HashError(err.to_string()))
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), AuthError> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|err| AuthError::HashError(err.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidPassword)
}

fn stable_id(input: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(input.as_bytes());
    hex_prefix(&digest, 12)
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| {
            let value = format!("{:02x}", byte);
            value.chars().collect::<Vec<_>>()
        })
        .take(len)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> AuthService {
        AuthService::new("test-secret")
    }

    fn register_request() -> RegisterRequest {
        RegisterRequest {
            email: "user@example.com".to_string(),
            password: "correct horse battery staple".to_string(),
        }
    }

    #[test]
    fn auth_register_login_and_validate_jwt() {
        let mut auth = service();
        let registered = auth.register(register_request()).unwrap();

        assert_eq!(registered.account.email, "user@example.com");
        assert_eq!(registered.account.plan, Plan::Free);
        assert!(!registered.token.is_empty());

        let login = auth
            .login(LoginRequest {
                email: "user@example.com".to_string(),
                password: "correct horse battery staple".to_string(),
            })
            .unwrap();
        let account = auth.validate_jwt(&login.token).unwrap();

        assert_eq!(account.id, registered.account.id);
    }

    #[test]
    fn auth_rejects_invalid_password() {
        let mut auth = service();
        auth.register(register_request()).unwrap();

        let err = auth
            .login(LoginRequest {
                email: "user@example.com".to_string(),
                password: "wrong".to_string(),
            })
            .unwrap_err();

        assert_eq!(err, AuthError::InvalidPassword);
    }

    #[test]
    fn auth_rejects_invalid_jwt() {
        let mut auth = service();
        auth.register(register_request()).unwrap();

        let err = auth.validate_jwt("not-a-token").unwrap_err();

        assert_eq!(err, AuthError::InvalidToken);
    }

    #[test]
    fn auth_validates_api_key() {
        let mut auth = service();
        let registered = auth.register(register_request()).unwrap();

        let account = auth.validate_api_key(&registered.account.api_key).unwrap();

        assert_eq!(account.id, registered.account.id);
    }

    #[test]
    fn auth_rejects_invalid_api_key() {
        let mut auth = service();
        auth.register(register_request()).unwrap();

        let err = auth.validate_api_key("ak_missing").unwrap_err();

        assert_eq!(err, AuthError::InvalidApiKey);
    }
}

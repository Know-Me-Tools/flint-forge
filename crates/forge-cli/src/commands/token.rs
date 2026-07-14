//! `forge token mint` — mint a smoke-test JWT.

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::Args;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Args)]
pub struct TokenMintArgs {
    /// JWT signing secret. Falls back to FLINT_JWT_SECRET env.
    #[arg(long, env = "FLINT_JWT_SECRET")]
    pub secret: Option<String>,
    /// JWT subject.
    #[arg(long, default_value = "smoke")]
    pub subject: String,
    /// Caller role.
    #[arg(long, default_value = "authenticated")]
    pub role: String,
    /// Token expiry in seconds.
    #[arg(long, default_value_t = 3600)]
    pub expiry_seconds: i64,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    iat: i64,
    exp: i64,
}

pub fn token_mint(args: TokenMintArgs) -> Result<()> {
    let secret = args
        .secret
        .with_context(|| "FLINT_JWT_SECRET or --secret required")?;
    let now = Utc::now();
    let claims = Claims {
        sub: args.subject,
        role: args.role,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(args.expiry_seconds)).timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .with_context(|| "failed to encode JWT")?;
    println!("{token}");
    Ok(())
}

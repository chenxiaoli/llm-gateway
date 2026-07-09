//! Email subsystem for the LLM Gateway.
//!
//! Provides a [`Mailer`] trait abstracting over transports (SMTP, file, noop)
//! and a [`dispatch_with_retry`] helper for fire-and-forget sends with
//! 3-attempt exponential backoff.

pub mod file;
pub mod noop;
pub mod smtp;
pub mod templates;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One outbound email. `html_body` is optional — plain-text is always present
/// for maximum client compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("SMTP transport error: {0}")]
    Smtp(String),
    #[error("file transport error: {0}")]
    File(String),
    #[error("invalid email address: {0}")]
    InvalidAddress(String),
    #[error("template render error: {0}")]
    Template(String),
    #[error("failed to build email message: {0}")]
    Build(String),
}

impl From<lettre::transport::smtp::Error> for EmailError {
    fn from(e: lettre::transport::smtp::Error) -> Self {
        EmailError::Smtp(e.to_string())
    }
}

impl From<lettre::transport::file::Error> for EmailError {
    fn from(e: lettre::transport::file::Error) -> Self {
        EmailError::File(e.to_string())
    }
}

impl From<lettre::address::AddressError> for EmailError {
    fn from(e: lettre::address::AddressError) -> Self {
        EmailError::InvalidAddress(e.to_string())
    }
}

impl From<lettre::error::Error> for EmailError {
    fn from(e: lettre::error::Error) -> Self {
        EmailError::Build(e.to_string())
    }
}

impl From<handlebars::RenderError> for EmailError {
    fn from(e: handlebars::RenderError) -> Self {
        EmailError::Template(e.to_string())
    }
}

/// Send an [`EmailMessage`]. Implementations must be safe to call from a
/// `tokio::spawn`'d task — no borrowed runtime state.
#[async_trait::async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, msg: EmailMessage) -> Result<(), EmailError>;
}

/// Parse a `"Name <addr@host>"` pair into a `lettre::Mailbox`. An empty
/// `name` is treated as if the address were written alone (`"addr@host>"`),
/// which is the canonical form lettre's parser accepts.
pub(crate) fn parse_mailbox(name: &str, address: &str) -> Result<lettre::message::Mailbox, EmailError> {
    use lettre::message::Mailbox;
    let s = if name.is_empty() {
        address.to_string()
    } else {
        format!("{name} <{address}>")
    };
    s.parse::<Mailbox>()
        .map_err(|e: lettre::address::AddressError| EmailError::InvalidAddress(e.to_string()))
}

/// Fire-and-forget dispatch with 3-attempt exponential backoff (1s → 2s → 4s).
///
/// Spawns a tokio task; never blocks the caller. On total failure, logs an
/// error and returns. The caller is responsible for any audit-row write on
/// failure before calling this helper.
///
/// Usage:
/// ```ignore
/// let mailer = state.mailer.clone();
/// let msg = state.templates.render_verification(ctx)?;
/// dispatch_with_retry(mailer, msg, "verification email".to_string());
/// ```
pub fn dispatch_with_retry(mailer: std::sync::Arc<dyn Mailer>, msg: EmailMessage, label: String) {
    tokio::spawn(async move {
        let mut backoff = std::time::Duration::from_secs(1);
        for attempt in 0..3u32 {
            match mailer.send(msg.clone()).await {
                Ok(()) => {
                    tracing::info!(%msg.to, %label, attempt, "email sent");
                    return;
                }
                Err(e) if attempt == 2 => {
                    tracing::error!(%msg.to, %label, error = ?e, "email delivery failed after 3 attempts");
                    return;
                }
                Err(e) => {
                    tracing::warn!(%msg.to, %label, attempt, error = ?e, "email send failed, retrying");
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
            }
        }
    });
}

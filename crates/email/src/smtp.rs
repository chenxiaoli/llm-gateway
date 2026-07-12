//! SMTP-backed `Mailer` using `lettre::AsyncSmtpTransport<Tokio1Executor>`.

use crate::{parse_mailbox, EmailError, EmailMessage, Mailer};
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Debug, Clone)]
pub struct SmtpMailer {
    transport: std::sync::Arc<AsyncSmtpTransport<Tokio1Executor>>,
    from_address: String,
    from_name: String,
}

/// Configuration for the SMTP mailer. Construct directly, or wire it from
/// the gateway's `EmailConfig` in `crates/gateway` (Task 6).
#[derive(Debug, Clone)]
pub struct SmtpMailerConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub use_tls: bool,
    pub from_address: String,
    pub from_name: String,
}

impl SmtpMailer {
    /// Build a `Mailer` from the config. The transport is constructed eagerly
    /// so that misconfiguration (bad host lookup, etc.) fails at boot rather
    /// than at first dispatch.
    pub fn new(cfg: SmtpMailerConfig) -> Result<Self, EmailError> {
        let mut builder = if cfg.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
                .map_err(|e| EmailError::Smtp(e.to_string()))?
                .port(cfg.port)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host).port(cfg.port)
        };
        if let (Some(u), Some(p)) = (cfg.username, cfg.password) {
            builder = builder.credentials(Credentials::new(u, p));
        }
        Ok(Self {
            transport: std::sync::Arc::new(builder.build()),
            from_address: cfg.from_address,
            from_name: cfg.from_name,
        })
    }
}

#[async_trait::async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, msg: EmailMessage) -> Result<(), EmailError> {
        let from = parse_mailbox(&self.from_name, &self.from_address)?;
        let to = parse_mailbox("", &msg.to)?;
        let builder = Message::builder().from(from).to(to).subject(&msg.subject);
        // When no HTML is provided, send a single text/plain body. Sending
        // an empty text/html alternative can cause some clients to render
        // nothing.
        let email = match msg.html_body.as_deref() {
            Some(html) => builder.multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(msg.text_body),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html.to_string()),
                    ),
            )?,
            None => {
                builder.multipart(
                    MultiPart::mixed().singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(msg.text_body),
                    ),
                )?
            }
        };
        self.transport.send(email).await?;
        Ok(())
    }
}

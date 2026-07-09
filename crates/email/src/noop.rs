//! `NoopMailer` — discards all messages. Used in unit tests.

use crate::{EmailError, EmailMessage, Mailer};

#[derive(Debug, Clone)]
pub struct NoopMailer;

impl NoopMailer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopMailer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Mailer for NoopMailer {
    async fn send(&self, _msg: EmailMessage) -> Result<(), EmailError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_succeeds() {
        let msg = EmailMessage {
            to: "alice@example.com".into(),
            subject: "test".into(),
            text_body: "hello".into(),
            html_body: None,
        };
        NoopMailer::new().send(msg).await.unwrap();
    }
}

//! File-backed `Mailer` for dev + tests. Writes each message as an `.eml`
//! file under a configured directory. Filename includes timestamp + recipient
//! so tests can grep for the token.

use crate::{parse_mailbox, EmailError, EmailMessage, Mailer};
use lettre::message::Mailbox;
use lettre::{AsyncFileTransport, AsyncTransport, Tokio1Executor};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileMailer {
    out_dir: PathBuf,
    from_address: String,
    from_name: String,
}

impl FileMailer {
    pub fn new(out_dir: impl Into<PathBuf>, from_address: String, from_name: String) -> Self {
        Self {
            out_dir: out_dir.into(),
            from_address,
            from_name,
        }
    }
}

#[async_trait::async_trait]
impl Mailer for FileMailer {
    async fn send(&self, msg: EmailMessage) -> Result<(), EmailError> {
        let transport = AsyncFileTransport::<Tokio1Executor>::new(&self.out_dir);
        let from: Mailbox = parse_mailbox(&self.from_name, &self.from_address)?;
        let to: Mailbox = parse_mailbox("", &msg.to)?;
        let email = lettre::Message::builder()
            .from(from)
            .to(to)
            .subject(&msg.subject)
            .body(msg.text_body)?;
        transport.send(email).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_mailer_writes_eml() {
        let tmp = tempfile::tempdir().unwrap();
        let mailer = FileMailer::new(tmp.path(), "noreply@example.com".into(), "Test".into());
        let msg = EmailMessage {
            to: "alice@example.com".into(),
            subject: "Hello".into(),
            text_body: "TOKEN_123456 body".into(),
            html_body: None,
        };
        mailer.send(msg).await.unwrap();
        // The file transport writes a file per message; assert at least one
        // .eml exists with the token in it.
        let mut found = false;
        for entry in std::fs::read_dir(tmp.path()).unwrap() {
            let entry = entry.unwrap();
            let content = std::fs::read_to_string(entry.path()).unwrap();
            if content.contains("TOKEN_123456") {
                found = true;
                break;
            }
        }
        assert!(found, "expected to find a written .eml containing the body");
    }
}

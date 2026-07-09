//! Handlebars-backed templates for the three Phase 4 email types.
//!
//! Templates are baked into the binary via `include_str!` so deployment is a
//! single artifact. The registry is constructed once at boot and stored in
//! `AppState`.

use crate::{EmailError, EmailMessage};
use handlebars::Handlebars;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    hb: Handlebars<'static>,
    from_address: String,
    from_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationCtx {
    pub username: String,
    pub recipient_email: String,
    pub verification_url: String,
    pub expires_in_hours: u32,
    pub public_base_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvitationCtx {
    pub org_name: String,
    pub inviter_username: String,
    pub role: String,
    pub recipient_email: String,
    pub accept_url: String,
    pub expires_in_days: u32,
    pub public_base_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordResetCtx {
    pub username: String,
    pub recipient_email: String,
    pub reset_url: String,
    pub expires_in_hours: u32,
    pub public_base_url: String,
}

impl TemplateRegistry {
    /// Load all templates from the baked-in include_str! constants. Called once
    /// at boot; errors here are fatal (config issue).
    pub fn load(from_address: String, from_name: String) -> Result<Self, EmailError> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(true);
        hb.register_template_string("verification.txt", include_str!("../templates/verification.txt.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("verification.html", include_str!("../templates/verification.html.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("invitation.txt", include_str!("../templates/invitation.txt.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("invitation.html", include_str!("../templates/invitation.html.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("password_reset.txt", include_str!("../templates/password_reset.txt.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        hb.register_template_string("password_reset.html", include_str!("../templates/password_reset.html.hbs"))
            .map_err(|e| EmailError::Template(e.to_string()))?;
        Ok(Self { hb, from_address, from_name })
    }

    pub fn render_verification(&self, ctx: VerificationCtx) -> Result<EmailMessage, EmailError> {
        Ok(EmailMessage {
            to: ctx.recipient_email.clone(),
            subject: "Verify your email".into(),
            text_body: self.hb.render("verification.txt", &ctx)?,
            html_body: Some(self.hb.render("verification.html", &ctx)?),
        })
    }

    pub fn render_invitation(&self, ctx: InvitationCtx) -> Result<EmailMessage, EmailError> {
        Ok(EmailMessage {
            to: ctx.recipient_email.clone(),
            subject: format!("Invitation to join {}", ctx.org_name),
            text_body: self.hb.render("invitation.txt", &ctx)?,
            html_body: Some(self.hb.render("invitation.html", &ctx)?),
        })
    }

    pub fn render_password_reset(&self, ctx: PasswordResetCtx) -> Result<EmailMessage, EmailError> {
        Ok(EmailMessage {
            to: ctx.recipient_email.clone(),
            subject: "Reset your password".into(),
            text_body: self.hb.render("password_reset.txt", &ctx)?,
            html_body: Some(self.hb.render("password_reset.html", &ctx)?),
        })
    }

    pub fn from_address(&self) -> &str {
        &self.from_address
    }

    pub fn from_name(&self) -> &str {
        &self.from_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> TemplateRegistry {
        TemplateRegistry::load("noreply@example.com".into(), "Test".into()).unwrap()
    }

    #[test]
    fn renders_verification_url() {
        let r = registry();
        let ctx = VerificationCtx {
            username: "alice".into(),
            recipient_email: "alice@example.com".into(),
            verification_url: "https://app.example.com/verify-email/TOKEN".into(),
            expires_in_hours: 24,
            public_base_url: "https://app.example.com".into(),
        };
        let msg = r.render_verification(ctx).unwrap();
        assert_eq!(msg.to, "alice@example.com");
        assert!(msg.text_body.contains("TOKEN"));
        assert!(msg.html_body.unwrap().contains("TOKEN"));
    }

    #[test]
    fn renders_invitation_recipient() {
        let r = registry();
        let ctx = InvitationCtx {
            org_name: "Acme".into(),
            inviter_username: "bob".into(),
            role: "member".into(),
            recipient_email: "alice@example.com".into(),
            accept_url: "https://app.example.com/accept-invite/TOKEN".into(),
            expires_in_days: 7,
            public_base_url: "https://app.example.com".into(),
        };
        let msg = r.render_invitation(ctx).unwrap();
        assert_eq!(msg.to, "alice@example.com");
        assert!(msg.subject.contains("Acme"));
    }

    #[test]
    fn renders_password_reset() {
        let r = registry();
        let ctx = PasswordResetCtx {
            username: "alice".into(),
            recipient_email: "alice@example.com".into(),
            reset_url: "https://app.example.com/reset-password/TOKEN".into(),
            expires_in_hours: 1,
            public_base_url: "https://app.example.com".into(),
        };
        let msg = r.render_password_reset(ctx).unwrap();
        assert_eq!(msg.to, "alice@example.com");
        assert!(msg.text_body.contains("TOKEN"));
    }
}

use std::sync::Arc;

/// Message to be sent via any email implementation.
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: crate::app::domain::Email,
    pub subject: String,
    pub body: String,
    pub from: String,
}

impl EmailMessage {
    /// Create a new email message with default from address (reads MAIL_FROM env var).
    pub fn new(to: crate::app::domain::Email, subject: String, body: String) -> Self {
        Self {
            to,
            subject,
            body,
            from: default_from(),
        }
    }

    /// Create a new email message with custom from address.
    pub fn with_from(to: crate::app::domain::Email, subject: String, body: String, from: String) -> Self {
        Self { to, subject, body, from }
    }
}

/// Abstract interface for sending email. Swappable per environment.
#[async_trait::async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, message: &EmailMessage) -> Result<(), EmailError>;
}

/// Errors that can occur during email sending.
#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("SMTP error: {0}")]
    Smtp(String),
    #[error("Send error: {0}")]
    Send(String),
}

// Re-export implementations
pub use console::ConsoleMailer;
pub use smtp::SmtpMailer;

mod console;
mod smtp;

/// Default from/reply address for emails. Reads MAIL_FROM env var.
pub fn default_from() -> String {
    std::env::var("MAIL_FROM").unwrap_or_else(|_| "please-configure@example.com".to_string())
}

/// Build the email sender from environment variables.
///
/// Reads `MAIL_ADAPTER` (default: "console") and returns the appropriate implementation.
/// For SMTP, also reads `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`.
/// The from address is read from `MAIL_FROM` (used by all adapters).
pub fn from_env() -> Result<Arc<dyn EmailSender>, EmailError> {
    let adapter = std::env::var("MAIL_ADAPTER").unwrap_or_else(|_| "console".to_string());

    match adapter.as_str() {
        "console" => Ok(Arc::new(ConsoleMailer)),
        "smtp" => {
            let host = std::env::var("SMTP_HOST")
                .map_err(|_| EmailError::Config("SMTP_HOST is required for SMTP adapter".to_string()))?;
            let port = std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse::<u16>()
                .map_err(|_| EmailError::Config("SMTP_PORT must be a valid port number".to_string()))?;
            let user = std::env::var("SMTP_USER").ok();
            let pass = std::env::var("SMTP_PASS").ok();
            let from = default_from();

            Ok(Arc::new(SmtpMailer::new(host, port, user, pass, from)?))
        }
        _ => Err(EmailError::Config(format!("Unknown MAIL_ADAPTER: {}", adapter))),
    }
}
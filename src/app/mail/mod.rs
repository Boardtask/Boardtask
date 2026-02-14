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
    /// Create a new email message with the given from address.
    pub fn new(
        to: crate::app::domain::Email,
        subject: String,
        body: String,
        from: impl Into<String>,
    ) -> Self {
        Self {
            to,
            subject,
            body,
            from: from.into(),
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

/// Build the email sender from config.
pub fn from_config(config: &crate::app::config::Config) -> Result<Arc<dyn EmailSender>, EmailError> {
    match config.mail_adapter.as_str() {
        "console" => Ok(Arc::new(ConsoleMailer)),
        "smtp" => {
            let host = config
                .smtp_host
                .clone()
                .ok_or_else(|| EmailError::Config("SMTP_HOST is required for SMTP adapter".to_string()))?;

            Ok(Arc::new(SmtpMailer::new(
                host,
                config.smtp_port,
                config.smtp_user.clone(),
                config.smtp_pass.clone(),
                config.mail_from.clone(),
            )?))
        }
        _ => Err(EmailError::Config(format!(
            "Unknown MAIL_ADAPTER: {}",
            config.mail_adapter
        ))),
    }
}
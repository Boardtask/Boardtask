use async_trait::async_trait;

use super::{EmailError, EmailMessage, EmailSender};

/// Console email sender for local development.
/// Logs email details to the terminal using tracing::info!.
#[derive(Debug)]
pub struct ConsoleMailer;

#[async_trait]
impl EmailSender for ConsoleMailer {
    async fn send(&self, message: &EmailMessage) -> Result<(), EmailError> {
        tracing::info!(
            to = %message.to.as_str(),
            from = %message.from,
            subject = %message.subject,
            body = %message.body,
            "Email sent (console)"
        );
        Ok(())
    }
}
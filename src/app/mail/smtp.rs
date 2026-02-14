use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, Tokio1Executor,
};

use super::{EmailError, EmailMessage, EmailSender};

/// SMTP email sender for production use.
#[derive(Debug)]
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpMailer {
    /// Create a new SMTP mailer.
    ///
    /// # Arguments
    /// * `host` - SMTP server hostname
    /// * `port` - SMTP server port (typically 587 for STARTTLS, 465 for TLS)
    /// * `user` - SMTP username (optional for some servers)
    /// * `pass` - SMTP password (optional for some servers)
    /// * `from` - Default from address
    pub fn new(
        host: String,
        port: u16,
        user: Option<String>,
        pass: Option<String>,
        from: String,
    ) -> Result<Self, EmailError> {
        let mut transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
            .port(port);

        // Add authentication if provided
        if let (Some(user), Some(pass)) = (user, pass) {
            let creds = Credentials::new(user, pass);
            transport = transport.credentials(creds);
        }

        let transport = transport.build();

        Ok(Self { transport, from })
    }
}

#[async_trait]
impl EmailSender for SmtpMailer {
    async fn send(&self, message: &EmailMessage) -> Result<(), EmailError> {
        let from: Mailbox = self.from.parse()
            .map_err(|e| EmailError::Config(format!("Invalid from address '{}': {}", self.from, e)))?;

        let to: Mailbox = message.to.as_str().parse()
            .map_err(|e| EmailError::Config(format!("Invalid to address '{}': {}", message.to.as_str(), e)))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(&message.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(message.body.clone())
            .map_err(|e| EmailError::Send(format!("Failed to build email message: {}", e)))?;

        lettre::AsyncTransport::send(&self.transport, email).await
            .map(|_| ())
            .map_err(|e| EmailError::Smtp(format!("SMTP send failed: {}", e)))?;

        Ok(())
    }
}
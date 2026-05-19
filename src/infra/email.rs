use crate::errors::AppError;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::env;

#[async_trait]
pub trait EmailService: Send + Sync + 'static {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError>;
}

pub struct MockEmailService;

#[async_trait]
impl EmailService for MockEmailService {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        tracing::info!(
            "📧 [EMAIL MOCK] Enviando e-mail...\nDestinatário: {}\nAssunto: {}\nCorpo: {}",
            to,
            subject,
            body
        );
        Ok(())
    }
}

pub struct SmtpEmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
}

impl SmtpEmailService {
    pub fn new() -> Result<Self, AppError> {
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "1025".to_string())
            .parse::<u16>()
            .unwrap_or(1025);
        let smtp_user = env::var("SMTP_USER").ok();
        let smtp_pass = env::var("SMTP_PASS").ok();
        let from_address =
            env::var("SMTP_FROM").unwrap_or_else(|_| "no-reply@mage.com".to_string());

        let mut transport_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
            .map_err(|e| AppError::Internal(format!("Erro ao criar SMTP relay: {}", e)))?
            .port(smtp_port);

        if let (Some(user), Some(pass)) = (smtp_user, smtp_pass) {
            let credentials = Credentials::new(user, pass);
            transport_builder = transport_builder.credentials(credentials);
        }

        let transport = transport_builder.build();

        Ok(Self {
            transport,
            from_address,
        })
    }
}

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.from_address.parse().map_err(|_| {
                AppError::Internal("Formato de remetente SMTP inválido".to_string())
            })?)
            .to(to.parse().map_err(|_| {
                AppError::BadRequest("Formato de destinatário inválido".to_string())
            })?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| AppError::Internal(format!("Falha ao construir e-mail: {}", e)))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("Falha no envio de e-mail SMTP: {}", e)))?;

        Ok(())
    }
}

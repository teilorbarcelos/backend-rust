use crate::errors::AppError;

pub struct EmailService;

impl EmailService {
    /// Sends a transactional notification email.
    /// Emits a diagnostic trace log in development, behaving as a fast mock.
    pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        tracing::info!(
            "📧 [EMAIL MOCK] Enviando e-mail...\nDestinatário: {}\nAssunto: {}\nCorpo: {}",
            to,
            subject,
            body
        );
        Ok(())
    }
}

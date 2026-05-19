use crate::config::AppConfig;
use crate::errors::AppError;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::{error, info};

pub static MESSAGING_PROVIDER: OnceLock<MessagingProvider> = OnceLock::new();

#[derive(Clone)]
pub struct MessagingProvider {
    connection: Option<Arc<Connection>>,
    channel: Option<Channel>,
    enabled: bool,
}

impl MessagingProvider {
    pub async fn init(config: &AppConfig) -> Result<(), AppError> {
        let enabled = config.messaging_enabled;

        let (connection, channel) = if enabled {
            info!("[RabbitMQ] Connecting to {}...", config.rabbit_url);
            let conn = Connection::connect(&config.rabbit_url, ConnectionProperties::default())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to connect to RabbitMQ: {}", e)))?;

            let chan = conn.create_channel().await.map_err(|e| {
                AppError::Internal(format!("Failed to create RabbitMQ channel: {}", e))
            })?;

            info!("[RabbitMQ] Connected successfully");
            (Some(Arc::new(conn)), Some(chan))
        } else {
            (None, None)
        };

        let provider = Self {
            connection,
            channel,
            enabled,
        };

        // We use set, if it fails because it is already set, we log and ignore (common in hot-reload or test environment)
        if MESSAGING_PROVIDER.set(provider).is_err() {
            error!("[RabbitMQ] MESSAGING_PROVIDER was already initialized");
        }

        Ok(())
    }

    pub fn get() -> &'static Self {
        MESSAGING_PROVIDER
            .get()
            .expect("MessagingProvider is not initialized")
    }

    pub async fn publish<T: Serialize>(&self, queue: &str, message: &T) -> Result<(), AppError> {
        let channel = match &self.channel {
            Some(c) => c,
            None => {
                if self.enabled {
                    return Err(AppError::Internal(
                        "RabbitMQ channel not initialized".to_string(),
                    ));
                }
                return Ok(());
            }
        };

        channel
            .queue_declare(
                queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to assert queue: {}", e)))?;

        let payload = serde_json::to_vec(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        channel
            .basic_publish(
                "",
                queue,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send publish command: {}", e)))?
            .await
            .map_err(|e| AppError::Internal(format!("Failed to confirm publish: {}", e)))?;

        Ok(())
    }

    pub async fn subscribe<T, F, Fut>(&self, queue: &str, callback: F) -> Result<(), AppError>
    where
        T: DeserializeOwned + Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let channel = match &self.channel {
            Some(c) => c,
            None => {
                if self.enabled {
                    return Err(AppError::Internal(
                        "RabbitMQ channel not initialized".to_string(),
                    ));
                }
                return Ok(());
            }
        };

        channel
            .queue_declare(
                queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to assert queue: {}", e)))?;

        use futures_util::stream::StreamExt;

        let mut consumer = channel
            .basic_consume(
                queue,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to basic_consume: {}", e)))?;

        tokio::spawn(async move {
            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => match serde_json::from_slice::<T>(&delivery.data) {
                        Ok(content) => {
                            callback(content).await;
                            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                error!("[RabbitMQ] Failed to ack message: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("[RabbitMQ] Failed to deserialize message: {}", e);
                            if let Err(ack_err) = delivery.ack(BasicAckOptions::default()).await {
                                error!("[RabbitMQ] Failed to ack corrupt message: {}", ack_err);
                            }
                        }
                    },
                    Err(e) => {
                        error!("[RabbitMQ] Error in consumer stream: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), AppError> {
        if let Some(channel) = &self.channel {
            let _ = channel.close(0, "Disconnecting").await;
        }
        if let Some(connection) = &self.connection {
            let _ = connection.close(0, "Disconnecting").await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_messaging_provider_disabled() {
        let provider = MessagingProvider {
            connection: None,
            channel: None,
            enabled: false,
        };

        // publish should return Ok(()) silently when disabled
        let res = provider.publish("test_queue", &"hello").await;
        assert!(res.is_ok());

        // subscribe should return Ok(()) silently when disabled
        let res = provider
            .subscribe("test_queue", |_msg: String| async {})
            .await;
        assert!(res.is_ok());
    }
}

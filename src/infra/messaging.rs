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

            let chan = conn.create_channel().await;

            #[cfg(test)]
            let chan = if config.rabbit_url.contains("FORCE_CHANNEL_ERR") {
                Err(lapin::Error::ChannelsLimitReached)
            } else {
                chan
            };

            let chan = chan.map_err(|e| {
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

        let res = provider.publish("test_queue", &"hello").await;
        assert!(res.is_ok());

        let res = provider
            .subscribe("test_queue", |_msg: String| async {})
            .await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_messaging_provider_enabled_failure_paths() {
        let provider = MessagingProvider {
            connection: None,
            channel: None,
            enabled: true,
        };

        let res = provider.publish("test_queue", &"hello").await;
        assert!(res.is_err());

        let res = provider
            .subscribe("test_queue", |_msg: String| async {})
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_messaging_provider_connect_failure() {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();
        config.messaging_enabled = true;
        config.rabbit_url = "amqp://127.0.0.1:9999".to_string();

        let res = MessagingProvider::init(&config).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_messaging_provider_double_init() {
        dotenvy::dotenv().ok();
        let config = AppConfig::load();
        let res = MessagingProvider::init(&config).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_messaging_provider_get() {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();
        config.messaging_enabled = false;
        let _ = MessagingProvider::init(&config).await;
        let _ = MessagingProvider::get();
    }

    #[tokio::test]
    async fn test_messaging_provider_full_flow() {
        dotenvy::dotenv().ok();
        let config = AppConfig::load();
        let conn = Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await;
        if let Ok(conn) = conn {
            let conn = Arc::new(conn);
            let chan = conn.create_channel().await.unwrap();
            let provider = MessagingProvider {
                connection: Some(conn),
                channel: Some(chan),
                enabled: true,
            };

            let res = provider
                .publish("test_queue_unit", &"hello".to_string())
                .await;
            assert!(res.is_ok());

            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let res = provider
                .subscribe("test_queue_unit", move |msg: String| {
                    let tx = tx.clone();
                    async move {
                        let _ = tx.send(msg).await;
                    }
                })
                .await;
            assert!(res.is_ok());

            let received = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
            assert!(received.is_ok());
            assert_eq!(received.unwrap(), Some("hello".to_string()));

            let res = provider.publish("test_queue_unit", &123).await;
            assert!(res.is_ok());

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            let res = provider.disconnect().await;
            assert!(res.is_ok());
        }
    }

    #[tokio::test]
    async fn test_messaging_provider_init_success() {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();
        config.messaging_enabled = true;

        if let Ok(conn) =
            Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
        {
            drop(conn);

            let res = MessagingProvider::init(&config).await;
            assert!(res.is_ok());
        }
    }

    #[tokio::test]
    async fn test_messaging_ack_failures_and_errors() {
        dotenvy::dotenv().ok();
        let config = AppConfig::load();
        if let Ok(conn) =
            Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
        {
            let conn = Arc::new(conn);
            let chan = conn.create_channel().await.unwrap();
            let provider = MessagingProvider {
                connection: Some(conn),
                channel: Some(chan),
                enabled: true,
            };

            let conn_pub = Arc::new(
                Connection::connect(&config.rabbit_url, ConnectionProperties::default())
                    .await
                    .unwrap(),
            );
            let provider_pub = MessagingProvider {
                connection: Some(conn_pub.clone()),
                channel: Some(conn_pub.create_channel().await.unwrap()),
                enabled: true,
            };

            let (tx1, mut rx1) = tokio::sync::mpsc::channel(1);
            let p_clone1 = provider.clone();
            provider
                .subscribe("queue_ack_fail", move |msg: String| {
                    let tx = tx1.clone();
                    let p = p_clone1.clone();
                    async move {
                        let _ = tx.send(msg).await;
                        let _ = p.disconnect().await;
                    }
                })
                .await
                .unwrap();

            provider_pub
                .publish("queue_ack_fail", &"hello".to_string())
                .await
                .unwrap();
            let _ = rx1.recv().await;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            if let Ok(conn2) =
                Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
            {
                let conn2 = Arc::new(conn2);
                let chan2 = conn2.create_channel().await.unwrap();
                let provider2 = MessagingProvider {
                    connection: Some(conn2),
                    channel: Some(chan2),
                    enabled: true,
                };

                provider2
                    .subscribe("queue_corrupt_fail", |_msg: String| async {})
                    .await
                    .unwrap();
                provider2.publish("queue_corrupt_fail", &123).await.unwrap();
                let _ = provider2.disconnect().await;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }

    #[tokio::test]
    async fn test_messaging_provider_channel_failure() {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();
        config.messaging_enabled = true;

        let separator = if config.rabbit_url.contains('?') {
            "&"
        } else {
            "?"
        };
        config.rabbit_url = format!("{}{}FORCE_CHANNEL_ERR=true", config.rabbit_url, separator);

        if let Ok(conn) =
            Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
        {
            drop(conn);
            let res = MessagingProvider::init(&config).await;
            assert!(res.is_err());
            assert!(res
                .unwrap_err()
                .message()
                .contains("Failed to create RabbitMQ channel"));
        }
    }

    #[tokio::test]
    async fn test_messaging_consumer_stream_error() {
        dotenvy::dotenv().ok();
        let config = AppConfig::load();
        if let Ok(conn) =
            Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
        {
            let conn = Arc::new(conn);
            let chan = conn.create_channel().await.unwrap();
            let provider = MessagingProvider {
                connection: Some(conn),
                channel: Some(chan.clone()),
                enabled: true,
            };

            let queue_name = format!("queue_stream_err_{}", uuid::Uuid::new_v4());
            provider
                .subscribe(&queue_name, |_msg: String| async {})
                .await
                .unwrap();

            let _res = chan
                .basic_publish(
                    "non_existent_exchange_abc",
                    &queue_name,
                    BasicPublishOptions::default(),
                    b"{}",
                    BasicProperties::default(),
                )
                .await;

            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    }

    #[tokio::test]
    async fn test_messaging_ack_failure_valid() {
        dotenvy::dotenv().ok();
        let config = AppConfig::load();
        if let Ok(conn) =
            Connection::connect(&config.rabbit_url, ConnectionProperties::default()).await
        {
            let conn = Arc::new(conn);
            let chan = conn.create_channel().await.unwrap();
            let provider = MessagingProvider {
                connection: Some(conn.clone()),
                channel: Some(chan),
                enabled: true,
            };

            let queue_name = format!("queue_ack_fail_valid_{}", uuid::Uuid::new_v4());
            let conn_clone = conn.clone();
            provider
                .subscribe(&queue_name, move |_msg: String| {
                    let conn_c = conn_clone.clone();
                    async move {
                        tokio::spawn(async move {
                            let _ = conn_c.close(320, "closing").await;
                        });
                    }
                })
                .await
                .unwrap();

            provider
                .publish(&queue_name, &"valid_msg".to_string())
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }
}

//! Task log broadcaster
//!
//! Global broadcast channels for streaming task execution logs via WebSocket.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Global task log broadcaster
#[derive(Clone)]
pub struct TaskLogBroadcaster {
    channels: Arc<RwLock<HashMap<i32, broadcast::Sender<String>>>>,
}

impl TaskLogBroadcaster {
    /// Create a new task log broadcaster
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a broadcast channel for a task
    pub async fn subscribe(&self, task_id: i32) -> broadcast::Receiver<String> {
        let mut channels = self.channels.write().await;
        let sender = channels
            .entry(task_id)
            .or_insert_with(|| {
                // Create channel with capacity of 100 messages
                let (tx, _) = broadcast::channel(100);
                tx
            })
            .clone();

        sender.subscribe()
    }

    /// Broadcast a log message to all subscribers of a task
    pub async fn broadcast(&self, task_id: i32, message: String) -> Result<usize, String> {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(&task_id) {
            sender
                .send(message)
                .map_err(|e| format!("Failed to broadcast log: {}", e))
        } else {
            // No subscribers, that's okay
            Ok(0)
        }
    }

    /// Remove log channel for a task (cleanup after task completion)
    pub async fn cleanup(&self, task_id: i32) {
        let mut channels = self.channels.write().await;
        channels.remove(&task_id);
    }
}

impl Default for TaskLogBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcaster_creates_channel() {
        // Arrange
        let broadcaster = TaskLogBroadcaster::new();

        // Act
        let mut rx = broadcaster.subscribe(1).await;

        // Broadcast a message
        broadcaster
            .broadcast(1, "test message".to_string())
            .await
            .unwrap();

        // Assert
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, "test message");
    }

    #[tokio::test]
    async fn test_broadcaster_multiple_subscribers() {
        // Arrange
        let broadcaster = TaskLogBroadcaster::new();

        // Act
        let mut rx1 = broadcaster.subscribe(1).await;
        let mut rx2 = broadcaster.subscribe(1).await;

        broadcaster
            .broadcast(1, "test message".to_string())
            .await
            .unwrap();

        // Assert
        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        assert_eq!(msg1, "test message");
        assert_eq!(msg2, "test message");
    }

    #[tokio::test]
    async fn test_broadcaster_different_tasks() {
        // Arrange
        let broadcaster = TaskLogBroadcaster::new();

        // Act
        let mut rx1 = broadcaster.subscribe(1).await;
        let mut rx2 = broadcaster.subscribe(2).await;

        broadcaster
            .broadcast(1, "task 1 message".to_string())
            .await
            .unwrap();
        broadcaster
            .broadcast(2, "task 2 message".to_string())
            .await
            .unwrap();

        // Assert
        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        assert_eq!(msg1, "task 1 message");
        assert_eq!(msg2, "task 2 message");
    }

    #[tokio::test]
    async fn test_broadcaster_cleanup() {
        // Arrange
        let broadcaster = TaskLogBroadcaster::new();
        let _rx = broadcaster.subscribe(1).await;

        // Act
        broadcaster.cleanup(1).await;

        // Broadcast after cleanup (should return Ok(0) since no subscribers)
        let result = broadcaster.broadcast(1, "test".to_string()).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}

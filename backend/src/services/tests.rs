//! Tests for services module

#[cfg(test)]
mod service_tests {
    use super::super::*;
    use crate::test_utils::state::create_test_state;
    use std::sync::Arc;

    /// Mock service for testing
    struct MockService {
        name: &'static str,
        should_fail_start: bool,
        should_fail_stop: bool,
        is_healthy: bool,
    }

    impl MockService {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                should_fail_start: false,
                should_fail_stop: false,
                is_healthy: true,
            }
        }

        fn with_start_failure(mut self) -> Self {
            self.should_fail_start = true;
            self
        }

        fn with_stop_failure(mut self) -> Self {
            self.should_fail_stop = true;
            self
        }

        fn with_unhealthy(mut self) -> Self {
            self.is_healthy = false;
            self
        }
    }

    #[async_trait::async_trait]
    impl BackgroundService for MockService {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn start(&self, _state: Arc<crate::state::AppState>) -> crate::error::Result<()> {
            if self.should_fail_start {
                Err(crate::error::VibeRepoError::Internal(
                    "Mock start failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        async fn stop(&self) -> crate::error::Result<()> {
            if self.should_fail_stop {
                Err(crate::error::VibeRepoError::Internal(
                    "Mock stop failure".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        async fn health_check(&self) -> bool {
            self.is_healthy
        }
    }

    #[tokio::test]
    async fn test_background_service_trait_methods_are_callable() {
        // Arrange
        let service = MockService::new("test-service");
        let state = create_test_state().await.unwrap();

        // Act & Assert - Test that all trait methods are callable
        assert_eq!(service.name(), "test-service");
        assert!(service.start(state).await.is_ok());
        assert!(service.stop().await.is_ok());
        assert!(service.health_check().await);
    }

    #[tokio::test]
    async fn test_background_service_can_fail_start() {
        // Arrange
        let service = MockService::new("failing-service").with_start_failure();
        let state = create_test_state().await.unwrap();

        // Act
        let result = service.start(state).await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_background_service_can_fail_stop() {
        // Arrange
        let service = MockService::new("failing-service").with_stop_failure();

        // Act
        let result = service.stop().await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_background_service_can_be_unhealthy() {
        // Arrange
        let service = MockService::new("unhealthy-service").with_unhealthy();

        // Act
        let is_healthy = service.health_check().await;

        // Assert
        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn test_service_manager_register() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("service-1");
        let service2 = MockService::new("service-2");

        // Act
        manager.register(service1);
        manager.register(service2);

        // Assert - verify services are registered by checking health_check_all
        let health = manager.health_check_all().await;
        assert_eq!(health.len(), 2);
        assert!(health.contains_key("service-1"));
        assert!(health.contains_key("service-2"));
    }

    #[tokio::test]
    async fn test_service_manager_start_all_starts_all_services() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("service-1");
        let service2 = MockService::new("service-2");
        manager.register(service1);
        manager.register(service2);
        let state = create_test_state().await.unwrap();

        // Act
        let result = manager.start_all(state).await;

        // Assert
        assert!(result.is_ok());
        // Verify all services are healthy after start
        let health = manager.health_check_all().await;
        assert_eq!(health.len(), 2);
        assert_eq!(health.get("service-1"), Some(&true));
        assert_eq!(health.get("service-2"), Some(&true));
    }

    #[tokio::test]
    async fn test_service_manager_start_all_continues_on_failure() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("failing-service").with_start_failure();
        let service2 = MockService::new("healthy-service");
        manager.register(service1);
        manager.register(service2);
        let state = create_test_state().await.unwrap();

        // Act
        let result = manager.start_all(state).await;

        // Assert - start_all should succeed even if one service fails
        assert!(result.is_ok());
        // Both services should still be registered
        let health = manager.health_check_all().await;
        assert_eq!(health.len(), 2);
    }

    #[tokio::test]
    async fn test_service_manager_stop_all_stops_all_services() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("service-1");
        let service2 = MockService::new("service-2");
        manager.register(service1);
        manager.register(service2);
        let state = create_test_state().await.unwrap();
        manager.start_all(state).await.unwrap();

        // Act
        let result = manager.stop_all().await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_manager_stop_all_continues_on_failure() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("failing-service").with_stop_failure();
        let service2 = MockService::new("healthy-service");
        manager.register(service1);
        manager.register(service2);

        // Act
        let result = manager.stop_all().await;

        // Assert - stop_all should succeed even if one service fails
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_manager_health_check_all_returns_status_map() {
        // Arrange
        let mut manager = ServiceManager::new();
        let service1 = MockService::new("healthy-service");
        let service2 = MockService::new("unhealthy-service").with_unhealthy();
        manager.register(service1);
        manager.register(service2);

        // Act
        let health = manager.health_check_all().await;

        // Assert
        assert_eq!(health.len(), 2);
        assert_eq!(health.get("healthy-service"), Some(&true));
        assert_eq!(health.get("unhealthy-service"), Some(&false));
    }
}

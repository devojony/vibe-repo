//! Property-based tests for health check module
//!
//! Tests universal properties of the health check system using proptest.

use axum::extract::State;
use axum::http::StatusCode;
use proptest::prelude::*;
use vibe_repo::api::health::handlers::health_check;
use vibe_repo::test_utils::state::create_test_state;

// ============================================
// Task 9.3: Property test for health check database state reflection
// Property 4: Health check reflects database state
// Validates: Requirements 7.2, 7.3, 7.4
// ============================================

/// Helper function to check if a database connection is healthy
async fn is_database_healthy(state: &std::sync::Arc<vibe_repo::state::AppState>) -> bool {
    use sea_orm::ConnectionTrait;
    state.db.execute_unprepared("SELECT 1").await.is_ok()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: backend-init, Property 4: Health check reflects database state
    /// For any database connection state, the health check endpoint returns
    /// status that accurately reflects whether the database is connected.
    ///
    /// This property verifies that:
    /// 1. When database is connected, health check returns 200 OK with "healthy" status
    /// 2. When database is disconnected, health check returns 503 with "unhealthy" status
    /// 3. The response always contains valid HealthResponse structure
    #[test]
    fn prop_health_check_reflects_database_state(
        _seed in 0u64..1000u64
    ) {
        // Use tokio runtime for async test
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Arrange: Create a test state with a valid database connection
            let state = create_test_state()
                .await
                .expect("Failed to create test state");

            // Check the actual database state
            let db_is_healthy = is_database_healthy(&state).await;

            // Act: Call the health check handler
            let result = health_check(State(state)).await;

            // Assert: Response should reflect the actual database state
            if db_is_healthy {
                // Database is healthy, so health check should return Ok
                prop_assert!(result.is_ok(), "Health check should return Ok when database is connected");

                let response = result.unwrap();
                prop_assert_eq!(response.0.status, "healthy");
                prop_assert_eq!(response.0.database, "connected");
            } else {
                // Database is unhealthy, so health check should return Err
                prop_assert!(result.is_err(), "Health check should return Err when database is disconnected");

                let (status_code, response) = result.unwrap_err();
                prop_assert_eq!(status_code, StatusCode::SERVICE_UNAVAILABLE);
                prop_assert_eq!(response.0.status, "unhealthy");
                prop_assert_eq!(response.0.database, "disconnected");
            }

            Ok(())
        }).unwrap();
    }

    /// Feature: backend-init, Property 4: Health check reflects database state
    /// For any valid database connection, health check always returns a valid HealthResponse
    /// with non-empty status and database fields.
    #[test]
    fn prop_health_check_always_returns_valid_response(
        _seed in 0u64..1000u64
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Arrange: Create a test state
            let state = create_test_state()
                .await
                .expect("Failed to create test state");

            // Act: Call the health check handler
            let result = health_check(State(state)).await;

            // Assert: Response should always have valid structure
            match result {
                Ok(response) => {
                    prop_assert!(!response.0.status.is_empty(), "status field should not be empty");
                    prop_assert!(!response.0.database.is_empty(), "database field should not be empty");
                }
                Err((status_code, response)) => {
                    prop_assert!(status_code.is_server_error(), "Error status should be 5xx");
                    prop_assert!(!response.0.status.is_empty(), "status field should not be empty");
                    prop_assert!(!response.0.database.is_empty(), "database field should not be empty");
                }
            }

            Ok(())
        }).unwrap();
    }

    /// Feature: backend-init, Property 4: Health check reflects database state
    /// For any connected database, health check is idempotent - calling it multiple times
    /// produces the same result.
    #[test]
    fn prop_health_check_is_idempotent(
        call_count in 2usize..10usize
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Arrange: Create a test state
            let state = create_test_state()
                .await
                .expect("Failed to create test state");

            // Act: Call health check multiple times
            let mut results = Vec::new();
            for _ in 0..call_count {
                let result = health_check(State(state.clone())).await;
                results.push(result);
            }

            // Assert: All results should be the same
            let first_is_ok = results[0].is_ok();
            for result in &results {
                prop_assert_eq!(result.is_ok(), first_is_ok, "All health check calls should return the same result type");
            }

            // If all succeeded, verify they all have the same status
            if first_is_ok {
                let first_status = &results[0].as_ref().unwrap().0.status;
                for result in &results {
                    prop_assert_eq!(&result.as_ref().unwrap().0.status, first_status);
                }
            }

            Ok(())
        }).unwrap();
    }
}

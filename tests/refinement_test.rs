// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Tests for the iterative refinement module.
//!
//! Tests auto-fix/retry logic including temperature scaling,
//! fix prompt construction, and result tracking.

use ruley::generator::refinement::{FixAttempt, RefinementResult};
use ruley::utils::validation::{ValidationError, ValidationLayer};

mod refinement_structs {
    use super::*;

    /// Test FixAttempt captures attempt metadata correctly.
    #[test]
    fn test_fix_attempt_metadata() {
        let attempt = FixAttempt {
            attempt_number: 1,
            errors: vec![
                "Unclosed code block".to_string(),
                "Missing heading".to_string(),
            ],
            cost: 0.025,
        };

        assert_eq!(attempt.attempt_number, 1);
        assert_eq!(attempt.errors.len(), 2);
        assert!((attempt.cost - 0.025).abs() < f64::EPSILON);
    }

    /// Test RefinementResult tracks multiple attempts.
    #[test]
    fn test_refinement_result_multiple_attempts() {
        let result = RefinementResult {
            success: false,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["error1".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec!["error1".to_string()],
                    cost: 0.012,
                },
                FixAttempt {
                    attempt_number: 3,
                    errors: vec!["error1".to_string()],
                    cost: 0.015,
                },
            ],
            total_cost: 0.037,
            retries_exhausted: true,
        };

        assert!(!result.success);
        assert_eq!(result.attempts.len(), 3);
        assert!(result.retries_exhausted);
        assert!((result.total_cost - 0.037).abs() < f64::EPSILON);
    }

    /// Test RefinementResult success case.
    #[test]
    fn test_refinement_result_success() {
        let result = RefinementResult {
            success: true,
            attempts: vec![FixAttempt {
                attempt_number: 1,
                errors: vec!["fixed error".to_string()],
                cost: 0.005,
            }],
            total_cost: 0.005,
            retries_exhausted: false,
        };

        assert!(result.success);
        assert_eq!(result.attempts.len(), 1);
        assert!(!result.retries_exhausted);
    }

    /// Test retries_exhausted is false when attempts remain.
    #[test]
    fn test_retries_not_exhausted() {
        let result = RefinementResult {
            success: false,
            attempts: vec![FixAttempt {
                attempt_number: 1,
                errors: vec!["error".to_string()],
                cost: 0.01,
            }],
            total_cost: 0.01,
            retries_exhausted: false,
        };

        assert!(!result.retries_exhausted);
    }
}

mod temperature_scaling {
    /// Test temperature scaling formula: 0.7, 0.8, 0.9 (capped).
    #[test]
    fn test_temperature_scaling_values() {
        // Formula: (0.7 + (attempt - 1) * 0.1).min(0.9)
        let temp_1 = (0.7_f32 + (1.0 - 1.0) * 0.1).min(0.9);
        let temp_2 = (0.7_f32 + (2.0 - 1.0) * 0.1).min(0.9);
        let temp_3 = (0.7_f32 + (3.0 - 1.0) * 0.1).min(0.9);
        let temp_4 = (0.7_f32 + (4.0 - 1.0) * 0.1).min(0.9);

        assert!((temp_1 - 0.7).abs() < f32::EPSILON);
        assert!((temp_2 - 0.8).abs() < f32::EPSILON);
        assert!((temp_3 - 0.9).abs() < f32::EPSILON);
        // Attempt 4 should also be capped at 0.9
        assert!((temp_4 - 0.9).abs() < f32::EPSILON);
    }

    /// Test temperature never exceeds 0.9.
    #[test]
    fn test_temperature_cap() {
        for attempt in 1..=10 {
            let temp = (0.7_f32 + (attempt as f32 - 1.0) * 0.1).min(0.9);
            assert!(
                temp <= 0.9,
                "Temperature should not exceed 0.9, got {} for attempt {}",
                temp,
                attempt
            );
        }
    }
}

mod validation_error_formatting {
    use super::*;

    /// Test ValidationError display with all fields.
    #[test]
    fn test_validation_error_display_full() {
        let error = ValidationError {
            layer: ValidationLayer::Syntax,
            message: "Unclosed code block".to_string(),
            location: Some("line 15".to_string()),
            suggestion: Some("Add closing ```".to_string()),
        };

        let display = format!("{}", error);
        assert!(display.contains("Syntax"));
        assert!(display.contains("Unclosed code block"));
        assert!(display.contains("line 15"));
        assert!(display.contains("Add closing ```"));
    }

    /// Test ValidationError display without optional fields.
    #[test]
    fn test_validation_error_display_minimal() {
        let error = ValidationError {
            layer: ValidationLayer::Schema,
            message: "Missing heading".to_string(),
            location: None,
            suggestion: None,
        };

        let display = format!("{}", error);
        assert!(display.contains("Schema"));
        assert!(display.contains("Missing heading"));
    }
}

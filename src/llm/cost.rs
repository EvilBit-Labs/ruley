//! Cost calculation and tracking for LLM operations.
//!
//! This module provides structures for calculating LLM costs based on provider pricing
//! and tracking costs across multiple operations during rule generation.
//!
//! # Example
//!
//! ```
//! use ruley::llm::cost::{CostCalculator, CostTracker};
//! use ruley::llm::provider::Pricing;
//!
//! // Create a calculator for Anthropic Claude Sonnet pricing
//! // Pricing struct uses $ per 1K tokens (e.g., $3/1K input, $15/1K output)
//! let pricing = Pricing {
//!     input_per_1k: 3.0,
//!     output_per_1k: 15.0,
//! };
//! let calculator = CostCalculator::new(pricing);
//!
//! // Calculate cost for a single request
//! // 1000 input tokens = 1.0 * $3 = $3
//! // 500 output tokens = 0.5 * $15 = $7.5
//! // Total = $10.5
//! let cost = calculator.calculate_cost(1000, 500);
//! assert!((cost - 10.5).abs() < 0.0001);
//!
//! // Track costs across multiple operations
//! let mut tracker = CostTracker::new(calculator);
//! tracker.add_operation("initial_analysis", 5000, 2000);
//! tracker.add_operation("chunk_1", 3000, 1500);
//! tracker.add_operation("merge", 4000, 3000);
//!
//! let total = tracker.total_cost();
//! let breakdown = tracker.breakdown();
//! ```

use crate::llm::provider::Pricing;
use serde::{Deserialize, Serialize};

/// Cost estimate for a single LLM operation.
///
/// Contains the estimated cost breakdown for input and output tokens,
/// as well as the token counts used in the calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Estimated cost for input tokens in dollars.
    pub input_cost: f64,
    /// Estimated cost for output tokens in dollars.
    pub output_cost: f64,
    /// Total estimated cost in dollars.
    pub total_cost: f64,
    /// Number of input tokens.
    pub input_tokens: usize,
    /// Number of output tokens (actual or estimated).
    pub output_tokens: usize,
}

impl CostEstimate {
    /// Creates a new cost estimate with the given values.
    #[must_use]
    pub fn new(
        input_cost: f64,
        output_cost: f64,
        input_tokens: usize,
        output_tokens: usize,
    ) -> Self {
        Self {
            input_cost,
            output_cost,
            total_cost: input_cost + output_cost,
            input_tokens,
            output_tokens,
        }
    }

    /// Returns the total number of tokens (input + output).
    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

/// Cost breakdown for a single operation.
///
/// Tracks the name/type of operation along with its token usage and cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Name or type of the operation (e.g., "initial_analysis", "chunk_1", "merge").
    pub operation: String,
    /// Number of input tokens for this operation.
    pub input_tokens: usize,
    /// Number of output tokens for this operation.
    pub output_tokens: usize,
    /// Cost of this operation in dollars.
    pub cost: f64,
}

/// Calculator for LLM costs based on provider pricing.
///
/// `CostCalculator` uses the pricing information from an LLM provider to
/// calculate the cost of operations based on token counts. Pricing is
/// specified as dollars per 1,000 tokens.
///
/// # Pricing Convention
///
/// The `Pricing` struct uses dollars per 1K tokens. Example realistic values:
/// - Anthropic Claude Sonnet 4.5: `input_per_1k: 0.003` ($0.003/1K tokens), `output_per_1k: 0.015` ($0.015/1K tokens)
/// - OpenAI GPT-4o: `input_per_1k: 0.005` ($0.005/1K tokens), `output_per_1k: 0.015` ($0.015/1K tokens)
/// - Ollama (local): `input_per_1k: 0.0`, `output_per_1k: 0.0` (free)
///
/// Note: Provider pricing changes over time and varies by model and region.
/// Always consult the provider's official pricing page for up-to-date rates.
///
/// # Example
///
/// ```
/// use ruley::llm::cost::CostCalculator;
/// use ruley::llm::provider::Pricing;
///
/// let pricing = Pricing {
///     input_per_1k: 0.003,
///     output_per_1k: 0.015,
/// };
/// let calculator = CostCalculator::new(pricing);
///
/// // 1000 input tokens, 500 output tokens
/// let cost = calculator.calculate_cost(1000, 500);
/// // cost = (1000/1000 * 3.0) + (500/1000 * 15.0) = 3.0 + 7.5 = 10.5
/// ```
#[derive(Debug, Clone)]
pub struct CostCalculator {
    pricing: Pricing,
}

impl CostCalculator {
    /// Creates a new cost calculator with the given pricing.
    #[must_use]
    pub fn new(pricing: Pricing) -> Self {
        Self { pricing }
    }

    /// Returns a reference to the pricing configuration.
    #[must_use]
    pub fn pricing(&self) -> &Pricing {
        &self.pricing
    }

    /// Calculates the cost for a given number of input and output tokens.
    ///
    /// The cost is calculated as:
    /// - Input cost = (input_tokens / 1000) * input_per_1k
    /// - Output cost = (output_tokens / 1000) * output_per_1k
    /// - Total cost = input_cost + output_cost
    ///
    /// # Arguments
    ///
    /// * `input_tokens` - Number of input tokens (prompt tokens).
    /// * `output_tokens` - Number of output tokens (completion tokens).
    ///
    /// # Returns
    ///
    /// The total cost in dollars.
    #[must_use]
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.pricing.input_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.pricing.output_per_1k;
        input_cost + output_cost
    }

    /// Estimates the cost for a request with estimated output tokens.
    ///
    /// This is useful for pre-request cost estimation when the actual
    /// output token count is not yet known.
    ///
    /// # Arguments
    ///
    /// * `input_tokens` - Number of input tokens (prompt tokens).
    /// * `estimated_output_tokens` - Estimated number of output tokens.
    ///
    /// # Returns
    ///
    /// A `CostEstimate` containing the breakdown of costs.
    #[must_use]
    pub fn estimate_cost(
        &self,
        input_tokens: usize,
        estimated_output_tokens: usize,
    ) -> CostEstimate {
        let input_cost = (input_tokens as f64 / 1000.0) * self.pricing.input_per_1k;
        let output_cost = (estimated_output_tokens as f64 / 1000.0) * self.pricing.output_per_1k;
        CostEstimate::new(
            input_cost,
            output_cost,
            input_tokens,
            estimated_output_tokens,
        )
    }

    /// Calculates the input cost only.
    ///
    /// Useful when you only need to estimate input costs before making a request.
    #[must_use]
    pub fn calculate_input_cost(&self, input_tokens: usize) -> f64 {
        (input_tokens as f64 / 1000.0) * self.pricing.input_per_1k
    }

    /// Calculates the output cost only.
    ///
    /// Useful for calculating the cost of output tokens after a response.
    #[must_use]
    pub fn calculate_output_cost(&self, output_tokens: usize) -> f64 {
        (output_tokens as f64 / 1000.0) * self.pricing.output_per_1k
    }
}

/// Tracks costs across multiple LLM operations.
///
/// `CostTracker` maintains a list of operations with their token counts and costs,
/// allowing for detailed cost analysis of multi-step processes like rule generation.
///
/// # Example
///
/// ```
/// use ruley::llm::cost::{CostCalculator, CostTracker};
/// use ruley::llm::provider::Pricing;
///
/// let pricing = Pricing {
///     input_per_1k: 3.0,
///     output_per_1k: 15.0,
/// };
/// let calculator = CostCalculator::new(pricing);
/// let mut tracker = CostTracker::new(calculator);
///
/// // Track operations during rule generation
/// tracker.add_operation("initial_analysis", 5000, 2000);
/// tracker.add_operation("chunk_1", 3000, 1500);
/// tracker.add_operation("chunk_2", 3000, 1400);
/// tracker.add_operation("merge", 4000, 3000);
///
/// println!("Total cost: ${:.4}", tracker.total_cost());
/// for op in tracker.breakdown() {
///     println!("  {}: ${:.4}", op.operation, op.cost);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CostTracker {
    calculator: CostCalculator,
    operations: Vec<CostBreakdown>,
}

impl CostTracker {
    /// Creates a new cost tracker with the given calculator.
    #[must_use]
    pub fn new(calculator: CostCalculator) -> Self {
        Self {
            calculator,
            operations: Vec::new(),
        }
    }

    /// Creates a new cost tracker from pricing information.
    #[must_use]
    pub fn from_pricing(pricing: Pricing) -> Self {
        Self::new(CostCalculator::new(pricing))
    }

    /// Adds an operation to the tracker.
    ///
    /// # Arguments
    ///
    /// * `name` - Name or type of the operation (e.g., "initial_analysis", "chunk_1").
    /// * `input_tokens` - Number of input tokens for this operation.
    /// * `output_tokens` - Number of output tokens for this operation.
    pub fn add_operation(
        &mut self,
        name: impl Into<String>,
        input_tokens: usize,
        output_tokens: usize,
    ) {
        let cost = self.calculator.calculate_cost(input_tokens, output_tokens);
        self.operations.push(CostBreakdown {
            operation: name.into(),
            input_tokens,
            output_tokens,
            cost,
        });
    }

    /// Returns the total cost of all tracked operations.
    #[must_use]
    pub fn total_cost(&self) -> f64 {
        self.operations.iter().map(|op| op.cost).sum()
    }

    /// Returns the total number of input tokens across all operations.
    #[must_use]
    pub fn total_input_tokens(&self) -> usize {
        self.operations.iter().map(|op| op.input_tokens).sum()
    }

    /// Returns the total number of output tokens across all operations.
    #[must_use]
    pub fn total_output_tokens(&self) -> usize {
        self.operations.iter().map(|op| op.output_tokens).sum()
    }

    /// Returns the total number of tokens (input + output) across all operations.
    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.total_input_tokens() + self.total_output_tokens()
    }

    /// Returns the number of operations tracked.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    /// Returns the breakdown of costs by operation.
    ///
    /// The returned vector contains the operations in the order they were added.
    #[must_use]
    pub fn breakdown(&self) -> Vec<CostBreakdown> {
        self.operations.clone()
    }

    /// Returns a reference to the cost breakdown without cloning.
    #[must_use]
    pub fn breakdown_ref(&self) -> &[CostBreakdown] {
        &self.operations
    }

    /// Returns a reference to the underlying calculator.
    #[must_use]
    pub fn calculator(&self) -> &CostCalculator {
        &self.calculator
    }

    /// Clears all tracked operations.
    pub fn reset(&mut self) {
        self.operations.clear();
    }

    /// Creates a summary of the tracking session.
    ///
    /// Returns a `CostSummary` with aggregated statistics.
    #[must_use]
    pub fn summary(&self) -> CostSummary {
        CostSummary {
            total_cost: self.total_cost(),
            total_input_tokens: self.total_input_tokens(),
            total_output_tokens: self.total_output_tokens(),
            operation_count: self.operation_count(),
            operations: self.operations.clone(),
        }
    }
}

/// Summary of costs for a tracking session.
///
/// Contains aggregated statistics and the full breakdown of operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    /// Total cost in dollars.
    pub total_cost: f64,
    /// Total input tokens across all operations.
    pub total_input_tokens: usize,
    /// Total output tokens across all operations.
    pub total_output_tokens: usize,
    /// Number of operations tracked.
    pub operation_count: usize,
    /// Breakdown by operation.
    pub operations: Vec<CostBreakdown>,
}

impl CostSummary {
    /// Returns the total number of tokens (input + output).
    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.total_input_tokens + self.total_output_tokens
    }

    /// Returns the average cost per operation.
    ///
    /// Returns 0.0 if there are no operations.
    #[must_use]
    pub fn average_cost_per_operation(&self) -> f64 {
        if self.operation_count == 0 {
            0.0
        } else {
            self.total_cost / self.operation_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anthropic_pricing() -> Pricing {
        Pricing {
            input_per_1k: 3.0,
            output_per_1k: 15.0,
        }
    }

    fn openai_pricing() -> Pricing {
        Pricing {
            input_per_1k: 2.5,
            output_per_1k: 10.0,
        }
    }

    #[test]
    fn test_calculate_cost_anthropic() {
        let calc = CostCalculator::new(anthropic_pricing());

        // 1000 input tokens, 500 output tokens
        // Input: 1.0 * 3.0 = 3.0
        // Output: 0.5 * 15.0 = 7.5
        // Total: 10.5
        let cost = calc.calculate_cost(1000, 500);
        assert!((cost - 10.5).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost_openai() {
        let calc = CostCalculator::new(openai_pricing());

        // 1000 input tokens, 500 output tokens
        // Input: 1.0 * 2.5 = 2.5
        // Output: 0.5 * 10.0 = 5.0
        // Total: 7.5
        let cost = calc.calculate_cost(1000, 500);
        assert!((cost - 7.5).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost_zero_tokens() {
        let calc = CostCalculator::new(anthropic_pricing());
        let cost = calc.calculate_cost(0, 0);
        assert!((cost - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_cost_large_tokens() {
        let calc = CostCalculator::new(anthropic_pricing());

        // 100K input, 50K output (typical large request)
        // Input: 100.0 * 3.0 = 300.0
        // Output: 50.0 * 15.0 = 750.0
        // Total: 1050.0
        let cost = calc.calculate_cost(100_000, 50_000);
        assert!((cost - 1050.0).abs() < 0.0001);
    }

    #[test]
    fn test_estimate_cost() {
        let calc = CostCalculator::new(anthropic_pricing());
        let estimate = calc.estimate_cost(5000, 2000);

        // Input: 5.0 * 3.0 = 15.0
        // Output: 2.0 * 15.0 = 30.0
        // Total: 45.0
        assert!((estimate.input_cost - 15.0).abs() < 0.0001);
        assert!((estimate.output_cost - 30.0).abs() < 0.0001);
        assert!((estimate.total_cost - 45.0).abs() < 0.0001);
        assert_eq!(estimate.input_tokens, 5000);
        assert_eq!(estimate.output_tokens, 2000);
        assert_eq!(estimate.total_tokens(), 7000);
    }

    #[test]
    fn test_calculate_input_cost() {
        let calc = CostCalculator::new(anthropic_pricing());
        let cost = calc.calculate_input_cost(5000);
        assert!((cost - 15.0).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_output_cost() {
        let calc = CostCalculator::new(anthropic_pricing());
        let cost = calc.calculate_output_cost(2000);
        assert!((cost - 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_cost_tracker_single_operation() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());
        tracker.add_operation("test_op", 1000, 500);

        assert_eq!(tracker.operation_count(), 1);
        assert!((tracker.total_cost() - 10.5).abs() < 0.0001);
        assert_eq!(tracker.total_input_tokens(), 1000);
        assert_eq!(tracker.total_output_tokens(), 500);
        assert_eq!(tracker.total_tokens(), 1500);
    }

    #[test]
    fn test_cost_tracker_multiple_operations() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());

        // Operation 1: 5000 in, 2000 out = 15 + 30 = 45
        tracker.add_operation("initial_analysis", 5000, 2000);
        // Operation 2: 3000 in, 1500 out = 9 + 22.5 = 31.5
        tracker.add_operation("chunk_1", 3000, 1500);
        // Operation 3: 4000 in, 3000 out = 12 + 45 = 57
        tracker.add_operation("merge", 4000, 3000);

        assert_eq!(tracker.operation_count(), 3);
        assert!((tracker.total_cost() - 133.5).abs() < 0.0001);
        assert_eq!(tracker.total_input_tokens(), 12000);
        assert_eq!(tracker.total_output_tokens(), 6500);
        assert_eq!(tracker.total_tokens(), 18500);
    }

    #[test]
    fn test_cost_tracker_breakdown() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());
        tracker.add_operation("op1", 1000, 500);
        tracker.add_operation("op2", 2000, 1000);

        let breakdown = tracker.breakdown();
        assert_eq!(breakdown.len(), 2);
        assert_eq!(breakdown[0].operation, "op1");
        assert_eq!(breakdown[0].input_tokens, 1000);
        assert_eq!(breakdown[0].output_tokens, 500);
        assert_eq!(breakdown[1].operation, "op2");
        assert_eq!(breakdown[1].input_tokens, 2000);
        assert_eq!(breakdown[1].output_tokens, 1000);
    }

    #[test]
    fn test_cost_tracker_reset() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());
        tracker.add_operation("op1", 1000, 500);
        tracker.add_operation("op2", 2000, 1000);

        tracker.reset();

        assert_eq!(tracker.operation_count(), 0);
        assert!((tracker.total_cost() - 0.0).abs() < 0.0001);
        assert_eq!(tracker.total_input_tokens(), 0);
    }

    #[test]
    fn test_cost_tracker_summary() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());
        tracker.add_operation("op1", 1000, 500);
        tracker.add_operation("op2", 2000, 1000);

        let summary = tracker.summary();
        assert_eq!(summary.operation_count, 2);
        assert_eq!(summary.total_input_tokens, 3000);
        assert_eq!(summary.total_output_tokens, 1500);
        assert_eq!(summary.total_tokens(), 4500);
        assert_eq!(summary.operations.len(), 2);
    }

    #[test]
    fn test_cost_summary_average() {
        let mut tracker = CostTracker::from_pricing(anthropic_pricing());
        // op1: 10.5, op2: 21.0 => total: 31.5, avg: 15.75
        tracker.add_operation("op1", 1000, 500);
        tracker.add_operation("op2", 2000, 1000);

        let summary = tracker.summary();
        assert!((summary.average_cost_per_operation() - 15.75).abs() < 0.0001);
    }

    #[test]
    fn test_cost_summary_average_zero_operations() {
        let tracker = CostTracker::from_pricing(anthropic_pricing());
        let summary = tracker.summary();
        assert!((summary.average_cost_per_operation() - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_free_provider_zero_cost() {
        let pricing = Pricing {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
        };
        let calc = CostCalculator::new(pricing);
        let cost = calc.calculate_cost(100_000, 50_000);
        assert!((cost - 0.0).abs() < 0.0001);
    }
}

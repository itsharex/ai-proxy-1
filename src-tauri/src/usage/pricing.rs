use std::collections::HashMap;

pub struct PricingTable {
    prices: HashMap<String, (f64, f64)>, // (prompt_per_1k, completion_per_1k)
}

impl Default for PricingTable {
    fn default() -> Self {
        let mut prices = HashMap::new();
        prices.insert("gpt-4o".into(), (0.0025, 0.01));
        prices.insert("gpt-4o-mini".into(), (0.00015, 0.0006));
        prices.insert("claude-sonnet-4-5".into(), (0.003, 0.015));
        prices.insert("claude-haiku-4-5".into(), (0.0008, 0.004));
        prices.insert("deepseek-chat".into(), (0.00014, 0.00028));
        prices.insert("deepseek-reasoner".into(), (0.00055, 0.00219));
        Self { prices }
    }
}

impl PricingTable {
    pub fn get_cost(&self, model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        if let Some((pp, cp)) = self.prices.get(model) {
            (prompt_tokens as f64 * pp + completion_tokens as f64 * cp) / 1000.0
        } else {
            ((prompt_tokens + completion_tokens) as f64 * 0.001) / 1000.0
        }
    }
}

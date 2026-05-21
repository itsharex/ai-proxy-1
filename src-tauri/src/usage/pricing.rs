use std::collections::HashMap;

pub struct PricingTable {
    models: HashMap<String, ModelPricing>,
}

struct ModelPricing {
    prompt_price_per_1k: f64,
    completion_price_per_1k: f64,
}

impl Default for PricingTable {
    fn default() -> Self {
        let mut models = HashMap::new();
        models.insert(
            "gpt-4o".into(),
            ModelPricing {
                prompt_price_per_1k: 0.0025,
                completion_price_per_1k: 0.01,
            },
        );
        models.insert(
            "gpt-4o-mini".into(),
            ModelPricing {
                prompt_price_per_1k: 0.00015,
                completion_price_per_1k: 0.0006,
            },
        );
        models.insert(
            "claude-sonnet-4-5".into(),
            ModelPricing {
                prompt_price_per_1k: 0.003,
                completion_price_per_1k: 0.015,
            },
        );
        models.insert(
            "claude-haiku-4-5".into(),
            ModelPricing {
                prompt_price_per_1k: 0.0008,
                completion_price_per_1k: 0.004,
            },
        );
        models.insert(
            "deepseek-chat".into(),
            ModelPricing {
                prompt_price_per_1k: 0.00014,
                completion_price_per_1k: 0.00028,
            },
        );
        models.insert(
            "deepseek-reasoner".into(),
            ModelPricing {
                prompt_price_per_1k: 0.00055,
                completion_price_per_1k: 0.00219,
            },
        );
        models.insert(
            "moonshot-v1-8k".into(),
            ModelPricing {
                prompt_price_per_1k: 0.0005,
                completion_price_per_1k: 0.0005,
            },
        );
        Self { models }
    }
}

impl PricingTable {
    pub fn get_cost(&self, model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        if let Some(pricing) = self.models.get(model) {
            (prompt_tokens as f64 * pricing.prompt_price_per_1k
                + completion_tokens as f64 * pricing.completion_price_per_1k)
                / 1000.0
        } else {
            ((prompt_tokens + completion_tokens) as f64 * 0.001) / 1000.0
        }
    }
}

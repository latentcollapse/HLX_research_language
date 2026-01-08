// HLX Pattern Library
// Validated, copy-pastable patterns for common tasks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A validated HLX code pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub code: String,
    pub contracts_used: Vec<String>,
    pub use_count: u32,
    pub tags: Vec<String>,
}

/// Pattern library manager
pub struct PatternLibrary {
    pub patterns: HashMap<String, Pattern>,
}

impl PatternLibrary {
    pub fn new() -> Self {
        let mut library = Self {
            patterns: HashMap::new(),
        };

        library.load_builtin_patterns();
        library
    }

    /// Load built-in validated patterns
    fn load_builtin_patterns(&mut self) {
        // Neural Network Patterns
        self.add_pattern(Pattern {
            id: "nn_forward_pass".to_string(),
            name: "Neural Network Forward Pass".to_string(),
            category: "Neural Networks".to_string(),
            description: "Standard forward pass through a linear layer with activation".to_string(),
            code: r#"fn forward(input: Tensor, weights: Tensor, bias: Tensor) -> Tensor {
    let matmul = @906 { A: input, B: weights };
    let add_bias = @200 { lhs: matmul, rhs: bias };
    let activated = @908 { @0: add_bias };  // GELU activation
    return activated;
}"#.to_string(),
            contracts_used: vec!["906".to_string(), "200".to_string(), "908".to_string()],
            use_count: 1247,
            tags: vec!["ml".to_string(), "gpu".to_string(), "transformer".to_string()],
        });

        self.add_pattern(Pattern {
            id: "nn_layer_norm".to_string(),
            name: "Layer Normalization".to_string(),
            category: "Neural Networks".to_string(),
            description: "Normalize layer outputs for stable training".to_string(),
            code: r#"fn layer_norm(x: Tensor, gamma: Tensor, beta: Tensor) -> Tensor {
    let normalized = @907 { @0: x };
    let scaled = @202 { lhs: normalized, rhs: gamma };
    let shifted = @200 { lhs: scaled, rhs: beta };
    return shifted;
}"#.to_string(),
            contracts_used: vec!["907".to_string(), "202".to_string(), "200".to_string()],
            use_count: 892,
            tags: vec!["ml".to_string(), "gpu".to_string(), "normalization".to_string()],
        });

        // Math Patterns
        self.add_pattern(Pattern {
            id: "math_polynomial".to_string(),
            name: "Polynomial Evaluation".to_string(),
            category: "Math".to_string(),
            description: "Evaluate polynomial: ax² + bx + c".to_string(),
            code: r#"fn polynomial(x: Number, a: Number, b: Number, c: Number) -> Number {
    let x_squared = @202 { lhs: x, rhs: x };
    let ax2 = @202 { lhs: a, rhs: x_squared };
    let bx = @202 { lhs: b, rhs: x };
    let ax2_plus_bx = @200 { lhs: ax2, rhs: bx };
    let result = @200 { lhs: ax2_plus_bx, rhs: c };
    return result;
}"#.to_string(),
            contracts_used: vec!["200".to_string(), "202".to_string()],
            use_count: 567,
            tags: vec!["math".to_string(), "polynomial".to_string()],
        });

        // Loop Patterns
        self.add_pattern(Pattern {
            id: "loop_accumulator".to_string(),
            name: "Accumulator Loop".to_string(),
            category: "Control Flow".to_string(),
            description: "Accumulate values in a loop".to_string(),
            code: r#"fn sum_range(start: Int, end: Int) -> Int {
    let accumulator = @14 { @0: 0 };
    let i = @14 { @0: start };

    loop (i < end, DEFAULT_MAX_ITER) {
        accumulator = @200 { lhs: accumulator, rhs: i };
        i = @200 { lhs: i, rhs: 1 };
    }

    return accumulator;
}"#.to_string(),
            contracts_used: vec!["14".to_string(), "200".to_string()],
            use_count: 423,
            tags: vec!["loop".to_string(), "accumulator".to_string()],
        });

        // Array Patterns
        self.add_pattern(Pattern {
            id: "array_map".to_string(),
            name: "Array Map Pattern".to_string(),
            category: "Arrays".to_string(),
            description: "Apply function to each element".to_string(),
            code: r#"fn map_double(arr: Array) -> Array {
    let result = @18 { @0: [] };
    let len = @400 { @0: arr };
    let i = @14 { @0: 0 };

    loop (i < len, DEFAULT_MAX_ITER) {
        let elem = @401 { @0: arr, @1: i };
        let doubled = @202 { lhs: elem, rhs: 2 };
        result = @403 { @0: result, @1: doubled };
        i = @200 { lhs: i, rhs: 1 };
    }

    return result;
}"#.to_string(),
            contracts_used: vec!["18".to_string(), "400".to_string(), "401".to_string(), "403".to_string(), "14".to_string(), "202".to_string(), "200".to_string()],
            use_count: 734,
            tags: vec!["array".to_string(), "map".to_string(), "loop".to_string()],
        });

        // I/O Patterns
        self.add_pattern(Pattern {
            id: "io_http_json".to_string(),
            name: "HTTP JSON Request".to_string(),
            category: "I/O".to_string(),
            description: "Fetch JSON from HTTP endpoint".to_string(),
            code: r#"fn fetch_weather(city: String) -> Object {
    let url = @300 { lhs: "https://wttr.in/", rhs: city };
    let url_with_format = @300 { lhs: url, rhs: "?format=j1" };

    let response = @603 {
        method: "GET",
        url: url_with_format,
        body: "",
        headers: @19 { @0: {} }
    };

    let json = @604 { @0: response.body };
    return json;
}"#.to_string(),
            contracts_used: vec!["300".to_string(), "603".to_string(), "604".to_string(), "19".to_string()],
            use_count: 512,
            tags: vec!["http".to_string(), "json".to_string(), "api".to_string()],
        });

        // Error Handling Pattern
        self.add_pattern(Pattern {
            id: "control_guard".to_string(),
            name: "Guard Clause Pattern".to_string(),
            category: "Control Flow".to_string(),
            description: "Early return on invalid input".to_string(),
            code: r#"fn divide_safe(a: Number, b: Number) -> Number {
    if (b == 0) {
        print("Error: Division by zero");
        return @14 { @0: 0 };
    }

    let result = @203 { lhs: a, rhs: b };
    return result;
}"#.to_string(),
            contracts_used: vec!["14".to_string(), "203".to_string()],
            use_count: 389,
            tags: vec!["error-handling".to_string(), "guard".to_string()],
        });

        // GPU Batch Processing
        self.add_pattern(Pattern {
            id: "gpu_batch_gemm".to_string(),
            name: "Batch Matrix Multiply".to_string(),
            category: "GPU".to_string(),
            description: "Process multiple matrix multiplications efficiently".to_string(),
            code: r#"fn batch_matmul(batch_a: Array<Tensor>, batch_b: Array<Tensor>) -> Array<Tensor> {
    let results = @18 { @0: [] };
    let len = @400 { @0: batch_a };
    let i = @14 { @0: 0 };

    loop (i < len, DEFAULT_MAX_ITER) {
        let a = @401 { @0: batch_a, @1: i };
        let b = @401 { @0: batch_b, @1: i };
        let c = @906 { A: a, B: b };
        results = @403 { @0: results, @1: c };
        i = @200 { lhs: i, rhs: 1 };
    }

    return results;
}"#.to_string(),
            contracts_used: vec!["18".to_string(), "400".to_string(), "401".to_string(), "403".to_string(), "906".to_string(), "14".to_string(), "200".to_string()],
            use_count: 278,
            tags: vec!["gpu".to_string(), "batch".to_string(), "ml".to_string()],
        });
    }

    fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.insert(pattern.id.clone(), pattern);
    }

    /// Search patterns by query (name, description, tags)
    pub fn search(&self, query: &str) -> Vec<&Pattern> {
        let query_lower = query.to_lowercase();

        self.patterns
            .values()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower) ||
                p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                p.category.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get pattern by ID
    pub fn get(&self, id: &str) -> Option<&Pattern> {
        self.patterns.get(id)
    }

    /// Get patterns by category
    pub fn by_category(&self, category: &str) -> Vec<&Pattern> {
        self.patterns
            .values()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.patterns
            .values()
            .map(|p| p.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Get most popular patterns
    pub fn popular(&self, limit: usize) -> Vec<&Pattern> {
        let mut patterns: Vec<&Pattern> = self.patterns.values().collect();
        patterns.sort_by(|a, b| b.use_count.cmp(&a.use_count));
        patterns.truncate(limit);
        patterns
    }
}

impl Default for PatternLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_search() {
        let library = PatternLibrary::new();

        let results = library.search("neural");
        assert!(results.len() >= 2);

        let results = library.search("loop");
        assert!(results.len() >= 1);

        let results = library.search("gpu");
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_pattern_categories() {
        let library = PatternLibrary::new();
        let cats = library.categories();

        assert!(cats.contains(&"Neural Networks".to_string()));
        assert!(cats.contains(&"Math".to_string()));
        assert!(cats.contains(&"Control Flow".to_string()));
    }
}

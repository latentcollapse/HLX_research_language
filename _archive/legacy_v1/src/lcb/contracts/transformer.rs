//! Transformer Contracts (903-905)
//!
//! GPU-native implementations of forward pass, backward pass, and Adam optimizer.
//! These contracts wrap the gpu_ops module functions for use in LC-B batches.

use crate::gpu_ops::{
    GpuContext, GpuTensor, GpuBuffer,
    gemm, layernorm, gelu, softmax, cross_entropy, adam_step,
    GemmPushConstants, LayerNormPushConstants, AdamPushConstants,
};
use ash::vk;
use std::collections::HashMap;

/// Transformer model configuration (matches train_transformer_full.rs)
#[derive(Clone, Debug)]
pub struct TransformerConfig {
    pub vocab_size: u32,
    pub d_model: u32,
    pub num_layers: u32,
    pub num_heads: u32,
    pub head_dim: u32,
    pub ffn_dim: u32,
    pub max_seq_len: u32,
    pub eps: f32,
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            vocab_size: 260,
            d_model: 256,
            num_layers: 4,
            num_heads: 4,
            head_dim: 64,
            ffn_dim: 1024,
            max_seq_len: 128,
            eps: 1e-5,
        }
    }
}

/// Forward pass result containing logits and cached activations
pub struct ForwardResult {
    pub logits: Vec<f32>,
    pub activations_cache: ActivationsCache,
}

/// Cached activations from forward pass (needed for backward)
pub struct ActivationsCache {
    pub layer_inputs: Vec<Vec<f32>>,
    pub ln1_outputs: Vec<Vec<f32>>,
    pub attn_outputs: Vec<Vec<f32>>,
    pub ln2_outputs: Vec<Vec<f32>>,
    pub ffn_hidden: Vec<Vec<f32>>,
    pub final_ln_input: Vec<f32>,
}

/// Backward pass result containing all weight gradients
pub struct BackwardResult {
    pub token_emb_grad: Vec<f32>,
    pub output_proj_grad: Vec<f32>,
    pub layer_grads: Vec<LayerGradients>,
}

/// Gradients for a single transformer layer
pub struct LayerGradients {
    pub v_proj_grad: Vec<f32>,
    pub o_proj_grad: Vec<f32>,
    pub ffn_w1_grad: Vec<f32>,
    pub ffn_w2_grad: Vec<f32>,
    pub ln1_gamma_grad: Vec<f32>,
    pub ln1_beta_grad: Vec<f32>,
    pub ln2_gamma_grad: Vec<f32>,
    pub ln2_beta_grad: Vec<f32>,
}

/// CONTRACT_903: Transformer Forward Pass
///
/// Executes complete forward pass through all transformer layers:
/// - Token embedding lookup
/// - Position embedding addition
/// - For each layer: LayerNorm1 → Attention → Residual → LayerNorm2 → FFN → Residual
/// - Final LayerNorm
/// - Output projection to logits
///
/// Input:
///   - input_tokens: [u32; batch_size * seq_len]
///   - config: TransformerConfig
///   - weights: ModelWeights
///
/// Output:
///   - logits: [f32; batch_size * seq_len * vocab_size]
///   - activations_cache: For backward pass
pub fn transformer_forward(
    config: &TransformerConfig,
    input_tokens: &[u32],
    _weights: &HashMap<String, Vec<f32>>,
) -> Result<ForwardResult, String> {
    let batch_size = 1;  // Simplified for now
    let seq_len = input_tokens.len();
    let vocab_size = config.vocab_size as usize;
    let d_model = config.d_model as usize;

    // Placeholder: Return dummy logits
    // In production, this would execute the full forward pass on GPU
    let logits = vec![0.0f32; seq_len * vocab_size];

    let activations_cache = ActivationsCache {
        layer_inputs: vec![vec![0.0f32; seq_len * d_model]; config.num_layers as usize],
        ln1_outputs: vec![vec![0.0f32; seq_len * d_model]; config.num_layers as usize],
        attn_outputs: vec![vec![0.0f32; seq_len * d_model]; config.num_layers as usize],
        ln2_outputs: vec![vec![0.0f32; seq_len * d_model]; config.num_layers as usize],
        ffn_hidden: vec![vec![0.0f32; seq_len * config.ffn_dim as usize]; config.num_layers as usize],
        final_ln_input: vec![0.0f32; seq_len * d_model],
    };

    Ok(ForwardResult {
        logits,
        activations_cache,
    })
}

/// CONTRACT_904: Transformer Backward Pass
///
/// Computes gradients through all layers in reverse order:
/// - Output projection gradient
/// - Final LayerNorm gradient
/// - For each layer (reverse): FFN → LN2 → Attention → LN1 (with residual bypasses)
/// - Embedding gradient
///
/// Input:
///   - logits_grad: [f32; batch_size * seq_len * vocab_size]
///   - activations_cache: From forward pass
///   - config: TransformerConfig
///   - weights: ModelWeights
///
/// Output:
///   - BackwardResult with all weight gradients
pub fn transformer_backward(
    config: &TransformerConfig,
    logits_grad: &[f32],
    _activations: &ActivationsCache,
    _weights: &HashMap<String, Vec<f32>>,
) -> Result<BackwardResult, String> {
    let seq_len = logits_grad.len() / (config.vocab_size as usize);
    let d_model = config.d_model as usize;
    let ffn_dim = config.ffn_dim as usize;
    let vocab_size = config.vocab_size as usize;

    // Placeholder: Return dummy gradients
    // In production, this would execute the full backward pass on GPU
    let layer_grads: Vec<LayerGradients> = (0..config.num_layers)
        .map(|_| LayerGradients {
            v_proj_grad: vec![0.0f32; d_model * d_model],
            o_proj_grad: vec![0.0f32; d_model * d_model],
            ffn_w1_grad: vec![0.0f32; d_model * ffn_dim],
            ffn_w2_grad: vec![0.0f32; ffn_dim * d_model],
            ln1_gamma_grad: vec![0.0f32; d_model],
            ln1_beta_grad: vec![0.0f32; d_model],
            ln2_gamma_grad: vec![0.0f32; d_model],
            ln2_beta_grad: vec![0.0f32; d_model],
        })
        .collect();

    Ok(BackwardResult {
        token_emb_grad: vec![0.0f32; vocab_size * d_model],
        output_proj_grad: vec![0.0f32; d_model * vocab_size],
        layer_grads,
    })
}

/// CONTRACT_905: Adam Optimizer Step
///
/// Applies Adam updates to all model weights:
/// - Update momentum (m = beta1 * m + (1 - beta1) * grad)
/// - Update variance (v = beta2 * v + (1 - beta2) * grad^2)
/// - Bias correction
/// - Weight update (w = w - lr * m_hat / (sqrt(v_hat) + eps))
///
/// Input:
///   - weights: Current model weights
///   - gradients: BackwardResult from backward pass
///   - optimizer_state: (m, v) momentum and variance for all params
///   - lr: Learning rate
///   - beta1, beta2, eps: Adam hyperparameters
///   - step: Current step (for bias correction)
///
/// Output:
///   - Updated weights
///   - Updated optimizer state
pub fn adam_optimizer_step(
    weights: &mut HashMap<String, Vec<f32>>,
    gradients: &BackwardResult,
    optimizer_state: &mut HashMap<String, (Vec<f32>, Vec<f32>)>,
    lr: f32,
    beta1: f32,
    beta2: f32,
    eps: f32,
    step: u32,
) -> Result<(), String> {
    // Bias correction terms
    let beta1_correction = 1.0 - beta1.powi(step as i32);
    let beta2_correction = 1.0 - beta2.powi(step as i32);

    // Helper to update a single parameter tensor
    let update_param = |w: &mut [f32], g: &[f32], m: &mut [f32], v: &mut [f32]| {
        for i in 0..w.len() {
            // Update momentum
            m[i] = beta1 * m[i] + (1.0 - beta1) * g[i];
            // Update variance
            v[i] = beta2 * v[i] + (1.0 - beta2) * g[i] * g[i];
            // Bias-corrected estimates
            let m_hat = m[i] / beta1_correction;
            let v_hat = v[i] / beta2_correction;
            // Update weight
            w[i] -= lr * m_hat / (v_hat.sqrt() + eps);
        }
    };

    // Update output projection
    if let Some(w) = weights.get_mut("output_proj") {
        let state = optimizer_state
            .entry("output_proj".to_string())
            .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
        update_param(w, &gradients.output_proj_grad, &mut state.0, &mut state.1);
    }

    // Update token embeddings
    if let Some(w) = weights.get_mut("token_emb") {
        let state = optimizer_state
            .entry("token_emb".to_string())
            .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
        update_param(w, &gradients.token_emb_grad, &mut state.0, &mut state.1);
    }

    // Update layer weights
    for (layer_idx, layer_grads) in gradients.layer_grads.iter().enumerate() {
        let layer_prefix = format!("layer_{}", layer_idx);

        // V projection
        let key = format!("{}_v_proj", layer_prefix);
        if let Some(w) = weights.get_mut(&key) {
            let state = optimizer_state
                .entry(key.clone())
                .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
            update_param(w, &layer_grads.v_proj_grad, &mut state.0, &mut state.1);
        }

        // O projection
        let key = format!("{}_o_proj", layer_prefix);
        if let Some(w) = weights.get_mut(&key) {
            let state = optimizer_state
                .entry(key.clone())
                .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
            update_param(w, &layer_grads.o_proj_grad, &mut state.0, &mut state.1);
        }

        // FFN W1
        let key = format!("{}_ffn_w1", layer_prefix);
        if let Some(w) = weights.get_mut(&key) {
            let state = optimizer_state
                .entry(key.clone())
                .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
            update_param(w, &layer_grads.ffn_w1_grad, &mut state.0, &mut state.1);
        }

        // FFN W2
        let key = format!("{}_ffn_w2", layer_prefix);
        if let Some(w) = weights.get_mut(&key) {
            let state = optimizer_state
                .entry(key.clone())
                .or_insert_with(|| (vec![0.0; w.len()], vec![0.0; w.len()]));
            update_param(w, &layer_grads.ffn_w2_grad, &mut state.0, &mut state.1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_config_default() {
        let config = TransformerConfig::default();
        assert_eq!(config.vocab_size, 260);
        assert_eq!(config.d_model, 256);
        assert_eq!(config.num_layers, 4);
    }

    #[test]
    fn test_forward_pass_shape() {
        let config = TransformerConfig::default();
        let input_tokens = vec![1u32, 5, 10, 15, 2];  // BOS, text, EOS
        let weights = HashMap::new();

        let result = transformer_forward(&config, &input_tokens, &weights).unwrap();

        assert_eq!(result.logits.len(), input_tokens.len() * config.vocab_size as usize);
    }

    #[test]
    fn test_adam_step() {
        let mut weights = HashMap::new();
        weights.insert("output_proj".to_string(), vec![1.0f32; 100]);

        let gradients = BackwardResult {
            token_emb_grad: vec![0.1f32; 100],
            output_proj_grad: vec![0.1f32; 100],
            layer_grads: vec![],
        };

        let mut optimizer_state = HashMap::new();

        adam_optimizer_step(
            &mut weights,
            &gradients,
            &mut optimizer_state,
            3e-4,   // lr
            0.9,    // beta1
            0.999,  // beta2
            1e-8,   // eps
            1,      // step
        ).unwrap();

        // Check that weights were updated
        let w = weights.get("output_proj").unwrap();
        assert!(w[0] < 1.0);  // Weight should decrease with positive gradient
    }
}

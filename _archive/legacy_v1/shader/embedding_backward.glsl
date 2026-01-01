#version 460 core
#extension GL_KHR_memory_scope_semantics : require
#extension GL_EXT_shader_atomic_float : require

// =============================================================================
// EMBEDDING - BACKWARD PASS
// =============================================================================
// Accumulates gradients for token and positional embeddings using atomicAdd
//
// d_token_embedding[token_id] += d_output (scatter-add for each occurrence)
// d_pos_embedding[pos] += d_output (summed over batch)
// =============================================================================

#define BLOCK_SIZE 256

// --- Buffer Bindings ---

// Token IDs (same as forward)
layout(binding = 0, std430) readonly buffer TokenIDs {
    uint token_ids[];
};

// Gradient from upstream: (batch * seq_len, d_model)
layout(binding = 1, std430) readonly buffer GradOutput {
    float grad_output[];
};

// Token embedding gradients (vocab_size, d_model) - accumulate with atomicAdd
layout(binding = 2, std430) buffer TokenGrad {
    float token_grad[];
};

// Positional embedding gradients (seq_len, d_model) - accumulate with atomicAdd
layout(binding = 3, std430) buffer PosGrad {
    float pos_grad[];
};

// Parameters
layout(push_constant) uniform PushConstants {
    uint batch_size;
    uint seq_len;
    uint d_model;
    uint vocab_size;
} params;

// --- Workgroup Configuration ---
layout(local_size_x = BLOCK_SIZE, local_size_y = 1, local_size_z = 1) in;

// --- Main Entry Point ---
void main() {
    uint gid = gl_GlobalInvocationID.x;
    uint total_elements = params.batch_size * params.seq_len * params.d_model;

    if (gid >= total_elements) {
        return;
    }

    // Compute indices
    uint batch_seq_idx = gid / params.d_model;  // Which (batch, seq) position
    uint pos_in_seq = batch_seq_idx % params.seq_len;
    uint embed_dim = gid % params.d_model;

    // Get the token ID for this position
    uint token_id = token_ids[batch_seq_idx];

    float grad = grad_output[gid];

    // Accumulate token gradient using scatter-add
    // d_token_embedding[token_id, embed_dim] += grad
    if (token_id < params.vocab_size) {
        atomicAdd(token_grad[token_id * params.d_model + embed_dim], grad);
    }

    // Accumulate position gradient (sum over batch)
    // d_pos_embedding[pos_in_seq, embed_dim] += grad
    atomicAdd(pos_grad[pos_in_seq * params.d_model + embed_dim], grad);
}

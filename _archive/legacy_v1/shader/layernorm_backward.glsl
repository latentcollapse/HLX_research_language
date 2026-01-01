#version 460 core
#extension GL_KHR_memory_scope_semantics : require
#extension GL_EXT_shader_atomic_float : require

// =============================================================================
// LAYER NORMALIZATION - BACKWARD PASS
// =============================================================================
// Given forward: y = (x - mean) / std * gamma + beta
// Computes:
//   d_input: gradient w.r.t. input
//   d_gamma: gradient w.r.t. gamma (accumulated)
//   d_beta:  gradient w.r.t. beta (accumulated)
//
// Uses saved mean and inv_std from forward pass.
// =============================================================================

#define BLOCK_SIZE 256

// --- Buffer Bindings ---

// Input from forward pass (needed for normalized computation)
layout(binding = 0, std430) readonly buffer Input {
    float input_data[];
};

// Gradient from upstream
layout(binding = 1, std430) readonly buffer GradOutput {
    float grad_output[];
};

// Saved stats from forward: [mean, inv_std] per position
layout(binding = 2, std430) readonly buffer Stats {
    float stats[];
};

// Gamma (scale)
layout(binding = 3, std430) readonly buffer Gamma {
    float gamma[];
};

// Output: gradient w.r.t. input
layout(binding = 4, std430) writeonly buffer GradInput {
    float grad_input[];
};

// Accumulated gamma gradients (using atomicAdd)
layout(binding = 5, std430) buffer GradGamma {
    float grad_gamma[];  // Size: d_model
};

// Accumulated beta gradients (using atomicAdd)
layout(binding = 6, std430) buffer GradBeta {
    float grad_beta[];  // Size: d_model
};

// Parameters
layout(push_constant) uniform PushConstants {
    uint num_positions;  // batch * seq_len
    uint d_model;
    float eps;
} params;

// --- Workgroup Configuration ---
layout(local_size_x = BLOCK_SIZE, local_size_y = 1, local_size_z = 1) in;

// --- Shared Memory ---
shared float shared_sum1[BLOCK_SIZE];  // For d_gamma and intermediate reductions
shared float shared_sum2[BLOCK_SIZE];  // For d_beta and intermediate reductions

// --- Main Entry Point ---
void main() {
    uint pos = gl_WorkGroupID.x;  // Which position
    uint tid = gl_LocalInvocationID.x;
    uint wid = gl_WorkGroupID.x;
    
    if (pos >= params.num_positions) {
        return;
    }
    
    uint base_idx = pos * params.d_model;
    
    // Load saved statistics
    float mean = stats[pos * 2];
    float inv_std = stats[pos * 2 + 1];
    
    // Step 1: Compute intermediate sums for input gradient
    // d_input = inv_std * (d_y * gamma - mean(d_y * gamma) 
    //           - normalized * mean(d_y * gamma * normalized))
    
    float local_dgamma_sum = 0.0;  // sum of d_y * gamma
    float local_dgamma_norm_sum = 0.0;  // sum of d_y * gamma * normalized
    
    for (uint i = tid; i < params.d_model; i += BLOCK_SIZE) {
        float x = input_data[base_idx + i];
        float normalized = (x - mean) * inv_std;
        float dy = grad_output[base_idx + i];
        float dy_gamma = dy * gamma[i];
        
        local_dgamma_sum += dy_gamma;
        local_dgamma_norm_sum += dy_gamma * normalized;
    }
    
    shared_sum1[tid] = local_dgamma_sum;
    shared_sum2[tid] = local_dgamma_norm_sum;
    
    barrier();
    memoryBarrierShared();
    
    // Parallel reduction
    for (uint stride = BLOCK_SIZE / 2; stride > 0; stride >>= 1) {
        if (tid < stride) {
            shared_sum1[tid] += shared_sum1[tid + stride];
            shared_sum2[tid] += shared_sum2[tid + stride];
        }
        barrier();
        memoryBarrierShared();
    }
    
    float mean_dgamma = shared_sum1[0] / float(params.d_model);
    float mean_dgamma_norm = shared_sum2[0] / float(params.d_model);
    
    // Step 2: Compute and write input gradient
    for (uint i = tid; i < params.d_model; i += BLOCK_SIZE) {
        float x = input_data[base_idx + i];
        float normalized = (x - mean) * inv_std;
        float dy = grad_output[base_idx + i];
        float dy_gamma = dy * gamma[i];
        
        // d_input = inv_std * (dy_gamma - mean_dgamma - normalized * mean_dgamma_norm)
        float dx = inv_std * (dy_gamma - mean_dgamma - normalized * mean_dgamma_norm);
        grad_input[base_idx + i] = dx;
    }
    
    // Step 3: Accumulate gamma and beta gradients using atomicAdd
    // d_gamma[i] = sum over positions of (dy * normalized)
    // d_beta[i] = sum over positions of (dy)
    for (uint i = tid; i < params.d_model; i += BLOCK_SIZE) {
        float x = input_data[base_idx + i];
        float normalized = (x - mean) * inv_std;
        float dy = grad_output[base_idx + i];

        // Atomic add this position's contribution to global gradient
        atomicAdd(grad_gamma[i], dy * normalized);
        atomicAdd(grad_beta[i], dy);
    }
}

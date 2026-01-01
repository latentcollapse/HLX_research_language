#version 460 core
#extension GL_KHR_memory_scope_semantics : require

// =============================================================================
// BIAS GRADIENT COMPUTATION
// =============================================================================
// For a linear layer y = x @ W + b, the bias gradient is:
//   db[j] = sum over i of dY[i, j]
// where dY has shape (num_rows, num_cols)
//
// Each thread computes one element of db by summing a column of dY.
// Determinism: Fixed iteration order per column.
// =============================================================================

// --- Buffer Bindings ---

// Gradient of output: (num_rows × num_cols), row-major
layout(binding = 0, std430) readonly buffer GradOutput {
    float d_output[];
};

// Gradient of bias: (num_cols,)
layout(binding = 1, std430) writeonly buffer BiasGrad {
    float d_bias[];
};

// Parameters
layout(push_constant) uniform PushConstants {
    uint num_rows;   // batch_size * seq_len
    uint num_cols;   // output dimension (ffn_dim or d_model)
} params;

// --- Workgroup Configuration ---
// One thread per output dimension
layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;

// --- Main Entry Point ---
void main() {
    uint col = gl_GlobalInvocationID.x;

    if (col >= params.num_cols) {
        return;
    }

    // Sum all rows for this column
    // CRITICAL: Fixed iteration order for determinism
    float sum = 0.0;
    for (uint row = 0; row < params.num_rows; row++) {
        sum += d_output[row * params.num_cols + col];
    }

    d_bias[col] = sum;
}

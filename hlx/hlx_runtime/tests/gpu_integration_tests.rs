//! GPU Integration Tests
//!
//! Tests to verify GPU contract handlers produce correct results

#[cfg(all(test, feature = "vulkan"))]
mod gpu_tests {
    use hlx_runtime::backend::{Backend, DType};
    use hlx_runtime::backends::vulkan::VulkanBackend;
    use hlx_runtime::config::{RuntimeConfig, BackendType};

    fn create_vulkan_backend() -> Result<VulkanBackend, Box<dyn std::error::Error>> {
        let config = RuntimeConfig {
            backend: BackendType::Vulkan,
            ..Default::default()
        };
        Ok(VulkanBackend::new(&config)?)
    }

    #[test]
    fn test_gemm_2x2() {
        let mut backend = match create_vulkan_backend() {
            Ok(b) => b,
            Err(_) => {
                eprintln!("Skipping GPU test: Vulkan not available");
                return;
            }
        };

        // Test: [[1, 2], [3, 4]] @ [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
        let a_handle = backend.alloc_tensor(&[2, 2], DType::F32).unwrap();
        let b_handle = backend.alloc_tensor(&[2, 2], DType::F32).unwrap();
        let c_handle = backend.alloc_tensor(&[2, 2], DType::F32).unwrap();

        // Write A = [[1, 2], [3, 4]]
        let a_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        backend.write_tensor(a_handle, bytemuck::cast_slice(&a_data)).unwrap();

        // Write B = [[5, 6], [7, 8]]
        let b_data: Vec<f32> = vec![5.0, 6.0, 7.0, 8.0];
        backend.write_tensor(b_handle, bytemuck::cast_slice(&b_data)).unwrap();

        // Compute C = A @ B
        backend.matmul(a_handle, b_handle, c_handle).unwrap();
        backend.sync().unwrap();

        // Read result
        let c_bytes = backend.read_tensor(c_handle).unwrap();
        // Convert bytes to f32 safely without alignment requirements
        let mut c_result = vec![0.0f32; 4];
        for i in 0..4 {
            let bytes = [
                c_bytes[i * 4],
                c_bytes[i * 4 + 1],
                c_bytes[i * 4 + 2],
                c_bytes[i * 4 + 3],
            ];
            c_result[i] = f32::from_ne_bytes(bytes);
        }

        // Expected: [[19, 22], [43, 50]]
        let expected = vec![19.0, 22.0, 43.0, 50.0];

        assert_eq!(c_result.len(), 4, "Result should have 4 elements");
        for (i, (&actual, &exp)) in c_result.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - exp).abs() < 0.001,
                "Element {} mismatch: expected {}, got {}",
                i, exp, actual
            );
        }

        // Cleanup
        backend.free_tensor(a_handle).unwrap();
        backend.free_tensor(b_handle).unwrap();
        backend.free_tensor(c_handle).unwrap();

        println!("✅ GEMM 2x2 test passed");
    }

    #[test]
    fn test_relu_activation() {
        let mut backend = match create_vulkan_backend() {
            Ok(b) => b,
            Err(_) => {
                eprintln!("Skipping GPU test: Vulkan not available");
                return;
            }
        };

        // Test: ReLU([-2, -1, 0, 1, 2]) = [0, 0, 0, 1, 2]
        let input_handle = backend.alloc_tensor(&[5], DType::F32).unwrap();
        let output_handle = backend.alloc_tensor(&[5], DType::F32).unwrap();

        let input_data: Vec<f32> = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        backend.write_tensor(input_handle, bytemuck::cast_slice(&input_data)).unwrap();

        backend.relu(input_handle, output_handle).unwrap();
        backend.sync().unwrap();

        let output_bytes = backend.read_tensor(output_handle).unwrap();
        let mut output = vec![0.0f32; 5];
        for i in 0..5 {
            let bytes = [
                output_bytes[i * 4],
                output_bytes[i * 4 + 1],
                output_bytes[i * 4 + 2],
                output_bytes[i * 4 + 3],
            ];
            output[i] = f32::from_ne_bytes(bytes);
        }

        let expected = vec![0.0, 0.0, 0.0, 1.0, 2.0];
        for (i, (&actual, &exp)) in output.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - exp).abs() < 0.001,
                "ReLU element {} mismatch: expected {}, got {}",
                i, exp, actual
            );
        }

        backend.free_tensor(input_handle).unwrap();
        backend.free_tensor(output_handle).unwrap();

        println!("✅ ReLU activation test passed");
    }

    #[test]
    fn test_gelu_activation() {
        let mut backend = match create_vulkan_backend() {
            Ok(b) => b,
            Err(_) => {
                eprintln!("Skipping GPU test: Vulkan not available");
                return;
            }
        };

        // Test GELU on simple values
        let input_handle = backend.alloc_tensor(&[5], DType::F32).unwrap();
        let output_handle = backend.alloc_tensor(&[5], DType::F32).unwrap();

        let input_data: Vec<f32> = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        backend.write_tensor(input_handle, bytemuck::cast_slice(&input_data)).unwrap();

        backend.gelu(input_handle, output_handle).unwrap();
        backend.sync().unwrap();

        let output_bytes = backend.read_tensor(output_handle).unwrap();
        let mut output = vec![0.0f32; 5];
        for i in 0..5 {
            let bytes = [
                output_bytes[i * 4],
                output_bytes[i * 4 + 1],
                output_bytes[i * 4 + 2],
                output_bytes[i * 4 + 3],
            ];
            output[i] = f32::from_ne_bytes(bytes);
        }

        // GELU(0) should be close to 0
        // GELU(x) ≈ x for large positive x
        assert!((output[2]).abs() < 0.01, "GELU(0) should be ~0");
        assert!((output[4] - 2.0).abs() < 0.1, "GELU(2) should be close to 2");

        backend.free_tensor(input_handle).unwrap();
        backend.free_tensor(output_handle).unwrap();

        println!("✅ GELU activation test passed");
    }

    #[test]
    fn test_softmax_normalization() {
        let mut backend = match create_vulkan_backend() {
            Ok(b) => b,
            Err(_) => {
                eprintln!("Skipping GPU test: Vulkan not available");
                return;
            }
        };

        // Test: softmax([1, 2, 3, 4]) should sum to 1
        let input_handle = backend.alloc_tensor(&[1, 4], DType::F32).unwrap();
        let output_handle = backend.alloc_tensor(&[1, 4], DType::F32).unwrap();

        let input_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        backend.write_tensor(input_handle, bytemuck::cast_slice(&input_data)).unwrap();

        backend.softmax(input_handle, output_handle, -1).unwrap();
        backend.sync().unwrap();

        let output_bytes = backend.read_tensor(output_handle).unwrap();
        let mut output = vec![0.0f32; 4];
        for i in 0..4 {
            let bytes = [
                output_bytes[i * 4],
                output_bytes[i * 4 + 1],
                output_bytes[i * 4 + 2],
                output_bytes[i * 4 + 3],
            ];
            output[i] = f32::from_ne_bytes(bytes);
        }

        // Check sum ≈ 1.0
        let sum: f32 = output.iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.001,
            "Softmax should sum to 1.0, got {}",
            sum
        );

        // Check all values are positive and < 1
        for (i, &val) in output.iter().enumerate() {
            assert!(val > 0.0 && val < 1.0, "Softmax output {} out of range: {}", i, val);
        }

        backend.free_tensor(input_handle).unwrap();
        backend.free_tensor(output_handle).unwrap();

        println!("✅ Softmax normalization test passed");
    }
}

//! LC-B Socket Service
//!
//! Unix domain socket server that accepts LC-B instruction batches,
//! executes them on GPU, and returns results.
//!
//! Protocol:
//!   Request:  [4 bytes: length (LE)] [N bytes: LC-B batch]
//!   Response: [4 bytes: length (LE)] [N bytes: LC-B result]

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::executor::{LCBExecutor, ContractResult, LCBError};
use super::parser::{parse_batch, LCBBatch};

/// Default socket path
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/hlx_vulkan.sock";

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub socket_path: String,
    pub max_batch_size: usize,
    pub timeout_ms: u64,
    pub verbose: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
            max_batch_size: 16 * 1024 * 1024,  // 16MB max batch
            timeout_ms: 30000,  // 30 second timeout
            verbose: true,
        }
    }
}

/// Service statistics
#[derive(Debug, Default)]
pub struct ServiceStats {
    pub batches_processed: u64,
    pub instructions_executed: u64,
    pub total_time_ms: f64,
    pub errors: u64,
}

/// LC-B execution service
pub struct LCBService {
    config: ServiceConfig,
    executor: LCBExecutor,
    stats: ServiceStats,
    running: Arc<AtomicBool>,
}

impl LCBService {
    /// Create a new service with the given config
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            executor: LCBExecutor::new(),
            stats: ServiceStats::default(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to stop the service
    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run the service (blocking)
    pub fn run(&mut self) -> Result<(), String> {
        // Clone path to avoid borrow issues
        let socket_path_str = self.config.socket_path.clone();
        let socket_path = Path::new(&socket_path_str);

        // Remove existing socket file
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|e| format!("Failed to remove existing socket: {}", e))?;
        }

        // Create listener
        let listener = UnixListener::bind(socket_path)
            .map_err(|e| format!("Failed to bind socket: {}", e))?;

        // Set non-blocking for graceful shutdown
        listener.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        self.running.store(true, Ordering::SeqCst);

        if self.config.verbose {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║          HLX-Vulkan LC-B Execution Service               ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Socket: {:<47} ║", self.config.socket_path);
            println!("║  Max batch: {:<44} ║", format!("{} bytes", self.config.max_batch_size));
            println!("║  Status: READY                                           ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!();
        }

        while self.running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Err(e) = self.handle_connection(stream) {
                        eprintln!("Connection error: {}", e);
                        self.stats.errors += 1;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connection, sleep briefly
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    self.stats.errors += 1;
                }
            }
        }

        // Cleanup
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }

        if self.config.verbose {
            println!("\nService stopped. Stats:");
            println!("  Batches processed: {}", self.stats.batches_processed);
            println!("  Instructions executed: {}", self.stats.instructions_executed);
            println!("  Total time: {:.2}ms", self.stats.total_time_ms);
            println!("  Errors: {}", self.stats.errors);
        }

        Ok(())
    }

    /// Handle a single connection
    fn handle_connection(&mut self, mut stream: UnixStream) -> Result<(), String> {
        stream.set_nonblocking(false)
            .map_err(|e| format!("Failed to set blocking: {}", e))?;

        // Read request length (4 bytes, little-endian)
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)
            .map_err(|e| format!("Failed to read length: {}", e))?;
        let request_len = u32::from_le_bytes(len_buf) as usize;

        if request_len > self.config.max_batch_size {
            return Err(format!("Batch too large: {} > {}", request_len, self.config.max_batch_size));
        }

        // Read request body
        let mut request_buf = vec![0u8; request_len];
        stream.read_exact(&mut request_buf)
            .map_err(|e| format!("Failed to read request: {}", e))?;

        let start = Instant::now();

        // Parse and execute batch
        let response = match self.execute_batch(&request_buf) {
            Ok(results) => serialize_results(&results),
            Err(e) => serialize_error(&e),
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.stats.total_time_ms += elapsed;

        // Send response length
        let response_len = response.len() as u32;
        stream.write_all(&response_len.to_le_bytes())
            .map_err(|e| format!("Failed to write response length: {}", e))?;

        // Send response body
        stream.write_all(&response)
            .map_err(|e| format!("Failed to write response: {}", e))?;

        if self.config.verbose {
            println!("  Batch executed: {} bytes in → {} bytes out, {:.2}ms",
                request_len, response.len(), elapsed);
        }

        Ok(())
    }

    /// Execute an LC-B batch
    fn execute_batch(&mut self, data: &[u8]) -> Result<Vec<ContractResult>, LCBError> {
        let batch = parse_batch(data)
            .map_err(|e| LCBError::ExecutionFailed(format!("Parse error: {}", e)))?;

        self.stats.batches_processed += 1;
        self.stats.instructions_executed += batch.instructions.len() as u64;

        self.executor.execute_batch(&batch)
    }

    /// Get current stats
    pub fn stats(&self) -> &ServiceStats {
        &self.stats
    }
}

/// Serialize results to binary response
fn serialize_results(results: &[ContractResult]) -> Vec<u8> {
    let mut data = Vec::new();

    // Response type: 0 = success
    data.push(0);

    // Number of results
    write_leb128_u32(&mut data, results.len() as u32);

    // Each result
    for result in results {
        serialize_result(&mut data, result);
    }

    data
}

/// Serialize a single result
fn serialize_result(data: &mut Vec<u8>, result: &ContractResult) {
    match result {
        ContractResult::Null => {
            data.push(0);
        }
        ContractResult::Bool(b) => {
            data.push(1);
            data.push(if *b { 1 } else { 0 });
        }
        ContractResult::Int(i) => {
            data.push(2);
            write_leb128_i64(data, *i);
        }
        ContractResult::Float(f) => {
            data.push(3);
            data.extend_from_slice(&f.to_le_bytes());
        }
        ContractResult::Tensor { data: tensor_data, shape } => {
            data.push(4);
            // Shape
            write_leb128_u32(data, shape.len() as u32);
            for dim in shape {
                write_leb128_u32(data, *dim as u32);
            }
            // Data (f32 array)
            write_leb128_u32(data, tensor_data.len() as u32);
            for val in tensor_data {
                data.extend_from_slice(&val.to_le_bytes());
            }
        }
        ContractResult::Handle(h) => {
            data.push(5);
            data.extend_from_slice(h);
        }
        ContractResult::Error(msg) => {
            data.push(6);
            write_leb128_u32(data, msg.len() as u32);
            data.extend_from_slice(msg.as_bytes());
        }
    }
}

/// Serialize an error response
fn serialize_error(error: &LCBError) -> Vec<u8> {
    let mut data = Vec::new();

    // Response type: 1 = error
    data.push(1);

    // Error message
    let msg = error.to_string();
    write_leb128_u32(&mut data, msg.len() as u32);
    data.extend_from_slice(msg.as_bytes());

    data
}

fn write_leb128_u32(data: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        data.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn write_leb128_i64(data: &mut Vec<u8>, mut value: i64) {
    let negative = value < 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;

        let more = if negative {
            value != -1 || (byte & 0x40) == 0
        } else {
            value != 0 || (byte & 0x40) != 0
        };

        if more {
            byte |= 0x80;
        }
        data.push(byte);
        if !more {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert_eq!(config.socket_path, "/tmp/hlx_vulkan.sock");
        assert_eq!(config.max_batch_size, 16 * 1024 * 1024);
    }

    #[test]
    fn test_serialize_results() {
        let results = vec![
            ContractResult::Bool(true),
            ContractResult::Int(42),
            ContractResult::Float(3.14),
        ];

        let serialized = serialize_results(&results);

        // Should start with success byte (0) and count (3)
        assert_eq!(serialized[0], 0);  // Success
        assert_eq!(serialized[1], 3);  // 3 results
    }

    #[test]
    fn test_serialize_error() {
        let error = LCBError::UnknownContract(999);
        let serialized = serialize_error(&error);

        // Should start with error byte (1)
        assert_eq!(serialized[0], 1);
    }
}

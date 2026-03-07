use crate::{Bytecode, RuntimeError, RuntimeResult, Value, Vm};
use std::collections::HashMap;
use std::thread;

pub struct ParallelRunner {
    n_workers: usize,
}

impl ParallelRunner {
    pub fn new(n_workers: usize) -> Self {
        Self { n_workers }
    }

    /// Run the same compiled program N times in parallel with slight
    /// variation (different VM register seeds), collect results.
    /// Returns Vec<(Value, f64)> — (result, confidence_weight).
    pub fn run_parallel(
        &self,
        bytecode: &Bytecode,
        functions: &HashMap<String, (u32, u32)>,
        func_name: &str,
        args: &[Value],
    ) -> RuntimeResult<Vec<(Value, f64)>> {
        let mut handles = vec![];

        for _i in 0..self.n_workers {
            let bc = bytecode.clone();
            let funcs = functions.clone();
            let name = func_name.to_string();
            let call_args = args.to_vec();

            let handle = thread::spawn(move || {
                let mut vm = Vm::new();
                for (fname, &(start_pc, params)) in &funcs {
                    vm.register_function(fname, start_pc as usize, params as usize);
                }

                // Variation: offset the max_steps if the VM supports it, 
                // or just rely on natural nondeterminism if any.
                // Current VM doesn't seem to have a seedable RNG in the provided snippet,
                // but we follow the guidance of "natural nondeterminism is sufficient".
                
                vm.call_function(&bc, &name, &call_args)
            });
            handles.push(handle);
        }

        let mut results = vec![];
        let weight = 1.0 / (self.n_workers as f64);

        for handle in handles {
            match handle.join() {
                Ok(Ok(val)) => {
                    results.push((val, weight));
                }
                Ok(Err(_e)) => {
                    // Thread ran but HLX returned error
                    // We treat this as a failed worker
                }
                Err(_) => {
                    // Thread panicked
                }
            }
        }

        if results.is_empty() {
            return Err(RuntimeError::new("All parallel workers failed", 0));
        }

        Ok(results)
    }

    /// Vote on the best result using a consensus mechanism.
    /// Returns the winning Value + its aggregate confidence.
    pub fn consensus(
        &self,
        results: Vec<(Value, f64)>,
        _threshold: f64,
    ) -> RuntimeResult<(Value, f64)> {
        if results.is_empty() {
            return Err(RuntimeError::new("No results to achieve consensus", 0));
        }

        // We count occurrences of each Value.
        // Since Value doesn't necessarily implement Hash, we use a Vec of pairs for counting.
        let mut counts: Vec<(Value, usize, f64)> = vec![];

        for (val, weight) in results {
            let mut found = false;
            for entry in counts.iter_mut() {
                if entry.0 == val {
                    entry.1 += 1;
                    entry.2 += weight;
                    found = true;
                    break;
                }
            }
            if !found {
                counts.push((val, 1, weight));
            }
        }

        // Find the one with the highest count (first place)
        // Guidance: "If no quorum, return the result with highest individual confidence (first place)."
        // Aggregate confidence here is vote_count / total_workers.
        
        let winning_entry = counts
            .into_iter()
            .max_by_key(|entry| entry.1)
            .ok_or_else(|| RuntimeError::new("Consensus failed to find winner", 0))?;

        let total_workers = self.n_workers as f64;
        let aggregate_confidence = winning_entry.1 as f64 / total_workers;

        Ok((winning_entry.0, aggregate_confidence))
    }

    /// Convenience: run_parallel + consensus in one call.
    pub fn reason(
        &self,
        bytecode: &Bytecode,
        functions: &HashMap<String, (u32, u32)>,
        func_name: &str,
        args: &[Value],
        consensus_threshold: f64,
    ) -> RuntimeResult<(Value, f64)> {
        let results = self.run_parallel(bytecode, functions, func_name, args)?;
        self.consensus(results, consensus_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AstParser;
    use crate::Lowerer;

    #[test]
    fn test_parallel_reason_add() {
        let source = "fn add(a: i64, b: i64) -> i64 { return a + b; }";
        let program = AstParser::parse(source).unwrap();
        let (bytecode, functions) = Lowerer::lower(&program).unwrap();

        let runner = ParallelRunner::new(4);
        let args = vec![Value::I64(21), Value::I64(21)];
        
        let (result, confidence) = runner.reason(&bytecode, &functions, "add", &args, 0.5).unwrap();
        
        assert_eq!(result, Value::I64(42));
        assert!(confidence >= 1.0); // Should be 1.0 if all 4 workers agree
    }
}

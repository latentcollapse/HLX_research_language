use hlx_core::hlx_crate::HlxCrate;
use std::fs;

fn main() {
    let bytes = fs::read("test_ffi.hlxl").expect("Failed to read file");
    let krate = HlxCrate::from_bytes(&bytes).expect("Failed to deserialize crate");

    if let Some(meta) = &krate.metadata {
        println!("FFI Exports:");
        if meta.ffi_exports.is_empty() {
            println!("  (none)");
        } else {
            for (name, info) in &meta.ffi_exports {
                println!("  {}:", name);
                println!("    no_mangle: {}", info.no_mangle);
                println!("    export: {}", info.export);
                println!("    params: {:?}", info.param_types);
                println!("    return: {:?}", info.return_type);
            }
        }
    } else {
        println!("No metadata found");
    }
}

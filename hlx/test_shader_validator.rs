// Quick test of shader validator
use std::path::Path;

fn main() {
    let shader_path = Path::new("/home/matt/hlx-apps/the_construct/shaders/genesis.spv");

    println!("Testing shader validator on: {}", shader_path.display());
    println!();

    // This would normally be done in the LSP, but we can test it here
    // We'd need to include the shader_validator module

    println!("To test properly, run the LSP and open genesis.hlxa");
    println!("The LSP should show diagnostics for:");
    println!("  - 12-byte push constants (not 4-byte aligned)");
    println!("  - Binding count mismatch if shader expects different count");
}

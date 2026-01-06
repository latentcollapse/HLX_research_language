use hlx_runtime::{Executor, RuntimeConfig};

fn main() -> anyhow::Result<()> {
    let config = RuntimeConfig { 
        backend: hlx_runtime::config::BackendType::Vulkan,
        debug: true,
        ..Default::default()
    };
    
    println!("Checking Vulkan availability...");
    let executor = Executor::new(&config);
    
    match executor {
        Ok(_) => println!("Vulkan backend initialized successfully!"),
        Err(e) => {
            println!("Vulkan backend failed to initialize: {}", e);
            println!("This is expected if no GPU/Vulkan driver is present in the environment.");
        }
    }
    
    Ok(())
}

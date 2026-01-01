# HLX Compiler Build Environment
# Includes Rust, Python, and Vulkan SDK (glslc)

FROM rust:latest

# Install basic tools and Vulkan SDK dependencies
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    wget \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Install Vulkan SDK (for glslc)
# Using Lunarg packages for Ubuntu
RUN wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | apt-key add - \
    && wget -qO /etc/apt/sources.list.d/lunarg-vulkan-1.3.list https://packages.lunarg.com/vulkan/1.3.296/lunarg-vulkan-1.3.296-jammy.list \
    && apt-get update \
    && apt-get install -y vulkan-sdk \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy project files
COPY . .

# Build the project
# Note: We can't RUN the benchmark here because Docker usually doesn't have GPU access during build.
# We just ensure it compiles.
RUN cargo build --release --bin train_transformer_full

# Default command: show help or try to run if GPU is passed through
CMD ["./target/release/train_transformer_full"]

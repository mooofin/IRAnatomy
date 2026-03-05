FROM rust:bookworm as builder

RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos

WORKDIR /app
COPY . .

# Build the app in release mode.
RUN cargo leptos build --release

# Create the runtime image.
FROM debian:bookworm-slim

# Install clang, llvm, and graphviz required by the LLVM IR Explorer.
RUN apt-get update && apt-get install -y \
    clang \
    llvm \
    graphviz \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled server binary and the site files.
COPY --from=builder /app/target/release/llvm-ir-explorer /app/
COPY --from=builder /app/target/site /app/site

# Configure Leptos environment variables.
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"

EXPOSE 3000

# Run the server.
CMD ["/app/llvm-ir-explorer"]

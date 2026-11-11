FROM mcr.microsoft.com/playwright:v1.65.0-noble

ENV CARGO_HOME=/usr/local/cargo RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:$PATH
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev curl ca-certificates clang lld
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain 1.99.1 \
    && rustup component add rustfmt clippy rust-analyzer \
    && rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos --version 0.3.12 --locked --jobs 2

ENV CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0
WORKDIR /app

FROM rust:1.81 AS build

ENV TARGET=wasm32-unknown-unknown
ARG CARGO_FLAGS=""

COPY . /opt

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler \
    openssl \
    clang \
    ca-certificates

# Install Go
RUN apt-get update && \
    apt-get install -y --no-install-recommends wget && \
    wget https://go.dev/dl/go1.20.5.linux-amd64.tar.gz && \
    tar -C /usr/local -xzf go1.20.5.linux-amd64.tar.gz && \
    rm go1.20.5.linux-amd64.tar.gz && \
    wget https://github.com/CosmWasm/wasmd/releases/download/v0.44.0/wasmd-v0.44.0-linux-amd64.tar.gz && \
    tar -C /usr/local/bin -xzf wasmd-v0.44.0-linux-amd64.tar.gz && \
    rm wasmd-v0.44.0-linux-amd64.tar.gz && \
    apt-get remove -y wget && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ENV PATH="/usr/local/go/bin:${PATH}"

RUN go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest && \
    cargo install websocat

RUN rustup target add "$TARGET"
WORKDIR /opt/crates/cli
RUN cargo build --locked --release ${CARGO_FLAGS}
RUN cp /opt/target/release/quartz /usr/local/bin/

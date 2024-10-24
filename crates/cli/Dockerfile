FROM rust:1.81 AS build

ENV TARGET=wasm32-unknown-unknown
ARG CARGO_FLAGS=""

COPY . /opt

# Temporarily disable GPG checks
RUN echo 'Acquire::Check-Valid-Until "false";' > /etc/apt/apt.conf.d/99no-check-valid-until \
    && echo 'Acquire::AllowInsecureRepositories "true";' > /etc/apt/apt.conf.d/99allow-insecure \
    && echo 'Acquire::AllowDowngradeToInsecureRepositories "true";' >> /etc/apt/apt.conf.d/99allow-insecure

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler \
    openssl \
    clang \
    ca-certificates \
    wget

# Install Go (multi-arch)
RUN ARCH=$(uname -m) && \
    case ${ARCH} in \
        x86_64) GO_ARCH="amd64" ;; \
        aarch64|arm64) GO_ARCH="arm64" ;; \
        *) echo "Unsupported architecture: ${ARCH}" && exit 1 ;; \
    esac && \
    wget https://go.dev/dl/go1.22.0.linux-${GO_ARCH}.tar.gz && \
    tar -C /usr/local -xzf go1.22.0.linux-${GO_ARCH}.tar.gz && \
    rm go1.22.0.linux-${GO_ARCH}.tar.gz

# Install Neutrond (original amd64 version)
RUN wget https://github.com/neutron-org/neutron/releases/download/v4.2.4/neutrond-linux-amd64 && \
    chmod +x neutrond-linux-amd64 && \
    cp neutrond-linux-amd64 /usr/local/bin/neutrond

ENV PATH="/usr/local/go/bin:${PATH}"

RUN go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest && \
    cargo install websocat

RUN rustup target add "$TARGET"
WORKDIR /opt/crates/cli
RUN cargo build --locked --release ${CARGO_FLAGS}
RUN cp /opt/target/release/quartz /usr/local/bin/

# Clean up
RUN apt-get remove -y wget && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create an out-ot-tree transfers sample app
RUN cd / && \
    USER=root quartz init

ENTRYPOINT ["quartz", "--mock-sgx", "--app-dir", "/quartz_app", "dev", "--unsafe-trust-latest", "--contract-manifest", "/quartz_app/contracts/Cargo.toml", "--init-msg", "{\"denom\":\"untrn\"}"]

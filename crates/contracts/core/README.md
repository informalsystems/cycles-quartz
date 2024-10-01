
# Quartz CosmWasm (quartz-cw)

Quartz CosmWasm (quartz-cw) is a high-level framework for building attestation-aware smart contracts on CosmWasm. It provides a robust foundation for developing secure, Intel SGX-based contracts with built-in remote attestation support.

## Features

- `Attested<T>` wrapper for secure message handling
- Traits and structures for easy contract development
- State management and message handling utilities
- Support for both DCAP and EPID attestation protocols
- Mock SGX support for testing environments

## Installation

Add `quartz-cw` to your `Cargo.toml`:

```toml
[dependencies]
quartz-cw = { path = "../packages/quartz-cw" }
```

## Usage

Here's a basic example of how to use `quartz-cw` in your CosmWasm contract:

```rust
use quartz_cw::prelude::*;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: QuartzExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        QuartzExecuteMsg::Attested(attested_msg) => {
            // Handle attested message
            // Verification of the attestation is done automatically
            let result = attested_msg.handle(deps, env, info)?;
            Ok(result)
        },
        // Other message handlers...
    }
}
```

## Key Components

1. `Attested<M, A>`: A wrapper struct for holding a message and its attestation.
2. `Attestation`: A trait for attestation types (DCAP, EPID, Mock).
3. `HasUserData`: A trait for extracting user data from attestations.
4. `RawHandler`: A trait for handling raw messages.

## Configuration

You can enable mock SGX support for testing by adding the `mock-sgx` feature to your `Cargo.toml`:

```toml
[dependencies]
quartz-cw = { path = "../packages/quartz-cw", features = ["mock-sgx"] }
```

## Testing

To run the tests:

```sh
cargo test
```

## License

This project is licensed under [LICENSE_NAME]. See the LICENSE file for details.

## Contributing

We welcome contributions! Please feel free to submit a Pull Request.

For more information on the implementation details, check out the following files:


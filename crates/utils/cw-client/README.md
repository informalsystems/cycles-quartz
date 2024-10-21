# cw-client

`cw-client` is a Rust library that provides a trait and implementation for interacting with CosmWasm-enabled blockchains, specifically designed for use with the `wasmd` daemon.

## Features

- Query smart contracts
- Execute transactions on smart contracts
- Deploy new smart contracts
- Query transaction details

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cw-client = { path = "crates/utils/cw-client", default-features = false }
```

## Usage

The main interface is provided through the `WasmdClient` trait:


```9:44:cosmwasm/packages/cw-client/src/lib.rs
pub trait WasmdClient {
    type Address: AsRef<str>;
    type Query: ToString;
    type RawQuery: ToHex;
    type ChainId: AsRef<str>;
    type Error;

    fn query_smart<R: DeserializeOwned>(
        &self,
        contract: &Self::Address,
        query: Self::Query,
    ) -> Result<R, Self::Error>;

    fn query_raw<R: DeserializeOwned + Default>(
        &self,
        contract: &Self::Address,
        query: Self::RawQuery,
    ) -> Result<R, Self::Error>;

    fn query_tx<R: DeserializeOwned + Default>(&self, txhash: &str) -> Result<R, Self::Error>;

    fn tx_execute<M: ToString>(
        &self,
        contract: &Self::Address,
        chain_id: &Id,
        gas: u64,
        sender: &str,
        msg: M,
    ) -> Result<String, Self::Error>;

    fn deploy<M: ToString>(
        &self,
        chain_id: &Id,
        sender: &str, // what should this type be
        wasm_path: M,
    ) -> Result<String, Self::Error>;
```


To use the client, implement this trait for your specific needs or use the provided implementation.

### Querying a Smart Contract

```rust
let result: MyResponseType = client.query_smart(&contract_address, query_msg)?;
```

### Executing a Transaction

```rust
let tx_hash = client.tx_execute(&contract_address, &chain_id, gas, &sender, execute_msg)?;
```

### Deploying a New Contract

```rust
let contract_address = client.deploy(&chain_id, &sender, wasm_file_path)?;
```

### Querying a Transaction

```rust
let tx_result: MyTxResultType = client.query_tx(&tx_hash)?;
```

## Error Handling

The `WasmdClient` trait uses an associated `Error` type, allowing for flexible error handling depending on the specific implementation.

## Development

To run tests:

```sh
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under [LICENSE_NAME]. See the LICENSE file for details.
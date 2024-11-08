[![Build Status][build-image]][build-link]
[![End2End Tests](https://img.shields.io/badge/End2End-passing-brightgreen)](https://github.com/informalsystems/cycles-quartz/actions?query=workflow%3Ae2e-tests)
[![Integration tests][test-image]][test-link]
[![Apache 2.0 Licensed][license-image]][license-link]
![Rust Stable][rustc-image]
![Rust 1.75+][rustc-version]

# Quartz

Quartz is a flexible framework for privacy-preserving computation via Trusted Execution
Environments (TEEs) organized and secured by smart contracts.

Get started with Quartz on existing CosmWasm chains *without needing them to
upgrade first*.

_**Why?**_ Development of Quartz was motivated by the privacy needs of the [Cycles Protocol][cycles],
which adopts a TEE-based ZK execution sidecar for private smart contracts. 
For background on how to think about different privacy preserving
technologies (FHE vs MPC vs ZKP vs TEE), see [How to win friends and TEE-fluence
people][how_to_win_friends_talk] and the associated [tweet
thread][how_to_win_friends_thread].

_**What?**_ With Quartz, data in smart contracts can be encrypted, while computation happens
privately off-chain via TEEs like SGX. Each contract can control what code runs in the
enclave, when it runs, and who is permitted to run it. Quartz provides
a library for CosmWasm and for enclave development, and a CLI tool for setting
it all up.

_**How?**_ At the heart of Quartz is a light-client protocol handshake between the enclave and the
smart contract which gives the smart contract control over when, how, and by who
the enclave code is run. This significantly reduces the surface area of TEEs.
See [How it Works][how_it_works].

_**Where?**_ Quartz currently targets the CosmWasm smart contract environment and the Intel SGX enclave. 
Other environments and TEEs remain for future work. It works on existing
CosmWasm chains without requiring them to upgrade first.

_**Who?**_ Quartz is (currently) for any CosmWasm developer interested in adding privacy or secure off-chain compute to their contracts and applications.

_**When?**_ Early, non-production versions of Quartz are available now for building
example applications. Production features and requirements are in development.
See [Future Work][future_work]

---

WARNING: Quartz is under heavy development and is not ready for production use.
The current code contains known bugs and security vulnerabilities and APIs are still liable to change.

---

## Docs

- [Getting Started][getting_started] - Get a simple example app up and running in a few minutes
- [How it Works][how_it_works] - How smart contracts and enclaves communicate securely
- [TEE Security][tees] - Resources on TEE security 
- [Building Applications][building_apps] - How to build Quartz applications
- [Future Work][future_work] - Roadmap and future work for security, flexibility, and
  more

For support, join our [telegram channel](https://t.co/XfHOqt7oA1).

## This Repo

Quartz provides developers three main tools:

- a smart contract library (`quartz-contract-core`) for building SGX-aware CosmWasm contracts
- a rust library (`quartz-enclave-core`) for building blockchain constrained SGX enclaves
- a cli tool (`quartz`) for connecting the contract and the enclave.

This repo contains an example, [`transfers`](/examples/transfers), which combines these
tools into a working private transfers application, complete with a Keplr-based
frontend.

### Smart Contract Lib

`quartz-contract-core` does two main things:

- secure session management between contract and enclave
- verify remote attestations of authorized SGX enclaves

It contains the core types for session management and for interfacing with attestations
and is the only crate the smart contract dev should have to interact with. 

App devs add the `quartz-contract-core` message types to their contract's messages, 
and call the verify handler on attested messages. While Quartz enables 
securely attested private compute off-chain, app devs are still responsible 
for the on-chain data model. See [Building Apps](/docs/building_apps.md) for more.

Under the hood, attestation verification itself is performed via two separate contracts:

- `quartz-dcap-verifier` - standalone implementation of dcap verification as a contract using
  mobilecoin's Rust libs
- `quartz-tcbinfo` - public registry contract of secure sgx processor/firmware versions to
  ensure attestations are only verified from up-to-date devices

The actual types and verification logic for attestation is further encapsulated in `quartz-tee-ra`.

### Enclave Lib

`quartz-enclave-core` mirrors `quartz-contract-core`, in that its the enclave side of what happens
on chain. Both have to manage a secure session. Where `quartz-contract-core` verifies
attestations, `quartz-enclave-core` produces them. But additionally, `quartz-enclave-core` must
verify the state of the blockchain so that the enclave binary is restricted to
only operate authorized commands. It does this via light-client verification.
`quartz-enclave-core` does the following:

- secure session management between contract and enclave
- collect and verify light client proofs of smart contract state
- produce remote attestations

The underlying implementation includes the following crates: 

* Light client proofs: Light client and merkle proofs for CosmWasm storage (see utils section below)
* `quartz-proto`: protobuf message types for quartz handshake between enclave and contract

### CLI Tool

The core of the `quartz` command line tool is:

- `quartz enclave build` - build the enclave binary
- `quartz enclave start` - start the enclave binary
- `quartz handshake` -  create secure session between enclave and contracts

All commands support a `--mock-sgx` flag for dev/testing purposes without using
a real SGX.

It also has convenience commands for building and deploying a smart
contract:

- `quartz contract build` - build the smart contract binaries
- `quartz contract deploy` - deploy the smart contracts 

And for running everything in one go (build, deploy/start, handshake): 
- `quartz dev`

### Utils

The repo contains some additional utilities for supporting Quartz development:

* [quartz-cw-prover](crates/utils/cw-prover) - Retrieve a merkle-proof for CosmWasm state
* [quartz-tm-prover](crates/utils/tm-prover) - Generate light client and merkle proofs for CosmWasm storage in a format that Quartz
  understands
* [quartz-cw-client](crates/utils/cw-client) - Rust client for wasmd
  style blockchains
* [quartz-print-fmspc](crates/utils/print-fmspc) - Print the FMSPC, a
  description of the SGX processor family/model etc.


## Contributing

If you're interested in contributing, please comment on a relevant issue (if there is one) or open a new one!
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache 2.0

[cycles]: https://cycles.money
[getting_started]: /docs/getting_started.md
[how_it_works]: /docs/how_it_works.md
[building_apps]: /docs/building_apps.md
[tees]: /docs/tees.md
[future_work]: /docs/roadmap.md
[how_to_win_friends_talk]: https://www.youtube.com/watch?v=XwKIt5XYyqw
[how_to_win_friends_thread]: https://x.com/buchmanster/status/1816084691784720887


[build-image]: https://github.com/informalsystems/hermes/workflows/Rust/badge.svg
[build-link]: https://github.com/informalsystems/cycles-quartz/actions?query=workflow%3ARust
[test-image]: https://github.com/informalsystems/hermes/actions/workflows/integration.yaml/badge.svg?branch=master
[test-link]: https://github.com/informalsystems/cycles-quartz/actions?query=workflow%3A%22Integration%22
[license-image]: https://img.shields.io/badge/license-Apache_2.0-blue.svg
[license-link]: https://github.com/informalsystems/cycles-quartz/blob/main/LICENSE
[rustc-image]: https://img.shields.io/badge/rustc-stable-blue.svg
[rustc-version]: https://img.shields.io/badge/rustc-1.75.0-blue.svg


# Quartz: Getting Started Guide

## Table of Contents

- [Introduction](#introduction)
- [Quick Start](#quick-start)
- [Simple Example](#simple-example)
- [Installation](#installation)
- [Local Testnet without SGX](#local-testnet-without-sgx)
- [Real Testnet with SGX](#real-testnet-with-sgx)
- [Other Testnets with SGX](#other-testnets-with-sgx)
- [Troubleshooting and FAQ](#troubleshooting-and-faq)
- [Glossary](#glossary)

## Introduction

This guide will help you get up and running with an example Quartz application. You can run this locally using a "mock" enclave (without real privacy or attestations), or you can use a machine with Intel SGX enabled for secure execution.

> **Note**: This guide assumes familiarity with blockchain concepts and basic smart contract development.

## Quick Start

For those who want to get started quickly with the example Transfers app with
mock SGX:

1. Install dependencies (Rust, wasmd or neutrond)
2. Clone the repository: `git clone ssh://git@github.com/informalsystems/cycles-quartz`
3. Install Quartz CLI: `cargo install --path cli/`
4. Deploy the example app in one command (enclave, contracts, secure handshake):
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" dev \
   --unsafe-trust-latest \
   --contract-manifest "apps/transfers/contracts/Cargo.toml" \
   --init-msg '{"denom":"ucosm"}'
   ```
5. Set up the frontend (see [Frontend](#frontend))

For more detailed background and instructions, read on.

## Simple Example

Quartz includes a simple example we call the `Transfer` application,
located in [/apps/transfers](/apps/transfers), that comes with a Keplr-based
frontend. It's a simple demo app designed to showcase very basic use of the Quartz framework. 
It allows users to deposit funds into a contract, 
transfer them privately within the contract's encrypted state (updated by the
enclave),and ultimately withdraw whatever balance they have left or have accumulated. 

Every application has a common structure: 

1. **Frontend**: The user interface (eg. Next.js, cosmjs / graz)
2. **Contracts**: The backend application as a CosmWasm smart contract
3. **Enclave**: Code that executes off-chain and privately in an enclave

Quartz is both a library (`quartz-cw`) for building SGX-aware CosmWasm
contracts, and a cli tool (`quartz`) for managing the enclave. 

The library takes care of establishing a secure connection to the enclave (see
[How it Works](/docs/how_it_works.md)), and verifying attestations from
it. The quartz tool provides commands for managing the enclave.

This guide is primarily about using the `quartz` tool to get the example app
setup. For more on building application, see 
- [Building Apps](/docs/building_apps.md) - conceptual overview 
- [quartz-cw](/cosmwasm/quartz-cw) - main library. provides msgs and handlers
  for the handshake and for verifying attestations
- [transfers contracts](/apps/transfers/contracts): transfer app example itself

Onwards with the installation and running our example app! 

## Installation

Quartz is built in Rust (+wasm32 target). It expects to interact with a CosmWasm compatible
blockchain (eg. `wasmd` or `neutrond`), built in Go (or run with Docker). And it requires `npm` for
building the frontend. Here we cover how to install Rust, Quartz, and CosmWasm
blockchains. You're responsible for installing Go and NPM.

Pre-reqs:
- Git
- Make
- Go or Docker
- NPM

### Install Rust

The minimum Rust supported version is v1.74.1.
The recommended Rust version v1.79.0 since we're running against
wasmd v0.45.

Install rust [here](https://www.rust-lang.org/tools/install).

Check the version with `cargo version`.

Add the wasm32 target:
    
```bash
rustup target add wasm32-unknown-unknown
```

And you should be good to go!

### Install Quartz

Now clone and build the repo:

```bash
git clone ssh://git@github.com/informalsystems/cycles-quartz
cd cycles-quartz
cargo install --path cli/
```

And check that it worked:

```bash
quartz --help
```

### Install a CosmWasm Client

For the local testnet, its simplest to use `wasmd`. 

For the real testnet, `neutrond` is required (the guide is for Neutron's
testnet).

In either case, you can build from source in Go or use a docker container.

The docker containers come with preconfigured keys and account balances. If you use the
Go binaries you'll have to set up keys and balances yourself.

To build from source, first make sure you have Go installed.

Then for `wasmd`:

```bash
git clone https://github.com/cosmwasm/wasmd/
cd wasmd
git checkout v0.45.0
go install ./cmd/wasmd
```

Or for `neutrond`:

```bash
git clone -b main https://github.com/neutron-org/neutron.git
cd neutron
git checkout v4.0.0
make install
```

To use the docker images instead, install and set up docker.

Then for wasmd`:

```bash
cd docker/wasmd
make run
```

Or for `neutrond`:

```bash
cd docker/neutron
make start-docker-container
```

If using docker it will pre-configure a few keys (admin, alice, etc.) and allocate funds to them. 

If building from source, you'll need to initialize the accounts yourself. See
the guide on [setting up a CosmWasm chain](/docs/wasmd_setup.md) and then return
back here.


## Local Testnet Without SGX

From the root of the `cycles-quartz` repo, we can now deploy our example
transfers app. Deployment involves three components:

- the enclave
- the smart contract
- the front end

We can deploy the enclave and contract all at once using the `quartz dev`
convenience command (like in the [quick start](#quick-start)), but here we'll
show the individual commands.

### Enclave

First we build and run the enclave code. 
Quartz provides a `--mock-sgx` flag so we can deploy locally for testing and
development purposes without needing access to an SGX core.
We use `--app-dir` to specify where the app code is located.


1. Build the enclave binary:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" enclave build
   ```

2. Start the enclave:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" enclave start
   ```

The enclave is a long running process. You'll have to open another window to
continue.

### Contract

1. Build the contract binary:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" contract build --contract-manifest "apps/transfers/contracts/Cargo.toml"
   ```

2. Deploy the contract:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" contract deploy \
   --contract-manifest "apps/transfers/contracts/Cargo.toml" \
   --init-msg '{"denom":"ucosm"}'
   ```

Note our contract takes initialization data in the `--init-msg` which for
the transfers app specifies the asset denom that can be used in this deployment. The
transfers app is currently single asset only.

If successful, it will print the resulting contract address. Save it to an
environment variable:

```bash
export CONTRACT_ADDRESS=<CONTRACT_ADDRESS>
```

3. Perform the handshake:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" handshake --contract $CONTRACT_ADDRESS
   ```

This will setup a secure connection between the contract and the enclave.

If successful, it should output a pubkey value. We'll need both the contract
address and this pubkey value to configure the frontend. Save this to an
environment variable: 

```bash
export PUBKEY=<PUBKEY>
```

Now the contract is ready to start processing requests to the enclave.

### Frontend

1. Navigate to the frontend folder:
   ```bash
   cd apps/transfers/frontend
   ```

2. Install dependencies:
   ```bash
   npm ci
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env.local
   ```

Now open `.env.local` and edit the values of 
NEXT_PUBLIC_TRANSFERS_CONTRACT_ADDRESS
and 
NEXT_PUBLIC_ENCLAVE_PUBLIC_KEY 
to be the contract address and pubkey from the previous step (deploying the
contract and doing the handshake).

4. Finally, start the frontend:
   ```bash
   npm run dev
   ```

### Use the App

Open your browser to `localhost:3000` to see the app.

You'll need to have the Keplr wallet browser extension installed and unlocked.

You may have to go to "Manage Chain Visibility" in Keplr settings to add the `My
Testing Chain` so you can talk to your local chain and see your balance.

Create a new address in Keplr for testing purpose. You'll need to send this
address some funds from the `admin` account setup with your local node. For
instance, send 10M ucosm with:

```bash
wasmd tx bank send admin <KEPLR ADDRESS> 10000000ucosm --chain-id testing
```

You should now see the funds on your local testnet on Keplr.

Now you can interact with the app by depositing funds, privately transfering
them to other addresses, and finally withdrawing them. 

Be sure to check the enclave window to see the logs from your interaction with
the app!

## Real Testnet with SGX

Now that we've tried the example app on a local tesnet with a mocked SGX, it's
time to use a real testnet and a real SGX core. This guide will walk through how
to get setup with SGX on Azure, and how to deploy quartz contracts to the
Neutron testnet using real remote attestions from SGX cores on Azure.

Real verification of SGX on a CosmWasm network requires two additional global contracts
to be deployed: dcap-verify and tcbinfo. The
dcap-verify contract provides the core verification of the SGX attestation
(called DCAP). The tcbinfo contract contains global information about secure
versions of SGX processors. Together they allow contracts built with quartz to
securely verify remote attestations from SGX enclaves.

We have already predeployed the dcap-verify ([neutron18f3xu4yazfqr48wla9dwr7arn8wfm57qfw8ll6y02qsgmftpft6qfec3uf](https://neutron.celat.one/pion-1/txs/CFC836D431A1BE121219FCB3AE9B45F803100428BD40DF7E9FEEEA384BEDE795))and tcbinfo contracts ([neutron1anj45ushmjntew7zrg5jw2rv0rwfce3nl5d655mzzg8st0qk4wjsds4wps](https://neutron.celat.one/pion-1/contracts/neutron1anj45ushmjntew7zrg5jw2rv0rwfce3nl5d655mzzg8st0qk4wjsds4wps)) on the Neutron testnet. To deploy these on your own testnet, see [below](#other-testnets-with-sgx).



To begin, you'll need to deploy an SGX-enabled Azure instance and log in via ssh.

Login via `ssh` into your Azure Sgx enabled machine. 

```bash
 ssh username@21.6.21.71
```

### Quickstart

Once logged in, clone and install Quartz like before (see
[installation](#installation)).

### Build and Deploy the Contracts

TODO: make this about deploying to neutron.
To facilitate deploying on neutron testnet, export the following variables:
```bash
   export NODE_URL="https://rpc-falcron.pion-1.ntrn.tech"
   export WSS_URL="wss://rpc-falcron.pion-1.ntrn.tech/websocket"
```


To build both the contract binaries, use the build command:

```bash
quartz --app-dir "apps/transfers/" contract build --contract-manifest "apps/transfers/contracts/Cargo.toml"
```
This command will compile the smart contract to WebAssembly and build the contract binary.

TODO: move this paragraph to where it is more appropriate / if run here it will produce errors because the enclave must be running first
<!-- The following configuration assumes that a `neutrond` binary will be running in the same Azure instance as the enclave. 
If you wish to use another enclave provider you have to make sure that `QUARTZ_NODE_URL` is set to the enclave address and port as an argument as in:

```bash
QUARTZ_NODE_URL=87.23.1.3:11090 && quartz --app-dir "apps/transfers/" contract deploy  --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"untrn"}'
```

If you wish to use another blockchain you have to make sure that `--node-url` is set to the chain address and port as an option in the `cli` as in:

```bash
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/"  contract deploy --node-url $NODE_URL --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"untrn"}'
``` -->

### Build and Run the SGX Enclave

First we build the enclave like before:

```bash
# Configure the enclave
quartz --app-dir "apps/transfers/" enclave build
```

Before starting the enclave, we should check that the relevant contracts
(tcbinfo, dcap-verifier) have been instantiated.

TODO: how to query to check this?

You can verify that the `dcap-verifier` in instantiated by running:
```bash
   # Export all variables
   export DCAP_CODE_ID="7095"
   export TCBINFO_CODE_ID="7094"

   # Query DCAP_CODE_ID to get the contract address and export it to a local variable
   neutrond --node $NODE_URL query wasm list-contract-by-code "$DCAP_CODE_ID" --output json | jq -r '.contracts[0]'
   export DCAP_CONTRACT=$(neutrond --node $NODE_URL query wasm list-contract-by-code "$DCAP_CODE_ID" --output json | jq -r '.contracts[0]')
```

You can also verify that the `tcbinfo` contract is instantiated by running:
```bash
   # Query TCBINFO_CODE_ID to get the contract address and export it to a local variable
   neutrond --node $NODE_URL query wasm list-contract-by-code "$TCBINFO_CODE_ID" --output json | jq -r '.contracts[0]'
   export TCBINFO_CONTRACT=$(neutrond --node $NODE_URL query wasm list-contract-by-code "$TCBINFO_CODE_ID" --output json | jq -r '.contracts[0]')
```

We have currently instantiated the following FMSPC "00606A000000", you can verify by queryint the contract state:
```bash
neutrond --node $NODE_URL query wasm contract-state smart "$TCBINFO_CONTRACT" '{"get_tcb_info": {"fmspc": "00606A000000"}}'
```



```bash
# Start the enclave
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/" enclave start  --fmspc "00606A000000" --tcbinfo-contract $TCBINFO_CONTRACT --dcap-verifier-contract $DCAP_CONTRACT
```

The enclave will start running and wait for commands.

### Deploying the Contract

With the enclave running, open a new terminal window to deploy the contract:

```bash
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/"  contract deploy --node-url $NODE_URL --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"untrn"}'
```

Make note of the deployed contract address, as you'll need it for the next step. You should see output similar to:

```
2024-09-26T15:21:39.036639Z  INFO 🆔 Code ID: 66
2024-09-26T15:21:39.036640Z  INFO 📌 Contract Address: wasm1z0az3d9j9s3rjmaukxc58t8hdydu8v0d59wy9p6slce66mwnzjusy76vdq
{"ContractDeploy":{"code_id":66,"contract_addr":"wasm1z0az3d9j9s3rjmaukxc58t8hdydu8v0d59wy9p6slce66mwnzjusy76vdq"}}
```

### Performing the Handshake + activating listener

To establish communication between the contract and the enclave, perform the handshake:

```bash
quartz --app-dir "apps/transfers/" handshake --contract <CONTRACT_ADDRESS>
```

Replace `<CONTRACT_ADDRESS>` with the address you received when deploying the contract.

Make note of the handshake generate public key, as you'll need it for the `.env.local` files on the front-end. You should see output similar to:

```bash
2024-09-24T11:12:16.961641Z  INFO Handshake complete: 02360955ff74750f6ea0b539f41cce89451f591e4c835d0a5406e6effa96dd169d
```

Events coming from the contract will be logged following the handshake as they are retrieved by the listener:

```bash
2024-09-24T11:12:25.156779Z  INFO Enclave is listening for requests...
```

## Other Testnets With SGX

To setup on another testnet we need to deploy a tcinfo contract and a
dcap-verifier contract.

### Get the FMSPC of the host machine

```bash
export QUOTE="/* quote generated during the handshake should work */"
cd utils/print-fmspc/
cargo run > /dev/null
```

### Deploying the `tcbinfo` contract

1. Build and store the contract on-chain
```bash
cargo run -- contract build --contract-manifest "../cosmwasm/packages/tcbinfo/Cargo.toml"
RES=$(wasmd tx wasm store ./target/wasm32-unknown-unknown/release/tcbinfo.wasm --from alice -y --output json --chain-id "testing" --gas-prices 0.0025ucosm --gas auto --gas-adjustment 1.3)
TX_HASH=$(echo $RES | jq -r '.["txhash"]')
```

2. Instantiate the contract using Intel's root CA cert.
```bash
CERT=$(sed ':a;N;$!ba;s/\n/\\n/g' ../cosmwasm/packages/quartz-tee-ra/data/root_ca.pem)
RES=$(wasmd query tx "$TX_HASH" --output json)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[1].attributes[1].value')
wasmd tx wasm instantiate "$CODE_ID" "{\"root_cert\": \"$CERT\"}" --from "alice" --label "tcbinfo" --chain-id "testing" --gas-prices 0.0025ucosm --gas auto --gas-adjustment 1.3 -y --no-admin --output json	
TCB_CONTRACT=$(wasmd query wasm list-contract-by-code "$CODE_ID" --output json | jq -r '.contracts[0]')
```

3. Get the Tcbinfo for the given FMSPC.
```bash
HEADERS=$(wget -q -S -O - https://api.trustedservices.intel.com/sgx/certification/v4/tcb?fmspc=00606A000000 2>&1 >/dev/null)
TCB_INFO=$(wget -q -O - https://api.trustedservices.intel.com/sgx/certification/v4/tcb?fmspc=00606A000000)
export TCB_ISSUER_CERT=$(echo "$HEADERS" | 
        grep 'TCB-Info-Issuer-Chain:' | 
        sed 's/.*TCB-Info-Issuer-Chain: //' | 
        sed 's/%0A/\n/g' | 
        sed 's/%20/ /g' | 
        sed 's/-----BEGIN%20CERTIFICATE-----/-----BEGIN CERTIFICATE-----/' | 
        sed 's/-----END%20CERTIFICATE-----/-----END CERTIFICATE-----/' | 
        perl -MURI::Escape -ne 'print uri_unescape($_)' | 
        awk '/-----BEGIN CERTIFICATE-----/{flag=1; print; next} /-----END CERTIFICATE-----/{print; flag=0; exit} flag')

TCB_ISSUER_CERT=$(echo "$TCB_ISSUER_CERT" | sed ':a;N;$!ba;s/\n/\\n/g')
echo "TCB_INFO:"
echo "$TCB_INFO"
echo
echo "TCB_ISSUER_CERT:"
echo "$TCB_ISSUER_CERT"
```

4. Add the Tcbinfo for the given FMSPC to the contract (and test it with a query)
```bash
wasmd tx wasm execute "$TCB_CONTRACT" "{\"tcb_info\": $(echo "$TCB_INFO" | jq -Rs .), \"certificate\": \"$TCB_ISSUER_CERT\"}" --from admin --chain-id testing --gas auto --gas-adjustment 1.2 -y 
wasmd query wasm contract-state smart "$TCB_CONTRACT" '{"get_tcb_info": {"fmspc": "00606A000000"}}'
```

### Deploying the `quartz-dcap-verifier` contract

1. Build the contract
```bash
cargo run -- contract build --contract-manifest "../cosmwasm/packages/quartz-dcap-verifier/Cargo.toml"
```

2. Optimize the contract
In order to optimize the contract, you need to install `wasm-opt` v.119. See the HOWTO section below for installation instructions.
```bash
wasm-opt -Oz ./target/wasm32-unknown-unknown/release/quartz_dcap_verifier.wasm -o ./target/wasm32-unknown-unknown/release/quartz_dcap_verifier.optimized.wasm
```

3. Store the optimized contract on-chain
```bash
RES=$(wasmd tx wasm store ./target/wasm32-unknown-unknown/release/quartz_dcap_verifier.optimized.wasm --from admin -y --output json --chain-id "testing" --gas-prices 0.0025ucosm --gas auto --gas-adjustment 1.3)
TX_HASH=$(echo $RES | jq -r '.["txhash"]')
RES=$(wasmd query tx "$TX_HASH" --output json)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[1].attributes[1].value')
```

4. Instantiate the `quartz-dcap-verifier` contract.
```bash
wasmd tx wasm instantiate "$CODE_ID" null --from "admin" --label "dcap-verifier" --chain-id "testing" --gas-prices 0.0025ucosm --gas auto --gas-adjustment 1.3 -y --no-admin --output json
DCAP_CONTRACT=$(wasmd query wasm list-contract-by-code "$CODE_ID" --output json | jq -r '.contracts[0]')
```

### Quartz setup
```bash
quartz --app-dir "../apps/transfers/" \
    --contract-manifest "../apps/transfers/contracts/Cargo.toml" \
    --unsafe-trust-latest \
    --init-msg '{"denom":"ucosm"}' \
     dev \
    --fmspc "00606A000000" \
    --tcbinfo-contract "$TCB_CONTRACT" \
    --dcap-verifier-contract "$DCAP_CONTRACT"
```

#### HOWTO Install `wasm-opt`

To install `wasm-opt` version 119 on an Azure SGX machine running Ubuntu, follow these steps:

1. **Update and install dependencies:**

```bash
sudo apt update
sudo apt install -y build-essential cmake git
```

2. **Download and build `wasm-opt` version 119:**

```bash
git clone https://github.com/WebAssembly/binaryen.git
cd binaryen
git checkout version_119
```

3. **Build the project:**

```bash
cmake . && make
```

4. **Install `wasm-opt`:**

```bash
sudo make install
```

5. **Verify the installation:**

```bash
wasm-opt --version
```

This should return something like:

```
wasm-opt version_119
```


## Troubleshooting and FAQ

1. **Q: The enclave fails to start. What should I do?**
   A: Ensure all dependencies are correctly installed and that you're using the correct version of each tool.

2. **Q: I'm getting a "contract not found" error during handshake. How do I fix this?**
   A: Double-check that you're using the correct contract address from the deployment step.

3. **Q: The frontend isn't connecting to the blockchain. What's wrong?**
   A: Verify that your `.env.local` file has the correct contract address and public key.

For more issues, please refer to our GitHub issues page or community forums.

## Glossary

- **Enclave**: A protected area of execution in memory.
- **SGX (Software Guard Extensions)**: Intel's technology for hardware-based isolation and memory encryption.
- **FMSPC**: Flexible Memory Sharing Protocol Component.
- **TCB**: Trusted Computing Base.
- **DCAP**: Data Center Attestation Primitives.
- **Wasmd**: Go implementation of a Cosmos SDK-based blockchain with WebAssembly smart contracts.
- **Neutron**: A CosmWasm-enabled blockchain built with the Cosmos SDK.

# Quartz: Getting Started Guide

---

WARNING: Quartz is under heavy development and is not ready for production use.
The current code contains known bugs and security vulnerabilities and APIs are still liable to change.

We are making it available for devleopers to start playing with and to gather
feedback on APIs and roadmap. It can be used today on CosmWasm testnets
(testnets only, with no real funds at risk!).

---

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

For those who want to get started quickly:

1. Install dependencies (Rust, wasmd or neutrond)
2. Clone the repository: `git clone ssh://git@github.com/informalsystems/cycles-quartz`
3. Install Quartz CLI: `cargo install --path cli/`
4. Deploy the example app in one command:
   ```bash
   quartz --mock-sgx --app-dir "apps/transfers/" dev \
   --unsafe-trust-latest \
   --contract-manifest "apps/transfers/contracts/Cargo.toml" \
   --init-msg '{"denom":"ucosm"}'
   ```
5. Set up the frontend (see [Frontend](#frontend))

For more detailed instructions, continue reading the following sections.

## Simple Example

Quartz includes a simple example we call the `Transfer` application. It's
located in [/apps/transfers](/apps/transfers). It's a simple demo app 
designed to showcase very basic use of the Quartz framework. 
It allows users to deposit funds into a contract, 
transfer them privately within the contract's encrypted state, 
and ultimately withdraw whatever balance they have left or have accumulated. 

### Key Features
- Deposit funds into a smart contract
- Transfer funds privately within the contract via encrypted transactions that are handled by Quartz (ie. processed by the enclave and remote attested to).
- Withdraw funds from the contract based on balances in the encrypted state.

### Application Structure

1. **Frontend**: The user interface built with Next.js, cosmjs / graz.
2. **Contracts**: The backend application as a CosmWasm smart contract
3. **Enclave**: Code that executes off-chain and privately in an enclave

## Installation

Quartz is built in Rust and requires an up-to-date version with the
wasm32 target to be installed. It also expects the system to have a
CosmWasm-compatible Cosmos-SDK blockchain client installed, for instance `wasmd`
or `neutrond`. CosmWasm binaries can be built with `Go` or downloaded from their
developers. Finally, you'll need `npm` to build the frontend.

### Install Rust

The minimum Rust supported version is v1.74.1.
The recommended Rust version v1.79.0 since we're running against
wasmd v0.45.

Install rust by executing a script from the internet (😅):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Check the version with `cargo version`.

Finally add the wasm target:
    
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

The docker containers come with preconfigured keys and balances. If you use the
Go binaries you'll have to set up keys and balances yourself.

To build from source, first make sure you have Go installed.

Then for `wasmd`:

```bash
git clone https://github.com/cosmwasm/wasmd/
cd wasmd
git checkout v0.44.0
go install ./cmd/wasmd
```

Or for `neutrond`:

```bash
git clone -b main https://github.com/neutron-org/neutron.git
cd neutron
make install
```

To use the docker images, install and set up docker.

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

If using docker it will pre-configure a few keys and allocate funds to them. 

If building from source, you'll need to initialize the accounts yourself. See
the guide on [setting up a CosmWasm chain](/docs/wasmd_setup.md) and then return
back here.


## Local Testnet Without SGX

From the root of the `cycles-quartz` repo, we can now deploy our example
transfers app. Deployment involves three components:

- the enclave
- the smart contract
- the front end

Quartz provides a `dev` command to simplify building and running the enclave and smart contract in a single command.
Use of the `dev` command was shown in the [quick start](#quick-start) section
above. Here we show the individual steps and quartz commands that allow devs to
independently build and run the encalve, to build and deploy the contract,
and to perform the quartz handshake between running enclave and deployed
contract.

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

We have already predeployed the dcap-verify and tcbinfo contracts on the Neutron
testnet at TODO. To deploy these on your own testnet, see [below](#other-testnets-with-sgx).

To begin, you'll need to deploy an SGX-enabled Azure instance and log in via ssh.

Once logged in, clone and install Quartz like before (see
[installation](#installation).

### Build and Deploy the Contracts

TODO: make this about deploying to neutron.

To build both the contract binaries, use the build command:

```bash
quartz --app-dir "apps/transfers/" contract build --contract-manifest "apps/transfers/contracts/Cargo.toml"
```
This command will compile the smart contract to WebAssembly and build the contract binary.

The following configuration assumes that the `wasmd` node will be running in the same Azure instance as the enclave. 
If you wish to use another enclave provider you have to make sure that `QUARTZ_NODE_URL` is set to the enclave address and port as an argument as in:

```bash
QUARTZ_NODE_URL=87.23.1.3:11090 && quartz --app-dir "apps/transfers/" contract deploy  --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"ucosm"}'
```

If you wish to use another blockchain you have to make sure that `--node-url` is set to the chain address and port as an option in the `cli` as in:

```bash
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/" --node-url "https://92.43.1.4:26657" contract deploy  --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"ucosm"}'
```

### Build and Run the SGX Enclave

First we build the enclave like before:

```bash
# Configure the enclave
quartz --app-dir "apps/transfers/" enclave build
```

Before starting the enclave, we should check that the relevant contracts
(tcbinfo, dcap-verifier) have been instantiated.

TODO: how to query to check this?

TODO: use variables for the contract addresses 



```bash
# Start the enclave
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/" enclave start  --fmspc "00606A000000" --tcbinfo-contract "wasm1pk6xe9hr5wgvl5lcd6wp62236t5p600v9g7nfcsjkf6guvta2s5s7353wa" --dcap-verifier-contract "wasm107cq7x4qmm7mepkuxarcazas23037g4q9u72urzyqu7r4saq3l6srcykw2"
```

The enclave will start running and wait for commands.

### Deploying the Contract

With the enclave running, open a new terminal window to deploy the contract:

```bash
QUARTZ_NODE_URL=127.0.0.1:11090 && quartz --app-dir "apps/transfers/" contract deploy  --contract-manifest "apps/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"ucosm"}'
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

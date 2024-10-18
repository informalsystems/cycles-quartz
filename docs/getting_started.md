# Quartz: Getting Started Guide

## Table of Contents

- [Quartz: Getting Started Guide](#quartz-getting-started-guide)
  - [Table of Contents](#table-of-contents)
  - [Introduction](#introduction)
  - [Quick Start](#quick-start)
  - [Simple Example - Local Mock SGX Application](#simple-example---local-mock-sgx-application)
    - [Installation](#installation)
      - [Install Rust](#install-rust)
      - [Install Quartz](#install-quartz)
      - [Install a CosmWasm Client](#install-a-cosmwasm-client)
    - [Local neutrond Testnet Without SGX](#local-neutrond-testnet-without-sgx)
    - [Enclave](#enclave)
    - [Contract](#contract)
    - [Frontend](#frontend)
    - [Use the App](#use-the-app)
  - [Real Testnet with Azure SGX](#real-testnet-with-azure-sgx)
    - [Setting up an Azure machine](#setting-up-an-azure-machine)
    - [Using an enclave on another machine](#using-an-enclave-on-another-machine)
    - [Other Testnets With SGX](#other-testnets-with-sgx)
  - [Troubleshooting and FAQ](#troubleshooting-and-faq)
  - [Glossary](#glossary)

## Introduction

This guide will help you get up and running with an example Quartz application. You can run this locally using a "mock" enclave (without real privacy or attestations), or you can use a machine with Intel SGX enabled for secure execution. We will go over both setups.

> **Note**: This guide assumes familiarity with blockchain concepts and basic smart contract development.

## Quick Start

For those who want to get started quickly with the example Transfers app with
mock SGX:

1. Install dependencies (Rust, docker)
2. Clone the repository: `git clone ssh://git@github.com/informalsystems/cycles-quartz`
3. Run everything: `cd cycles-quartz/docker && docker compose up`
4. On docker desktop, go to the `enclave` logs and copy `contract address` and `pub key` to later setup the Frontend `env.local`
5. Set up the frontend (see [Frontend](#frontend))

For more detailed background and instructions, read on.

## Simple Example - Local Mock SGX Application

Quartz includes a simple example we call the `Transfer` application,
located in [/examples/transfers](/examples/transfers), that comes with a Keplr-based
frontend. It's a simple demo app designed to showcase very basic use of the Quartz framework. 
It allows users to deposit funds into a contract, 
transfer them privately within the contract's encrypted state (updated by the
enclave),and ultimately withdraw whatever balance they have left or have accumulated. 

Every application has a common structure: 

1. **Frontend**: The user interface (eg. Next.js, cosmjs / graz)
2. **Contracts**: The backend application as a CosmWasm smart contract
3. **Enclave**: Code that executes off-chain and privately in an enclave

Quartz is both a library (`quartz-contract-core`) for building SGX-aware CosmWasm
contracts, and a cli tool (`quartz`) for managing the enclave. 

The library takes care of establishing a secure connection to the enclave (see
[How it Works](/docs/how_it_works.md)), and verifying attestations from
it. The quartz tool provides commands for managing the enclave.

This guide is primarily about using the `quartz` tool to get the example app
setup. For more on building application, see 
- [Building Apps](/docs/building_apps.md) - conceptual overview 
- [quartz-contract-core](/crates/contracts/core/) - main library. provides msgs and handlers
  for the handshake and for verifying attestations
- [transfers contracts](/examples/transfers/contracts): transfer app example itself

Onwards with the installation and running our example app! 

### Installation

Quartz is built in Rust (+wasm32 target). It expects to interact with a CosmWasm compatible
blockchain (eg. `neutrond`), built in Go (or run with Docker). And it requires `npm` for
building the frontend. Here we cover how to install Rust, Quartz, and CosmWasm
blockchains. You're responsible for installing Go and NPM.

Pre-reqs:
- Git
- Make
- Go or Docker
- NPM

#### Install Rust

The minimum Rust supported version is v1.74.1.
The recommended Rust version v1.79.0.

Install rust [here](https://www.rust-lang.org/tools/install).

Check the version with `cargo version`.

Add the wasm32 target:
    
```bash
rustup target add wasm32-unknown-unknown
```

And you should be good to go!

#### Install Quartz

Now clone and build the repo:

```bash
git clone ssh://git@github.com/informalsystems/cycles-quartz
cd cycles-quartz
cargo install --path crates/cli
```

And check that it worked:

```bash
quartz --help
```

#### Install a CosmWasm Client

For the local testnet, we can use `neutrond` with a single validator (we have a docker image for this).

For `neutrond`:

```bash
cd docker
docker compose up node
```

It will pre-configure a few keys (admin, alice, etc.) and allocate funds to them. 
The default sending account for quartz txs is `admin`.
However, these accounts are setup in the docker image. Because we will be deploying our contracts outside of the docker image
we need to have these accounts imported locally. You can do this by install neutrond locally and importing the accounts:

```bash
cd ~
git clone -b main https://github.com/neutron-org/neutron.git
cd neutron/
make install-test-binary

cd cycles-quartz/docker/neutrond
make import-local-accounts
```

Your local `admin` will now be the exact same as the `admin` in the docker image.

If building from source, you'll need to initialize the accounts yourself. See
the guide on [setting up a CosmWasm chain](/docs/neutrond_setup.md) and then return back here.

### Local neutrond Testnet Without SGX

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

We can run everything from within the `examples/transfers` dir in this repo. To run
from elsewhere by specify a path, eg. from the root of the repo with `--app-dir examples/transfers`.

Now, from `examples/transfers`:

1. Build the enclave binary:
   ```bash
   quartz --mock-sgx enclave build
   ```

2. Start the enclave:
   ```bash
   quartz --mock-sgx enclave start
   ```

If the enclave says `Spawning enclave process....` it is working. Now open another window to
continue.

### Contract

1. Build the contract binary:
   ```bash
   quartz --mock-sgx contract build --contract-manifest "contracts/Cargo.toml"
   ```

2. Deploy the contract:
   ```bash
   quartz --mock-sgx contract deploy \
   --contract-manifest "contracts/Cargo.toml" \
   --init-msg '{"denom":"untrn"}'
   ```

Note our contract takes initialization data in the `--init-msg` which for
the transfers app specifies the asset denom that can be used in this deployment. The
transfers app is currently single asset only.

If successful, it will print the resulting contract address. Save it to an
environment variable:

```bash
export CONTRACT_ADDRESS=<CONTRACT_ADDRESS>
```

You must also set the admin secret key:

```bash
export ADMIN_SK=ffc4d3c9119e9e8263de08c0f6e2368ac5c2dacecfeb393f6813da7d178873d2
```

3. Perform the handshake:
   ```bash
   quartz --mock-sgx handshake --contract $CONTRACT_ADDRESS
   ```

This will setup a secure connection between the contract and the enclave.

If successful, it should output a pubkey value. We'll need both the contract
address and this pubkey value to configure the frontend. Save this to an
environment variable: 

```bash
export PUBKEY=<PUBKEY>
```

Now the contract is ready to start processing requests to the enclave. You will see logs from the enclave showing:

```bash
2024-09-24T11:12:25.156779Z  INFO Enclave is listening for requests...
```

### Frontend

You can run the front end on your local computer, so it is easy to test in a browser. If you are running your application in the cloud (such as an Azure SGX machine), you can configure the front end to talk to that blockchain over the internet. You will need node `>= v18.17.0` to build the front end.

1. Navigate to the frontend folder:
   ```bash
   cd examples/transfers/frontend
   ```

2. Install dependencies:
   ```bash
   npm ci
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env.local
   ```

Now open `.env.local` and edit the values of `NEXT_PUBLIC_TRANSFERS_CONTRACT_ADDRESS` and 
`NEXT_PUBLIC_ENCLAVE_PUBLIC_KEY` to be the contract address and pubkey from the previous step. With quartz dev, they can be grabbed
from the logs. From the manual process, you would have already stored them as environment variables.

4. Finally, start the frontend:
   ```bash
   npm run dev
   ```

### Use the App

Open your browser to `localhost:3000` to see the app. You will be prompted to store a mnemonic. This key is stored
in the browser, and allows you to query your encrypted balance in the future. You should save this, but in general
if you are just testing and you don't clear your browser storage, you will be fine.

You'll need to have the Keplr wallet browser extension installed and unlocked.

You may have to go to "Manage Chain Visibility" in Keplr settings to add the `My
Testing Chain` so you can talk to your local chain and see your balance.

Create a new address in Keplr for testing purpose. You'll need to send this
address some funds from the `admin` account setup with your local node. For
instance, send 10M untrn with:

```bash
neutrond tx bank send admin <KEPLR ADDRESS> 10000000untrn --chain-id testing
```

You should now see the funds on your local testnet on Keplr.

Now you can interact with the app by depositing funds, privately transferring
them to other addresses, and finally withdrawing them. 

Be sure to check the enclave window to see the logs from your interaction with
the app!

## Real Testnet with Azure SGX

Now that we've tried the example app on a local testnet with a mocked SGX, it's
time to use a real testnet and a real SGX core. This guide will walk through how
to get setup with SGX on Azure, and how to deploy quartz contracts to the
Neutron testnet using real remote attestations from SGX cores on Azure. Since
this requires setting up an actual SGX setup, its naturally much more
complicated.

Real verification of SGX on a CosmWasm network requires two additional global contracts
to be deployed: `quartz-dcap-verify` and `quartz-tcbinfo`. The
`quartz-dcap-verify` contract provides the core verification of the SGX attestation
(called DCAP). The `quartz-tcbinfo` contract contains global information about secure
versions of SGX processors. Together they allow contracts built with quartz to
securely verify remote attestations from SGX enclaves.

We have already pre-deployed the `quartz-dcap-verify` and `quartz-tcbinfo` contracts on the Neutron
testnet at:
- verifier - `neutron18f3xu4yazfqr48wla9dwr7arn8wfm57qfw8ll6y02qsgmftpft6qfec3uf`
- tcbinfo - `neutron1anj45ushmjntew7zrg5jw2rv0rwfce3nl5d655mzzg8st0qk4wjsds4wps`

To deploy these on your own testnet, see [below](#other-testnets-with-sgx). Although for v0.1, we recommend going with these already deployed contracts.

### Setting up an Azure machine
To begin, you'll need to deploy an SGX-enabled Azure instance and log in via ssh.
Follow the [steps Microsoft lays out](https://learn.microsoft.com/en-us/azure/confidential-computing/quick-create-portal) to connect, choose Ubuntu 20.04, then ssh into the machine.

Once logged in, clone and install Quartz like before (see [installation](#installation)). Once you clone the Quartz repo, you'll have to add some things to your azure machine.

Below we have provided a long instruction set to get the azure machine setup. We plan on dockerizing all of this after the v0.1 launch, as it is quite complex. You can reach out for the team for help if you get stuck here. 

```bash
### INSIDE YOUR AZURE SGX MACHINE ###

# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install 1.79.0
rustup default 1.79.0
rustup target add wasm32-unknown-unknown

# install go
wget https://go.dev/dl/go1.22.2.linux-amd64.tar.gz
rm -rf /usr/local/go && tar -C /usr/local -xzf go1.22.2.linux-amd64.tar.gz
echo "export PATH=\$PATH:/usr/local/go/bin" >> ~/.profile

# necessary building packages
sudo apt update
sudo apt upgrade -y
sudo apt install build-essential
sudo apt install clang libclang-dev
export LIBCLANG_PATH=/usr/lib/llvm-10/lib
sudo apt install pkg-config
sudo apt install libssl-dev
sudo apt install protobuf-compiler
sudo apt-get install ca-certificates

# Clone the repo and install quartz. Reminder - to setup ssh key on azure, or use https
git clone ssh://git@github.com/informalsystems/cycles-quartz
cd cycles-quartz
cargo install --path crates/cli
quartz --help

# install gramine
# Taken from https://gramine.readthedocs.io/en/stable/installation.html#ubuntu-22-04-lts-or-20-04-lts
sudo curl -fsSLo /usr/share/keyrings/gramine-keyring.gpg https://packages.gramineproject.io/gramine-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/gramine-keyring.gpg] https://packages.gramineproject.io/ $(lsb_release -sc) main" \
| sudo tee /etc/apt/sources.list.d/gramine.list

sudo curl -fsSLo /usr/share/keyrings/intel-sgx-deb.asc https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx-deb.asc] https://download.01.org/intel-sgx/sgx_repo/ubuntu $(lsb_release -sc) main" \
| sudo tee /etc/apt/sources.list.d/intel-sgx.list

sudo apt-get update
sudo apt-get install gramine

# add attestation dependencies
# Taken from https://github.com/flashbots/geth-sgx-gramine/tree/main
sudo apt-key adv --fetch-keys 'https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key'
sudo add-apt-repository "deb [arch=amd64] https://download.01.org/intel-sgx/sgx_repo/ubuntu `lsb_release -cs` main"
sudo apt-get update && sudo apt-get install -y libsgx-dcap-ql
sudo apt-key adv --fetch-keys 'https://packages.microsoft.com/keys/microsoft.asc'
sudo apt-add-repository 'https://packages.microsoft.com/ubuntu/20.04/prod main'
sudo apt-get update && sudo apt-get install -y az-dcap-client

# generate gramine priv key
gramine-sgx-gen-private-key

# install neutron and setup accounts
git clone -b main https://github.com/neutron-org/neutron.git
cd neutron/
make install

neutrond keys add admin --keyring-backend test > ./accounts/val1.txt 2>&1

# install node (needed for pccs)
sudo apt-get install nodejs=20.10.0-1nodesource1

# install pccs - see appendix 2
# instructions from https://download.01.org/intel-sgx/latest/linux-latest/docs/Intel_SGX_SW_Installation_Guide_for_Linux.pdf 
# Note - You will be asked a bunch of configuration questions when setting up pcss - for testing, any values will work. In production, please give it careful thought
sudo apt-get install sgx-dcap-pccs
sudo systemctl start pccs

# update /etc/sgx_default_qcnl.conf to config in our repo
sudo cp sgx_default_qcnl.conf /etc/sgx_default_qcnl.conf

# reset pccs
sudo systemctl restart pccs
```

Now everything is installed and ready and we can start running quartz:

```
# build and start the enclave
export TCBINFO_CONTRACT=neutron1anj45ushmjntew7zrg5jw2rv0rwfce3nl5d655mzzg8st0qk4wjsds4wps
export DCAP_CONTRACT=neutron18f3xu4yazfqr48wla9dwr7arn8wfm57qfw8ll6y02qsgmftpft6qfec3uf
export ADMIN_SK=ffc4d3c9119e9e8263de08c0f6e2368ac5c2dacecfeb393f6813da7d178873d2
cd examples/transfers

# copy the neutron testnet config file to the default quartz.toml file, so we connect to the right nodes
cp quartz.neutron_pion-1.toml quartz.toml
quartz enclave build
quartz enclave start  --fmspc "00906ED50000" --tcbinfo-contract $TCBINFO_CONTRACT --dcap-verifier-contract $DCAP_CONTRACT --unsafe-trust-latest

# build and deploy the contracts
quartz contract build --contract-manifest "contracts/Cargo.toml"
quartz contract deploy --contract-manifest "contracts/Cargo.toml" --init-msg '{"denom":"untrn"}'

# store the output
export CONTRACT=<CONTRACT_ADDRESS>

# create the handshake
quartz handshake --contract $CONTRACT

### ENCLAVE IS SETUP AND RUNNING! CONGRATS!
```

Wahoo! Now follow the instructions in the [Front End section](#frontend) of this doc to test the application with a real enclave.

### Using an enclave on another machine
You can use a remote enclave machine by setting the following env var:
```bash
QUARTZ_NODE_URL=<YOUR_IP_ADDR>:11090
# You can now use that enclave to deploy
cd examples/transfers
quartz contract deploy  --contract-manifest "examples/transfers/contracts/Cargo.toml"   --init-msg '{"denom":"untrn"}'
```

### Other Testnets With SGX

To setup on another testnet we need to deploy a `quartz-tcbinfo` contract and a
`quartz-dcap-verifier` contract. However we recommend using the deployed contracts on neutrons public testnet for v0.1.

Instructions can be followed in [this guide](./tcbinfo_and_verifier.md).

## Troubleshooting and FAQ

1. **Q: The enclave fails to start. What should I do?**
   A: Ensure all dependencies are correctly installed and that you're using the correct version of each tool.

2. **Q: I'm getting a "contract not found" error during handshake. How do I fix this?**
   A: Double-check that you're using the correct contract address from the deployment step.

3. **Q: The frontend isn't connecting to the blockchain. What's wrong?**
   A: Verify that your `.env.local` file has the correct contract address and public key.

4. **Error in event handler: Unsupported event**
   This error is fine when it appears in the enclave logs, we are working to remove this erroneous message.

For more issues, please refer to our GitHub issues page or community forums.

## Glossary

- **Enclave**: A protected area of execution in memory.
- **SGX (Software Guard Extensions)**: Intel's technology for hardware-based isolation and memory encryption.
- **FMSPC**: Flexible Memory Sharing Protocol Component.
- **TCB**: Trusted Computing Base.
- **DCAP**: Data Center Attestation Primitives.
- **Wasmd**: Go implementation of a Cosmos SDK-based blockchain with WebAssembly smart contracts.
- **Neutron**: A CosmWasm-enabled blockchain built with the Cosmos SDK.

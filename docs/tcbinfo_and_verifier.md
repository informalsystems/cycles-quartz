# Deploying tcbinfo and dcap verifier on a chain
We have deployed the `dcap-verifier-contract` and `tcbinfo-contract` on neutrons public testnet. If you need to setup your own testnet, or use another testnet, you can use this guide. However in v0.1, we recommend sticking to the contracts we deployed. Remember to set the `QUARTZ_NODE_URL` env var to your testnet. 

## Get the FMSPC of the host machine

```bash
export QUOTE="/* quote generated during the handshake should work */"
cd crates/utils/print-fmspc/
cargo run > /dev/null
```

## Deploying the `quartz-tcbinfo` contract

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

## Deploying the `quartz-dcap-verifier` contract

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

## Quartz setup
```bash
quartz --app-dir "../examples/transfers/" \
    --contract-manifest "../examples/transfers/contracts/Cargo.toml" \
    --unsafe-trust-latest \
    --init-msg '{"denom":"ucosm"}' \
     dev \
    --fmspc "00606A000000" \
    --tcbinfo-contract "$TCB_CONTRACT" \
    --dcap-verifier-contract "$DCAP_CONTRACT"
```

### How To Install `wasm-opt`

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

#!/bin/sh
set e

# Clean previous keys & reset node
rm -rf /root/.neutrond/keyring-test &> /dev/null
neutrond tendermint unsafe-reset-all

# Init configuration files
neutrond init test --chain-id test-1 --overwrite

# Modify default configurations
sed -i 's/keyring-backend = "os"/keyring-backend = "test"/g' /root/.neutrond/config/client.toml
sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \[\"*\"\]/g' /root/.neutrond/config/config.toml
sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0.0025untrn"/g' /root/.neutrond/config/app.toml
sed -i 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' /root/.neutrond/config/app.toml
sed -i 's/enable = false/enable = true/g' /root/.neutrond/config/app.toml
sed -i 's/swagger = false/swagger = true/g' /root/.neutrond/config/app.toml

GENESIS_PATH="/root/.neutrond/config/genesis.json"

function set_genesis_param_jq() {
  param_path=$1
  param_value=$2
  jq "${param_path} = ${param_value}" > tmp_genesis_file.json < "$GENESIS_PATH" && mv tmp_genesis_file.json "$GENESIS_PATH"
}

# feemarket
set_genesis_param_jq ".app_state.feemarket.params.min_base_gas_price" "\"0.0025\""
set_genesis_param_jq ".app_state.feemarket.params.fee_denom" "\"untrn\""
set_genesis_param_jq ".app_state.feemarket.state.base_gas_price" "\"0.0025\""

# Import all test accounts
for filename in /root/accounts/*.txt; do
  tail -n 1 "$filename" | neutrond keys add "$(basename "$filename" .txt)" --no-backup --recover --keyring-backend=test

  neutrond add-genesis-account "$(neutrond keys show $(basename "$filename" .txt) -a)" 12000000000000untrn
done

# Enable as a single node consumer
neutrond add-consumer-section

# Start node
neutrond start --trace --rpc.laddr="tcp://0.0.0.0:26657" --api.address="tcp://0.0.0.0:1317" --grpc.address="0.0.0.0:9090"

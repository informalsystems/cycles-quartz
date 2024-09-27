#!/bin/bash

# Deploy the specified contract's `WASM_BIN` to the chain specified by `CHAIN_ID` using the `USER_ADDR` account.

set -eo pipefail

usage() {
    echo "Usage: $0 WASM_BIN [COUNT]"
    echo "Example: $0 artifacts/cofi_karma_game.wasm"
    exit 1
}

if [ -z "$1" ]; then
    echo "❌ Error: Missing WASM_BIN parameter. Please check if all parameters were specified."
    usage
fi

if [ "$#" -gt 9 ]; then
    echo "❌ Error: Incorrect number of parameters."
    usage
fi

USER_ADDR=${USER_ADDR:-$(wasmd keys show -a admin)}
WASM_BIN="$1"
CHAIN_ID=${CHAIN_ID:-testing}
NODE_URL=${NODE_URL:-127.0.0.1:26657}
LABEL=${LABEL:-bisenzone-mvp}
COUNT=${COUNT:-0}
INSTANTIATE_MSG=${INSTANTIATE_MSG:-"null"}

TXFLAG="--chain-id ${CHAIN_ID} --gas-prices 0.0025ucosm --gas auto --gas-adjustment 1.3"

CMD="wasmd --node http://$NODE_URL"

echo "🚀 Deploying WASM contract '${WASM_BIN}' on chain '${CHAIN_ID}' using account '${USER_ADDR}'..."
echo " with cmd : $CMD"
echo "===================================================================="

RES=$($CMD tx wasm store "$WASM_BIN" --from "$USER_ADDR" $TXFLAG -y --output json)
echo $RES
TX_HASH=$(echo $RES | jq -r '.["txhash"]')

while ! $CMD query tx $TX_HASH &> /dev/null; do
    echo "... 🕐 waiting for contract to deploy from tx hash $TX_HASH"
    sleep 1
done

RES=$($CMD query tx "$TX_HASH" --output json)
CODE_ID=$(echo $RES | jq -r '.logs[0].events[1].attributes[1].value')

echo ""
echo "🚀 Instantiating contract with the following parameters:"
echo "--------------------------------------------------------"
echo "Label: ${LABEL}"
echo "--------------------------------------------------------"

RES=$($CMD tx wasm instantiate "$CODE_ID" "$INSTANTIATE_MSG" --from "$USER_ADDR" --label $LABEL $TXFLAG -y --no-admin --output json)
TX_HASH=$(echo $RES | jq -r '.["txhash"]')


echo ""
while ! $CMD query tx $TX_HASH &> /dev/null; do
    echo "... 🕐 waiting for contract to be queryable from tx hash $TX_HASH"
    sleep 1
done

RES=$($CMD query wasm list-contract-by-code "$CODE_ID" --output json)
CONTRACT=$(echo $RES | jq -r '.contracts[0]')

echo "🚀 Successfully deployed and instantiated contract!"
echo "🔗 Chain ID: ${CHAIN_ID}"
echo "🆔 Code ID: ${CODE_ID}"
echo "📌 Contract Address: ${CONTRACT}"
echo "🔑 Contract Key: ${KEY}"
echo "🔖 Contract Label: ${LABEL}"

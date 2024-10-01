# CosmWasm smart contract to verify DCAP attestations

## Testing instructions
```
wasmd query wasm contract-state smart "$CONTRACT" '{
    "verify_dcap_attestation": {
        "quote": { /* ... */ },
        "collateral": { /* ... */ },
        "mrenclave": "e3c2f2a5b840d89e069acaffcadb6510ef866a73d3a9ee57100ed5f8646ee4bb", 
        "user_data": "9113b0be77ed5d0d68680ec77206b8d587ed40679b71321ccdd5405e4d54a6820000000000000000000000000000000000000000000000000000000000000000"
    }
}'
```
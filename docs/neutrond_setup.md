# Neutrond Setup

> Note - We would like to highly recommend setting up the `neutrond` node with the docker image provided. However, we also understand that docker is a poison upon the human soul, and thus provide these instructions for setting up the neutrond node locally, without imbibing such poision. It will probably take less time to follow these instructions than it will to build the docker image anyways. Up to you 🫡 

Quartz expects to interact with a CosmWasm-based blockchain. 
The default/archetypal binary is `neutrond`. Currently its hardcoded to use
`neutrond` though of course we will make this configurable soon.

Here we describe how to get setup from scratch or how to use an existing `neutrond`
binary/network you have access to.

- [Install](#install)
- [Configure from scratch](#configure-from-scratch)
- [Configure existing](#configure-existing)

## Install

To build from source, first make sure you have Go installed.

For `neutrond`:

```bash
git clone -b main https://github.com/neutron-org/neutron.git
cd neutron
git checkout v4.0.1
make install-test-binary
```

## Configure From Scratch

We have to initialize a new chain and load it with some accounts.

If you already have `neutrond` keys, you may need to rename them or use
different names if the names overlap.

We also have to give the chain a chain ID. We'll use `testing`.

Run 

```bash
neutrond init yourname --chain-id testing --default-denom untrn
```

to initialize the local neutrond folder.

Now open the file `~/.neutrond/config/client.toml` and change the field
`keyring-backend` from `os` to `test`:

```toml 
keyring-backend = "test" 
```

Now, finally, we can create a local admin key for your neutrond. You'll use this to
deploy contracts:

```bash 
neutrond keys add admin 
```

If you already have a key called `admin` in your keystore it's advised to rename it first.
If you want to use a different name then `admin`, be sure to also change it in
the `examples/transfers/quartz.toml` and everywhere we use it below.

This should output a neutron address. 

Now create the genesis file.

```bash 
# fund the account in genesis 
neutrond add-genesis-account admin 100000000000untrn

# configure the ICS setup (neutrond expects to run as a consumer chain)
neutrond add-consumer-section
```

Before finally starting the node, for it to work with the front end, you need to
configure CORS and a min gas price.

### Configure CORS

In `~/.neutrond/config/config.toml`, you'll need to make sure the listen address
binds to the public IP (0.0.0.0) and the CORS allows all origins:

```toml 
[rpc] 
laddr = "tcp://0.0.0.0:26657" 
cors_allowed_origins = ["*"] 
```

And in `~/.neutrond/config/app.toml`:

```toml 
[api] 
enable = true 
address = "tcp://0.0.0.0:1317" 
enabled-unsafe-cors = true 
```

### Configure min gas

In `~/.neutrond/config/app.toml`, set the min gas price:

```toml
minimum-gas-prices = "0.0001untrn"
```

And in `~/.neutrond/config/genesis.json`, set the denom and the feemarket min gas price:

```json
        "fee_denom": "untrn", 
```

```json
        "min_base_gas_price": "0.000100000000000000",
```

and 

```json
        "base_gas_price": "0.000100000000000000",
```

Now, finally:

## neutrond start

```bash 
neutrond start 
```

And you should have a chain making blocks!

You can also reduce the block time by lowering `timeout_commit` in
`~/.neutrond/config/config.toml`.

Now that you have the chain running, you can start running the enclave and proxy
in other windows.

Return to the [getting started guide](/docs/getting_started.md#installation)

## Configure Existing

If you want to join an existing testnet you either need to setup a node and 
sync that testnet or find a node to use.

You'll also need to setup an account and get it funded.

Assuming you're using the Neutron testnet, create a new account called `admin`:

```bash
neutrond keys add admin
```

Now use a faucet or send this address to someone who can give you funds on the tesnet. 

If you have funds yourself you can simply transfer them:

```bash 
neutrond tx bank send <sender key name> <recipient address> <amount>
--chain-id testing -y 
```

One your `admin` account is funded on the network, return to the [getting started guide](/docs/getting_started.md#installation)

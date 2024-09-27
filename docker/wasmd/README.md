# Quartz app wasmd image

This folder contains a `Dockerfile` that helps build a single-node [wasmd]
validator for use in testing your Quartz application.

It facilitates the creation of a Docker image with 4 accounts pre-loaded, each
having a small amount of `ucosm` preloaded from genesis for experimentation.

- `admin`
- `alice`
- `bob`
- `charlie`

These accounts' details are stored in clear text in the [/accounts](./accounts/)
folder.

**Note: this image is _not_ intended to be used in production.**

## Running the image

From this directory, simply run:

```bash
make run
```

This will implicitly call `make build` to build the Docker image, and then run
it, binding to the following ports on the local machine:

- 1317
- 9090
- 26656
- 26657

It also implicitly creates a Docker volume called `wasmd_data` such that just
the contents of the `/root` directory in the container persist across restarts.
To wipe this volume and start from scratch, simply terminate the container and
run:

```bash
docker volume rm wasmd_data
```

## Building the image

To build the image without running it, simply run:

```bash
make
```

This will, by default, build a Docker image tagged `informaldev/wasmd:v0.52.0`.

## Transacting on behalf of the accounts

The accounts listed in the [`/accounts`](./accounts/) folder are all already
imported into the `test` keyring within the Docker image. Once the container is
running, you can run the following to list them:

```bash
# Assumes the container is called "wasmd"
docker exec -it wasmd \
  wasmd keys list --keyring-backend=test
```

## Importing the account keys

As previously mentioned, the [`/accounts`](./accounts/) folder contains all of
the necessary material to construct the public/private keypairs of the accounts.

A convenient helper target is provided in [`/wasmd/Makefile`](./wasmd/Makefile) to facilitate
importing of these accounts into a local `wasmd` configuration (i.e. on your
host machine, outside of the Docker container). This will allow you to transact
on behalf of any of those accounts from outside of the Docker container.

**NB**: For this to work, you will need the same version of `wasmd` installed on
your local machine as what is built into the `wasmd` Docker image.

```bash
make import-local-accounts
```

To check that the accounts have been imported correctly, on your host machine
run:

```bash
# List all keys available in your local wasmd configuration
wasmd keys list --keyring-backend=test
```

> Note - You don't need to run `make create-local-accounts` because that was already done, hence why the text files (i.e. `alice.txt`) are included on git. However, they won't be in your local keyring until you run `make import-local-accounts`. If we ever have to reset the accounts, you'd run `make delete-local-accounts` followed by `make create-local-accounts` and push the updated accounts to github.

## Querying accounts in the wasmd container

To query, for example, the `admin` account's balance, where the `admin`
account's address is `wasm1mkrm9m8g0dzv5z73xg8yzlj6srqc72qru5xfv3`, once the
Docker container is running, in a separate terminal run:

```bash
# The first "wasmd" indicates the name of the running Docker container, whereas
# the second "wasmd" indicates the name of the command to run inside the
# container.
docker exec -it wasmd \
    wasmd query bank balances wasm1mkrm9m8g0dzv5z73xg8yzlj6srqc72qru5xfv3
```

You should see output like:

```bash
balances:
- amount: "12000000000000"
  denom: ucosm
- amount: "12000000000000"
  denom: ustake
pagination:
  next_key: null
  total: "0"
```

[wasmd]: https://github.com/CosmWasm/wasmd

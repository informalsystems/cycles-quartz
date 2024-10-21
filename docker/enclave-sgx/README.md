# Quartz Enclave Build/Run Image for SGX

> NOTE - As of v0.1 release, this dockerfile is out of date and does not work.

This folder contains the basis for a multi-stage Docker image that:

1. Builds the enclave
2. Takes the binary from the build stage and embeds it in the [Gramine Docker
   image][gramine-docker], such that it can run on an SGX-enabled machine.

## Requirements

- Docker

The build process itself does not require an SGX-capable processor, but running
the image does.

## Setup

### Secrets

**TODO: Remove this subsection once all necessary subcomponents are
open-sourced.**

Before building the image, you will need to ensure that the image has access to
a public/private keypair that will allow access to the private dependencies
needed by the build process.

For example, you could generate a public/private keypair as follows. **NB: This
keypair must _not_ be password-protected, since it needs to be accessible in an
unsupervised manner during the Docker image build.**

```bash
# Should generate ~/.ssh/{id_ed25519,id_ed25519.pub}
ssh-keygen -t ed25519
```

Both the public and private keys must be copied into a `.secrets` folder in the
root of this repository prior to building the image.

```bash
# From the root of the cycles-quartz repo
mkdir -p .secrets
cp ~/.ssh/id_ed25519* .secrets/
```

You will, of course, need to make sure that you have added the public key to
your GitHub account in your SSH key settings before these keys will be useful.
Once you have built the image, you can delete the keys and remove them from your
GitHub account.

### Trusted Height and Hash

The enclave needs to know that it can trust the chain with which it interacts,
and to do so it uses a light client that needs to be initialized with a root
trusted height along with the hash of the block at that height.

A simple way of initializing such a light client for testing/experimentation
purposes is to query the chain:

```bash
# Assumes a CometBFT-based chain accessible via localhost. Replace "localhost"
# with the host/IP address of the full node/validator you want to query.
#
# Gets the latest block for the chain. Pipes the output through jq to format it
# nicely.
curl http://localhost:26657/block | jq
```

The hash in which we are interested is the _last_ block hash, meaning that the
height of that block is the height of the response from the above command minus
1.

## Building

As an example, to build a Docker image for the transfers app's enclave:

```bash
# From the root of the cycles-quartz repository
docker build \
   --platform linux/amd64 \
   --build-arg ENCLAVE_DIR=apps/transfers/enclave \
   --build-arg ENCLAVE_BIN=quartz-app-transfers-enclave \
   --build-arg TRUSTED_HEIGHT=1234 \
   --build-arg TRUSTED_HASH=0123456789abcdef \
   -t informaldev/transfers-enclave \
   -f ./docker/enclave-sgx/Dockerfile \
   .
```

This builds an image tagged `informaldev/transfers-enclave:latest`.

The following build arguments are important:

- `ENCLAVE_DIR` - The relative path, from the root of the repository, to the
  enclave source code.
- `ENCLAVE_BIN` - The filename of the enclave binary (usually defined in the
  `Cargo.toml` file).
- `TRUSTED_HEIGHT` - The trusted height of the chain to build into the image for
  the enclave light client.
- `TRUSTED_HASH` - The trusted hash of the chain to build into the image for the
  enclave light client.

## Running

On an SGX-enabled machine:

```bash
# The devices need to be mounted into the container.
docker run --rm -it \
   --device /dev/sgx_enclave \
   --device /dev/sgx_provision \
   informaldev/transfers-enclave
```

[gramine-docker]: https://hub.docker.com/r/gramineproject/gramine

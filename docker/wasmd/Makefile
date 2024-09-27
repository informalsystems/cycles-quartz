WASMD_VERSION?=v0.52.0
ORG?=informaldev
IMAGE?=$(ORG)/wasmd
.DEFAULT_GOAL := build

build:
	docker build \
		--build-arg="WASMD_VERSION=$(WASMD_VERSION)" \
		-t $(IMAGE):$(WASMD_VERSION) \
		.
.PHONY: build

run: build
	docker run --rm -it \
		-p 26657:26657 -p 26656:26656 -p 1317:1317 -p 9090:9090 \
		--mount type=volume,source=wasmd_data,target=/root \
		--name wasmd \
		$(ORG)/wasmd:$(WASMD_VERSION)
.PHONY: run

# The create-accounts and delete-accounts commands are exclusively for local use
# to refresh the accounts in ./accounts/
#
# Both targets require that the correct version of wasmd be installed locally.
create-local-accounts:
	mkdir -p ./accounts
	wasmd keys add admin > ./accounts/admin.txt 2>&1
	wasmd keys add alice > ./accounts/alice.txt 2>&1
	wasmd keys add bob > ./accounts/bob.txt 2>&1
	wasmd keys add charlie > ./accounts/charlie.txt 2>&1
.PHONY: create-accounts

delete-local-accounts:
	wasmd keys delete -y admin
	wasmd keys delete -y alice
	wasmd keys delete -y bob
	wasmd keys delete -y charlie
.PHONY: delete-accounts

# Imports the accounts for admin, alice, bob and charlie from the text files in
# the ./accounts/ folder. We assume that the mnemonic for each account is the
# last line of each of the text files.
import-local-accounts:
	tail -n 1 accounts/admin.txt | wasmd keys add admin --recover --keyring-backend=test
	tail -n 1 accounts/alice.txt | wasmd keys add alice --recover --keyring-backend=test
	tail -n 1 accounts/bob.txt | wasmd keys add bob --recover --keyring-backend=test
	tail -n 1 accounts/charlie.txt | wasmd keys add charlie --recover --keyring-backend=test
.PHONY: import-accounts

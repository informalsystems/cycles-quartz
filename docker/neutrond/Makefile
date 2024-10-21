# Setup local accounts. This is only for local development and should not be used in production.
import-local-accounts:
	neutrond keys add admin --keyring-backend test > ./data/accounts/admin.txt 2>&1
	neutrond keys add alice --keyring-backend test > ./data/accounts/alice.txt 2>&1
	neutrond keys add bob --keyring-backend test > ./data/accounts/bob.txt 2>&1
	neutrond keys add charlie --keyring-backend test > ./data/accounts/charlie.txt 2>&1

create-local-accounts: import-local-accounts
	@echo "Local accounts have been created."

.PHONY: create-local-accounts import-local-accounts

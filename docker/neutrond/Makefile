# the ./accounts/ folder. We assume that the mnemonic for each account is the
# last line of each of the text files.
import-local-accounts:
	tail -n 1 data/accounts/admin.txt | neutrond keys add admin --recover --keyring-backend=test
	tail -n 1 data/accounts/alice.txt | neutrond keys add alice --recover --keyring-backend=test
	tail -n 1 data/accounts/bob.txt | neutrond keys add bob --recover --keyring-backend=test
	tail -n 1 data/accounts/charlie.txt | neutrond keys add charlie --recover --keyring-backend=test
.PHONY: import-accounts

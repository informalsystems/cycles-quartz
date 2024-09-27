ARG WASMD_VERSION=v0.52.0

FROM cosmwasm/wasmd:${WASMD_VERSION}

# Increase the amount of ucosm given to accounts
RUN <<EOF
sed -i 's/keyring-backend = "os"/keyring-backend = "test"/g' /root/.wasmd/config/client.toml
sed -i  's/1000000000/12000000000000/g' /opt/setup_wasmd.sh
EOF

# Set up wasmd. The account numbers correspond to those in the ./accounts/
# folder.
RUN /opt/setup_wasmd.sh \
        wasm1mkrm9m8g0dzv5z73xg8yzlj6srqc72qru5xfv3 \
        wasm19azg82cx3qx88gar8nl08rz7x0p27amtmadfep \
        wasm1adcnk7lt6qst7p5d0g5607e28k77um7nxwxuqy \
        wasm1jn34x50hy3my0a2mxwcx8fttgfxu2n5gpvu0ty

# Import the accounts' private keys into the image such that they can be used to
# initiate transactions from within the container.
COPY accounts /tmp/accounts
RUN <<EOF
tail -n 1 /tmp/accounts/admin.txt | wasmd keys add admin --recover --keyring-backend=test
tail -n 1 /tmp/accounts/alice.txt | wasmd keys add alice --recover --keyring-backend=test
tail -n 1 /tmp/accounts/bob.txt | wasmd keys add bob --recover --keyring-backend=test
tail -n 1 /tmp/accounts/charlie.txt | wasmd keys add charlie --recover --keyring-backend=test
rm -rf /tmp/accounts
EOF

# Configure the validator for single-node operation within a Docker container
RUN <<EOF
sed -i 's/keyring-backend = "os"/keyring-backend = "test"/g' /root/.wasmd/config/client.toml
sed -i 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' /root/.wasmd/config/app.toml
sed -i 's/address = \"tcp:\/\/localhost:1317\"/address = \"tcp:\/\/0.0.0.0:1317\"/g' /root/.wasmd/config/app.toml
sed -i 's/address = \"localhost:909/address = \"0.0.0.0:909/g' /root/.wasmd/config/app.toml
sed -i 's/enable = false/enable = true/g' /root/.wasmd/config/app.toml
sed -i 's/rpc-max-body-bytes = 1000000$/rpc-max-body-bytes = 1000000000/g' /root/.wasmd/config/app.toml
sed -i 's/laddr = \"tcp:\/\/127.0.0.1:26657\"/laddr = \"tcp:\/\/0.0.0.0:26657\"/g' /root/.wasmd/config/config.toml
sed -i 's/cors_allowed_origins = \[\]/cors_allowed_origins = \[\"*\"\]/g' /root/.wasmd/config/config.toml
sed -i 's/max_body_bytes = 1000000$/max_body_bytes = 1000000000/g' /root/.wasmd/config/config.toml
sed -i 's/max_tx_bytes = 1048576$/max_tx_bytes = 104857600/g' /root/.wasmd/config/config.toml
EOF

CMD ["/opt/run_wasmd.sh"]

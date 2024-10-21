FROM golang:1.22-bullseye

RUN apt-get update && apt-get install -y jq  && \
    git clone https://github.com/neutron-org/neutron.git /root/neutron && \
    cd /root/neutron && make install-test-binary

COPY data /root/

CMD bash /root/entrypoint.sh

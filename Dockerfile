FROM continuumio/miniconda3

WORKDIR /app
RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
   apt-get install curl dnsutils jq make -y --no-install-recommends

RUN curl -Lo /tmp/bw.zip 'https://vault.bitwarden.com/download/?app=cli&platform=linux'
RUN gunzip -S .zip /tmp/bw.zip
RUN chmod +x /tmp/bw

RUN curl -Lo ytt https://github.com/vmware-tanzu/carvel-ytt/releases/download/v0.40.1/ytt-linux-amd64 && chmod +x ytt
RUN /tmp/bw config server https://vaultwarden.gambaru.io

COPY target/release/ymlex ymlex
COPY configs ./configs
COPY solvers ./solvers
COPY Makefile ./Makefile
RUN mv -v /tmp/bw solvers/bitwarden/bw

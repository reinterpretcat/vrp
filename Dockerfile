FROM ubuntu:20.04

# install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl

# install rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /repo

ENTRYPOINT cargo test && /bin/bash

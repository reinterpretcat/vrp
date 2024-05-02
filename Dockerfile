FROM rust:1.78-alpine AS Builder

LABEL maintainer="Ilya Builuk <ilya.builuk@gmail.com>" \
      org.opencontainers.image.title="A Vehicle Routing Problem solver CLI" \
      org.opencontainers.image.description="A tool to solve real world Vehicle Routing Problems" \
      org.opencontainers.image.source="https://github.com/reinterpretcat/vrp" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.authors="Ilya Builuk <ilya.builuk@gmail.com>"

RUN apk add --no-cache musl-dev

WORKDIR /src/

# copy source code
COPY Cargo.toml ./
COPY experiments/heuristic-research ./experiments/heuristic-research
COPY examples ./examples
COPY rosomaxa ./rosomaxa
COPY vrp-core ./vrp-core
COPY vrp-scientific ./vrp-scientific
COPY vrp-pragmatic ./vrp-pragmatic
COPY vrp-cli ./vrp-cli

RUN cargo build --release -p vrp-cli


FROM alpine:3.18

ENV SOLVER_DIR=/solver

RUN mkdir $SOLVER_DIR
COPY --from=Builder /src/target/release/vrp-cli $SOLVER_DIR/vrp-cli

WORKDIR $SOLVER_DIR

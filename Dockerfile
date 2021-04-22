FROM rust:latest

WORKDIR /mech
COPY . .

RUN pwd
RUN rustup override set nightly
RUN rustup default nightly-2021-02-22
RUN cargo build --bin mech --release

ENV PATH="/mech/target/release:${PATH}"
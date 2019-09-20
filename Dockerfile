FROM rust:latest

WORKDIR /mech
COPY . .

RUN pwd
RUN rustup override set nightly
RUN cargo build --bin mech

ENV PATH="/mech/target/debug:${PATH}"
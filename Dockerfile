FROM rust:slim-buster

RUN apt-get upgrade -y && apt update \
    && apt-get install openssl git ca-certificates --no-install-recommends -y \
    && git clone https://github.com/LoliGothick/sort_nazonazo_rs.git \
    && cd sort_nazonazo_rs \
    && git checkout sandbox/test-bot \
    && cargo build --release

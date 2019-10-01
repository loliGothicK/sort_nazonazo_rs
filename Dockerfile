FROM rust:slim-buster AS build-env

RUN apt-get upgrade -y && apt update \
    && apt-get install openssl git ca-certificates musl-tools --no-install-recommends -y \
    && git clone https://github.com/LoliGothick/sort_nazonazo_rs.git \
    && cd sort_nazonazo_rs \
    && git checkout develop \
    && rustup update nightly \
    && rustup default nightly \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo build --release --target=x86_64-unknown-linux-musl

FROM rust:alpine

RUN apk update && apk add ca-certificates openssl && rm -rf /var/cache/apk/*
COPY --from=build-env /sort_nazonazo_rs/target/x86_64-unknown-linux-musl/release/mitama-test-bot /usr/local/bin/mitama-test-bot
COPY --from=build-env /sort_nazonazo_rs/dictionaries/* /usr/dictionaries/
ENV DIC_DIR="/usr/dictionaries/"
ENTRYPOINT ["/usr/local/bin/mitama-test-bot"]

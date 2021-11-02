FROM rust:slim-buster as builder
RUN apt-get update && apt-get -y --no-install-recommends install \
    pkg-config libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /src
COPY . .

RUN RUSTFLAGS='-C link-arg=-s' \
    OPENSSL_STATIC=yes \
    OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu \
    OPENSSL_INCLUDE_DIR=/usr/include \
    cargo build --release

FROM debian:buster-slim
COPY --from=builder /src/target/release/tws-rust /usr/local/bin/tws-rust
CMD ["tws-rust"]

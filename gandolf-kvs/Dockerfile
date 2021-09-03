From rust:latest As builder

Workdir /usr/src/gandolf

Copy . .

Run rustup component add rustfmt
Run cargo install --path .

From debian:buster-slim

COPY --from=builder /usr/local/cargo/bin/gandolf-kvs-server /usr/local/bin/gandolf-kvs-server

CMD gandolf-kvs-server

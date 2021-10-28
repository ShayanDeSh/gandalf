From rust:latest As builder

Workdir /usr/src/gandalf

Copy . .

Run rustup component add rustfmt
Run cargo install --path .

From debian:buster-slim

COPY --from=builder /usr/local/cargo/bin/gandalf-kvs-server /usr/local/bin/gandalf-kvs-server

CMD gandalf-kvs-server

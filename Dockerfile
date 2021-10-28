From rust:latest As builder

Workdir /usr/src/gandalf

Copy . .

Run rustup component add rustfmt
Run cargo install --path ./gandalf-consensus


From debian:buster-slim

COPY --from=builder /usr/local/cargo/bin/gandalf /usr/local/bin/gandalf

CMD gandalf

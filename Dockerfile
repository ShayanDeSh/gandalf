From rust:latest As builder

Workdir /usr/src/gandolf

Copy . .

Run rustup component add rustfmt
Run cargo install --path ./gandolf-consensus


From debian:buster-slim

COPY --from=builder /usr/local/cargo/bin/gandolf /usr/local/bin/gandolf

CMD gandolf

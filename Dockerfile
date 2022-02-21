FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools
RUN update-ca-certificates

WORKDIR /build
RUN USER=root cargo new droplet
WORKDIR /build/droplet

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /

COPY --from=builder /usr/local/cargo/bin/droplet /

USER 1000


ENV DROPLET_TARGET_DIR=/target
VOLUME "/target"

ENV DROPLET_ADDRESS=0.0.0.0:3000
EXPOSE 3000

CMD ["/droplet"]
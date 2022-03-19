FROM lukemathwalker/cargo-chef:0.1.34-rust-alpine3.15 AS chef
WORKDIR app

# Prepare dependdencies, utlizing docker caching
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl

# Build the app
COPY . .
RUN cargo build --target x86_64-unknown-linux-musl
RUN strip target/release/droplet

# Assemble the release container
FROM scratch
WORKDIR /
COPY --from=builder /app/target/release/droplet /

USER 1000:1000
VOLUME "/target"
ENV DROPLET_TARGET_DIR=/target
ENV DROPLET_ADDRESS=0.0.0.0:3000
EXPOSE 3000

CMD ["/droplet"]

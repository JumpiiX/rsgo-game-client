FROM rust:latest as builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

# Generate new Cargo.lock and build
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rsgo-backend ./

EXPOSE 6969
ENV PORT=6969
# env_logger shows nothing below `error` unless RUST_LOG is set. Default to info
# so the lobby/join/team logs (log::info!) are visible in `docker logs`.
ENV RUST_LOG=info
CMD ["./rsgo-backend"]
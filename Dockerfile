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

EXPOSE 8080
CMD ["./rsgo-backend"]
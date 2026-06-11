FROM rust:1.87-slim-bookworm AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler clang lld && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef

FROM chef AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin bilibili-crawler

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/bilibili-crawler .
RUN mkdir -p /app/output /app/data
ENTRYPOINT ["./bilibili-crawler"]

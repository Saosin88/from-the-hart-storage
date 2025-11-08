FROM rust:1.91-slim AS builder

WORKDIR /app

COPY Cargo.toml ./

COPY src ./src

RUN cargo build --release --bin from-the-hart-storage

FROM debian:bookworm-slim

WORKDIR /app

ARG ENVIRONMENT=prod
ARG APP_SERVER_HOST=0.0.0.0
ARG APP_SERVER_PORT=8080
ARG RUST_LOG=info

ENV ENVIRONMENT=$ENVIRONMENT
ENV APP_SERVER_HOST=$APP_SERVER_HOST
ENV APP_SERVER_PORT=$APP_SERVER_PORT
ENV RUST_LOG=$RUST_LOG

COPY --from=builder /app/target/release/from-the-hart-storage .

EXPOSE 8080

CMD ["./from-the-hart-storage"]
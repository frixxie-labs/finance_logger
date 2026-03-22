FROM rust:1 AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install --yes --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/finance_logger /usr/local/bin/finance_logger

ENTRYPOINT ["/usr/local/bin/finance_logger"]

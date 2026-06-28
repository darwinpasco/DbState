# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system dbstate \
    && useradd --system --gid dbstate --home-dir /workspace --shell /usr/sbin/nologin dbstate \
    && mkdir -p /workspace \
    && chown dbstate:dbstate /workspace \
    && git config --system --add safe.directory /workspace

COPY --from=build /build/target/release/dbstate /usr/local/bin/dbstate

USER dbstate
WORKDIR /workspace

CMD ["dbstate", "--help"]

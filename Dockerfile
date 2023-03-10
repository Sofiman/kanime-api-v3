FROM rust:1.68-bullseye AS build
ARG BUILD_ID
LABEL stage=build
LABEL build=$BUILD_ID

WORKDIR /usr/src/kanime-api-v3

# prefetch dependencies
COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo fetch

# compile
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libwebp6

WORKDIR /usr/src/kanime-api-v3
COPY assets /usr/src/kanime-api-v3/assets
COPY --from=build /usr/src/kanime-api-v3/target/release/kanime-api-v3 /usr/src/kanime-api-v3/kanime-api-v3

EXPOSE 80
CMD ["./kanime-api-v3"]

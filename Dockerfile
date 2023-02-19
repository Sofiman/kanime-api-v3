FROM rust:1.65-bullseye AS build
ARG BUILD_ID
LABEL stage=build
LABEL build=$BUILD_ID

WORKDIR /usr/src/kanime-api-v3

# prefetch dependencies
RUN cargo init
COPY Cargo.toml Cargo.toml
RUN cargo fetch

# compile
COPY src src
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y libwebp6

WORKDIR /usr/src/kanime-api-v3
COPY --from=build /usr/src/kanime-api-v3/target/release/kanime-api-v3 /usr/src/kanime-api-v3/kanime-api-v3

EXPOSE 80
CMD ["./kanime-api-v3"]

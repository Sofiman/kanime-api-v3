FROM rust:1.65 AS build
ARG BUILD_ID
LABEL stage=build
LABEL build=$BUILD_ID

WORKDIR /usr/src/kanime-api-v3
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /usr/src/kanime-api-v3
COPY --from=build /usr/src/kanime-api-v3/target/release/kanime-api-v3 /usr/src/kanime-api-v3/kanime-api-v3

EXPOSE 80
CMD ["./kanime-api-v3"]
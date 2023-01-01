FROM rust:1.65 AS build

WORKDIR /usr/src/kanime-api-v3
COPY . .

RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /usr/src/kanime-api-v3/target/release/kanime-api-v3 /usr/src/kanime-api-v3/kanime-api-v3

EXPOSE 80
CMD ["./kanime-api-v3"]
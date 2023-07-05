FROM rust:1-alpine AS build
WORKDIR /build
COPY src src
COPY Cargo.toml .
COPY Cargo.lock .
RUN apk add --no-cache build-base
RUN cargo build --release --locked

FROM alpine:latest
COPY --from=build /build/target/release/rush /bin/rush
ENTRYPOINT [ "/bin/rush" ]
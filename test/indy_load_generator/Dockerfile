FROM rust:1.58-buster AS builder

RUN USER=root cargo new --bin indy_load_generator
WORKDIR /indy_load_generator
RUN apt-get update && apt-get install -y libzmq3-dev cmake 
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

# Get dependencies
RUN rm src/*.rs
COPY ./src ./src

# Build
RUN rm ./target/release/deps/indy_load_generator*
RUN cargo build --release

# Check licenses and generate report
RUN cargo install --locked cargo-about
RUN cargo about generate about.hbs > licenses.html

FROM debian:buster-slim
USER root
RUN apt-get update && apt-get install ca-certificates -y && rm -rf /var/lib/apt/lists/*

RUN mkdir /app/
COPY --from=builder /app/target/release/indy_load_generator /app/
COPY --from=builder /app/licenses.html /app/
ENV RUST_LOG=info

WORKDIR /app
RUN printf "General information about third-party software components and their licenses, \
which are distributed with this image, can be found in the the licenses.html \
file distributed with this image at /app/licenses.html." \
USER 1000
ENTRYPOINT ["/app/indy_load_generator"]
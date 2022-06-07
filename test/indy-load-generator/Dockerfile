# Rust as the base image
FROM rust:1.61 as  builder

RUN USER=root cargo new --bin indy-load-generator
WORKDIR /indy-load-generator
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

RUN rm src/*.rs
COPY ./src ./src

# 5. Build for release.
RUN rm ./target/release/deps/indy-load-generator*
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM scratch
COPY --from=builder /usr/local/cargo/bin/indy-load-generator .
USER 1000
CMD ["./indy-load-generator", "-g", "/var/lib/"]
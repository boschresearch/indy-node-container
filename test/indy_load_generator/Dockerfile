FROM rust:1.58

RUN USER=root cargo new --bin indy_load_generator
WORKDIR /indy_load_generator
RUN apt-get update && apt-get install -y libzmq3-dev cmake 
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

RUN rm src/*.rs
COPY ./src ./src

RUN rm ./target/release/deps/indy_load_generator*
RUN cargo install --path .

RUN mkdir /app/
RUN cp /usr/local/cargo/bin/indy_load_generator /app/indy_load_generator
RUN rm -rf /indy_load_generator
USER 1000
CMD ["/app/indy_load_generator"]
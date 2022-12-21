FROM rust:1.65
RUN apt install -y curl
RUN cargo install cargo-watch

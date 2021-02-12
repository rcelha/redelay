FROM rust:1.49-slim AS build
RUN apt update && apt install -y clang
WORKDIR /usr/src/myapp
COPY . .
CMD cargo build --release

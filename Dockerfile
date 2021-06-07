FROM docker.io/library/rust:1.52-slim
RUN apt update && apt install -y clang
WORKDIR /usr/src/myapp
COPY . .
CMD cargo build --release

ENV CARGO_HOME=/opt/cargo
RUN mkdir -p /opt/cargo && chmod 777 /opt/cargo

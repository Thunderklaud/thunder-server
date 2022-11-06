FROM rust:1.64.0 as builder
WORKDIR /usr/src/thunder-server
COPY . .
RUN cargo install --path .


FROM debian:bullseye-slim
ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /usr/src/thunder-server
COPY --from=builder /usr/local/cargo/bin/thunder-server /usr/local/bin/thunder-server
COPY --from=builder /usr/src/thunder-server/config /usr/src/thunder-server/config
CMD ["thunder-server"]

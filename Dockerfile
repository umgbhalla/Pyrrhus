FROM rust:1 as builder
WORKDIR /app
COPY . .
RUN cargo install --path .


FROM debian:buster-slim as runner
COPY --from=builder /usr/local/cargo/bin/rocket-app /usr/local/bin/rocket-app
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000
CMD ["rust-rocket-app"]

# Start with a rust alpine image
FROM rust:1-alpine3.15
# This is important, see https://github.com/rust-lang/docker-rust/issues/85
ENV RUSTFLAGS="-C target-feature=-crt-static"
# if needed, add additional dependencies here
RUN apk add --no-cache musl-dev
# set the workdir and copy the source into it
WORKDIR /app
COPY ./ /app
# do a release build
RUN cargo build --release
RUN strip target/release/pyrrhus

# use a plain alpine image, the alpine version needs to match the builder
FROM alpine:3.15
# if needed, install additional dependencies here
RUN apk add --no-cache libgcc
# copy the binary into the final image
COPY --from=0 /app/target/release/pyrrhus .
# set the binary as entrypoint
ENTRYPOINT ["/pyrrhus"]

# FROM rust as planner
# WORKDIR app
# RUN cargo install cargo-chef
# COPY . .
# RUN cargo chef prepare --recipe-path recipe.json
#
#
# FROM rust as cacher
# WORKDIR app
# RUN cargo install cargo-chef
# COPY --from=planner /app/recipe.json recipe.json
# RUn cargo chef cook --release --recipe-path recipe.json
#
# FROM rust as builder
# COPY . /app
# WORKDIR /app
#
# COPY --from=cacher /app/target target
# COPY --from=cacher /usr/local/cargo /usr/local/cargo
#
# RUN cargo build --release
#
#
# FROM debian:11
# # RUN apt-get update && \
# #     apt-get install --yes sha1sum base64 
# COPY --from=builder /app/target/release/pyrrhus /app/main
# WORKDIR /app
#
# CMD ["./main"]
#
#

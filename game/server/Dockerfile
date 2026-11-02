# Built from the repository root (the server shares a crate with the game):
#   docker build -f server/Dockerfile .
# Same Rust as rust-toolchain.toml (left out by .dockerignore, so rustup
# doesn't fetch another toolchain mid-build); Dependabot bumps the tag.
FROM rust:1.99.1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p carmen-server

FROM debian:bookworm-slim
# ghcr lists the image under this repository, not the one it was first pushed from.
LABEL org.opencontainers.image.source=https://github.com/jonorl/carmen-sandiego
COPY --from=build /src/target/release/carmen-server /usr/local/bin/carmen-server
# Nothing it does needs root.
RUN useradd --system --no-create-home --shell /usr/sbin/nologin carmen
USER carmen
EXPOSE 8085
CMD ["carmen-server"]

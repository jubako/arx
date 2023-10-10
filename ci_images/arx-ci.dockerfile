FROM docker.io/fedora:latest
#FROM docker.io/giansalex/rust:nightly

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain none -y

ARG CACHEBUST=1

RUN rustup toolchain install nightly --allow-downgrade --profile minimal --component clippy --component rustfmt
RUN rustup --version && \
    cargo --version && \
    rustc --version

RUN dnf install -y fuse-devel gcc

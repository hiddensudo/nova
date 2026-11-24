# syntax=docker/dockerfile:1
FROM rust:1.95.0-bookworm@sha256:6258907abe69656e41cd992e0b705cdcfabcbbe3db374f92ed2d47121282d4a1 AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    desktop-file-utils libegl1-mesa-dev libgl1-mesa-dev libwayland-dev libx11-dev \
    libxkbcommon-dev pkg-config python3 && rm -rf /var/lib/apt/lists/*
WORKDIR /source
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/source/target,sharing=locked \
    cargo build --release --locked && install -D -m 755 target/release/nova-control /out/nova-control

FROM scratch AS binary
COPY --from=build /out/nova-control /nova-control

FROM build AS package
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/source/target,sharing=locked \
    cargo test --locked && cargo test --locked --no-default-features && python3 -m unittest discover -s tests && \
    python3 scripts/package.py --binary /out/nova-control --output /artifacts

FROM scratch AS artifacts
COPY --from=package /artifacts/ /

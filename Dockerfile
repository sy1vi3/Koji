FROM --platform=$BUILDPLATFORM node:22-alpine AS client
WORKDIR /app
COPY ./client .
RUN yarn install
RUN yarn build

FROM rust:1.93-bookworm AS server
WORKDIR /usr/src/koji
COPY ./server .
RUN cargo install --path . --locked

FROM rust:1.93-bookworm AS tsp
# Build on the target platform, just like the server. Pin the solver for reproducibility.
ARG TSP_MT_REV=e8d0599699aa5d9b4820e41c64d097c7826759d8
WORKDIR /usr/src/tsp-mt
RUN git init . \
    && git remote add origin https://github.com/TurtIeSocks/tsp-mt.git \
    && git fetch --depth 1 origin "$TSP_MT_REV" \
    && git checkout --detach FETCH_HEAD
RUN cargo build --release --locked --bin tsp-mt

FROM debian:bookworm-slim AS runner
# Keep the existing plugin name so sort_by=tsp and Dragonite use tsp-mt.
COPY --from=tsp /usr/src/tsp-mt/target/release/tsp-mt /algorithms/src/routing/plugins/tsp
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

CMD ["koji"]

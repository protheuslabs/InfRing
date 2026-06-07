FROM node:22-alpine AS deps-dev
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --include=dev && npm cache clean --force

FROM node:22-alpine AS deps-prod
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force

FROM node:22-alpine AS dist-builder
WORKDIR /app
COPY --from=deps-dev /app/node_modules ./node_modules
COPY . .
RUN npm run -s runtime:dist:dashboard:build

FROM rust:1.89-alpine AS rust-builder
WORKDIR /app
RUN apk add --no-cache build-base musl-dev pkgconfig openssl-dev \
  && ln -sf /usr/bin/gcc /usr/bin/x86_64-linux-musl-gcc \
  && x86_64-linux-musl-gcc --version >/dev/null
COPY . .
RUN cargo build --release --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops --bin infringd

FROM node:22-alpine AS runtime
WORKDIR /app

COPY --from=deps-prod /app/node_modules ./node_modules
COPY . .
COPY --from=rust-builder /app/target/release/infring-ops /app/target/release/infring-ops
COPY --from=rust-builder /app/target/release/infringd /app/target/release/infringd
COPY --from=dist-builder /app/dist/client/runtime/systems/ui/infring_dashboard.js /app/dist/client/runtime/systems/ui/infring_dashboard.js
COPY --from=dist-builder /app/dist/client/runtime/systems/ui/infring_static /app/dist/client/runtime/systems/ui/infring_static

ARG INFRING_FIPS_MODE=0
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

RUN addgroup -S infring && adduser -S infring -G infring \
  && mkdir -p /app/state /app/tmp /app/logs /app/secrets \
  && chown -R infring:infring /app \
  && test "$INFRING_FIPS_MODE" = "0" -o "$INFRING_FIPS_MODE" = "1"

ENV NODE_ENV=production
ENV CLEARANCE=3
ENV TZ=UTC
ENV INFRING_FIPS_MODE=${INFRING_FIPS_MODE}
ENV INFRING_NPM_BINARY=/app/target/release/infring-ops
ENV INFRING_RUNTIME_MODE=dist
# Legacy compatibility alias for older wrappers still reading INFRING_NPM_BINARY.
ENV INFRING_NPM_BINARY=${INFRING_NPM_BINARY}

LABEL org.opencontainers.image.title="infring" \
      org.opencontainers.image.description="InfRing runtime image" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.vendor="InfRing Project" \
      org.opencontainers.image.licenses="Apache-2.0 AND LicenseRef-InfRing-NC-1.0" \
      org.opencontainers.image.base.name="node:22-alpine"

USER infring

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:4173/healthz',{cache:'no-store'}).then((r)=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

EXPOSE 4173

CMD ["node", "dist/client/runtime/systems/ui/infring_dashboard.js", "serve", "--host=0.0.0.0", "--port=4173", "--team=ops", "--refresh-ms=2000"]

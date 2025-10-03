# build docker image for arch gateway
FROM rust:1.82.0 AS builder
RUN rustup -v target add wasm32-wasip1
WORKDIR /arch
COPY crates .
RUN cargo build --release --target wasm32-wasip1 -p prompt_gateway -p llm_gateway
RUN cargo build --release -p brightstaff

FROM docker.io/envoyproxy/envoy:v1.34-latest AS envoy

FROM python:3.13.6-slim AS arch
# Purge PAM to avoid CVE-2025-6020 and install needed tools

# 1) Install what you need while apt still works
RUN set -eux; \
  apt-get update; \
  apt-get install -y --no-install-recommends supervisor gettext-base curl; \
  apt-get clean; rm -rf /var/lib/apt/lists/*

# 2) Force-remove PAM packages (don’t use apt here)
#    We ignore dependencies and remove files so scanners don’t find them.
RUN set -eux; \
  dpkg -r --force-depends libpam-modules libpam-modules-bin libpam-runtime libpam0g || true; \
  dpkg -P --force-all libpam-modules libpam-modules-bin libpam-runtime libpam0g || true; \
  rm -rf /etc/pam.d /lib/*/security /usr/lib/security || true

COPY --from=builder /arch/target/wasm32-wasip1/release/prompt_gateway.wasm /etc/envoy/proxy-wasm-plugins/prompt_gateway.wasm
COPY --from=builder /arch/target/wasm32-wasip1/release/llm_gateway.wasm /etc/envoy/proxy-wasm-plugins/llm_gateway.wasm
COPY --from=builder /arch/target/release/brightstaff /app/brightstaff
COPY --from=envoy /usr/local/bin/envoy /usr/local/bin/envoy

WORKDIR /app
COPY arch/requirements.txt .
RUN pip install -r requirements.txt
COPY arch/tools .
COPY arch/envoy.template.yaml .
COPY arch/arch_config_schema.yaml .
COPY arch/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

RUN pip install requests
RUN mkdir -p /var/log/supervisor && touch /var/log/envoy.log /var/log/supervisor/supervisord.log

RUN mkdir -p /var/log && \
    touch /var/log/access_ingress.log /var/log/access_ingress_prompt.log /var/log/access_internal.log /var/log/access_llm.log

ENTRYPOINT ["sh","-c", "/usr/bin/supervisord"]

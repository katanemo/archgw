# build docker image for arch gateway
FROM rust:1.82.0 AS builder
RUN rustup -v target add wasm32-wasip1
WORKDIR /arch
COPY crates .

RUN cargo build --release --target wasm32-wasip1 -p prompt_gateway -p llm_gateway
RUN cargo build --release -p brightstaff

# copy built filter into envoy image
FROM docker.io/envoyproxy/envoy:v1.32-latest AS envoy

#Build config generator, so that we have a single build image for both Rust and Python
FROM python:3.12-slim AS arch

RUN apt-get update && apt-get install -y supervisor gettext-base curl && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /arch/target/wasm32-wasip1/release/prompt_gateway.wasm /etc/envoy/proxy-wasm-plugins/prompt_gateway.wasm
COPY --from=builder /arch/target/wasm32-wasip1/release/llm_gateway.wasm /etc/envoy/proxy-wasm-plugins/llm_gateway.wasm
COPY --from=builder /arch/target/release/brightstaff /app/brightstaff
COPY --from=envoy /usr/local/bin/envoy /usr/local/bin/envoy

WORKDIR /app
COPY arch/requirements.txt .
RUN pip install -r requirements.txt
COPY arch/tools/cli/config_generator.py .
COPY arch/envoy.template.yaml .
COPY arch/arch_config_schema.yaml .
COPY arch/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

RUN pip install requests
RUN touch /var/log/envoy.log
RUN mkdir -p /var/log/supervisor/
RUN touch /var/log/supervisor/supervisord.log

ENTRYPOINT ["sh","-c", "/usr/bin/supervisord"]

# ENTRYPOINT ["sh","-c", "python config_generator.py && envsubst < /etc/envoy/envoy.yaml > /etc/envoy.env_sub.yaml && envoy -c /etc/envoy.env_sub.yaml --log-level trace 2>&1 | tee /var/log/envoy.log"]
# ENTRYPOINT ["sh","-c", "python config_generator.py && envsubst < /etc/envoy/envoy.yaml > /etc/envoy.env_sub.yaml && envoy -c /etc/envoy.env_sub.yaml --component-log-level wasm:info 2>&1 | tee /var/log/envoy.log"]

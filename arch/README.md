# Envoy filter code for gateway

## Add toolchain

```sh
$ rustup target add wasm32-wasip1
```

## Container Runtime Support

```sh
# Build the gateway WASM modules (must be run from the crates directory)
$ cd ../crates
$ cargo build --target wasm32-wasip1 --release -p llm_gateway
$ cargo build --target wasm32-wasip1 --release -p prompt_gateway
```

Or build both at once:
```sh
$ cd ../crates
$ cargo build --target wasm32-wasip1 --release -p llm_gateway -p prompt_gateway
```

## Testing
```sh
$ cargo test
```

## Local development
- Build docker image for arch gateway. Note this needs to be built once.
  ```
  $ sh build_filter_image.sh
  ```

- Build filter binary,
  ```
  $ cd ../crates
  $ cargo build --target wasm32-wasip1 --release -p llm_gateway -p prompt_gateway
  ```
- Start envoy with arch_config.yaml and test,
  ```
  $ docker compose -f docker-compose.dev.yaml up archgw
  ```
- dev version of docker-compose file uses following files that are mounted inside the container. That means no docker rebuild is needed if any of these files change. Just restart the container and change will be picked up,
  - envoy.template.yaml
  - llm_gateway.wasm
  - prompt_gateway.wasm
  - logs/ directory (for container logs)

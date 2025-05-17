#!/bin/bash
set -eu

echo "docker images"
docker images

# for demo in currency_exchange hr_agent
for demo in samples_python/currency_exchange use_cases/preference_based_routing
do
  echo "******************************************"
  echo "Running tests for $demo ..."
  echo "****************************************"
  cd ../../$demo
  echo "starting archgw"
  archgw up arch_config.yaml
  echo "starting docker containers"
  docker compose up -d 2>&1 > /dev/null
  echo "starting hurl tests"
  hurl --test hurl_tests/*.hurl
  echo "stopping docker containers and archgw"
  archgw down
  docker compose down -v
  cd ../../shared/test_runner
done

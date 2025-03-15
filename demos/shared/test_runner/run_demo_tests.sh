#!/bin/bash
set -eu

# for demo in currency_exchange hr_agent
for demo in currency_exchange weather_forecast
do
  echo "******************************************"
  echo "Running tests for $demo ..."
  echo "****************************************"
  cd ../../samples_python/$demo
  archgw up arch_config.yaml
  docker compose up -d
  hurl --test hurl_tests
  archgw down
  docker compose down -v
  cd ../../shared/test_runner
done

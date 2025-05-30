#!/bin/bash
set -eu

# load demo name from arguments
if [ $# -eq 0 ]; then
  echo "No demo names provided. Please provide demo names as arguments."
  # print usage
  echo "Usage: $0 <demo_name1> <demo_name2> ..."
  exit 1
fi

# extract demo names from arguments
DEMOS="$@"

echo "Running tests for demos: $DEMOS"

for demo in $DEMOS
do
  echo "******************************************"
  echo "Running tests for $demo ..."
  echo "****************************************"
  cd ../../$demo
  echo "starting archgw"
  archgw up arch_config.yaml
  echo "starting docker containers"
  # only execute docker compose if demo is use_cases/preference_based_routing
  if [ "$demo" == "use_cases/preference_based_routing" ]; then
    echo "starting docker compose for $demo"
    docker compose -f docker-compose.yaml up -d 2>&1 > /dev/null
  else
    echo "skipping docker compose for $demo"
  fi
  echo "starting hurl tests"
  if ! hurl hurl_tests/*.hurl; then
    echo "Hurl tests failed for $demo"
    echo "docker logs for archgw:"
    docker logs archgw | tail -n 100
    exit 1
  fi
  echo "stopping docker containers and archgw"
  archgw down
  docker compose down -v
  cd ../../shared/test_runner
done

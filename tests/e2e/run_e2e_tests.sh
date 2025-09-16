#/bin/bash
# if any of the commands fail, the script will exit
set -e

. ./common_scripts.sh

print_disk_usage

mkdir -p ~/archgw_logs
touch ~/archgw_logs/modelserver.log

print_debug() {
  log "Received signal to stop"
  log "Printing debug logs for model_server"
  log "===================================="
  tail -n 100 ~/archgw_logs/modelserver.log
  log "Printing debug logs for docker"
  log "===================================="
  tail -n 100 ../build.log
  archgw logs --debug | tail -n 100
}

trap 'print_debug' INT TERM ERR

log starting > ../build.log

log building and running function_calling demo
log ===========================================
cd ../../demos/samples_python/weather_forecast/
docker compose up weather_forecast_service --build -d
cd -

log building and install model server
log =================================
cd ../../model_server
poetry install
cd -

log building and installing archgw cli
log ==================================
cd ../../arch/tools
poetry install
cd -

log building docker image for arch gateway
log ======================================
cd ../../
archgw build
cd -

# Once we build archgw we have to install the dependencies again to a new virtual environment.
poetry install

log startup arch gateway with function calling demo
cd ../../
tail -F ~/archgw_logs/modelserver.log &
model_server_tail_pid=$!
archgw down
archgw up demos/samples_python/weather_forecast/arch_config.yaml
kill $model_server_tail_pid
cd -

log running e2e tests for prompt gateway
log ====================================
poetry run pytest test_prompt_gateway.py

log shutting down the arch gateway service for prompt_gateway demo
log ===============================================================
archgw down

log startup arch gateway with model alias routing demo
cd ../../
archgw up demos/use_cases/model_alias_routing/arch_config_with_aliases.yaml
cd -

log running e2e tests for model alias routing
log ========================================
poetry run pytest test_model_alias_routing.py

log shutting down the weather_forecast demo
log =======================================
cd ../../demos/samples_python/weather_forecast
docker compose down
cd -

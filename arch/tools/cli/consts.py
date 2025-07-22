import os


KATANEMO_DOCKERHUB_REPO = "katanemo/archgw"
KATANEMO_LOCAL_MODEL_LIST = [
    "katanemo/Arch-Guard",
]
SERVICE_NAME_ARCHGW = "archgw"
SERVICE_NAME_MODEL_SERVER = "model_server"
SERVICE_ALL = "all"
MODEL_SERVER_LOG_FILE = "~/archgw_logs/modelserver.log"
ARCHGW_DOCKER_NAME = "archgw"
ARCHGW_DOCKER_IMAGE = os.getenv("ARCHGW_DOCKER_IMAGE", "katanemo/archgw:0.3.6")

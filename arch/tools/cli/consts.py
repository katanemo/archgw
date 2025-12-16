import os

SERVICE_NAME_ARCHGW = "archgw"
ARCHGW_DOCKER_NAME = "archgw"
ARCHGW_DOCKER_IMAGE = os.getenv("ARCHGW_DOCKER_IMAGE", "katanemo/archgw:0.3.22")

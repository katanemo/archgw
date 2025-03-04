import os
from typing import List, Optional

from fastapi import FastAPI, HTTPException
from openai import OpenAI
from pydantic import BaseModel, Field

app = FastAPI()
DEMO_DESCRIPTION = """This demo illustrates how **Arch** can be used to perform function calling
 with network-related tasks. In this demo, you act as a **network assistant** that provides factual
 information, without offering advice on manufacturers or purchasing decisions."""


# Define the request model
class DeviceSummaryRequest(BaseModel):
    device_id: str
    time_range: Optional[int] = Field(
        default=7, description="Time range in days, defaults to 7"
    )


# Define the response model
class DeviceStatistics(BaseModel):
    device_id: str
    time_range: str
    data: str


class DeviceSummaryResponse(BaseModel):
    statistics: List[DeviceStatistics]

    # Request model for device reboot


class DeviceRebootRequest(BaseModel):
    device_id: str


# Response model for the device reboot
class CoverageResponse(BaseModel):
    status: str
    summary: dict


@app.post("/agent/device_reboot", response_model=CoverageResponse)
def reboot_network_device(request_data: DeviceRebootRequest):
    """
    Endpoint to reboot network devices based on device IDs and an optional time range.
    """

    # Access data from the Pydantic model
    device_id = request_data.device_id

    # Validate 'device_id'
    # (This is already validated by Pydantic, but additional logic can be added if needed)
    if not device_id:
        raise HTTPException(status_code=400, detail="'device_id' parameter is required")

    # Simulate reboot operation and return the response
    statistics = []
    # Placeholder for actual data retrieval or device reboot logic
    stats = {"data": f"Device {device_id} has been successfully rebooted."}
    statistics.append(stats)

    # Return the response with a summary
    return CoverageResponse(status="success", summary={"device_id": device_id})


# Post method for device summary
@app.post("/agent/device_summary", response_model=DeviceSummaryResponse)
def get_device_summary(request: DeviceSummaryRequest):
    """
    Endpoint to retrieve device statistics based on device IDs and an optional time range.
    """

    # Extract 'device_id' and 'time_range' from the request
    device_id = request.device_id
    time_range = request.time_range

    # Simulate retrieving statistics for the given device IDs and time range
    statistics = []
    minutes = 4
    stats = {
        "device_id": device_id,
        "time_range": f"Last {time_range} days",
        "data": f"""Device {device_id} over the last {time_range} days experienced {minutes}
        minutes of downtime.""",
    }

    statistics.append(DeviceStatistics(**stats))

    return DeviceSummaryResponse(statistics=statistics)

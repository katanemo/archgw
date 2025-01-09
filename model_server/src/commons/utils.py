import torch
import logging

from datetime import datetime


def get_model_server_logger():
    """
    Get or initialize the logger instance for the model server.

    Returns:
    - logging.Logger: Configured logger instance.
    """

    # Check if the logger is already configured
    logger = logging.getLogger("model_server")

    # Return existing logger instance if already configured
    if logger.hasHandlers():
        return logger

    # Configure logging to only log to console
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        handlers=[logging.StreamHandler()],
    )

    return logger


def get_device():
    if torch.cuda.is_available():
        device = "cuda"
    elif torch.backends.mps.is_available():
        device = "mps"
    else:
        device = "cpu"

    return device


def get_today_date():
    # Get today's date
    today = datetime.now()

    # Get full date with day of week
    full_date = today.strftime("%Y-%m-%d")

    return full_date

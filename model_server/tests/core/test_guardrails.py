from unittest.mock import patch, MagicMock
from src.core.guardrails import get_guardrail_handler


# Test for `get_guardrail_handler()` function on `cuda`
@patch("src.core.guardrails.AutoTokenizer.from_pretrained")
@patch("src.core.guardrails.AutoModelForSequenceClassification.from_pretrained")
def test_guardrail_handler_on_cuda(mock_auto_model, mock_tokenizer):
    device = "cuda"

    mock_auto_model.return_value = MagicMock()
    mock_tokenizer.return_value = MagicMock()

    guardrail = get_guardrail_handler(device=device)

    mock_tokenizer.assert_called_once_with(guardrail.model_name, trust_remote_code=True)

    mock_auto_model.assert_called_once_with(
        guardrail.model_name,
        device_map=device,
        low_cpu_mem_usage=True,
    )


# Test for `get_guardrail_handler()` function on `mps`
@patch("src.core.guardrails.AutoTokenizer.from_pretrained")
@patch("src.core.guardrails.AutoModelForSequenceClassification.from_pretrained")
def test_guardrail_handler_on_mps(mock_auto_model, mock_tokenizer):
    device = "mps"

    mock_auto_model.return_value = MagicMock()
    mock_tokenizer.return_value = MagicMock()

    guardrail = get_guardrail_handler(device=device)

    mock_tokenizer.assert_called_once_with(guardrail.model_name, trust_remote_code=True)

    mock_auto_model.assert_called_once_with(
        guardrail.model_name,
        device_map=device,
        low_cpu_mem_usage=True,
    )

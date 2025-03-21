import asyncio
import aiohttp
import time

API_URL = "http://localhost:12000/v1/chat/completions"


async def fetch_response(
    session: aiohttp.ClientSession, prompt: str
) -> aiohttp.ClientResponse:
    headers = {
        "Content-Type": "application/json",
    }
    payload = {"messages": [{"role": "user", "content": prompt}]}

    start_time = time.monotonic()
    async with session.post(API_URL, json=payload, headers=headers) as response:
        result = await response.json()
        end_time = time.monotonic()
        elapsed_time = end_time - start_time
        return prompt, result, elapsed_time


async def main():
    prompts = [
        "Hello!",
        "Tell me a joke.",
        "Who was the president of the United States in the 1990?",
    ]

    async with aiohttp.ClientSession() as session:
        tasks = [fetch_response(session, prompt) for prompt in prompts]

        for completed in asyncio.as_completed(tasks):
            prompt, result, elapsed_time = await completed
            print("user prompt: ", prompt)
            resp = result.get("choices")[0].get("message", {}).get("content", {})
            print("assistant response: ", resp)
            print(
                f"logs: request time: {elapsed_time:.3f}s, model name: {result.get('model', '')}"
            )
            print()
        for task in tasks:
            task.close()


asyncio.run(main())

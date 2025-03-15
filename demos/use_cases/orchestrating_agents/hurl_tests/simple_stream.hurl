POST http://localhost:10000/v1/chat/completions
Content-Type: application/json

{
  "messages": [
    {
      "role": "user",
      "content": "I want to sell red shoes"
    }
  ],
  "stream": true
}
HTTP 200
[Asserts]
header "content-type" matches /text\/event-stream/
body matches /^data: .*?sales_agent.*?\n/

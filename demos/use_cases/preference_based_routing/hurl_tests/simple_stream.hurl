POST http://localhost:12000/v1/chat/completions
Content-Type: application/json

{
  "messages": [
    {
      "role": "user",
      "content": "I am running under debt, how should I keep a tab on my expenses?"
    }
  ],
  "model": "none",
  "stream": true
}
HTTP 200
[Asserts]
header "content-type" matches /text\/event-stream/
body matches /^data: .*?gpt-4o.*?\n/

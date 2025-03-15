POST http://localhost:10000/v1/chat/completions
Content-Type: application/json

{
  "messages": [
    {
      "role": "user",
      "content": "I want to sell red shoes"
    }
  ]
}
HTTP 200
[Asserts]
header "content-type" == "application/json"
jsonpath "$.model" matches /^gpt-4o-mini/
jsonpath "$.metadata.x-arch-state" != null
jsonpath "$.usage" != null
jsonpath "$.choices[0].message.content" != null
jsonpath "$.choices[0].message.role" == "assistant"

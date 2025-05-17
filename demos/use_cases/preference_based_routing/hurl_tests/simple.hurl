POST http://localhost:12000/v1/chat/completions
Content-Type: application/json

{
  "messages": [
    {
      "role": "user",
      "content": "I am running under debt, how should I keep a tab on my expenses?"
    }
  ]
}
HTTP 200
[Asserts]
header "content-type" == "application/json"
jsonpath "$.model" matches /^o4-mini/
jsonpath "$.usage" != null
jsonpath "$.choices[0].message.content" != null
jsonpath "$.choices[0].message.role" == "assistant"

# `lm-proxy` - (large) language model proxy

A proxy for (large) language models that forwards to external servers. It manages external servers
and spins them up and down on demand.

Config:

```toml
[proxy]
port = 8080

# without running requests, keep models alive for 60s
keep_alive = 60

# with a running request, keep models alive for 300s
request_keep_alive = 300

[models.phi3]
args = [
    "llama-server",
    "--model",
    "phi-3-mini-4k-instruct-q4.gguf",
    "--port",
    "{{ port }}",
]

[models.gemma2]
args = [
    "llama-server",
    "--model",
    "gemma-2-9b-it-q5_k_m.gguf",
    "--port",
    "{{ port }}",
]
```

Start the server:

```bash
lm-proxy serve config.toml
```

Use the server:

```python
from openai import OpenAI

client = OpenAI(
    base_url = 'http://localhost:8080/v1',
    api_key='unused',
)

# use the phi3 model
response = client.chat.completions.create(
  model="phi3",
  messages=[{"role": "user", "content": "What is 2 + 3?"}]
)
print(response.choices[0].message.content)

# use the gemma2 model
response = client.chat.completions.create(
  model="gemma2",
  messages=[{"role": "user", "content": "How can I add 2 and 3 in Python?"}]
)
print(response.choices[0].message.content)
```

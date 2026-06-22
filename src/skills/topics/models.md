# Models

The wiki supports 8 NVIDIA NIM models. List them with `wiki models`.

Embedders:
- `nvidia/nv-embed-v1` (default, 4096 dims, non-commercial, MTEB #1)
- `nvidia/nv-embedqa-e5-v5` (1024 dims, commercial, fast QA)
- `nvidia/nv-embedcode-7b-v1` (4096 dims, commercial, code-specialized)
- `nvidia/llama-nemotron-embed-1b-v2` (2048 dims, Matryoshka, commercial, multilingual)
- `nvidia/llama-nemotron-embed-vl-1b-v2` (2048 dims, multimodal, commercial)

Rerankers:
- `nvidia/llama-nemotron-rerank-1b-v2` (1B, commercial, multilingual)
- `nvidia/llama-nemotron-rerank-vl-1b-v2` (multimodal)
- `nvidia/nv-rerankqa-mistral-4b-v3` (4B, commercial, high accuracy)

Select a model via config:

```yaml
nim:
  embed_model: "nvidia/llama-nemotron-embed-1b-v2"
```

Or per-command:

```bash
wiki embed --model "nvidia/llama-nemotron-embed-1b-v2"
wiki search "..." --model "nvidia/llama-nemotron-embed-1b-v2"
```

The Matryoshka property on `llama-nemotron-embed-1b-v2` allows dimension slicing (384/512/768/1024/2048) without re-embedding. Set `embed_dim_override` in config.

Note: `nvidia/nv-embed-v1` is licensed non-commercial only. For commercial use, switch to a commercial-licensed model.
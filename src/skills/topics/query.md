# Query

Use `wiki query` for synthesized RAG-style answers with citations.

```bash
wiki query "what sparse attention mechanisms have I written about?"
wiki query "..." --top-k 10
wiki query "..." --llm-model "meta/llama-3.1-70b-instruct"
wiki query "..." --json
```

Workflow:
1. Builds prompt from top-K chunks (default 5)
2. Calls NIM chat-completions endpoint
3. Returns answer with `[1] [2] [3]` style citations mapped to source paths

For best results:
- Be specific in the question
- Use `--top-k 10` if the question is broad
- If the answer references a fact, click the citation to verify against the source

If a query produces a substantive comparison, decision, or reusable answer, file it as a new wiki page in `wiki/`.
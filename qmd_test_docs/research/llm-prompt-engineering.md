# Advanced LLM Prompt Engineering Techniques

## Fundamental Principles

### Clarity and Specificity
Vague prompts yield vague results. Be explicit about:
- Expected output format
- Tone and style
- Constraints and limitations
- Edge cases to handle

### Few-Shot Learning
Provide examples to guide the model:

```
Classify sentiment (positive/negative/neutral):

Text: "The product exceeded my expectations!"
Sentiment: positive

Text: "Shipping took longer than expected."
Sentiment: negative

Text: "The item arrived as described."
Sentiment: neutral

Text: "Customer service was incredibly helpful."
Sentiment:
```

## Advanced Techniques

### Chain-of-Thought (CoT) Prompting

Encourage step-by-step reasoning:

```
Q: Roger has 5 tennis balls. He buys 2 more cans of tennis balls.
Each can has 3 tennis balls. How many tennis balls does he have now?

A: Let's think step by step:
1. Roger starts with 5 tennis balls
2. He buys 2 cans, each with 3 balls: 2 × 3 = 6 balls
3. Total: 5 + 6 = 11 tennis balls

Answer: 11 tennis balls
```

### ReAct (Reasoning + Acting)

Combine reasoning with tool use:

```
Question: What is the population of the capital of France?

Thought: I need to find the capital of France first, then look up its population.
Action: search("capital of France")
Observation: Paris is the capital of France.

Thought: Now I need to find the population of Paris.
Action: search("population of Paris")
Observation: Paris has approximately 2.2 million inhabitants.

Answer: The capital of France is Paris, with a population of approximately 2.2 million.
```

### Tree of Thoughts (ToT)

Explore multiple reasoning paths:

```
Problem: Design an efficient algorithm to find duplicates in a large file.

Path 1: Hash-based approach
- Pros: O(n) time, simple
- Cons: High memory usage
- Evaluation: Good for small to medium files

Path 2: Sort-based approach
- Pros: O(n log n), less memory
- Cons: Modifies order
- Evaluation: Better for large files

Path 3: External merge sort
- Pros: Handles files larger than RAM
- Cons: Complex implementation
- Evaluation: Best for massive files

Recommendation: Use hash-based for <1GB, external sort for >1GB
```

### Self-Consistency

Generate multiple solutions and vote:

```
Solve this math problem using different approaches:

Approach 1 (Algebra): ...
Approach 2 (Graphical): ...
Approach 3 (Logical): ...

Final answer (most common): ...
```

## Domain-Specific Patterns

### Code Generation

```
Task: Write a Python function to validate email addresses.

Requirements:
- Must handle standard formats (user@domain.com)
- Reject invalid characters
- Support subdomains (user@mail.domain.com)
- Return True/False

Include:
- Type hints
- Docstring
- Unit tests
```

### Data Extraction

```
Extract structured data from this text:

Text: "John Doe, born 1985-03-15, works at Acme Corp as Senior Engineer since 2020."

Format:
{
  "name": "...",
  "birth_date": "YYYY-MM-DD",
  "employer": "...",
  "role": "...",
  "start_year": YYYY
}
```

## Anti-Patterns to Avoid

1. **Ambiguous instructions** - "Make it better" vs "Improve code readability by adding comments"
2. **Ignoring constraints** - Not specifying output length, format, or style
3. **Overloading context** - Cramming too much information
4. **No validation criteria** - Not defining success metrics

## Optimization Strategies

### Temperature Tuning
- **0.0-0.3** - Deterministic, factual tasks
- **0.4-0.7** - Balanced creativity
- **0.8-1.0** - High creativity, brainstorming

### System Prompts

```
You are an expert Python developer specializing in data engineering.
You write clean, efficient code following PEP 8 standards.
Always include type hints and comprehensive docstrings.
Prioritize readability over cleverness.
```

### Structured Outputs

```
Respond in JSON format:
{
  "summary": "...",
  "key_points": ["...", "..."],
  "confidence": 0.0-1.0,
  "sources": ["..."]
}
```

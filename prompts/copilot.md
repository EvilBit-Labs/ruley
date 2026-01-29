Convert the following codebase analysis into GitHub Copilot instructions.

Analysis: <analysis> {{analysis}} </analysis>

Format Requirements:

1. Use Markdown format suitable for .github/copilot-instructions.md
2. Focus on patterns that guide code completion
3. Be concise but specific

Structure the output as follows:

# Copilot Instructions

## Project Context

[Brief project description to give Copilot context]

## Code Style

### Language: {{primary_language}}

[Language-specific conventions]

### Formatting

[Indentation, line length, bracket placement, etc.]

### Naming Conventions

- Files: [pattern]
- Functions: [pattern]
- Variables: [pattern]
- Types/Classes: [pattern]
- Constants: [pattern]

## Patterns to Follow

### [Pattern Category]

```{{primary_language}}
// Example of correct pattern
```

## Patterns to Avoid

### [Anti-pattern Name]

```{{primary_language}}
// DON'T: Example of what to avoid
```

Instead:

```{{primary_language}}
// DO: Correct approach
```

## Testing Conventions

[Testing patterns and frameworks]

## Error Handling

[Error handling conventions]

## Import/Dependency Order

[How imports should be organized]

## Comments and Documentation

[Documentation standards]

Output the complete copilot-instructions.md file content, ready to save to .github/copilot-instructions.md

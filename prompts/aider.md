Convert the following codebase analysis into Aider conventions file format.

Analysis: <analysis> {{analysis}} </analysis>

Format Requirements:

1. Use Markdown format suitable for .aider.conf.yml or CONVENTIONS.md
2. Focus on patterns that guide code generation
3. Be concise but specific

Structure the output as follows:

# Aider Conventions

## Project Context

[Brief project description to give Aider context]

## Language: {{primary_language}}

[Language-specific conventions and idioms]

## Code Style

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

Output the complete conventions file content ready for Aider configuration.

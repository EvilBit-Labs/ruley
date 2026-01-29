Convert the following codebase analysis into Cursor IDE rules in .mdc format.

Analysis: <analysis> {{analysis}} </analysis>

Format Requirements:

1. Use Markdown with YAML frontmatter
2. Frontmatter must include:
   - description: Brief summary of what this rule covers
   - globs: File patterns this rule applies to (e.g., "**/\*.rs", "src/**/\*.ts")
   - alwaysApply: Set to {{always_apply}} based on rule type
3. Rule type: {{rule_type}}

Structure the output as follows:

```yaml
---
description: [Concise description]
globs: [Relevant file patterns]
alwaysApply: [true/false]
---
```

# Project Rules

## Overview

[Brief project description and purpose]

## Conventions

[Specific coding conventions with actionable directives]

## Patterns

[Design patterns used in the codebase]

## Examples

### Valid Examples

```[language]
// Example of correct pattern
```

### Invalid Examples

```[language]
// Example of what to avoid
```

## Key Files

[Important files developers should know about]

Be specific and actionable. Rules should guide AI assistants to provide accurate, project-consistent suggestions.

Output the complete .mdc file content ready to save to .cursor/rules/project.mdc

# Epic Brief: ruley v1.0.0 - Automated AI IDE Rules Generation

## Summary

ruley is a Rust CLI tool that automatically generates AI IDE rule files from codebases, helping open-source maintainers provide better guidance to contributors using AI coding assistants. Instead of manually writing and maintaining separate rule files for Cursor, Claude, Copilot, and other AI IDEs, maintainers can run ruley to analyze their codebase and generate comprehensive, convention-aware rules in minutes. The tool uses LLMs (Anthropic Claude, OpenAI GPT) to understand project patterns, conventions, and architecture, then outputs properly formatted rules for multiple AI IDEs from a single analysis. This dramatically reduces the time and effort required to help AI assistants provide accurate, project-specific suggestions to contributors.

## Context & Problem

**Who's Affected**:

- **Individual developers** adopting AI coding assistants for personal or professional projects
- **Small development teams** integrating AI tools into their workflow
- **Open-source maintainers** wanting to help contributors get better AI assistance
- **Developers new to AI/prompt engineering** who don't know how to write effective rules
- **Experienced developers** who don't want to spend time manually tuning rules files

This applies to both **greenfield projects** (starting fresh with AI assistance) and **brownfield projects** (adding AI assistance to existing codebases).

**Current Pain Points**:

1. **Generic AI Suggestions**: AI assistants provide generic code suggestions that don't match project-specific conventions, patterns, or architecture. Developers receive suggestions that violate style guides, use deprecated patterns, or ignore established conventions—especially problematic for those new to AI who don't know how to fix this.
2. **Manual Rule Writing is Complex**: Writing effective AI rules requires understanding both the codebase and prompt engineering. Developers must manually create and maintain separate rule files for each AI IDE (`.cursor/rules/*.mdc`, `CLAUDE.md`, `.github/copilot-instructions.md`). This is time-consuming, requires expertise many don't have, and quickly becomes outdated as codebases evolve.
3. **Barrier to AI Adoption**: Teams and individuals want to use AI coding assistants but are overwhelmed by the setup complexity. They either skip rules entirely (getting poor results) or spend hours learning prompt engineering and writing rules manually—a significant barrier to adoption, especially for those new to AI tools.

**Current Workarounds**:

- Manually write rules files for each AI IDE (hours of work, requires expertise)
- Copy-paste documentation into AI chat repeatedly (inefficient, doesn't scale)
- Accept generic AI suggestions and spend time correcting them (frustrating, slows development)
- Skip AI IDE features entirely (missed productivity opportunity)
- Hire consultants or spend time learning prompt engineering (expensive, time-consuming)

**Why This Matters**:

Developers and teams want to leverage AI coding assistants effectively without becoming prompt engineering experts. When AI assistants understand project conventions:

- **Individual developers** get accurate, context-aware suggestions without manual tuning
- **Small teams** can standardize AI assistance across team members
- **Open-source maintainers** lower the barrier to contribution
- **Newcomers to AI** can get great results without learning prompt engineering
- Everyone spends less time correcting AI suggestions and more time building features

**The Opportunity**:

ruley solves this by automating rule generation:

- **Speed**: Generate rules in minutes vs hours of manual work
- **Comprehensiveness**: Analyzes entire codebase, catches patterns humans miss
- **Multi-format**: One analysis produces rules for Cursor, Claude, and Copilot
- **Maintainability**: Easy to regenerate when conventions change or patterns emerge

**Key Constraint**: LLM costs must be managed carefully. The tool includes cost estimation, user confirmation, and tree-sitter compression (~70% token reduction) to keep costs reasonable for regular use.

## Success Criteria

We'll know ruley is successful when:

1. **Time Savings**: Developers save significant time vs manual rule writing (minutes vs hours)
2. **Adoption**: Teams adopt ruley as standard practice for new projects
3. **Quality**: Generated rules are comprehensive and accurate enough to use as-is
4. **Effectiveness**: AI suggestions match project conventions without manual correction

## Out of Scope (v1.0.0)

- Native git clone support (users handle repo checkout)
- Additional output formats beyond Cursor, Claude, Copilot
- Additional LLM providers beyond Anthropic and OpenAI
- Semantic search or caching mechanisms
- GUI or IDE plugins

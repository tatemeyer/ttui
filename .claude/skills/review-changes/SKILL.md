---
name: review-changes
description: Perform a structured code review using change detection and impact
---

## Review Changes

Perform a thorough, risk-aware code review using the knowledge graph.

### Steps

1. Run `detect_changes_tool` to get risk-scored change analysis.
2. Run `get_affected_flows_tool` to find impacted execution paths.
3. For each high-risk function, run `query_graph_tool` with pattern="tests_for" to check test coverage.
4. Run `get_impact_radius_tool` to understand the blast radius.
5. For any untested changes, suggest specific test cases.

### Output Format

Provide findings grouped by risk level (high/medium/low) with:
- What changed and why it matters
- Test coverage status
- Suggested improvements
- Overall merge recommendation

## Token Efficiency Rules
- ALWAYS start with `get_minimal_context_tool(task="<your task>")` before any other graph tool.
- Use `detail_level="minimal"` on the tools that accept it — `query_graph_tool`,
  `semantic_search_nodes_tool`, `list_communities_tool`, `get_architecture_overview_tool`,
  `list_flows_tool`, `detect_changes_tool`, `get_impact_radius_tool`,
  `get_review_context_tool`. Only escalate to "standard" when minimal is insufficient.
  Passing `detail_level` to a tool that does not accept it (e.g. `list_graph_stats_tool`,
  `find_large_functions_tool`, `get_flow_tool`, `get_minimal_context_tool`) is a hard
  validation error, not a silently-ignored argument.
- Target: complete any review/debug/refactor task in ≤5 tool calls and ≤800 total output tokens.

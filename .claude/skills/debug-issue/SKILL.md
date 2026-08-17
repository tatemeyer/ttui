---
name: debug-issue
description: Systematically debug issues using graph-powered code navigation
---

## Debug Issue

Use the knowledge graph to systematically trace and debug issues.

### Steps

1. Use `semantic_search_nodes_tool` to find code related to the issue.
2. Use `query_graph_tool` with `callers_of` and `callees_of` to trace call chains.
3. Use `get_flow_tool` to see full execution paths through suspected areas.
4. Run `detect_changes_tool` to check if recent changes caused the issue.
5. Use `get_impact_radius_tool` on suspected files to see what else is affected.

### Tips

- Check both callers and callees to understand the full context.
- Look at affected flows to find the entry point that triggers the bug.
- Recent changes are the most common source of new issues.

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

---
name: explore-codebase
description: Navigate and understand codebase structure using the knowledge graph
---

## Explore Codebase

Use the code-review-graph MCP tools to explore and understand the codebase.

### Steps

1. Run `list_graph_stats_tool` to see overall codebase metrics.
2. Run `get_architecture_overview_tool` for high-level community structure.
3. Use `list_communities_tool` to find major modules, then `get_community_tool` for details.
4. Use `semantic_search_nodes_tool` to find specific functions or classes.
5. Use `query_graph_tool` with `pattern=` values like `callers_of`, `callees_of`, `imports_of` to trace relationships.
6. Use `list_flows_tool` and `get_flow_tool` to understand execution paths.

### Tips

- Start broad (stats, architecture) then narrow down to specific areas.
- Use `pattern="children_of"` on a file to see all its functions and classes.
- Use `find_large_functions_tool` to identify complex code.
- `query_graph_tool` returns `status: "ambiguous"` with a candidate list when a
  bare name matches several nodes — re-run with the `qualified_name` it gives back.

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

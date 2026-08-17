---
name: refactor-safely
description: Plan and execute safe refactoring using dependency analysis
---

## Refactor Safely

Use the knowledge graph to plan and execute refactoring with confidence.

### Steps

1. Use `refactor_tool` with mode="suggest" for community-driven refactoring suggestions.
2. Use `refactor_tool` with mode="dead_code" to find unreferenced code.
3. For renames, use `refactor_tool` with mode="rename" to preview all affected locations.
4. Use `apply_refactor_tool` with the refactor_id to apply renames.
5. After changes, run `detect_changes_tool` to verify the refactoring impact.

### Safety Checks

- Always preview before applying (rename mode gives you an edit list).
- Check `get_impact_radius_tool` before major refactors.
- Use `get_affected_flows_tool` to ensure no critical paths are broken.
- Run `find_large_functions_tool` to identify decomposition targets.

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

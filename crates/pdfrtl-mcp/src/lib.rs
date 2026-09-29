//! pdfrtl MCP server (P2) — exposes the CLI contract to agents as MCP tools.
//!
//! Design rule: this crate is a *thin adapter*. It must never re-implement logic
//! that lives in `pdfrtl-core`; it maps MCP tool calls onto the same functions the
//! CLI calls, so the JSON contract and the MCP surface cannot drift apart.
//!
//! P0 status: placeholder so the workspace is complete. The transport decision
//! (Rust MCP SDK vs. thin TypeScript server over the CLI) is open question Q-001
//! in docs/OPEN-QUESTIONS.md and is resolved in Phase 2 (task 2.6).

/// Name of the MCP server as advertised to clients.
pub const SERVER_NAME: &str = "pdfrtl";

/// Tools this server will expose. Kept as data so the CLI, docs and MCP surface
/// are generated from one list instead of being hand-maintained in three places.
pub const PLANNED_TOOLS: &[&str] = &["inspect", "extract", "search", "generate", "edit", "sign"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_tools_are_the_contract_verbs() {
        assert_eq!(SERVER_NAME, "pdfrtl");
        assert_eq!(PLANNED_TOOLS[0], "inspect");
        assert!(PLANNED_TOOLS.contains(&"extract"));
        assert_eq!(PLANNED_TOOLS.len(), 6);
    }
}

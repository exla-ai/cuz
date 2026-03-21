use anyhow::Result;

pub fn run() -> Result<()> {
    crate::mcp::serve()
}

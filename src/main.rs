// src/main.rs

use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::memory::Memory;
use agentic_runtime::model::TaskModel;
use agentic_runtime::tools::{FakeEchoTool, GitStatusTool, LLMTool, ReflectorTool};
use colored::Colorize;

fn main() {
    let model = TaskModel::new("Check Git status");

    // Step 1: create initial context and register tools that don't need context
    let context = Context::new()
        .register_tool(FakeEchoTool)
        .register_tool(GitStatusTool)
        .register_tool(ReflectorTool::new())
        .register_tool(LLMTool::new("llama3"))
        .enable_dry_run();

    let mut agent = BasicAgent { model, context };

    let plan = agent.plan();
    let sim = agent.simulate(&plan);
    let exec = agent.execute(&plan);
    let feedback = agent.evaluate(&exec);

    println!("{}\n{:#?}", "--- PLAN ---".blue().bold(), plan);
    println!("{}\n{:#?}", "--- SIMULATION ---".yellow().bold(), sim);
    println!("{}\n{:#?}", "--- EXECUTION ---".green().bold(), exec);
    println!("{}\n{:#?}", "--- FEEDBACK ---".magenta().bold(), feedback);
    println!("{}", "--- MEMORY LOG ---".cyan().bold());

    for (label, content) in agent.context.memory().read_all() {
        println!(
            "{} {}",
            label.green().bold(),
            format_args!("input: {}", content)
        );
    }

    // Step 2: run ReflectorTool manually using memory log as input
    if let Some(tool) = agent.context.get_tool("reflect") {
        let memory_as_text = agent
            .context
            .memory()
            .read_all()
            .iter()
            .map(|(k, v)| format!("[{}] {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let reflection = tool.execute(&memory_as_text);
        println!(
            "{}\n{:#?}",
            "--- REFLECTION ---".bright_white().bold(),
            reflection
        );
    } else {
        println!("{}", "ReflectorTool not found".red());
    }
}

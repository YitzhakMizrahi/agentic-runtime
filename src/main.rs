// src/main.rs

use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::memory::Memory;
use agentic_runtime::model::TaskModel;
use agentic_runtime::protocol::planner::LLMPlanner;
use agentic_runtime::tools::{FakeEchoTool, GitStatusTool, LLMTool, ReflectorTool};
use colored::Colorize;
use std::io::{self, Write};

fn main() {
    // Ask user for a goal
    println!("\nEnter a goal for the agent to achieve:");
    print!("> ");
    io::stdout().flush().unwrap();

    let mut goal = String::new();
    io::stdin().read_line(&mut goal).unwrap();
    let goal = goal.trim();

    let model = TaskModel::new(goal);

    // Set up LLMTool and planner
    let llm_tool = LLMTool::new("llama3");
    let planner = LLMPlanner::new(llm_tool.clone());

    let context = Context::new()
        .register_tool(FakeEchoTool)
        .register_tool(GitStatusTool)
        .register_tool(ReflectorTool::new())
        .register_tool(llm_tool)
        .enable_dry_run();

    let mut agent = BasicAgent::new(model, context, Some(Box::new(planner)));

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

    // Step 2: Run ReflectorTool manually
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

// src/main.rs

use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::memory::Memory;
use agentic_runtime::model::TaskModel;
use agentic_runtime::protocol::planner::LLMPlanner;
use agentic_runtime::protocol::replanner::LLMReplanner;
use agentic_runtime::tools::{FakeEchoTool, GitStatusTool, LLMTool, ReflectorTool, RunCommandTool};
use colored::Colorize;

fn main() {
    let model = TaskModel::new(
        "Review the current project state and make a recommendation to improve developer experience.",
    );

    let llm = LLMTool::new("llama3");
    let planner = Box::new(LLMPlanner::new(llm.clone()));
    let replanner = Box::new(LLMReplanner::new(llm));

    let context = Context::new()
        .register_tool(FakeEchoTool)
        .register_tool(GitStatusTool)
        .register_tool(ReflectorTool::new())
        .register_tool(LLMTool::new("llama3"))
        .register_tool(RunCommandTool)
        .enable_dry_run();

    let mut agent = BasicAgent {
        model,
        context,
        planner: Some(planner),
        replanner: Some(replanner),
    };

    // Primary Planning Cycle
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

    // Reflection Tool Summary
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

        if let Some(summary) = reflection.output {
            agent.context.log("reflect", &summary);
        }
    } else {
        println!("{}", "ReflectorTool not found".red());
    }

    // 🔁 Follow-up Plan Based on Reflection
    if let Some((_, reflection)) = agent
        .context
        .memory()
        .read_all()
        .into_iter()
        .find(|(k, _)| k == "reflect")
    {
        if let Some(followup_plan) = agent.replan(&reflection) {
            println!(
                "{}\n{:#?}",
                "--- FOLLOW-UP PLAN ---".bright_blue().bold(),
                followup_plan
            );
            let sim = agent.simulate(&followup_plan);
            println!("{}\n{:#?}", "--- SIMULATION (2) ---".yellow().bold(), sim);
            let exec = agent.execute(&followup_plan);
            println!("{}\n{:#?}", "--- EXECUTION (2) ---".green().bold(), exec);
        }
    }
}

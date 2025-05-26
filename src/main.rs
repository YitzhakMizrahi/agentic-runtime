// src/main.rs

use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::memory::Memory;
use agentic_runtime::model::TaskModel;
use agentic_runtime::protocol::planner::LLMPlanner;
use agentic_runtime::protocol::replanner::LLMReplanner;
use agentic_runtime::tools::{ErrorAnalyzerTool, LLMTool, ReflectorTool, RunCommandTool};
use colored::Colorize;

fn main() {
    let model = TaskModel::new(
        "Analyze the current git repository status, identify any modified files, and create a meaningful commit if there are changes to commit.",
    );

    let llm = LLMTool::new("qwen3:8b");
    let planner = Box::new(LLMPlanner::new(llm.clone()));
    let replanner = Box::new(LLMReplanner::new(llm.clone())); // also uses it

    let context = Context::new()
        .register_tool(ReflectorTool::new(llm.clone())) // give one clone to Reflector
        .register_tool(llm.clone()) // register as a tool under "llm"
        .register_tool(RunCommandTool)
        .register_tool(ErrorAnalyzerTool::new(llm.clone())) // AI-powered error analysis
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

    // 🔁 Follow-up Plan Based on Error Analysis or Reflection
    let memory_entries = agent.context.memory().read_all();

    // 🎯 DYNAMIC INTELLIGENCE: Only replan if there were critical failures
    // Don't replan for auxiliary tool failures (like reflection failures)
    if !exec.success {
        // Check if we have error analysis for critical failures
        if let Some((_, error_analysis)) =
            memory_entries.iter().find(|(k, _)| k == "error_analysis")
        {
            if let Some(followup_plan) = agent.replan(error_analysis) {
                println!(
                    "{}\n{:#?}",
                    "--- FOLLOW-UP PLAN (Error Recovery) ---"
                        .bright_blue()
                        .bold(),
                    followup_plan
                );
                let sim = agent.simulate(&followup_plan);
                println!("{}\n{:#?}", "--- SIMULATION (2) ---".yellow().bold(), sim);
                let exec = agent.execute(&followup_plan);
                println!("{}\n{:#?}", "--- EXECUTION (2) ---".green().bold(), exec);
            }
        }
        // If no error analysis, fall back to reflection-based planning
        else if let Some((_, reflection)) = memory_entries.iter().find(|(k, _)| k == "reflect") {
            if let Some(followup_plan) = agent.replan(reflection) {
                println!(
                    "{}\n{:#?}",
                    "--- FOLLOW-UP PLAN (Reflection) ---".bright_blue().bold(),
                    followup_plan
                );
                let sim = agent.simulate(&followup_plan);
                println!("{}\n{:#?}", "--- SIMULATION (2) ---".yellow().bold(), sim);
                let exec = agent.execute(&followup_plan);
                println!("{}\n{:#?}", "--- EXECUTION (2) ---".green().bold(), exec);
            }
        }
    } else {
        println!(
            "{}",
            "✅ Goal completed successfully - no replanning needed"
                .green()
                .bold()
        );
    }
}

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
    let model = TaskModel::new("## Goal: Commit each modified file with a meaningful, standalone commit.

You have access to the following tools:
- `git_status`: See which files were modified.
- `run_command`: You can use this to run any `git` command or other shell commands such as `git diff <file>`, `git add <file>`, and `git commit -m \"<message>\"`
### Rules:
1. You must create **one commit per file**.
2. For each file:
   - Read the **diff** using `git diff <file>`.
   - Analyze the content to determine what changed.
   - Generate a clear and accurate **Conventional Commit message**:
     - Format: `<type>(<scope>): <description>`
     - Examples:
       - `feat(agent): add error handling to planner`
       - `fix(context): correct logging behavior`
       - `chore: format code in main.rs`
     - Choose an appropriate `type` from: `feat`, `fix`, `refactor`, `chore`, `docs`, or `style`.
     - `scope` should reflect the logical file/module name (e.g., `planner`, `agent`, `context`, etc).
     - `description` should briefly explain what was changed, ideally inferred from the `diff`.
3. After committing each file, ensure the working directory is clean before moving to the next.
4. End with an info message confirming all files were committed.

### Example Workflow:
1. Run `git_status` and parse the output.
2. For each modified file:
   - Run `git diff <file>`
   - Run `run_command: git add <file>`
   - Run `run_command: git commit -m \"<message>\"`
3. Confirm that `git status` shows no remaining staged or modified files.
4. Output `info: all files committed with structured messages`.

Make sure all commits are clear and meaningful. This will be used to audit your understanding of the changes made.
");

    let llm = LLMTool::new("deepseek-r1:7b");
    let planner = Box::new(LLMPlanner::new(llm.clone()));
    let replanner = Box::new(LLMReplanner::new(llm));

    let context = Context::new()
        .register_tool(FakeEchoTool)
        .register_tool(GitStatusTool)
        .register_tool(ReflectorTool::new())
        .register_tool(LLMTool::new("deepseek-r1:7b"))
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

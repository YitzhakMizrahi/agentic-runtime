use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::model::TaskModel;
use agentic_runtime::tools::FakeEchoTool;

fn main() {
    let model = TaskModel::new("Hello tools!");
    let context = Context::new().register_tool(FakeEchoTool).enable_dry_run();

    let mut agent = BasicAgent { model, context };

    let plan = agent.plan();
    let sim = agent.simulate(&plan);
    let exec = agent.execute(&plan);
    let feedback = agent.evaluate(&exec);

    println!("--- PLAN ---\n{:#?}", plan);
    println!("--- SIMULATION ---\n{:#?}", sim);
    println!("--- EXECUTION ---\n{:#?}", exec);
    println!("--- FEEDBACK ---\n{:#?}", feedback);
}

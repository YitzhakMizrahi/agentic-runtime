use agentic_runtime::agent::{Agent, BasicAgent};
use agentic_runtime::context::Context;
use agentic_runtime::model::TaskModel;

fn main() {
    let model = TaskModel::new("Write a README");
    let context = Context::new().enable_dry_run();
    let mut agent = BasicAgent { model, context };

    let plan = agent.plan();
    let sim = agent.simulate(&plan);
    let exec = agent.execute(&plan);
    let feedback = agent.evaluate(&exec);

    println!("Plan: {:#?}", plan);
    println!("Simulation: {:#?}", sim);
    println!("Execution: {:#?}", exec);
    println!("Feedback: {:#?}", feedback);
}

# Developer Log — Agentic Runtime

## [Day 0] — Project Initialization

* ✅ Decided to build a modular agent runtime in Rust, grounded in the Model-Context-Protocol (MCP) architecture.
* ✅ Intentionally not using LangChain or other agent wrappers; starting from first principles.
* ✅ Defined "Agent" as a composable, protocol-driven unit of autonomy.
* ✅ Committed to slow, thoughtful development in a single-threaded flow.
* ✅ Created project structure and initial trait for Agent.
* ✅ Defined `Model`, `Context`, `Plan`, `SimulationResult`, `ExecutionResult`, and `Feedback` types.
* ✅ Implemented CLI-based main.rs runner.
* ✅ Added tool system: `FakeEchoTool`, `GitStatusTool`, `ReflectorTool`, `LLMTool`
* ✅ Integrated `ollama` support for local LLMs.
* ✅ Designed and enforced structured memory log with post-run reflection.
* ✅ Enabled pre-commit formatting/lint checks.
* ✅ Refactored folder structure for idiomatic modular Rust.
* ✅ Polished CLI with colored output.

---

## 📍 Roadmap & Milestones

### 🛠️ Phase 1 — Core Runtime (Complete)
- [x] Trait-based `Agent` lifecycle
- [x] Pluggable `Tool` trait with metadata
- [x] Basic `Context`, `Memory`, and `Model` modules
- [x] Manual tool registration
- [x] Local LLM tool using Ollama
- [x] Structured `Plan` with multi-step execution
- [x] Feedback, simulation, and memory logging

### ⚙️ Phase 2 — Developer Ergonomics (In Progress)
- [x] ✅ Color-coded CLI output
- [x] ✅ Introduced LLM-based `Planner` and `Replanner` with structured JSON generation
- [x] ✅ Added structured plan validation via `src/validation/plan.rs`
- [x] ✅ Integrated fallback errors when plan parsing fails
- [x] ✅ Display raw, cleaned, and error details in debug logs
- [ ] 🔄 Hot-reloading tools or modular tool registration (`fn tools::register_all(context)`)
- [ ] 🧱 Scaffold a lightweight plugin-style tool architecture
- [ ] 🔍 Add `--dry-run`, `--plan-only`, `--interactive` flags to CLI

### 🧠 Phase 3 — Smarter Agent Capabilities (Next)
- [ ] 🧠 Introduce semantic diff parsing and commit generation
- [ ] 🗂️ Define and handle user-defined goals
- [ ] 🔄 Feedback loop that updates model/goals
- [ ] 🧠 Memory queries (`agent.context.memory().query("...")`)
- [ ] 🧩 Dynamic sub-agent spawning
- [ ] 🗃️ Workspace-aware file tools (e.g., `FileReaderTool`, `CodeSearchTool`)
- [ ] 🧠 Reflect on memory to generate new plans

### 🕸️ Phase 4 — Orchestration
- [ ] 🛠️ Tool scheduler with resource/time limits
- [ ] 🤖 Multi-agent runtime (Coordinator + Worker agents)
- [ ] 📎 Planning refinement loop (sim → revise → exec)

### 🧪 Phase 5 — UX & Observability
- [ ] 📟 REPL shell / interactive CLI
- [ ] 🌐 (Optional) Web dashboard for inspecting plans, logs, memory
- [ ] 📊 Metrics, debugging, and agent tracing
- [ ] 🧠 Persist memory state (e.g., JSON, SQLite)

---

## 🧠 Long-Term Vision

- 🧩 Dynamic WASM plugins for portable tools
- 🧠 Swap-in different LLMs (LLaMA, deepseek-r1:7b, Claude, etc.)
- 🧬 Declarative agents / config-driven behavior
- 🪟 Native OS shell extension / terminal companion
- 🧠 Meta-cognition: self-evaluation and planning refinement
- 🌱 Self-hosting: agents building agents

---

## 📅 Development Log

### [Day 1] — Planning Begins
- Identified current limitation: plans are static
- Goal: Add a dynamic `Planner` that can generate steps using LLMTool and memory context
- Integrated `planner.rs` with raw+cleaned prompt capture and error logging
- Added support for only approved `PlanStep` types: `tool` and `info`
- Ensured planner instructs LLM not to emit invalid JSON variants

### [Day 2] — Replanner, Validation, and Plan Enforcement
- Introduced `replanner.rs` and integrated it into `main.rs`
- Created `validate_plan()` function and helper error types
- Hooked validation into both planner and replanner flows
- Improved error reporting for missing inputs, unknown types, and unregistered tools
- Migrated planner/replanner to call `validate_plan(json_plan, tools)` before execution
- Ensured `serde_json::json!` macro used consistently across validation hints
- Decoupled validation logic into `src/validation/plan.rs` for reuse
- Ready to start refining LLM prompt and plan fidelity based on validation feedback

### [2025-05-25] Planner and Replanner Refactor Complete
- Rewrote `planner.rs` and `replanner.rs` to sanitize LLM output:
  - Stripped markdown/code blocks (` ``` `, `<think>`, etc)
  - Used safe JSON extraction regex (`{"plan": [...]}`)
  - Added structured validation warnings for:
    - Unknown tool names
    - Missing required fields
    - Placeholder detection (e.g. `<file>`, `$output[...]`)
- Both planner and replanner now successfully parse clean plans and fallback gracefully if JSON is malformed.
- Validation does not yet block execution — unsafe plans still run.
- `git diff <file>` and `git add <file>` fail silently due to unresolved placeholder `<file>`, but marked as successful because shell did not error fatally.

### 🧠 Reflection Summary
- The LLM goal was to "create one meaningful commit per file"
- The plan returned hardcoded placeholder values instead of resolving real filenames
- Replanner repeated the same invalid plan
- `ExecutionResult.success = true` is misleading (did not check stderr or shell return codes)

### 🩹 Short-Term Fixes Identified
- Inject `git_status` output into memory and planner prompts
- Block execution if unsafe placeholders are detected
- Allow validation feedback to influence replanner prompt

### 🚀 Long-Term Ideas
- Dynamic tool chaining: use outputs of one tool (e.g., git_status) to expand subplans for each file
- Plan scoring / confidence evaluation
- Tool schema registry with expected inputs, output types, and validation logic

### ✅ Next Steps

1. **Inject git_status into memory**:
   - Label: `[git_status]`
   - Include in prompt context under `Modified Files:` section

2. **Prevent execution of plans with placeholders**:
   - If validation errors include `ToolInputMismatch` for placeholder input, block plan execution
   - Trigger replanner with feedback like: "input contains placeholder like `<file>`"

3. **Improve planner feedback loop**:
   - Include tool validation messages in memory/context
   - Summarize validation and pass to replanner if plan was structurally valid but semantically unsafe

---

# Worker Agent Creation

## Overview

Worker agents are created dynamically by the personal agent.

Instead of cloning the personal agent directly, worker agents are **purpose-built intelligences** designed for specific tasks.

---

## Creation Lifecycle

Task received
↓
Personal agent analyzes task
↓
Worker specification created
↓
Worker agent instantiated
↓
Worker executes task
↓
Results returned
↓
Worker terminates

---

## Worker Specification

Before creation, the personal agent defines a worker specification.

Example:

Role: Market Research Analyst

Capabilities:

* web search
* data extraction
* summarization

Constraints:

* respect user preferences
* avoid untrusted sources

Output Format:

* structured report

---

## Apprenticeship Model

Worker creation may involve a **briefing phase** where the personal agent explains the task and constraints.

Personal agent → explanation
Worker agent → confirms understanding
Worker agent → executes mission

This ensures alignment before execution.

---

## Advantages

### Specialization

Workers are tailored to their specific mission.

### Security

Workers only receive the context required for the task.

### Efficiency

Workers remain lightweight and disposable.

### Alignment

Workers inherit principles and constraints from the personal agent.

---

## Learning Loop

The personal agent can improve worker design over time.

Worker result
↓
Evaluation by personal agent
↓
Improved worker specifications

This allows the system to evolve better worker designs.

---

## Conceptual Model

The personal agent acts as an **agent architect**, capable of designing new agents on demand.

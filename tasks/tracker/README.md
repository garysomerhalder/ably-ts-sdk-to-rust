# Task Management System

## Overview
This directory contains the task tracking system for the Ably Rust SDK port project.

## Structure
```
tasks/
├── tracker/              # Task management files
│   ├── tracker.md       # Main task inventory
│   ├── README.md        # This file
│   ├── dashboard.md     # Progress metrics
│   ├── archive.md       # Completed tasks >30 days
│   └── decisions.md     # Project decisions
└── task-files/          # Implementation files
    ├── foundation/      # Project setup tasks
    ├── infrastructure/  # Core infrastructure tasks
    ├── core/           # Protocol and client tasks
    ├── features/       # Feature implementation tasks
    └── bindings/       # Language binding tasks
```

## Task ID Convention
- FOUND-XXX: Foundation tasks
- INFRA-XXX: Infrastructure tasks
- CORE-XXX: Core implementation tasks
- FEAT-XXX: Feature tasks
- BIND-XXX: Binding tasks

## Traffic-Light Development
Each task follows three phases:
1. 🔴 RED: Write failing tests against real Ably API
2. 🟡 YELLOW: Minimal implementation to pass tests
3. 🟢 GREEN: Production hardening with full features

## Integration-First Requirement
- NO mocks or fakes allowed
- All tests must use real Ably sandbox or production APIs
- Credentials stored in `/reference/` directory
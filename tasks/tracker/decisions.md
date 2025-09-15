# 📋 Project Decisions & Open Questions

## 🚨 MCP Server Issues (Session: 2025-09-15)

### Problem
- context7 MCP server: ✅ Connected  
- sequentialthinking MCP server: ❌ Failed to connect
- playwright MCP server: ❌ Failed to connect

### Impact
- Limited complex reasoning capabilities (no UltraThink coordination)
- No browser automation testing
- Information lookup still available via context7

### Mitigation
- Proceed with context7 for fact verification
- Use manual reasoning for complex architecture decisions
- Plan browser testing without playwright automation

---

## 🔑 Ably Credentials Management

### Decision Needed
Where to securely store Ably API credentials for Integration-First testing?

### Options
1. `/reference/` directory (as mentioned in task README)
2. Environment variables
3. Separate credentials file (gitignored)

### Recommendation
- Use `/reference/ably-credentials.env` (gitignored)
- Document setup in project CLAUDE.md
- Provide example template

---

## 🎯 TypeScript → Rust Porting Priority

### Decision Needed  
Which ably-js TypeScript files should we port first?

### Analysis from Codebase Review
- `auth.ts` (1010 lines) - Complex authentication system
- `connectionmanager.ts` (2074 lines) - Core transport logic
- `baseclient.ts` (214 lines) - Foundation client
- `realtimechannel.ts` (34KB) - Channel operations

### Recommendation
Start with smaller, foundational components:
1. Protocol message types
2. Base client structure  
3. Authentication system
4. Transport layer
5. Channel operations

---

## 📊 Universal Task Management Status

### Completed Sections
- ✅ A: End-to-End Codebase Review
- ✅ B: Verify Environment (limited MCP)
- ✅ C: Review CLAUDE Docs
- ✅ D: Update Docs (in progress)

### Next Sections
- 🔄 E: Sync /tasks
- 🔄 F: Plan with MCP  
- 🔄 G: Commit Plan
- 🔄 H: Work Tasks

---

## 🛠️ Environment Notes

- **Rust Version**: 1.89.0 (latest stable)
- **Node.js Available**: Yes (for ably-js reference)
- **MCP Status**: 1/3 servers working (context7 only)
- **Git Status**: Clean, on feature branch

---

Last Updated: 2025-09-15 by Claude Code Universal Task Management System
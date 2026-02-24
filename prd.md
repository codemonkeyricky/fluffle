# Nano Code Agent - Product Requirements Document

## Overview
**Name:** nano code
**Purpose:** Claude-Code-like code agent with plugin architecture
**Target Users:** Developers needing AI-assisted coding in terminal
**Core Value:** Extensible plugin system for tool integration

## Architecture Decisions
- **Language:** Rust (full-stack)
- **AI Backend:** AI-SDK with 73+ provider support
- **Frontend:** Ratatui TUI
- **Plugin System:** Static compile-time registration via `inventory` crate
- **Configuration:** Layered system with `.env` file support

## MVP Features
1. **Chat Interface:** TUI-based conversation with AI agent
2. **File Operations:** Read, write, list files in working directory
3. **Bash Execution:** Run shell commands (initially without sandboxing)
4. **Git Operations:** Status, diff, basic git commands
5. **Plugin Architecture:** Extensible tool system for future additions

## Technical Design
- Single Rust crate with modular structure
- Event-driven TUI with async execution
- Tool execution flow with context management
- Error handling and recovery strategies

## Success Metrics
- Application starts and loads plugins successfully
- All three tool types work end-to-end
- API keys configured via `.env` file
- TUI displays conversation history and tool outputs
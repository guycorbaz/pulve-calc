# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a pre-development planning project for an agricultural sprayer (pulvérisateur/atomiseur) system. The project is in early design/planning phase — there is no source code yet. The domain is vineyard/orchard spraying equipment mounted on a Landini tractor (Perkins A4-212 diesel, 67 HP).

Reference documentation in `doc/` includes:
- **Pulvérisateur**: Atomizer calibration tables (flow rates in L/ha by nozzle type, pressure, speed, and working width) for a Blower 620mm with ceramic nozzles and ATI 60 nozzles
- **Tracteur**: Landini tractor service manual with Perkins engine specifications

The project language context is French (agricultural terminology, documentation).

## Project Structure

- `_bmad/` — BMad methodology framework (skills, agents, workflows). Do not modify.
- `.claude/skills/` — Installed BMad skills for Claude Code. Do not modify.
- `_bmad-output/` — Generated BMad artifacts (planning, implementation, test)
- `design-artifacts/` — Design deliverables organized by phase:
  - `A-Product-Brief/`, `B-Trigger-Map/`, `C-UX-Scenarios/`, `D-Design-System/`, `E-PRD/`, `F-Testing/`, `G-Product-Development/`
- `doc/` — Equipment reference documentation (PDFs)
- `docs/` — Project documentation (currently empty)

## BMad Methodology

This project uses the [BMad](https://github.com/bmadcode/bmad-agent) framework for structured product development. The typical workflow progresses through phases:

1. **Analysis** — Product brief, domain/market/technical research
2. **Planning** — PRD creation, UX design, architecture
3. **Implementation** — Epic/story creation, sprint planning, development
4. **Testing** — Test design, automation, quality gates

Key BMad skills are invoked as slash commands (e.g., `/bmad-create-prd`, `/bmad-product-brief`, `/bmad-help`). Use `/bmad-help` to get guidance on which skill to use next.

## Domain Notes

- Flow rates are measured in litres/hectare (L/ha)
- Key variables: nozzle type (ceramic disc size, ATI color), pressure (bars), tractor speed (km/h), working width (m), number of nozzles (jets)
- Row spacing adjustments: multiply L/ha by table width, divide by actual row width

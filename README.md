# Dimo

Dimo is an open-source **drawing ballooning and inspection planning** desktop application.

It reads engineering drawings (vector PDF, outlined PDF, scanned raster), finds every inspectable characteristic, numbers it with a balloon, interprets its tolerance, and produces the inspection documents a supplier needs: ballooned drawing, characteristic list, first article and initial sample reports, measurement records.

Status: **M0 Foundations**. Specification complete, implementation starting. Progress: [docs/plan/STATUS.md](docs/plan/STATUS.md).

## Guiding principles

1. **Local first.** Drawings never leave the machine unless the user exports them. No account, no license server, no telemetry.
2. **Human in the loop.** Automation proposes, the inspector confirms. Nothing unverified is exported silently.
3. **Traceable.** Every value can be traced back to the exact drawing region and rule it came from.
4. **Manual is first class.** Fast manual ballooning is not a fallback, it is a core workflow.
5. **Open formats.** Project files, report templates, tolerance tables and exports are documented plain formats.
6. **Independent design.** Features are derived from public standards and from user problems, never from other products. See the clean room policy.

## Specification

| Doc | Content |
|---|---|
| [00 Clean room policy](docs/spec/00-clean-room-policy.md) | Rules for independent development |
| [01 Vision and scope](docs/spec/01-vision-and-scope.md) | Problem, users, scope, glossary |
| [02 User pain points](docs/spec/02-user-pain-points.md) | Digested market feedback and our answer to each point |
| [03 Functional requirements](docs/spec/03-functional-requirements.md) | Numbered requirements with priority |
| [04 Non-functional requirements](docs/spec/04-non-functional-requirements.md) | Performance, privacy, quality targets |
| [05 Architecture](docs/spec/05-architecture.md) | Components, data flow, IPC, crate layout |
| [06 Tech stack](docs/spec/06-tech-stack.md) | Technology choices and the frontend framework comparison |
| [07 Data model](docs/spec/07-data-model.md) | Entities and project file format |
| [08 Recognition pipeline](docs/spec/08-recognition-pipeline.md) | Text recognition, parsing, tolerance interpretation |
| [09 Roadmap](docs/spec/09-roadmap.md) | Milestones from MVP to 1.0 |
| [10 Licensing](docs/spec/10-licensing.md) | Apache 2.0 and alternatives, trade-offs |
| [11 Open questions](docs/spec/11-open-questions.md) | Decision log for former open questions |
| [12 Implementation defaults](docs/spec/12-implementation-defaults.md) | Binding defaults so implementation needs no further questions |
| [ADRs](docs/adr/) | Architecture decision records |

## Development

- Setup: [docs/dev/setup.md](docs/dev/setup.md)
- Contributing: [CONTRIBUTING.md](CONTRIBUTING.md) (DCO sign-off, clean room policy)
- AI agent instructions: [AGENTS.md](AGENTS.md)

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

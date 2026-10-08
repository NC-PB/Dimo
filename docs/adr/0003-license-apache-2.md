# ADR 0003: License

Status: **accepted** (October 2026)

## Context
The project is open source. Goals: wide adoption in industry, low friction for companies, patent protection for users and contributors. Concern: commercial vendors could build closed products on top of it. Analysis in [10 Licensing](../spec/10-licensing.md).

## Decision
Apache License 2.0 for all crates and the application. Contributions under the Developer Certificate of Origin.

## Consequences
- Maximum adoption, no friction for shops and companies.
- Closed forks are allowed. Accepted: the target audience (small shops, one person companies) is not the market large vendors compete for, and the public repository keeps the reference implementation open.
- No dual licensing. Sustainability relies on other options (see Q-07).
- Contributors sign off commits under the DCO. No CLA.

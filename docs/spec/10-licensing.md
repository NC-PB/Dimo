# 10 Licensing

> Not legal advice. Summary to support the decision. Confirm with a lawyer before the first public release, especially regarding patents.

Decision: **Apache License 2.0** with DCO. Status: accepted, see [ADR 0003](../adr/0003-license-apache-2.md).

## What Apache 2.0 gives

- **Permissive:** anyone can use, modify, and redistribute, including in commercial and closed products.
- **Explicit patent grant:** every contributor grants users a license to their patents that the contribution needs. If someone sues over patents relating to the project, their license under it terminates. This is the main advantage over MIT.
- **Enterprise friendly:** legal departments in industry know and accept it. Quality departments can install and use it without long approvals.
- **Compatible with the planned dependencies** (MIT, BSD, Apache 2.0) and with GPLv3 (one direction: Apache code can go into GPLv3 projects, not the reverse).
- **Requirements for redistributors:** keep license and NOTICE file, mark changed files. Light burden.

## Downsides of Apache 2.0

1. **Closed forks are allowed.** A commercial vendor can take the recognition engine or the whole application, improve it, and sell it closed, without giving anything back. In a niche market with expensive incumbents, this is a real possibility.
2. **No obligation to contribute back.** Companies that improve it internally may never share their fixes.
3. **Dual licensing is mostly off the table.** Selling commercial licenses only works if the open license is restrictive enough that some buyers prefer a paid one. With Apache, everyone already has broad rights. Monetization would rely on services, support, signed builds, sponsorship, or paid extras (open core).
4. **No trademark protection.** Apache 2.0 explicitly does not grant trademark rights, which is good, but it means the project name needs separate protection if forks should not use it.

## Alternatives

| License | Copyleft | Effect for Dimo | Fit |
|---|---|---|---|
| **MIT** | None | Like Apache without patent grant | Weaker than Apache, no reason to prefer it |
| **Apache 2.0** | None | Maximum adoption, closed forks allowed | Good default |
| **MPL 2.0** | File level | Changes to existing project files must be published when distributed. New files in a combined product may stay closed. Companies can embed it. | Good middle ground if closed improvements of the engine itself should be prevented |
| **GPL 3.0** | Strong | Distributed derivatives must be GPL as a whole. Internal use is unrestricted. Some companies avoid GPL by policy, even for internal tools | Protects the project, may reduce corporate adoption |
| **AGPL 3.0** | Strong plus network | Also covers hosted versions: someone running a modified version as a web service must publish the source | Strongest protection against a closed cloud service built on the engine |

Mixed licensing is also possible: for example core crates under Apache 2.0 (reusable by anyone) and the desktop application under MPL 2.0 or GPL 3.0.

## Contributions

- **DCO (Developer Certificate of Origin):** contributors sign off each commit. Lightweight, widely used, fits Apache 2.0.
- **CLA (Contributor License Agreement):** needed only if the project might later relicense or dual license. Adds friction for contributors.

Recommendation: Apache 2.0 with DCO, unless a dual licensing business model is a real goal. In that case choose GPL 3.0 or AGPL 3.0 with a CLA from day one, because relicensing later requires consent from every contributor.

## Patents

Status: **done, no conflicts found** (October 2026).

- The project owner reviewed the patent landscape for automatic drawing ballooning, characteristic extraction and inspection reporting. Nothing relevant to Dimo's approach was found.
- The closest granted patent found (a 2003 US filing on extracting data from engineering drawings by ballooning and producing first article reports) has expired. Its European counterpart was withdrawn. It also documents that the basic concept was public more than 20 years ago.
- A marketing claim of a "patented" auto ballooning engine by one vendor could not be linked to any patent document.
- Dimo targets small machine shops and one person companies, not the enterprise customers of large software vendors.

Ongoing practice: publish the specification and code early in a public repository. Dated public commits act as defensive publication, so the techniques described here cannot be patented by others later.

## Third party assets

- Dependencies are checked in CI with cargo-deny and an npm license checker. Only MIT, Apache 2.0, BSD, ISC, Zlib and similar permissive licenses are allowed.
- OCR models must have permissive licenses and documented training data.
- Standard tolerance tables (ISO 2768, ISO 286 and others) are shipped with the application as data files (decision Q-04).
- Report form structures: see [11 Open questions](11-open-questions.md), Q-05.

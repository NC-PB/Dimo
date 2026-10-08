# 11 Open questions

| ID | Question | Notes | Needed by |
|---|---|---|---|
| Q-01 | ~~Final product name~~ | **Resolved (October 2026):** **Dimo**. Web presence at cnc-master.xyz/dimo, repository under the owner's GitHub account. Trademark search on Swissreg and EUIPO still recommended before 0.1 | done |
| Q-02 | ~~Final license~~ | **Resolved (October 2026):** Apache 2.0 with DCO. See [ADR 0003](../adr/0003-license-apache-2.md) | done |
| Q-03 | ~~Patent freedom to operate~~ | **Resolved (October 2026):** reviewed, no conflicts found. See [10](10-licensing.md) | done |
| Q-04 | ~~Shipping tolerance tables from ISO standards~~ | **Resolved (October 2026):** tables are shipped with the application as data files in `data/tolerances/` | done |
| Q-05 | ~~Report form layouts~~ | **Resolved:** own layouts in compatible structure, customer forms as user templates. See D-31 | done |
| Q-06 | ~~Project governance~~ | **Resolved:** public repo NC-PB/dimo, single maintainer. See D-01, D-02 | done |
| Q-07 | Sustainability | **Deferred, not blocking:** free with optional sponsor link. Business options after 0.1. See D-07 | after 0.1 |
| Q-08 | ~~Code signing~~ | **Resolved:** unsigned for 0.1, reconsider before 1.0. See D-08 | done |
| Q-09 | ~~Minimum OS versions~~ | **Resolved:** see D-10, D-11 | done |
| Q-10 | Corpus sourcing | **Open, not blocking before M4.** First drawing added (test_drawing_1, own Onshape drawing). More drawings needed, especially scans and outlined text. See D-42 | M4 |
| Q-11 | ~~Large scan performance~~ | **Resolved:** see D-41 | done |
| Q-12 | ~~Default numbering strategy~~ | **Resolved:** see D-21 | done |
| Q-13 | ~~Inch drawings and ASME specifics~~ | **Resolved:** ISO first, see D-20 | done |
| Q-14 | ~~Source of tolerance table values~~ | **Resolved:** agent drafts, owner verifies against his Tabellenbuch, CI blocks release of unverified tables. See D-43 | done |

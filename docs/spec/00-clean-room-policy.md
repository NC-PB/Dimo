# 00 Clean room policy

## Purpose

Dimo must be designed and implemented independently. This protects contributors and users from copyright and trade secret claims, and it keeps the project focused on solving user problems instead of copying existing tools.

This is an **independent development policy**, a practical version of a clean room process for a small open-source team. A strict two-team clean room (one team writes a specification from studying a product, a second team implements only from that specification) is not needed as long as nobody studies another product's internals in the first place.

## Allowed sources

- Public standards and the concepts they define: first article inspection (AS9102 / EN 9102), production part approval, initial sample inspection reports (VDA volume 2), ISO 2768, ISO 286, ISO 1101, ISO 13920, ISO 9013, ASME Y14.5, QIF (ISO 23952).
- General engineering knowledge: how drawings are dimensioned, how inspectors work.
- Public user feedback about the *problems* people have with existing tools (reviews, forum posts), digested into neutral problem statements. See [02 User pain points](02-user-pain-points.md).
- Open-source libraries under compatible licenses.
- Our own experiments and our own test drawings.

## Not allowed

- Decompiling, disassembling, or inspecting binaries, file formats, or network traffic of other products.
- Installing trial versions to study and replicate their UI, workflows, wording, report layouts, or file formats.
- Copying screenshots, icons, documentation text, report templates, or marketing text.
- Using drawings or data files obtained from other products or from customers without a clear right to use them.
- Naming other products anywhere in the repository: code, comments, docs, issues, commit messages, test names.

## Rules for contributors

1. Describe features in terms of the **user problem** and the **standard**, never as "like product X".
2. If you have used a competing product professionally, that is fine. Do not reproduce specific UI layouts, terminology unique to that product, or report designs from memory.
3. Every pull request is signed off under the Developer Certificate of Origin (`git commit -s`), affirming the contribution is your own work.
4. Test drawings in `corpus/` must be created by contributors or come from sources with an explicit permissive license. Each file has a provenance entry in `corpus/PROVENANCE.md`.
5. Machine learning models are trained only on synthetic data or on data with documented rights. Each model ships with a model card stating its training data.

## What this policy does not cover

Independent development protects against copyright claims. It does **not** protect against patents. A patent can be infringed even by an independent invention. The patent review is done with no conflicts found, see [10 Licensing](10-licensing.md).

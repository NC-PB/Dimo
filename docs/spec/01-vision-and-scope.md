# 01 Vision and scope

## Problem

Before a manufactured part can be shipped, the supplier often has to prove that **every** requirement on the customer drawing was checked. To do this, a quality engineer marks each dimension, tolerance, geometric callout and note with a numbered balloon, transfers each one into a characteristic list, measures the parts, and fills in a report. On a complex drawing this means hundreds of characteristics and several hours of error-prone manual work per part number and revision.

Commercial tools that automate this exist, but they are expensive, mostly per-seat subscriptions or contact-sales pricing, closed, often cloud based, and their automation still leaves the hardest part (interpreting tolerances) to the user. Small job shops, apprentices, schools, and companies with confidential drawings are badly served.

## Vision

A free, open, local-first desktop tool that turns a customer drawing into a verified, numbered characteristic list in minutes, and carries that list all the way to the finished inspection report. It reads the drawing **and interprets it**: general tolerances applied by size, fit designations expanded to limits, geometric tolerance frames fully parsed, notes captured. It is honest about uncertainty and makes verification fast.

## Primary audience

**Small machine shops and one person companies.** People who make parts to customer drawings, have to deliver inspection reports now and then, and have no budget or time for enterprise quality software. For them the tool must be:

- **simple:** useful within minutes, without training or configuration,
- **free:** no subscription, no license server, no sales call,
- **complete enough:** ballooned drawing, characteristic list, measurement record and a clean report.

Dimo does not compete for the enterprise customers of large software vendors. Advanced features (customer profiles, revision handling, CLI, interchange formats) exist, but they stay out of the way until needed.

## Target users

| Persona | Context | Main need |
|---|---|---|
| **Shop owner / one person company** | Makes parts to customer drawings, does quality himself next to programming and machining | Get a correct report done quickly, without learning a complex tool |
| **Quality engineer** at a small job shop | Receives customer PDFs, prepares first article and initial sample reports | Speed, completeness, customer-specific report formats |
| **Inspector / CMM programmer** | Measures parts, enters or imports results | Clear characteristic list in a sensible order, easy result entry |
| **Production engineer / machinist** | Uses ballooned drawing for in-process checks | Simple, readable ballooned drawing and check sheet |
| **Supplier quality at an OEM** | Reviews incoming reports | Consistent, traceable reports |
| **Teacher / apprentice** | Learns drawing reading and inspection planning | Free tool, explanations of each interpreted tolerance |

## In scope (1.0)

- Import of vector PDF, outlined vector PDF, scanned PDF and TIFF/PNG drawings, multi-sheet.
- Manual, assisted (box select) and automatic ballooning.
- Characteristic extraction: dimensions, tolerances, fits, threads, chamfers, radii, angles, surface texture, geometric tolerances, notes, flag notes, title block requirements.
- Tolerance engine for general tolerance standards and ISO fits.
- Review and verification workflow.
- Measurement entry and import, pass/fail evaluation, multiple serial numbers.
- Exports: ballooned PDF, spreadsheet characteristic list, first article report forms, initial sample report (VDA style), generic inspection report, CSV, QIF.
- Drawing revision comparison and carry-over of characteristics.
- Command line interface for batch processing.
- English and German UI.

## Out of scope (1.0)

- Native CAD file reading and 3D model based definition. Considered after 1.0.
- Statistical process control, nonconformance and CAPA management. Dimo exports data, it is not a full quality management system.
- Cloud hosting and multi-user collaboration servers.
- Direct CMM program generation.

## Glossary

| Term | Meaning |
|---|---|
| **Characteristic** | One inspectable requirement on a drawing: a dimension, a geometric tolerance, a note, a surface requirement |
| **Balloon** | Numbered marker on the drawing linking a location to a characteristic |
| **Ballooned drawing** | The drawing with all balloons, part of the inspection package |
| **Characteristic list** | Table of all characteristics with requirement, limits, method, results |
| **First article inspection (FAI)** | Full verification of the first production part. Aerospace uses AS9102 / EN 9102 with three forms; form 3 holds the characteristic results |
| **Initial sample report** | European automotive equivalent (VDA volume 2) |
| **General tolerance** | Tolerance that applies to all untoleranced dimensions, declared in the title block (e.g. ISO 2768-mK) |
| **Fit designation** | ISO 286 code like `H7` or `g6` that defines limits relative to a nominal size |
| **Feature control frame** | Boxed geometric tolerance callout: symbol, tolerance value, modifiers, datum references |
| **Basic dimension** | Theoretically exact dimension (boxed), not inspected itself but feeds a geometric tolerance |
| **Reference dimension** | Dimension in parentheses, informational, usually not inspected |
| **Flag note** | Numbered note that applies only to features marked with its flag symbol |
| **Zone** | Grid location on the drawing frame, e.g. `C4` |
| **Confidence** | Score expressing how sure the recognition is about a proposed value |

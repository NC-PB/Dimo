# 02 User pain points

Research summary, digested under the [clean room policy](00-clean-room-policy.md). Sources were public reviews and forum discussions by quality engineers about ballooning and inspection reporting tools in general. Statements are generalized. No product is named and no product design was studied.

Each pain point maps to requirements in [03](03-functional-requirements.md) and [04](04-non-functional-requirements.md).

## P-01 Recognition errors cost as much time as they save

**Reported:** Text recognition reads values, tolerances or instance counts (e.g. `4X`) wrongly. Users have to double check everything, and some say this takes about as long as doing it by hand in a spreadsheet. Tolerance recognition is unreliable on customer drawings of varying quality.

**Our answer:** Every recognized field carries a confidence score and a link to its source crop. The review view shows the drawing snippet next to the parsed value so checking takes one glance. Low confidence items are sorted to the top. Parsing is grammar based and validated (limits in order, fit exists, value plausible). FR-REV-01..05, FR-REC-10.

## P-02 Automation misses exactly the slowest items

**Reported:** Detection rates around 85 to 90 percent are typical, but the misses concentrate on general tolerances from the title block, fit classes, datum references inside geometric frames, flag notes, notes in general, and requirements on later sheets. These are the items that take longest by hand.

**Our answer:** A dedicated tolerance engine applies general tolerances by size range, expands fits to limits, and records the rule used. Feature control frames are parsed completely. Notes and flag notes are first class characteristics. Multi-sheet drawings are processed as one document. FR-TOL-*, FR-REC-06..09, FR-CHR-05..07.

## P-03 Auto numbering ignores the preferred order

**Reported:** Automatic ballooning does not follow the order the shop wants. Renumbering afterwards is painful.

**Our answer:** Numbering strategies are configurable (by sheet then zone, by view, clockwise per view, by characteristic type, manual). Users can preview, drag to reorder in the list, and renumber in one action. FR-BAL-04..08.

## P-04 Renumbering breaks everything

**Reported:** Once a report has been sent, numbers must not change, but tools make it hard to insert or remove a characteristic without shifting all numbers.

**Our answer:** Characteristics have stable internal IDs separate from the displayed balloon number. Numbering can be locked. Inserts use sub-numbers or the next free number according to a project setting. FR-BAL-09..11.

## P-05 Report templates are hard or impossible to customize

**Reported:** Users cannot adapt report layouts, and template support with placeholders is poorly documented and limited.

**Our answer:** Report templates are plain, documented files: spreadsheet templates with named placeholders, and text based report templates. Templates are shareable and versionable. A template editor preview shows the result live. FR-EXP-05..08.

## P-06 Stagnant recognition, no way to improve it

**Reported:** The recognition component has persistent bugs for years and the underlying library cannot be updated or extended by the user.

**Our answer:** Recognition backends sit behind a stable interface. Models are separate, replaceable files. Users can add custom symbol sets and dictionaries. Being open source, anyone can improve the engine. FR-REC-11..13, architecture section on engines.

## P-07 Price, per-seat licensing and paid deployment support

**Reported:** Cost is a recurring complaint. Pricing is hidden behind sales calls, per-user subscriptions add up, and some vendors started charging for deployment and integration support.

**Our answer:** Free and open source. No seat limits, no activation. Installation is a normal installer or a portable build. Documentation is good enough that no paid onboarding is needed.

## P-08 Licensing prevents flexible work

**Reported:** The tool cannot be used remotely or from home because the license is bound to one workstation.

**Our answer:** No license binding at all. Runs offline on Windows, macOS and Linux.

## P-09 Confidential and export controlled drawings

**Reported (implied by market):** Many buyers need special hosting guarantees because drawings are confidential or export controlled. Cloud tools are a problem for them.

**Our answer:** Local only by default. No network access in the default permission set. Any optional online or AI feature is opt in, clearly marked, and can be disabled by policy file. NFR-SEC-*.

## P-10 Large drawings fail to load

**Reported:** Large format drawings do not upload or render. Users are told to experiment with resolution settings, try other computers, or close other programs.

**Our answer:** Tiled, on demand rendering. The full drawing is never rasterized at full resolution in memory. A0 sheets and 50 sheet documents must open smoothly. NFR-PERF-*.

## P-11 Tool changes data on its own

**Reported:** The software inserted content into fields the user never touched.

**Our answer:** Automation writes only proposals. Proposals become data only when accepted (individually, in bulk, or by an explicit auto accept rule). Full undo history and audit log. FR-REV-06, FR-CHR-10.

## P-12 Steep learning curve, poor documentation

**Reported:** Getting used to the tool takes time, advanced features and report customization are hard to learn, documentation is thin and users search forums for answers.

**Our answer:** Documentation is part of the definition of done. Sample projects ship with the app. A guided first run uses a demo drawing. Each interpreted tolerance shows a short "why" explanation. NFR-UX-*.

## P-13 Manual ballooning is frustrating

**Reported:** Many users prefer to choose which features to inspect rather than accept full auto ballooning. When manual ballooning is clumsy, the tool loses its value.

**Our answer:** Manual mode is keyboard driven and fast: click to place, type value, Enter, next. Box select runs recognition on just one region. Auto ballooning can be scoped to a sheet, a view or a region. FR-BAL-01..03, FR-REC-02.

## P-14 Composite callouts fall apart

**Reported:** Balloons do not attach correctly when a geometric tolerance is attached to a dimension. Dimensions that reference a table or a letter variable do not import.

**Our answer:** The data model supports composite characteristics (dimension plus attached frame, frame with multiple segments) and variable dimensions resolved from a table on the drawing. FR-CHR-03..04, FR-CHR-08.

## P-15 Drawing revisions restart the work

**Reported:** When a new drawing revision arrives, the link between drawing and report breaks and ballooning often starts from scratch. Inspecting against the wrong revision is a common quality escape.

**Our answer:** Revision comparison highlights changed regions and carries unchanged characteristics over with their numbers. The project records which revision each report refers to. FR-RVC-*.

## P-16 Some tools fit only one industry dialect

**Reported (market observation):** The same characteristic list feeds aerospace, automotive (North America and Europe) and generic first article reports, yet tools are often built around only one.

**Our answer:** One neutral characteristic model, many output templates. European initial sample reports and ISO general tolerances are treated as first class, not as an afterthought. FR-EXP-02..04.

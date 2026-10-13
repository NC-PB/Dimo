# M3 UI test with a shop floor user

> Status: **to run by the owner** with one person who has not used Dimo before. This page is the
> procedure and the empty observation sheet (M3 task T3.21).

Goal (M3 "UI rework", extra exit condition): a shop floor person who knows common office software
but has never seen Dimo does one real job, from the customer drawing to an issued report, while
the owner watches **without helping**. The test is passed when the person finishes and every point
where they hesitated longer than about 10 seconds is recorded, then fixed or logged as a decision.

The owner's own exit run (`docs/perf/M3-exit.md`) comes after this test, on the fixed UI.

## Choose the person and the job

- One person from the shop floor (machinist or inspector) who uses Word or Excel, not a software
  person, and who has never seen Dimo or its mockups.
- A real drawing with at least 10 dimensions to inspect, and 2 parts that can be measured in a few
  minutes. Note the drawing (file name, revision) in the result table.
- A release build of Dimo, started fresh: no project open, Options on default (language of the
  person), no recent projects that give hints.

## What the owner says, and nothing more

Read this once, then stay silent:

> "This program is for numbering a drawing, measuring the parts and making the inspection report.
> Please do the whole job for this drawing and these two parts, as you would at work. Think aloud:
> say what you look for and what you expect before you click. I will not help; if you are stuck
> for a long time, say so and decide yourself whether to go on."

If the person asks a question, answer only "What would you try?". If they give up on a step, note
it, then let them continue with the next step (show the minimum needed and mark it as "helped").

## Tasks for the person (hand them this list on paper)

1. Start a new project from the drawing.
2. Tell Dimo the general tolerance from the title block.
3. Number all dimensions to be inspected.
4. Correct one dimension where Dimo read the value wrong or did not get limits (if none, change the
   tolerance of one dimension by hand).
5. Print the check sheet.
6. Measure the two parts and enter the values.
7. Enter part number, customer and report number.
8. Issue the inspection report as PDF.
9. Save the project and close Dimo.

## What the owner records

For every hesitation longer than about 10 seconds, every wrong click that had to be undone, every
question and every "helped": the time, the task number, what the person said or looked for, what
they expected, and what happened. Note also what went well without hesitation.

Do not explain or defend the UI during the test. After the last task, ask three questions and
write down the answers:

1. What was the hardest moment?
2. Was there anything where you did not know what a button would do before clicking it?
3. What would you change first?

## Observation sheet

Person (role, office software used, no name needed): ...

Drawing: ...  Revision: ...  Dimensions inspected: ...  Parts: 2

Start time: ...  End time: ...  Finished all tasks: yes / no

| Time | Task | What happened (said, looked for, expected) | Hesitation > 10 s | Helped |
|---|---|---|---|---|
| | | | | |

Answers to the three questions:

1. ...
2. ...
3. ...

## After the test

- Copy every row with a hesitation or "helped" into the STATUS.md decision log or a follow-up
  task in `docs/plan/M3.md` (fix before the owner's exit run), or log why it stays.
- Record in STATUS.md: date, person's role, finished yes/no, number of hesitations and helps,
  and the follow-up list. Then tick the T3.21 acceptance box in `docs/plan/M3.md`.

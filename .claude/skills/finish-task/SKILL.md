---
name: finish-task
description: Close a Dimo task properly. Runs all checks, reviews the diff against the spec, updates STATUS.md and the milestone file, and creates a signed Conventional Commit. Use when implementation of a task or a coherent step is complete.
---

# Finish task

1. Run `./scripts/check.sh`. Fix every failure. Do not disable lints or tests to get green;
   if an allow is truly needed, scope it narrowly and comment why.
2. Check each acceptance criterion of the task. For each one name the test or the manual check that
   proves it. Behavior you did not actually run is "not verified", say so.
3. Delegate a review of the diff (`git diff` against the last commit or branch base) to the
   `spec-reviewer` subagent. If the change adds user visible text, docs, templates or test data, also
   run the `clean-room-auditor` subagent. Fix real findings, explain rejected ones.
4. Update docs:
   - tick criteria in `docs/plan/M<n>.md`
   - in `docs/plan/STATUS.md`: task state (`done`, or `review` if the owner must check something),
     notes, the next task, new decisions in the decision log, a log line with the date
   - user docs or crate READMEs if behavior changed
5. Stage only files that belong to the task. Commit with sign-off:
   `git commit -s -m "<type>(<scope>): <summary>"` and a body listing requirement IDs.
   Several logical steps become several commits.
6. Report to the user: what was done, how it was verified, anything waiting for them. Do not push.

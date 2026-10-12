import { describe, expect, it } from "vitest";
import type { CharId, MeasurementState } from "$lib/ipc/bindings";
import { MeasurementStore } from "./measurement.svelte";

const counts = { pass: 1, fail: 0, not_evaluated: 0, missing: 1 };

function state(verdict: "pass" | "fail"): MeasurementState {
  return {
    rows: [
      {
        characteristic: "c1",
        result_type: "variable",
        draft: false,
        cells: [verdict, "missing"],
        counts,
        verdict: "open",
      },
    ],
    samples: [
      { sample: "s1", counts, verdict: "open" },
      { sample: "s2", counts, verdict: "open" },
    ],
    counts,
    verdict: "open",
  };
}

describe("measurement store (T3.5)", () => {
  it("looks up verdicts by characteristic and sample", async () => {
    let hintsAsked = 0;
    const store = new MeasurementStore({
      measurementState: () => Promise.resolve({ status: "ok", data: state("pass") }),
      changedSinceIssuedReport: () => {
        hintsAsked += 1;
        return Promise.resolve({ status: "ok", data: ["c1"] as CharId[] });
      },
    });
    await store.refresh(false);
    expect(store.verdict("c1", "s1")).toBe("pass");
    expect(store.verdict("c1", "s2")).toBe("missing");
    expect(store.verdict("c1", "s3")).toBeUndefined();
    expect(store.verdict("c9", "s1")).toBeUndefined();
    expect(store.changed.size).toBe(0);
    expect(hintsAsked).toBe(0);
    await store.refresh(true);
    expect(store.changed.has("c1")).toBe(true);
    store.clear();
    expect(store.state).toBeNull();
  });

  it("drops the answer of an older call", async () => {
    const answers: ((s: MeasurementState) => void)[] = [];
    const store = new MeasurementStore({
      measurementState: () =>
        new Promise((resolve) => {
          answers.push((s) => {
            resolve({ status: "ok", data: s });
          });
        }),
      changedSinceIssuedReport: () => Promise.resolve({ status: "ok", data: [] }),
    });
    const first = store.refresh(false);
    const second = store.refresh(false);
    answers[1]?.(state("fail"));
    await second;
    answers[0]?.(state("pass"));
    await first;
    expect(store.verdict("c1", "s1")).toBe("fail");
  });
});

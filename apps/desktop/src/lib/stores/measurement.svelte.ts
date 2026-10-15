import {
  commands,
  type CharId,
  type CommandError,
  type LimitPosition,
  type MeasurementState,
  type RowState,
  type SampleId,
  type SampleState,
  type Verdict,
} from "$lib/ipc/bindings";
import { frozenSet, indexMap } from "$lib/sets";
import type { Result } from "$lib/ipc/result";

/** The queries the store calls. The generated `commands` in the app, fakes in tests. */
export interface MeasurementCommands {
  measurementState(): Promise<Result<MeasurementState>>;
  changedSinceIssuedReport(): Promise<Result<CharId[]>>;
}

const NO_IDS: ReadonlySet<CharId> = frozenSet([]);

/**
 * Verdicts and summaries of the measurement grid (T3.5, FR-MEA-03), as Rust computes them with
 * `measurement_state` (M3 decision 2, rule 2: the frontend never compares values with limits).
 * The Measure view calls {@link MeasurementStore.refresh} after every change of the project, so
 * the verdicts follow each patch. A late answer of an older call is dropped.
 */
export class MeasurementStore {
  /** The last state from Rust; `null` before the first answer or without a project. */
  state = $state.raw<MeasurementState | null>(null);
  /** Characteristics whose limits or result type changed after the last issued report (M3
   *  decision 16). */
  changed = $state.raw<ReadonlySet<CharId>>(NO_IDS);
  /** Why the last call failed, `null` if it worked. */
  error = $state.raw<CommandError | null>(null);

  readonly #api: MeasurementCommands;
  #seq = 0;

  /** Row index by characteristic and column index by sample, in the state's own order. */
  readonly #rowIndex = $derived(indexMap((this.state?.rows ?? []).map((r) => r.characteristic)));
  readonly #sampleIndex = $derived(indexMap((this.state?.samples ?? []).map((s) => s.sample)));

  constructor(api: MeasurementCommands = commands) {
    this.#api = api;
  }

  /** Measured characteristics in number order, as Rust lists them (M3 decision 5). */
  get rows(): readonly RowState[] {
    return this.state?.rows ?? [];
  }

  /** Asks Rust for the verdicts again; `hints` also asks for the decision 16 markers. */
  async refresh(hints: boolean): Promise<void> {
    const seq = ++this.#seq;
    const [state, changed] = await Promise.all([
      this.#api.measurementState(),
      hints ? this.#api.changedSinceIssuedReport() : Promise.resolve(null),
    ]);
    if (seq !== this.#seq) {
      return; // A newer call is running or done.
    }
    if (state.status === "error") {
      this.error = state.error;
      return;
    }
    this.error = null;
    this.state = state.data;
    if (changed === null) {
      this.changed = NO_IDS;
    } else if (changed.status === "ok") {
      this.changed = frozenSet(changed.data);
    } else {
      this.error = changed.error; // Keep the last markers rather than hiding them silently.
    }
  }

  /** Forgets the state, for a closed project. */
  clear(): void {
    this.#seq += 1;
    this.state = null;
    this.changed = NO_IDS;
    this.error = null;
  }

  /** The row of a characteristic, `undefined` if it is not measured. */
  row(id: CharId): RowState | undefined {
    const index = this.#rowIndex.get(id);
    return index === undefined ? undefined : this.state?.rows[index];
  }

  /** The summary of one sample, `undefined` if Rust has not seen it yet. */
  sample(id: SampleId): SampleState | undefined {
    const index = this.#sampleIndex.get(id);
    return index === undefined ? undefined : this.state?.samples[index];
  }

  /** The verdict of one cell, `undefined` while Rust has not answered for that cell yet. */
  verdict(characteristic: CharId, sample: SampleId): Verdict | undefined {
    const row = this.row(characteristic);
    const index = this.#sampleIndex.get(sample);
    return index === undefined ? undefined : row?.cells[index];
  }

  /** Where the value of one cell lies against the limits (T3.19), `null` if it has no place. */
  position(characteristic: CharId, sample: SampleId): LimitPosition | null {
    const row = this.row(characteristic);
    const index = this.#sampleIndex.get(sample);
    return index === undefined ? null : (row?.positions[index] ?? null);
  }
}

export const measurementStore = new MeasurementStore();

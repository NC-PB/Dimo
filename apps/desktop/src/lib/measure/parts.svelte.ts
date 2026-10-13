/**
 * Parts (samples) of the Measure tab (T3.5, FR-MEA-01, M3 decision 6), moved out of the old
 * sample bar for the ribbon (T3.15): add N parts, move the active part left or right, delete it.
 * Deleting a part with results asks first, since its results go with it (decision 16). Every
 * action is one undoable Rust command; Rust checks counts and limits.
 */

import { m } from "$lib/i18n";
import type { Command, CommandError, Sample, SampleId } from "$lib/ipc/bindings";
import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
import { refusalDetail } from "$lib/table/refusal";
import { measureUi } from "./measure-ui.svelte";

export class PartsStore {
  /** How many parts "Add parts" adds. */
  count = $state(1);
  /** Why the last action was refused. */
  notice = $state<string | null>(null);
  /** The part waiting for "delete with its results" to be confirmed. */
  confirming = $state<SampleId | null>(null);

  readonly #project: ProjectStore;
  readonly #active: () => SampleId | null;

  constructor(
    project: ProjectStore = projectStore,
    active: () => SampleId | null = () => measureUi.activeSample,
  ) {
    this.#project = project;
    this.#active = active;
  }

  get samples(): readonly Sample[] {
    return this.#project.project?.samples ?? [];
  }

  /** Position of the active part, -1 without one. */
  get index(): number {
    const id = this.#active();
    return id === null ? -1 : this.samples.findIndex((s) => s.id === id);
  }

  /** The active part, if any. */
  get sample(): Sample | undefined {
    return this.samples[this.index];
  }

  /** Results of part `id`, for the delete question. */
  resultCount(id: SampleId): number {
    return (this.#project.project?.results ?? []).filter((r) => r.sample === id).length;
  }

  async #send(command: Command): Promise<boolean> {
    const patch = await this.#project.execute(command);
    if (patch !== undefined) {
      this.notice = null;
      return true;
    }
    const error: CommandError | null = this.#project.error;
    this.#project.dismissError();
    this.notice = error === null ? null : refusalDetail(error);
    return false;
  }

  async add(): Promise<void> {
    const n = Number.isInteger(this.count) ? this.count : Number.NaN;
    if (Number.isNaN(n)) {
      this.notice = m.refusal_invalid_sample_count();
      return;
    }
    await this.#send({ type: "add_samples", count: n });
  }

  /** Moves the active part one place; inserting before the target, moving right skips one. */
  async moveBy(step: -1 | 1): Promise<void> {
    const index = this.index;
    const sample = this.sample;
    if (sample === undefined) {
      return;
    }
    if (step < 0 && index <= 0) {
      return;
    }
    if (step > 0 && index >= this.samples.length - 1) {
      return;
    }
    const before = step < 0 ? this.samples[index - 1]?.id : this.samples[index + 2]?.id;
    await this.#send({ type: "move_samples", ids: [sample.id], before: before ?? null });
  }

  /** Deletes the active part; one with results waits in {@link confirming} first. */
  async remove(): Promise<void> {
    const sample = this.sample;
    if (sample === undefined) {
      return;
    }
    if (this.resultCount(sample.id) > 0 && this.confirming !== sample.id) {
      this.confirming = sample.id;
      return;
    }
    this.confirming = null;
    await this.#send({ type: "delete_samples", ids: [sample.id] });
  }

  /** Keeps the part the question was about. */
  keep(): void {
    this.confirming = null;
  }
}

export const parts = new PartsStore();

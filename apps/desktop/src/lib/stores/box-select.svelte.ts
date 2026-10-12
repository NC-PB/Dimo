/**
 * Box select (T2.6, FR-REC-01, FR-REC-02, ADR 0006): the proposals read from a box drawn with
 * the place tool, shown in a card next to the box until the user accepts or discards them.
 *
 * View state only. Rust reads the PDF text, parses it and interprets it with the tolerance
 * engine (`propose_from_region`), explains the limits in the UI language, reads edited text
 * and the shorter leg of an angle again (`read_callout_text`, T2.9a), and accepting is one
 * document command (`accept_proposals`), so it is one undo step. Discarding changes nothing.
 */

import {
  commands,
  type BalloonPlacement,
  type CharId,
  type CommandError,
  type ExplainLanguage,
  type FieldValue,
  type InterpreterNote,
  type OrientedBox,
  type Proposal,
  type ProposalView,
  type SheetId,
  type TypedCallout,
  type Unit,
} from "$lib/ipc/bindings";
import { getLocale } from "$lib/i18n";
import type { Rect } from "$lib/viewport/view-math";
import { projectStore, type ProjectStore } from "./project.svelte";
import { selection, type SelectionStore } from "./selection.svelte";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The recognition commands the store calls. The generated `commands` in the app. */
export interface RecognitionCommands {
  proposeFromRegion(
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
    language: ExplainLanguage,
  ): Promise<Result<ProposalView[]>>;
  readCalloutText(
    sheet: SheetId,
    text: string,
    shorterLeg: string | null,
    language: ExplainLanguage,
  ): Promise<Result<TypedCallout>>;
}

/** The shorter leg of a stored proposal or characteristic as text for its field (T2.9a). */
export function legText(leg: { value: string } | null | undefined): string {
  return leg?.value ?? "";
}

/** The language of explanations: the UI language. */
export function explainLanguage(): ExplainLanguage {
  return getLocale() === "de" ? "de" : "en";
}

/**
 * The proposal with the values Rust read from its text. A text that does not parse keeps only
 * the text and the parse error, like a box selection of it (spec 08 stage 6).
 */
export function withReading(proposal: Proposal, reading: TypedCallout): Proposal {
  const next: Proposal = { ...proposal, parse_error: reading.parse_error, parse_hints: [] };
  if (reading.parse_error !== null) {
    Object.assign(next, {
      nominal: null,
      unit: null,
      upper_dev: null,
      lower_dev: null,
      upper_limit: null,
      lower_limit: null,
      fit: null,
      derivation: null,
      quantity: 1,
      inspect: true,
    } satisfies Partial<Proposal>);
  }
  for (const value of reading.values) {
    apply(next, value);
  }
  return next;
}

function apply(p: Proposal, value: FieldValue): void {
  switch (value.field) {
    case "kind":
      p.kind = value.value;
      break;
    case "requirement_text":
      p.requirement_text = value.value;
      break;
    case "nominal":
      p.nominal = value.value;
      break;
    case "unit":
      p.unit = value.value;
      break;
    case "upper_dev":
      p.upper_dev = value.value;
      break;
    case "lower_dev":
      p.lower_dev = value.value;
      break;
    case "upper_limit":
      p.upper_limit = value.value;
      break;
    case "lower_limit":
      p.lower_limit = value.value;
      break;
    case "fit":
      p.fit = value.value;
      break;
    case "quantity":
      p.quantity = value.value;
      break;
    case "inspect":
      p.inspect = value.value;
      break;
    case "derivation":
      p.derivation = value.value;
      break;
    case "shorter_leg":
      p.shorter_leg = value.value;
      break;
    default:
      // Other fields (classification, inspection, comment) are not part of a reading.
      break;
  }
}

export class BoxSelectStore {
  /** Proposals of the open card, empty when no card is open. */
  proposals = $state.raw<Proposal[]>([]);
  /** Explanation of each proposal's limits in the UI language, from Rust (FR-TOL-08). */
  explanations = $state.raw<(string | null)[]>([]);
  /** What the tolerance engine could not decide, per proposal. Shown only, never stored. */
  notes = $state.raw<InterpreterNote[][]>([]);
  /** The shorter leg of each proposal as typed, in the sheet unit (T2.9a). */
  legs = $state.raw<string[]>([]);
  /** Why Rust refused the typed shorter leg of each proposal, `null` if it did not. */
  legErrors = $state.raw<(CommandError | null)[]>([]);
  /** The box the proposals were read from, in sheet space. */
  region = $state.raw<Rect | null>(null);
  /** True while Rust reads the box or edited text. */
  busy = $state(false);

  readonly #project: ProjectStore;
  readonly #selection: SelectionStore;
  readonly #api: RecognitionCommands;
  /** Sheet of the open card. */
  #sheet: SheetId | null = null;
  /** The text and leg each proposal's values were read from, to see edits not read yet. */
  #read: { text: string; leg: string }[] = [];

  constructor(project: ProjectStore, chosen: SelectionStore, api: RecognitionCommands = commands) {
    this.#project = project;
    this.#selection = chosen;
    this.#api = api;
  }

  /** True while the card is open. */
  get open(): boolean {
    return this.proposals.length > 0;
  }

  /** The sheet of the open card, `null` when closed. */
  get sheet(): SheetId | null {
    return this.#sheet;
  }

  /** Unit of the card's sheet: the unit the shorter leg is typed in (T2.9a, D-20). */
  get sheetUnit(): Unit | null {
    const id = this.#sheet;
    return this.#project.sheets.find((s) => s.id === id)?.unit ?? null;
  }

  /**
   * Reads the text inside `rect` of `sheet` and opens the card with the proposals. Returns false,
   * leaving no card, when the box has no PDF text or Rust refuses; the caller falls back to
   * manual capture.
   */
  async select(sheet: SheetId, rect: Rect, box: OrientedBox, placement: BalloonPlacement) {
    this.discard();
    this.busy = true;
    try {
      const result = await this.#api.proposeFromRegion(sheet, box, placement, explainLanguage());
      if (result.status === "error" || result.data.length === 0) {
        return false;
      }
      this.#sheet = sheet;
      this.region = rect;
      this.proposals = result.data.map((v) => v.proposal);
      this.explanations = result.data.map((v) => v.explanation);
      this.notes = result.data.map((v) => v.notes);
      this.legs = this.proposals.map((p) => legText(p.shorter_leg));
      this.legErrors = this.proposals.map(() => null);
      this.#read = this.proposals.map((p, i) => ({
        text: p.requirement_text,
        leg: this.legs[i] ?? "",
      }));
      return true;
    } catch {
      return false;
    } finally {
      this.busy = false;
    }
  }

  /** Changes fields of one proposal in the card, for example its kind. */
  edit(index: number, change: Partial<Proposal>): void {
    this.proposals = this.proposals.map((p, i) => (i === index ? { ...p, ...change } : p));
  }

  /** Changes the typed shorter leg of one proposal; read with {@link reread} (T2.9a). */
  editLeg(index: number, text: string): void {
    this.legs = this.legs.map((l, i) => (i === index ? text : l));
  }

  /**
   * Reads the edited requirement text and shorter leg of a proposal again through Rust (M2
   * decision 5, T2.9a). A leg Rust refuses leaves the values as they were and shows why.
   */
  async reread(index: number): Promise<void> {
    const p = this.proposals[index];
    const sheet = this.#sheet;
    const leg = this.legs[index] ?? "";
    const read = this.#read[index];
    if (
      p === undefined ||
      sheet === null ||
      (read?.text === p.requirement_text && read.leg === leg)
    ) {
      return;
    }
    const text = p.requirement_text;
    const result = await this.#api
      .readCalloutText(sheet, text, leg.trim() === "" ? null : leg, explainLanguage())
      .catch(() => null);
    const current = this.proposals[index];
    if (current?.requirement_text !== text || (this.legs[index] ?? "") !== leg) {
      return;
    }
    if (result?.status === "error") {
      this.legErrors = this.legErrors.map((e, i) => (i === index ? result.error : e));
      return;
    }
    if (result?.status !== "ok") {
      return;
    }
    this.#read[index] = { text, leg };
    this.legErrors = this.legErrors.map((e, i) => (i === index ? null : e));
    this.edit(index, withReading(current, result.data));
    this.explanations = this.explanations.map((e, i) =>
      i === index ? result.data.explanation : e,
    );
    this.notes = this.notes.map((n, i) => (i === index ? result.data.notes : n));
  }

  /**
   * Accepts every proposal of the card as one undoable command (ADR 0006) and selects the new
   * characteristics. Edited text is read again first. With numbering locked the new numbers
   * follow the primary selected characteristic (FR-BAL-11). Returns the new characteristic IDs.
   */
  async accept(): Promise<CharId[]> {
    if (!this.open) {
      return [];
    }
    for (let i = 0; i < this.proposals.length; i++) {
      await this.reread(i);
    }
    const proposals = this.proposals;
    this.discard();
    const primary = this.#selection.primary;
    const patch = await this.#project.execute({
      type: "accept_proposals",
      proposals,
      insert_after:
        primary !== null && this.#project.characteristicById.has(primary) ? primary : null,
    });
    const ids = (patch?.changes ?? []).flatMap((c) =>
      c.type === "characteristic_inserted" ? [c.characteristic.id] : [],
    );
    const [first, ...rest] = ids;
    if (first !== undefined) {
      this.#selection.focus(first, "viewport");
      this.#selection.add(rest);
    }
    return ids;
  }

  /** Closes the card without changing the project. */
  discard(): void {
    this.proposals = [];
    this.explanations = [];
    this.notes = [];
    this.legs = [];
    this.legErrors = [];
    this.region = null;
    this.#sheet = null;
    this.#read = [];
  }
}

/** Box select of the app window. */
export const boxSelect = new BoxSelectStore(projectStore, selection);

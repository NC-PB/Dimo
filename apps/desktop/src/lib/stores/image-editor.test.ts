import { describe, expect, it } from "vitest";
import type {
  CornerStart,
  ImageEdit,
  ImageEditOutcome,
  ImagePreview,
  PreviewStage,
  ProjectPatched,
} from "$lib/ipc/bindings";
import type { Result } from "$lib/ipc/result";
import { DEFAULT_EDIT } from "$lib/image-edit";
import { ImageEditorStore, type ImageEditorApi } from "./image-editor.svelte";

const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: "ok", data });

const CORNERS: CornerStart = {
  proposed: {
    top_left: { x: "10", y: "12" },
    top_right: { x: "300", y: "8" },
    bottom_right: { x: "310", y: "220" },
    bottom_left: { x: "5", y: "230" },
  },
  page: {
    top_left: { x: "0", y: "0" },
    top_right: { x: "320", y: "0" },
    bottom_right: { x: "320", y: "240" },
    bottom_left: { x: "0", y: "240" },
  },
};

/** Rust's side of the editor: records previews and applies, answers as set. */
function fakeRust(pages: ImageEdit[] | null = [DEFAULT_EDIT]) {
  const previews: { edit: ImageEdit; stage: PreviewStage }[] = [];
  const applies: { edit: ImageEdit; move: boolean }[] = [];
  let closed = 0;
  let outcome: ImageEditOutcome = { type: "off_sheet", balloons: 2 };
  const preview = (stage: PreviewStage): ImagePreview => ({
    doc: { doc: `doc-${stage}`, name: "", sheets: [{ width: 842, height: 595 }] },
    to_view: [1, 0, 0, 0, 1, 0, 0, 0, 1],
    from_view: [1, 0, 0, 0, 1, 0, 0, 0, 1],
  });
  const api: ImageEditorApi = {
    previewImageEdit: (_page, edit, stage) => {
      previews.push({ edit, stage });
      return ok(preview(stage));
    },
    closeImagePreview: () => {
      closed += 1;
      return ok(null);
    },
    imageCornerStart: () => ok(CORNERS),
    applyImageEdit: (_page, edit, move) => {
      applies.push({ edit, move });
      return ok(outcome);
    },
  };
  const project = {
    imagePages: pages,
    editWith: async <T>(send: () => Promise<Result<T>>, patchedOf: (a: T) => unknown) => {
      const result = await send();
      if (result.status === "error") {
        return undefined;
      }
      patchedOf(result.data);
      return result.data;
    },
  };
  const editor = new ImageEditorStore({
    api,
    project,
    shownSheet: () => 0,
    later: (work) => {
      work();
    },
  });
  return {
    editor,
    previews,
    applies,
    closed: () => closed,
    answer: (next: ImageEditOutcome) => {
      outcome = next;
    },
  };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("image editor store (R.20)", () => {
  it("is not available for a PDF", () => {
    const { editor } = fakeRust(null);
    expect(editor.available).toBe(false);
    editor.show();
    expect(editor.open).toBe(false);
  });

  it("previews the stage of the tool and sends decimal strings", async () => {
    const { editor, previews } = fakeRust();
    editor.show();
    await settle();
    expect(editor.preview?.doc.doc).toBe("doc-result");
    editor.turn(1);
    editor.setAngle(2.345);
    await settle();
    expect(editor.edit.rotation).toBe("deg90");
    expect(editor.edit.fine_angle).toBe("2.35");
    editor.setAngle(99);
    expect(editor.edit.fine_angle).toBe("45");
    editor.setTool("trim");
    await settle();
    expect(previews.at(-1)?.stage).toBe("trim");
    expect(editor.changed).toBe(true);
    editor.reset();
    expect(editor.changed).toBe(false);
  });

  it("starts the corners at the proposal and sets the fine angle back to 0", async () => {
    const { editor } = fakeRust();
    editor.show();
    editor.setAngle(3);
    editor.setTool("corners");
    await settle();
    expect(editor.edit.corners).toEqual(CORNERS.proposed);
    expect(editor.edit.fine_angle).toBe("0");
    editor.moveCorner("top_left", { x: "1", y: "2" });
    expect(editor.edit.corners?.top_left).toEqual({ x: "1", y: "2" });
    // A quarter turn keeps the points and renames them.
    editor.turn(1);
    expect(editor.edit.corners?.top_right).toEqual({ x: "1", y: "2" });
    await editor.setCornersOn(false);
    expect(editor.edit.corners).toBeNull();
  });

  it("asks before balloons leave the sheet and applies on the answer", async () => {
    const rust = fakeRust();
    const { editor, applies } = rust;
    editor.show();
    editor.setCleanUp(true);
    expect(await editor.apply()).toBe(false);
    expect(editor.offSheet).toBe(2);
    expect(editor.open).toBe(true);
    const patched = { patch: { changes: [] } } as unknown as ProjectPatched;
    rust.answer({ type: "applied", patched });
    await editor.answerOffSheet(true);
    expect(applies.map((a) => a.move)).toEqual([false, true]);
    expect(applies[1]?.edit.clean_up).toBe(true);
    expect(editor.open).toBe(false);
    expect(rust.closed()).toBe(1);
  });

  it("closes without a call to Rust when nothing changed", async () => {
    const { editor, applies } = fakeRust();
    editor.show();
    expect(await editor.apply()).toBe(true);
    expect(applies).toEqual([]);
    expect(editor.open).toBe(false);
  });
});

describe("image editor previews (R.20)", () => {
  it("keeps one preview request in flight and asks again for the latest settings", async () => {
    const answers: ((r: Result<ImagePreview>) => void)[] = [];
    const sent: ImageEdit[] = [];
    const preview: ImagePreview = {
      doc: { doc: "d", name: "", sheets: [{ width: 1, height: 1 }] },
      to_view: [1, 0, 0, 0, 1, 0, 0, 0, 1],
      from_view: [1, 0, 0, 0, 1, 0, 0, 0, 1],
    };
    const editor = new ImageEditorStore({
      api: {
        previewImageEdit: (_page, edit) => {
          sent.push(edit);
          return new Promise((resolve) => answers.push(resolve));
        },
        closeImagePreview: () => ok(null),
        imageCornerStart: () => ok(CORNERS),
        applyImageEdit: () => ok({ type: "off_sheet", balloons: 0 }),
      },
      project: { imagePages: [DEFAULT_EDIT], editWith: () => Promise.resolve(undefined) },
      shownSheet: () => 0,
      later: (work) => {
        work();
      },
    });
    editor.show();
    editor.setAngle(1);
    editor.setAngle(2);
    expect(sent).toHaveLength(1);
    answers[0]?.({ status: "ok", data: preview });
    await settle();
    expect(sent).toHaveLength(2);
    expect(sent[1]?.fine_angle).toBe("2");
  });
});

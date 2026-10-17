// @vitest-environment happy-dom
/**
 * "Check for update" in File > About (R.9, ADR 0008): only a click asks Rust, every answer has
 * its own message, a policy file can hide the button (NFR-SEC-03).
 */
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";

import type { UpdateCheck } from "$lib/ipc/bindings";
import type { Result } from "$lib/ipc/result";
import AboutPane from "./AboutPane.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement | null = null;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target?.remove();
});

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

interface Fakes {
  allowed?: boolean;
  answer?: Result<UpdateCheck>;
  opened?: string[];
  openResult?: Result<null>;
}

async function mountWith(fakes: Fakes): Promise<{ root: HTMLElement; checks: () => number }> {
  let checks = 0;
  target = document.createElement("div");
  document.body.append(target);
  component = mount(AboutPane, {
    target,
    props: {
      fetchDiagnostics: () => Promise.resolve({ status: "ok", data: "" }),
      updateCheckAllowed: () => Promise.resolve({ status: "ok", data: fakes.allowed ?? true }),
      checkForUpdate: () => {
        checks += 1;
        return Promise.resolve(fakes.answer ?? { status: "ok", data: { kind: "no_release" } });
      },
      openReleasePage: (url: string) => {
        fakes.opened?.push(url);
        return Promise.resolve(fakes.openResult ?? { status: "ok", data: null });
      },
    },
  });
  await tick();
  flushSync();
  return { root: target, checks: () => checks };
}

function button(root: HTMLElement, id: string): HTMLButtonElement | null {
  return root.querySelector<HTMLButtonElement>(`[data-testid="${id}"]`);
}

async function click(root: HTMLElement, id: string): Promise<string> {
  button(root, id)?.click();
  await tick();
  flushSync();
  return root.querySelector('[data-testid="update-status"]')?.textContent?.trim() ?? "";
}

describe("Check for update (R.9, ADR 0008)", () => {
  it("asks nothing until the button is clicked", async () => {
    const { root, checks } = await mountWith({});
    expect(button(root, "check-update")).not.toBeNull();
    expect(checks()).toBe(0);
    await click(root, "check-update");
    expect(checks()).toBe(1);
  });

  it("is hidden when the policy turns it off", async () => {
    const { root, checks } = await mountWith({ allowed: false });
    expect(button(root, "check-update")).toBeNull();
    expect(checks()).toBe(0);
  });

  it("shows a newer version and opens its release page", async () => {
    const opened: string[] = [];
    const url = "https://github.com/NC-PB/Dimo/releases/tag/v0.2.0";
    const { root } = await mountWith({
      answer: { status: "ok", data: { kind: "available", version: "0.2.0", url } },
      opened,
    });
    expect(await click(root, "check-update")).toBe("DimoMark 0.2.0 is available.");
    button(root, "open-release")?.click();
    await tick();
    expect(opened).toEqual([url]);
  });

  it("shows the address when the browser cannot be opened", async () => {
    const url = "https://github.com/NC-PB/Dimo/releases/tag/v0.2.0";
    const { root } = await mountWith({
      answer: { status: "ok", data: { kind: "available", version: "0.2.0", url } },
      openResult: {
        status: "error",
        error: { kind: "export", reason: "open_failed", message: "x" } as never,
      },
    });
    await click(root, "check-update");
    button(root, "open-release")?.click();
    await tick();
    flushSync();
    expect(root.querySelector('[data-testid="open-release-failed"]')?.textContent).toContain(url);
  });

  it.each<[UpdateCheck, string]>([
    [{ kind: "latest", version: "0.1.0" }, "You have the latest version (0.1.0)."],
    [{ kind: "no_release" }, "No version of DimoMark has been published yet."],
    [{ kind: "offline" }, "GitHub could not be reached."],
    [{ kind: "failed", detail: "HTTP status 403" }, "The update check failed: HTTP status 403"],
    [{ kind: "disabled" }, "Your organization has turned off the update check."],
  ])("shows a message for %o", async (data, expected) => {
    const { root } = await mountWith({ answer: { status: "ok", data } });
    expect(await click(root, "check-update")).toContain(expected);
    expect(button(root, "open-release")).toBeNull();
  });
});

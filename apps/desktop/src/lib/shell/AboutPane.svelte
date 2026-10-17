<script lang="ts">
  /**
   * File > About (M3 decision 29, T3.16): the DimoMark version, its Apache 2.0 license, the NOTICE
   * file, for every bundled font and icon set the copyright notice with the full license text,
   * and the generated list of all other third party components (THIRD_PARTY_LICENSES, T3.23).
   * The texts are embedded in the app (`about.ts`), so they show without network. License texts
   * are legal documents and stay in their original English.
   *
   * "Copy diagnostics" (D-06, R.10) puts the text of the `diagnostics` command on the clipboard
   * for a bug report. Rust builds the text and leaves out personal data and drawing content.
   *
   * "Check for update" (R.9, ADR 0008) asks Rust to make one request to GitHub, only on click.
   * The button is missing when an organization policy file turns the check off (NFR-SEC-03).
   */
  import { BUNDLED, DIMO_LICENSE, DIMO_NOTICE, loadThirdPartyLicenses } from "$lib/about";
  import { asCommandError, commandErrorMessage } from "$lib/errors";
  import { buildProfileLabel, m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import { commands, type UpdateCheck } from "$lib/ipc/bindings";
  import type { Result } from "$lib/ipc/result";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { buttonClass } from "$lib/ui/fields";

  interface Props {
    /** The diagnostics text from Rust; tests pass a fake. */
    fetchDiagnostics?: () => Promise<Result<string>>;
    /** Whether the policy allows the update check; tests pass a fake. */
    updateCheckAllowed?: () => Promise<Result<boolean>>;
    /** The update check in Rust, the only network request of DimoMark; tests pass a fake. */
    checkForUpdate?: () => Promise<Result<UpdateCheck>>;
    /** Opens the release page in the browser; tests pass a fake. */
    openReleasePage?: (url: string) => Promise<Result<null>>;
  }

  let {
    fetchDiagnostics = commands.diagnostics,
    updateCheckAllowed = commands.updateCheckAllowed,
    checkForUpdate = commands.checkForUpdate,
    openReleasePage = commands.openReleasePage,
  }: Props = $props();

  const info = $derived(appInfo.current);

  let copied = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy(id: string, text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      return; // The clipboard may be unavailable; the text can still be selected by hand.
    }
    copied = id;
    clearTimeout(timer);
    timer = setTimeout(() => {
      copied = null;
    }, 1500);
  }

  /** Result of the last "Copy diagnostics": copied, or the reason it failed. */
  let diagnostics = $state<{ ok: true } | { ok: false; reason: string } | null>(null);

  /**
   * The diagnostics text, loaded when the pane opens. WebKit writes to the clipboard only right
   * after a click, not after waiting for Rust, so the click copies this text at once and then
   * loads a newer one for the next click.
   */
  let diagnosticsText: Result<string> | null = null;

  async function loadDiagnostics(): Promise<Result<string>> {
    try {
      diagnosticsText = await fetchDiagnostics();
    } catch (error) {
      diagnosticsText = { status: "error", error: asCommandError(error) };
    }
    return diagnosticsText;
  }

  $effect(() => {
    void loadDiagnostics();
  });

  async function copyDiagnostics(): Promise<void> {
    diagnostics = null;
    const result = diagnosticsText ?? (await loadDiagnostics());
    if (result.status === "error") {
      diagnostics = { ok: false, reason: commandErrorMessage(result.error) };
      void loadDiagnostics();
      return;
    }
    try {
      // Called before anything else awaits, so it still counts as part of the click.
      await navigator.clipboard.writeText(result.data);
      diagnostics = { ok: true };
    } catch {
      diagnostics = { ok: false, reason: m.about_diagnostics_no_clipboard() };
    }
    void loadDiagnostics();
  }

  /** THIRD_PARTY_LICENSES, loaded when the pane opens (T3.23). */
  let thirdParty = $state<string | null>(null);

  $effect(() => {
    void loadThirdPartyLicenses().then((text) => {
      thirdParty = text;
    });
  });

  /** Whether "Check for update" is shown; asked once when the pane opens, no network. */
  let updateShown = $state(false);

  $effect(() => {
    updateCheckAllowed().then(
      (result) => (updateShown = result.status === "ok" && result.data),
      () => (updateShown = false),
    );
  });

  /** State of the last update check: running, its result, or the failed command. */
  let update = $state<
    { kind: "checking" } | UpdateCheck | { kind: "error"; reason: string } | null
  >(null);
  let openFailed = $state<string | null>(null);

  async function checkUpdate(): Promise<void> {
    update = { kind: "checking" };
    openFailed = null;
    try {
      const result = await checkForUpdate();
      update =
        result.status === "ok"
          ? result.data
          : { kind: "error", reason: commandErrorMessage(result.error) };
    } catch (error) {
      update = { kind: "error", reason: commandErrorMessage(asCommandError(error)) };
    }
  }

  async function openRelease(url: string): Promise<void> {
    openFailed = null;
    let ok: boolean;
    try {
      ok = (await openReleasePage(url)).status === "ok";
    } catch {
      ok = false;
    }
    if (!ok) openFailed = url;
  }

  const boxClass = "flex flex-col gap-2 rounded-xl border border-border bg-surface p-4 shadow-sm";
  const textClass =
    "max-h-64 overflow-auto rounded-md border border-border bg-sunken p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-text";
  const CopyIcon = icons.copy;
</script>

{#snippet licenseText(id: string, label: string, text: string)}
  <div class="flex items-center justify-between gap-2">
    <span class="text-xs text-text-muted">{label}</span>
    <button
      type="button"
      class="flex min-h-8 items-center gap-1 rounded-md border border-border px-2 py-0.5 text-xs hover:bg-hover"
      aria-label={m.about_copy_aria({ name: label })}
      onclick={() => void copy(id, text)}
    >
      <CopyIcon size={14} strokeWidth={1.75} aria-hidden="true" class="text-text-muted" />
      {copied === id ? m.copy_hash_done() : m.about_copy()}
    </button>
  </div>
  <!-- Focusable so the text scrolls with the keyboard. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <pre class={textClass} tabindex="0" aria-label={label} data-license={id}>{text}</pre>
{/snippet}

<div class="flex max-w-3xl flex-col gap-5" data-testid="about-pane">
  <div>
    <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.about_title()}</h1>
    <p class="mt-1 text-text-muted">{m.about_lead()}</p>
  </div>

  <section class={boxClass} aria-labelledby="about-dimo">
    <h2 id="about-dimo" class="font-heading text-base font-semibold">DimoMark</h2>
    {#if info}
      <p data-testid="about-version">
        {m.app_version({ version: info.version })}, {buildProfileLabel(info.build_profile)}
      </p>
    {/if}
    {#if updateShown}
      <p class="text-text-muted">{m.about_update_text()}</p>
      <div class="flex flex-wrap items-center gap-3">
        <button
          type="button"
          class="flex min-h-8 items-center gap-1 rounded-md border border-border px-3 py-1 text-sm hover:bg-hover disabled:opacity-60"
          data-testid="check-update"
          disabled={update?.kind === "checking"}
          onclick={() => void checkUpdate()}
        >
          {m.about_update_check()}
        </button>
        {#if update?.kind === "checking"}
          <span role="status" class="text-sm text-text-muted" data-testid="update-status">
            {m.about_update_checking()}
          </span>
        {:else if update?.kind === "available"}
          <span role="status" class="text-sm font-medium" data-testid="update-status">
            {m.about_update_available({ version: update.version })}
          </span>
          <button
            type="button"
            class={buttonClass.primary}
            data-testid="open-release"
            onclick={() => update?.kind === "available" && void openRelease(update.url)}
          >
            {m.about_update_open()}
          </button>
        {:else if update?.kind === "latest"}
          <span role="status" class="text-sm" data-testid="update-status">
            {m.about_update_latest({ version: update.version })}
          </span>
        {:else if update?.kind === "no_release"}
          <span role="status" class="text-sm" data-testid="update-status">
            {m.about_update_no_release()}
          </span>
        {:else if update?.kind === "disabled"}
          <span role="status" class="text-sm" data-testid="update-status">
            {m.about_update_disabled()}
          </span>
        {:else if update?.kind === "offline"}
          <span role="alert" class="text-sm text-danger" data-testid="update-status">
            {m.about_update_offline()}
          </span>
        {:else if update?.kind === "failed" || update?.kind === "error"}
          <span role="alert" class="text-sm text-danger" data-testid="update-status">
            {m.about_update_failed({
              detail: update.kind === "failed" ? update.detail : update.reason,
            })}
          </span>
        {/if}
      </div>
      {#if openFailed}
        <p role="alert" class="text-sm break-all text-danger" data-testid="open-release-failed">
          {m.about_update_open_failed({ url: openFailed })}
        </p>
      {/if}
    {/if}
    <p class="text-text-muted">{m.about_license()}</p>
    {@render licenseText("dimo", "Apache License 2.0", DIMO_LICENSE)}
    {@render licenseText("notice", "NOTICE", DIMO_NOTICE)}
  </section>

  <section class={boxClass} aria-labelledby="about-diagnostics">
    <h2 id="about-diagnostics" class="font-heading text-base font-semibold">
      {m.about_diagnostics()}
    </h2>
    <p class="text-text-muted">{m.about_diagnostics_text()}</p>
    <div class="flex flex-wrap items-center gap-3">
      <button
        type="button"
        class="flex min-h-8 items-center gap-1 rounded-md border border-border px-3 py-1 text-sm hover:bg-hover"
        data-testid="copy-diagnostics"
        onclick={() => void copyDiagnostics()}
      >
        <CopyIcon size={16} strokeWidth={1.75} aria-hidden="true" class="text-text-muted" />
        {m.about_diagnostics_copy()}
      </button>
      {#if diagnostics?.ok === true}
        <span role="status" class="text-sm" data-testid="diagnostics-status">
          {m.about_diagnostics_copied()}
        </span>
      {:else if diagnostics?.ok === false}
        <span role="alert" class="text-sm text-danger" data-testid="diagnostics-status">
          {m.about_diagnostics_failed({ reason: diagnostics.reason })}
        </span>
      {/if}
    </div>
  </section>

  <section class="flex flex-col gap-3" aria-labelledby="about-bundled">
    <div>
      <h2 id="about-bundled" class="font-heading text-xl font-semibold">{m.about_bundled()}</h2>
      <p class="mt-1 text-text-muted">{m.about_bundled_lead()}</p>
    </div>
    {#each BUNDLED as asset (asset.id)}
      <article class={boxClass} aria-labelledby="about-{asset.id}">
        <h3 id="about-{asset.id}" class="font-heading font-semibold">{asset.name}</h3>
        <dl class="grid grid-cols-[minmax(6rem,10rem)_minmax(0,1fr)] gap-x-3 gap-y-1 text-sm">
          <dt class="text-text-muted">{m.about_copyright()}</dt>
          <dd>{asset.copyright}</dd>
          <dt class="text-text-muted">{m.about_license_name()}</dt>
          <dd>{asset.license}</dd>
          {#if asset.files.length > 0}
            <dt class="text-text-muted">{m.about_files()}</dt>
            <dd class="font-mono text-xs break-all">{asset.files.join(", ")}</dd>
          {/if}
        </dl>
        {@render licenseText(asset.id, asset.license, asset.text)}
      </article>
    {/each}
  </section>

  <section class={boxClass} aria-labelledby="about-third-party">
    <h2 id="about-third-party" class="font-heading text-base font-semibold">
      {m.about_third_party()}
    </h2>
    <p class="text-text-muted">{m.about_third_party_text()}</p>
    {#if thirdParty !== null}
      {@render licenseText("third-party", "THIRD_PARTY_LICENSES", thirdParty)}
    {/if}
  </section>
</div>

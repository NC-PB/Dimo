<script lang="ts">
  /**
   * The File page (ADR 0007, mockup "File view", T3.16): a rail on the left with Back, the
   * project commands and the panes Info, Open (with the recent projects), Options and About;
   * the pane on the right. Commands come from the registry, so labels, icons, tooltips and
   * disabled reasons are the same as on the ribbon and the title bar. The app version stands
   * below the rail (R.11).
   */
  import {
    command,
    commandTooltip,
    disabledReason,
    runCommand,
    type CommandId,
  } from "$lib/commands.svelte";
  import ExportView from "$lib/components/ExportView.svelte";
  import RecentProjects from "$lib/components/RecentProjects.svelte";
  import { m } from "$lib/i18n";
  import { icons, type Icon } from "$lib/icons";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { view, type FilePane } from "$lib/stores/view.svelte";
  import AboutPane from "./AboutPane.svelte";
  import CommandButton from "./CommandButton.svelte";
  import FileInfo from "./FileInfo.svelte";
  import OptionsPane from "./OptionsPane.svelte";

  const railClass =
    "flex min-h-10 w-full items-center gap-2.5 rounded-lg px-3.5 py-2 text-left text-sm text-text hover:bg-hover aria-disabled:cursor-default aria-disabled:opacity-45 aria-disabled:hover:bg-transparent aria-[current=page]:bg-accent-soft aria-[current=page]:font-semibold";

  /** Commands that open a pane of this page show as current while it is shown. */
  const PANE_OF: Partial<Record<CommandId, FilePane>> = {
    export_report: "export",
    options: "options",
    about: "about",
  };
</script>

{#snippet railItem(label: string, Glyph: Icon, pane: FilePane)}
  <button
    type="button"
    class={railClass}
    aria-current={view.filePane === pane ? "page" : undefined}
    data-pane={pane}
    onclick={() => {
      view.filePane = pane;
    }}
  >
    <Glyph size={18} strokeWidth={1.75} aria-hidden="true" class="shrink-0 text-text-muted" />
    {label}
  </button>
{/snippet}

{#snippet railCommand(id: CommandId)}
  {@const def = command(id)}
  {@const reason = disabledReason(id)}
  {@const pane = PANE_OF[id]}
  <button
    type="button"
    class={railClass}
    title={commandTooltip(id)}
    aria-disabled={reason !== null ? "true" : undefined}
    aria-current={pane !== undefined && view.filePane === pane ? "page" : undefined}
    data-command={id}
    onclick={() => {
      runCommand(id);
    }}
  >
    <def.icon size={18} strokeWidth={1.75} aria-hidden="true" class="shrink-0 text-text-muted" />
    {def.label()}
  </button>
{/snippet}

<div
  id="tab-panel"
  class="flex min-h-0 flex-1 bg-bg text-sm text-text"
  role="tabpanel"
  aria-labelledby="tab-file"
>
  <nav
    class="flex w-56 shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-border bg-surface px-2 py-3"
    aria-label={m.tab_file()}
  >
    <button
      type="button"
      class={railClass}
      data-file="back"
      onclick={() => {
        view.back();
      }}
    >
      <icons.back
        size={18}
        strokeWidth={1.75}
        aria-hidden="true"
        class="shrink-0 text-text-muted"
      />
      {m.file_back()}
    </button>
    <hr class="mx-3.5 my-2 border-border" />
    {@render railItem(m.file_info(), icons.projectInfo, "info")}
    {@render railCommand("new_project")}
    {@render railItem(m.cmd_open(), icons.openProject, "open")}
    {@render railCommand("save")}
    {@render railCommand("save_as")}
    <hr class="mx-3.5 my-2 border-border" />
    {@render railCommand("export_report")}
    {@render railCommand("print")}
    {@render railCommand("close_project")}
    <span class="flex-1"></span>
    {@render railCommand("about")}
    {@render railCommand("options")}
    {#if appInfo.current}
      <p class="px-3.5 pt-2 text-xs text-text-muted" data-testid="file-version">
        {m.app_version({ version: appInfo.current.version })}
      </p>
    {/if}
  </nav>

  <main class="min-w-0 flex-1 overflow-y-auto px-6 py-8 lg:px-12">
    {#if view.filePane === "info"}
      <FileInfo />
    {:else if view.filePane === "open"}
      <div class="flex max-w-3xl flex-col gap-5" data-testid="file-open">
        <div>
          <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.cmd_open()}</h1>
          <p class="mt-1 text-text-muted">{m.file_open_lead()}</p>
        </div>
        <div class="flex flex-wrap gap-2">
          <CommandButton id="open_project" size="large" />
          <CommandButton id="new_project" size="large" />
        </div>
        <div class="rounded-xl border border-border bg-surface p-4 shadow-sm">
          <RecentProjects heading={m.recent_title()} />
        </div>
      </div>
    {:else if view.filePane === "export"}
      <ExportView />
    {:else if view.filePane === "options"}
      <OptionsPane />
    {:else}
      <AboutPane />
    {/if}
  </main>
</div>

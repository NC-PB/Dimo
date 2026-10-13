/**
 * The icons of the app (M3 decision 26): Lucide, ISC license, pinned in package.json. Each icon
 * is imported from its own file so only these ship. Add new icons here, never import Lucide
 * elsewhere, so the list of shipped icons stays in one place.
 */

import type { LucideProps } from "@lucide/svelte";
import type { Component } from "svelte";

import ArrowLeft from "@lucide/svelte/icons/arrow-left";
import ArrowRight from "@lucide/svelte/icons/arrow-right";
import CircleDashed from "@lucide/svelte/icons/circle-dashed";
import CircleHelp from "@lucide/svelte/icons/circle-help";
import CirclePlus from "@lucide/svelte/icons/circle-plus";
import CircleX from "@lucide/svelte/icons/circle-x";
import ClipboardList from "@lucide/svelte/icons/clipboard-list";
import ClipboardPaste from "@lucide/svelte/icons/clipboard-paste";
import Eraser from "@lucide/svelte/icons/eraser";
import ExternalLink from "@lucide/svelte/icons/external-link";
import Eye from "@lucide/svelte/icons/eye";
import FileCheck from "@lucide/svelte/icons/file-check";
import FileOutput from "@lucide/svelte/icons/file-output";
import FilePlus from "@lucide/svelte/icons/file-plus";
import FolderOpen from "@lucide/svelte/icons/folder-open";
import Grid3x3 from "@lucide/svelte/icons/grid-3x3";
import Hash from "@lucide/svelte/icons/hash";
import Info from "@lucide/svelte/icons/info";
import Keyboard from "@lucide/svelte/icons/keyboard";
import ListOrdered from "@lucide/svelte/icons/list-ordered";
import Maximize from "@lucide/svelte/icons/maximize";
import Moon from "@lucide/svelte/icons/moon";
import MousePointer2 from "@lucide/svelte/icons/mouse-pointer-2";
import Palette from "@lucide/svelte/icons/palette";
import PanelBottom from "@lucide/svelte/icons/panel-bottom";
import PanelRight from "@lucide/svelte/icons/panel-right";
import Pencil from "@lucide/svelte/icons/pencil";
import Plus from "@lucide/svelte/icons/plus";
import Redo2 from "@lucide/svelte/icons/redo-2";
import RefreshCw from "@lucide/svelte/icons/refresh-cw";
import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
import RotateCw from "@lucide/svelte/icons/rotate-cw";
import Save from "@lucide/svelte/icons/save";
import SaveAll from "@lucide/svelte/icons/save-all";
import Settings from "@lucide/svelte/icons/settings";
import SquareDashed from "@lucide/svelte/icons/square-dashed";
import Stamp from "@lucide/svelte/icons/stamp";
import Sun from "@lucide/svelte/icons/sun";
import Table2 from "@lucide/svelte/icons/table-2";
import Trash2 from "@lucide/svelte/icons/trash-2";
import Undo2 from "@lucide/svelte/icons/undo-2";
import ZoomIn from "@lucide/svelte/icons/zoom-in";
import ZoomOut from "@lucide/svelte/icons/zoom-out";

export type Icon = Component<LucideProps>;

export const icons = {
  addParts: Plus,
  balloonStyle: Palette,
  checkSheet: ClipboardList,
  clearCell: Eraser,
  companyTables: Table2,
  decimalRules: Hash,
  delete: Trash2,
  drawView: SquareDashed,
  editValue: Pencil,
  fit: Maximize,
  help: CircleHelp,
  inspectionReport: FileCheck,
  issueReport: Stamp,
  keyboard: Keyboard,
  moveLeft: ArrowLeft,
  moveRight: ArrowRight,
  newProject: FilePlus,
  onlyFailed: CircleX,
  onlyMissing: CircleDashed,
  openAfter: ExternalLink,
  openProject: FolderOpen,
  paste: ClipboardPaste,
  place: CirclePlus,
  previewNumbers: Eye,
  projectInfo: Info,
  redo: Redo2,
  renumber: ListOrdered,
  rotateLeft: RotateCcw,
  rotateRight: RotateCw,
  save: Save,
  saveAs: SaveAll,
  saveCopy: FileOutput,
  select: MousePointer2,
  settings: Settings,
  sidePanel: PanelRight,
  table: PanelBottom,
  themeDark: Moon,
  themeLight: Sun,
  undo: Undo2,
  updateLimits: RefreshCw,
  zoneGrid: Grid3x3,
  zoomIn: ZoomIn,
  zoomOut: ZoomOut,
} satisfies Record<string, Icon>;

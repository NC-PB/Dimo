# Projekte

Ein Projekt enthält eine Zeichnung, ihre Merkmale und Ballons sowie das Protokoll aller
Änderungen. Es wird als eine `.dimo`-Datei gespeichert. Die Zeichnung liegt unverändert in der
Datei, darum öffnet sich das Projekt auch dann, wenn das Original-PDF verschoben oder gelöscht
wurde.

## Anlegen, öffnen, speichern

| Aktion | Werkzeugleiste | macOS | Windows und Linux |
|---|---|---|---|
| Neues Projekt aus einer PDF-Zeichnung | Neues Projekt | ⌘N | Ctrl+N |
| Projekt öffnen | Projekt öffnen | ⌘O | Ctrl+O |
| Speichern | Speichern | ⌘S | Ctrl+S |
| Unter neuem Namen speichern | Speichern unter | ⇧⌘S | Ctrl+Shift+S |
| Rückgängig | ↶ | ⌘Z | Ctrl+Z |
| Wiederholen | ↷ | ⇧⌘Z | Ctrl+Shift+Z oder Ctrl+Y |

**Neues Projekt** und **Projekt öffnen** fragen mit dem Dateidialog des Betriebssystems nach einer
Datei. Ein Projekt entsteht aus einer PDF-Zeichnung; andere Dateitypen werden nicht angeboten. Ein
neues Projekt hat bis zum ersten Speichern keine Datei; die Werkzeugleiste zeigt "Unbenannt".
Beim ersten Speichern fragt Dimo nach einem Dateinamen und hängt `.dimo` an, wenn du es weglässt.
Rückgängig ist unbegrenzt, solange das Projekt offen ist; nach erneutem Öffnen beginnt der Verlauf
neu.

Hat ein Projekt ungespeicherte Änderungen, fragt Dimo vor dem Anlegen oder Öffnen eines anderen
Projekts und vor dem Schließen des Fensters oder dem Beenden von Dimo: **Speichern**,
**Nicht speichern** oder **Abbrechen**.

## Was in einer Projektdatei steht

Eine `.dimo`-Datei enthält die Zeichnung, wie du sie importiert hast, die Merkmale und Ballons,
die Einstellungen des Projekts (zum Beispiel den Ballonstil), Drehung, Einheit und Maßstab jedes
Blatts, die Nummerierungssperre und das Änderungsprotokoll: jede Änderung mit Zeit und dem
Benutzernamen aus den [Einstellungen](settings.md). Die Datei ist ein ZIP-Archiv, du kannst sie also
mit jedem ZIP-Programm ansehen, aber nur Dimo sollte sie schreiben.

Ein Projekt, das eine ältere Dimo-Version gespeichert hat, lässt sich öffnen, und ein Hinweis sagt,
dass es umgewandelt wurde; beim Speichern wird das aktuelle Format geschrieben. Ein Projekt von
einer neueren Dimo-Version wird mit einer Meldung abgelehnt, damit nichts verloren geht;
aktualisiere Dimo, um es zu öffnen.

## Automatische Sicherung und Wiederherstellung

Jede Änderung wird sofort, spätestens aber alle 30 Sekunden, in ein Sicherungsjournal
geschrieben. Die Werkzeugleiste zeigt den Zustand: "Gespeichert", "Noch nicht gespeichert" (ein neues Projekt
ohne Änderungen), "Ungespeicherte Änderungen, automatisch gesichert" oder eine Warnung, wenn das
Journal nicht geschrieben werden konnte. Neben dem Dateinamen markiert ein Sternchen Änderungen,
die noch nicht in der Projektdatei stehen.

- Ein gespeichertes Projekt hat sein Journal neben der Datei: `part.dimo.journal`. Beim Speichern
  wird es in die Projektdatei übernommen und gelöscht.
- Ein noch nie gespeichertes Projekt hat sein Journal im Datenordner von Dimo (unter macOS zum
  Beispiel `~/Library/Application Support/io.github.nc-pb.dimo/autosave`).

Endet Dimo oder der Computer unerwartet, gehen höchstens die letzten 30 Sekunden verloren:

- Beim erneuten Öffnen des gespeicherten Projekts werden die Änderungen aus dem Journal
  wiederhergestellt.
- Ein noch nie gespeichertes Projekt wird beim nächsten Start von Dimo wiederhergestellt.

Ein Hinweis über der Zeichnung sagt, was wiederhergestellt wurde. Speichere das Projekt, um die
Änderungen zu behalten. Wählst du beim Schließen "Nicht speichern", wird das Journal gelöscht.

## Ein Fenster pro Projekt

Solange ein Projekt offen ist, legt Dimo daneben eine Sperrdatei an (`part.dimo.lock`). Ein
zweites Dimo-Fenster, das dasselbe Projekt öffnen will, erhält eine Meldung mit dem Benutzer, der
es geöffnet hat. Die Sperre endet, wenn Dimo schließt, auch nach einem Absturz; eine
liegengebliebene Sperrdatei blockiert also nie. Die Sperre gilt nur zwischen Dimo-Fenstern;
andere Programme können die Datei weiterhin ändern.
